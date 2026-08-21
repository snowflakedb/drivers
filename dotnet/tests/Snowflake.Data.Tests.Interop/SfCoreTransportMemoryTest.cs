using Snowflake.Data.Interop;

namespace Snowflake.Data.Tests.Interop;

/// <summary>
/// Memory and leak detection tests for the native interop stack.
/// Verifies GCHandle pinning survives compacting GC, and that repeated calls
/// produce bounded allocations with no native buffer leaks.
/// </summary>
[Collection("Interop")]
[Trait("Category", "Interop")]
[Trait("Category", "Memory")]
public sealed class SfCoreTransportMemoryTest
{
    private static readonly SfCoreTransport Transport = SfCoreTransport.Instance;
    private static readonly byte[] HelloBytes = "hello, stub!"u8.ToArray();

    [SnowflakeTheory]
    [InlineData(3, SkipCondition.None)]
    [InlineData(6, SkipCondition.None)]
    [InlineData(10, SkipCondition.SkipOnMacOS)] // ~1k — cumulative threads exceed macOS limit
    [InlineData(13, SkipCondition.SkipOnMacOS)] // ~8k
    [InlineData(15, SkipCondition.SkipOnCI)]    // ~32k
    [InlineData(17, SkipCondition.SkipOnCI)]    // ~131k
    public async Task ConcurrentAsyncCalls_SurviveAggressiveGcMidFlightAsync(int dopExp, SkipCondition skipCondition)
    {
        Skip.For(skipCondition, "Exceeds CI thread OS limit");

        var dop = 1 << dopExp;
        var tasks = Enumerable
            .Range(0, dop)
            .Select(i => Transport.HandleMessageAsync(
                "svc", "delay_ms:200",
                BitConverter.GetBytes(i),
                CancellationToken.None))
            .ToArray();

        GC.Collect(2, GCCollectionMode.Forced, blocking: true, compacting: true);
        GC.WaitForPendingFinalizers();
        GC.Collect(2, GCCollectionMode.Forced, blocking: true, compacting: true);

        var responses = await Task.WhenAll(tasks).WaitAsync(TimeSpan.FromSeconds(60), TestContext.Current.CancellationToken).ConfigureAwait(false);

        for (var i = 0; i < dop; i++)
        {
            Assert.Equal(0, responses[i].Code);
            Assert.Equal(BitConverter.GetBytes(i), responses[i].ResponseBytes.ToArray());
        }
    }

    [SnowflakeTheory(RetriesCount = RetriesCount.Thrice)]
    [InlineData(10, SkipCondition.None)]
    [InlineData(13, SkipCondition.None)]  // ~8k — sequential, no thread pressure
    [InlineData(15, SkipCondition.SkipOnCI)] // ~32k
    [InlineData(17, SkipCondition.SkipOnCI)] // ~131k
    public async Task AsyncCalls_Repeated_NoNativeBufferLeaksAsync(int dopExp, SkipCondition skipCondition)
    {
        Skip.For(skipCondition, "Exceeds CI time budget");

        long dop = 1 << dopExp;

        for (var i = 0; i < 3; i++)
            await Transport.HandleMessageAsync("svc", "echo_increment", HelloBytes, CancellationToken.None).ConfigureAwait(false);

        var allocsBefore = GC.GetTotalMemory(false);
        var memBefore = Environment.WorkingSet;
        var leaksBefore = StubNativeMethods.sf_stub_leaked_alloc_count();

        for (var i = 0; i < dop; i++)
            await Transport.HandleMessageAsync("svc", "echo_increment", HelloBytes, CancellationToken.None).ConfigureAwait(false);

        GC.Collect(2, GCCollectionMode.Forced, blocking: true, compacting: true);
        GC.WaitForPendingFinalizers();
        GC.Collect(2, GCCollectionMode.Forced, blocking: true, compacting: true);

        var leaksAfter = StubNativeMethods.sf_stub_leaked_alloc_count();
        var allocsAfter = GC.GetTotalMemory(false);

        Assert.Equal(leaksBefore, leaksAfter);

        var allocPerCall = (double)(allocsAfter - allocsBefore) / dop;
        Assert.True(allocPerCall < 2048, $"Managed allocations per async call too high: {allocPerCall:F0} bytes ");

        var memGrowth = Environment.WorkingSet - memBefore;
        GC.Collect(2, GCCollectionMode.Forced, blocking: true, compacting: true);
        GC.WaitForPendingFinalizers();
        GC.Collect(2, GCCollectionMode.Forced, blocking: true, compacting: true);
        var memGrowthPostGx = Environment.WorkingSet - memBefore;
        Assert.True(memGrowth < 8 * dop * 1024, $"Working set grew by {memGrowth / 1024} KB over {dop} async calls");
        Assert.True(memGrowthPostGx < 10 * 1024 * 1024, $"Working set post GC grew by {memGrowth / 1024} KB over {dop} async calls");
    }
}
