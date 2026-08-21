using System.Buffers;
using Snowflake.Data.Interop;
using Snowflake.Data.Proto;

namespace Snowflake.Data.Tests.Interop;

/// <summary>
/// Resilience tests for the native interop stack.
/// Proves the FFI boundary survives malformed responses, ArrayPool exhaustion,
/// and aggressive GC without corrupting driver state.
/// </summary>
[Collection("Interop")]
[Trait("Category", "Interop")]
[Trait("Category", "Resilience")]
public sealed class SfCoreTransportResilienceTest
{
    private static readonly SfCoreTransport Transport = SfCoreTransport.Instance;
    private static readonly byte[] HelloBytes = "hello, stub!"u8.ToArray();

    [SnowflakeTheory]
    [InlineData(10, SkipCondition.SkipOnMacOS)] // ~1k — cumulative threads exceed macOS limit
    [InlineData(13, SkipCondition.SkipOnMacOS)] // ~8k
    [InlineData(15, SkipCondition.SkipOnMacOS)] // ~32k
    public async Task MalformedResponse_NullPtr_DriverRemainsUsableAfterStormAsync(int dopExp, SkipCondition skipCondition)
    {
        Skip.For(skipCondition, "Exceeds macOS CI thread limit");

        var count = 1 << dopExp;

        // Storm: fire thousands of calls that produce a null-ptr malformed response.
        // Each triggers the error branch in ProtoAsyncCallbackProvider (ptr==0).
        var stormTasks = Enumerable
            .Range(0, count)
            .Select(_ => Transport.HandleMessageAsync("svc", "null_ptr", HelloBytes, CancellationToken.None))
            .ToArray();

        // Every call should complete with an exception (not hang, not crash).
        var results = await Task.WhenAll(stormTasks.Select(task => CatchExceptionAsync(task, TestContext.Current.CancellationToken)))
            .WaitAsync(TimeSpan.FromSeconds(30), TestContext.Current.CancellationToken)
            .ConfigureAwait(false);

        Assert.All(results, r => Assert.Equal(TaskStatus.Faulted, r));

        // Prove driver is still usable: concurrent normal calls must succeed.
        await AssertDriverUsableAfterStormAsync(count, TestContext.Current.CancellationToken).ConfigureAwait(false);
    }

    [SnowflakeTheory]
    [InlineData(10, SkipCondition.SkipOnMacOS)] // ~1k — cumulative threads exceed macOS limit
    [InlineData(13, SkipCondition.SkipOnMacOS)] // ~8k
    [InlineData(15, SkipCondition.SkipOnMacOS)] // ~32k
    public async Task MalformedResponse_HugeLen_DriverRemainsUsableAfterStormAsync(int dopExp, SkipCondition skipCondition)
    {
        Skip.For(skipCondition, "Exceeds macOS CI thread limit");

        var count = 1 << dopExp;

        // Storm: fire thousands of calls with len=usize::MAX.
        // Triggers the len>int.MaxValue guard in ProtoAsyncCallbackProvider.
        var stormTasks = Enumerable
            .Range(0, count)
            .Select(_ => Transport.HandleMessageAsync("svc", "huge_len", HelloBytes, CancellationToken.None))
            .ToArray();

        var results = await Task.WhenAll(stormTasks.Select(task => CatchExceptionAsync(task, TestContext.Current.CancellationToken)))
            .WaitAsync(TimeSpan.FromSeconds(30), TestContext.Current.CancellationToken)
            .ConfigureAwait(false);

        Assert.All(results, r => Assert.Equal(TaskStatus.Faulted, r));

        await AssertDriverUsableAfterStormAsync(count, TestContext.Current.CancellationToken).ConfigureAwait(false);
    }

    [SnowflakeTheory]
    [InlineData(10, SkipCondition.SkipOnMacOS)] // ~1k — cumulative threads exceed macOS limit
    [InlineData(13, SkipCondition.SkipOnMacOS)] // ~8k
    [InlineData(15, SkipCondition.SkipOnCI)]    // ~32k
    public async Task AsyncCalls_SurviveArrayPoolExhaustionAndGcPressureAsync(int dopExp, SkipCondition skipCondition)
    {
        Skip.For(skipCondition, "Exceeds CI thread OS limit");

        var dop = 1 << dopExp;

        // Exhaust ArrayPool's shared buckets by renting large buffers without returning them.
        // This forces subsequent Rent calls inside the callback to allocate fresh arrays,
        // increasing GC pressure and LOH activity.
        const int exhaustionBufferSize = 64 * 1024; // 64KB — targets a specific pool bucket
        const int exhaustionCount = 64;
        var exhaustionBuffers = new byte[exhaustionCount][];
        for (var i = 0; i < exhaustionCount; i++)
            exhaustionBuffers[i] = ArrayPool<byte>.Shared.Rent(exhaustionBufferSize);

        try
        {
            // Fire concurrent async calls while the pool is exhausted.
            // Using "delay_ms:50" so they overlap with GC pressure below.
            var tasks = Enumerable
                .Range(0, dop)
                .Select(i => Transport.HandleMessageAsync(
                    "svc", "delay_ms:50",
                    BitConverter.GetBytes(i),
                    CancellationToken.None))
                .ToArray();

            // Force aggressive GC while calls are in flight and pool is stressed.
            GC.Collect(2, GCCollectionMode.Forced, blocking: true, compacting: true);
            GC.WaitForPendingFinalizers();
            GC.Collect(2, GCCollectionMode.Forced, blocking: true, compacting: true);

            var responses = await Task.WhenAll(tasks)
                .WaitAsync(TimeSpan.FromSeconds(60), TestContext.Current.CancellationToken)
                .ConfigureAwait(false);

            // Every call must complete correctly despite pool exhaustion + GC pressure.
            for (var i = 0; i < dop; i++)
            {
                Assert.Equal(0, responses[i].Code);
                Assert.Equal(BitConverter.GetBytes(i), responses[i].ResponseBytes.ToArray());
            }
        }
        finally
        {
            for (var i = 0; i < exhaustionCount; i++)
                ArrayPool<byte>.Shared.Return(exhaustionBuffers[i]);
        }
    }

    /// <summary>
    /// After a malformed response storm, proves the driver is still functional:
    /// fires concurrent normal calls, asserts correct results, checks leak counter.
    /// </summary>
    private static async Task AssertDriverUsableAfterStormAsync(int concurrency, CancellationToken cancelToken)
    {
        var verifyCount = Math.Min(concurrency, 1024);
        var leaksBefore = StubNativeMethods.sf_stub_leaked_alloc_count();

        var verifyTasks = Enumerable
            .Range(0, verifyCount)
            .Select(i => Transport.HandleMessageAsync(
                "svc", "echo_increment",
                BitConverter.GetBytes(i),
                CancellationToken.None))
            .ToArray();

        var responses = await Task.WhenAll(verifyTasks)
            .WaitAsync(TimeSpan.FromSeconds(10), cancelToken).ConfigureAwait(false);

        for (var i = 0; i < verifyCount; i++)
        {
            Assert.Equal(0, responses[i].Code);
            // echo_increment adds 1 to each byte (wrapping)
            var expected = BitConverter.GetBytes(i).Select(b => (byte)(b + 1)).ToArray();
            Assert.Equal(expected, responses[i].ResponseBytes.ToArray());
        }

        var leaksAfter = StubNativeMethods.sf_stub_leaked_alloc_count();
        Assert.Equal(leaksBefore, leaksAfter);
    }

    private static async Task<TaskStatus> CatchExceptionAsync(Task<TransportResponse> task, CancellationToken _)
    {
        try
        {
            await task.ConfigureAwait(false);
            return TaskStatus.RanToCompletion;
        }
        catch
        {
            return TaskStatus.Faulted;
        }
    }
}
