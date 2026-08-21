using Snowflake.Data.Interop;
using Snowflake.Data.Proto;

namespace Snowflake.Data.Tests.Interop;

/// <summary>
/// Cancellation tests for the native interop stack.
/// Verifies correct TCS transitions, race safety across all timing windows,
/// and bounded memory growth under mass cancellation.
/// </summary>
[Collection("Interop")]
[Trait("Category", "Interop")]
[Trait("Category", "Cancellation")]
public sealed class SfCoreTransportCancellationTest
{
    private static readonly SfCoreTransport Transport = SfCoreTransport.Instance;
    private static readonly byte[] HelloBytes = "hello, stub!"u8.ToArray();

    [SnowflakeFact]
    public async Task AsyncCall_Cancellation_ReturnsWithCancellationStatusCodeAsync()
    {
        using var cts = new CancellationTokenSource();

        // "block" makes the stub park on a condvar until sf_core_api_cancel signals it.
        var task = Transport.HandleMessageAsync("svc", "block", HelloBytes, cts.Token);

        // Give the stub thread time to park before we cancel.
        await Task.Delay(50, TestContext.Current.CancellationToken).ConfigureAwait(false);
        cts.Cancel();

        // The stub fires the callback with status=2 (cancellation).
        // The task completes with Code=2 indicating the operation was cancelled.
        var resp = await task.WaitAsync(TimeSpan.FromSeconds(5), TestContext.Current.CancellationToken).ConfigureAwait(false);
        Assert.Equal(2, resp.Code);
    }

    [SnowflakeTheory]
    [InlineData(10, SkipCondition.SkipOnMacOS)] // ~1k — cumulative threads exceed macOS limit
    [InlineData(13, SkipCondition.SkipOnMacOS)] // ~8k
    [InlineData(15, SkipCondition.SkipOnCI)]
    public async Task CancellationRace_ConcurrentCancels_NoCrashesOrLeaksAsync(int dopExp, SkipCondition skipCondition)
    {
        Skip.For(skipCondition, "This exceeds thread OS limit");

        var count = 1 << dopExp;
        var leaksBefore = StubNativeMethods.sf_stub_leaked_alloc_count();

        // Use very short delays (1-5ms) so that at high concurrency we statistically
        // hit all three cancellation windows:
        // 1. Cancel arrives before callback fires (in-flight)
        // 2. Cancel arrives during callback execution (race)
        // 3. Cancel arrives after callback completes (stale handle)
        var random = new Random(42); // deterministic seed for reproducibility
        var tasks = new Task<TransportResponse>[count];
        var ctsSources = new CancellationTokenSource[count];

        for (var i = 0; i < count; i++)
        {
            ctsSources[i] = new CancellationTokenSource();
            var delayMs = random.Next(1, 6); // 1-5ms
            tasks[i] = Transport.HandleMessageAsync(
                "svc", $"delay_ms:{delayMs}",
                BitConverter.GetBytes(i),
                ctsSources[i].Token);
        }

        // Cancel all at once — some will race the callback, some will arrive late.
        for (var i = 0; i < count; i++)
            ctsSources[i].Cancel();

        // Wait for all tasks to settle. With the current contract, cancelled tasks
        // complete normally with Code=2 (not as TaskCanceledException), and tasks that
        // completed before cancel arrives return Code=0 with the echoed payload.
        var responses = await Task.WhenAll(tasks)
            .WaitAsync(TimeSpan.FromSeconds(30), TestContext.Current.CancellationToken).ConfigureAwait(false);

        // Every task must have settled — no hangs. Code is either 0 (completed) or 2 (cancelled).
        Assert.Equal(count, responses.Length);
        Assert.All(responses, r =>
            Assert.True(r.Code is 0 or 2,
                $"Unexpected response code: {r.Code}"));

        // Cleanup
        for (var i = 0; i < count; i++)
            ctsSources[i].Dispose();

        // No native leaks.
        GC.Collect(2, GCCollectionMode.Forced, blocking: true, compacting: true);
        GC.WaitForPendingFinalizers();
        var leaksAfter = StubNativeMethods.sf_stub_leaked_alloc_count();
        Assert.Equal(leaksBefore, leaksAfter);
    }

    [SnowflakeTheory(RetriesCount = RetriesCount.Thrice)]
    [InlineData(8, SkipCondition.None)]
    [InlineData(10, SkipCondition.SkipOnMacOS)]
    [InlineData(12, SkipCondition.SkipOnMacOS)] // ~4k (half blocked = ~2k parked threads)
    public async Task MassCancellation_MixedCancelAndComplete_BoundedMemoryGrowthAsync(int dopExp, SkipCondition skipCondition)
    {
        Skip.For(skipCondition, "Exceeds macOS CI thread limit");

        var count = 1 << dopExp;

        // Warm up to exclude JIT / type-init allocations from the baseline.
        for (var i = 0; i < 10; i++)
            await Transport.HandleMessageAsync("svc", "echo_increment", HelloBytes, CancellationToken.None).ConfigureAwait(false);

        GC.Collect(2, GCCollectionMode.Forced, blocking: true, compacting: true);
        GC.WaitForPendingFinalizers();

        var allocsBefore = GC.GetTotalMemory(false);
        var memBefore = Environment.WorkingSet;
        var leaksBefore = StubNativeMethods.sf_stub_leaked_alloc_count();

        // 50% use "block" + immediate cancel (cancellation path)
        // 50% use "echo_increment" with no cancel (normal completion path)
        var tasks = new Task[count];
        var ctsSources = new CancellationTokenSource[count / 2];

        for (var i = 0; i < count; i++)
        {
            if (i % 2 == 0)
            {
                // Cancelled path: "block" then cancel immediately
                var cts = new CancellationTokenSource();
                ctsSources[i / 2] = cts;
                tasks[i] = Transport.HandleMessageAsync("svc", "block", BitConverter.GetBytes(i), cts.Token);
            }
            else
            {
                // Normal completion path
                tasks[i] = Transport.HandleMessageAsync("svc", "echo_increment", BitConverter.GetBytes(i), CancellationToken.None);
            }
        }

        // Cancel all the "block" tasks.
        foreach (var t in ctsSources)
            t.Cancel();

        // Wait for everything to settle. Cancelled tasks complete with Code=2,
        // normal tasks complete with Code=0 — neither throws.
        await Task.WhenAll(tasks)
            .WaitAsync(TimeSpan.FromSeconds(30), TestContext.Current.CancellationToken).ConfigureAwait(false);

        // Cleanup CTS
        foreach (var t in ctsSources)
            t.Dispose();

        GC.Collect(2, GCCollectionMode.Forced, blocking: true, compacting: true);
        GC.WaitForPendingFinalizers();
        GC.Collect(2, GCCollectionMode.Forced, blocking: true, compacting: true);

        var leaksAfter = StubNativeMethods.sf_stub_leaked_alloc_count();
        var allocsAfter = GC.GetTotalMemory(false);
        var memAfter = Environment.WorkingSet;

        // No native leaks regardless of cancellation vs. completion path.
        Assert.Equal(leaksBefore, leaksAfter);

        // Managed allocations per call should be bounded.
        var allocPerCall = (double)(allocsAfter - allocsBefore) / count;
        Assert.True(allocPerCall < 4096,
            $"Managed allocations per call too high: {allocPerCall:F0} bytes (over {count} mixed calls)");

        // Working set growth must be bounded — proves no GCHandle / pinned buffer leaks.
        var memGrowth = memAfter - memBefore;
        Assert.True(memGrowth < 16L * count * 1024,
            $"Working set grew by {memGrowth / 1024} KB over {count} mixed calls");
    }
}
