using Snowflake.Data.Interop;

namespace Snowflake.Data.Tests.Interop;

/// <summary>
/// Synchronous P/Invoke tests for the native interop stack.
/// Verifies echo, error propagation, and native buffer lifecycle.
/// </summary>
[Collection("Interop")]
[Trait("Category", "Interop")]
[Trait("Category", "Unit")]
public sealed class SfCoreTransportTest
{
    private static readonly SfCoreTransport Transport = SfCoreTransport.Instance;
    private static readonly byte[] HelloBytes = "hello, stub!"u8.ToArray();
    private static readonly byte[] HelloBytesIncremented = HelloBytes.Select(b => (byte)(b + 1)).ToArray();
    private static readonly byte[] EmptyBytes = [];

    [SnowflakeFact]
    public void SyncCall_EchosRequestBytesAndStatusZero()
    {
        var resp = Transport.HandleMessage("svc", "echo_increment", HelloBytes);

        Assert.Equal(0, resp.Code);
        Assert.Equal(HelloBytesIncremented, resp.ResponseBytes.ToArray());
    }

    [SnowflakeFact]
    public void SyncCall_EmptyRequest_ReturnsEmptyResponse()
    {
        var resp = Transport.HandleMessage("svc", "echo_increment", EmptyBytes);

        Assert.Equal(0, resp.Code);
        Assert.Equal(0, resp.ResponseBytes.Count);
    }

    [SnowflakeFact]
    public void SyncCall_ErrorCode_PropagatesNonZeroCode()
    {
        var resp = Transport.HandleMessage("svc", "error:42", HelloBytes);

        Assert.Equal(42, resp.Code);
        Assert.Equal(HelloBytes, resp.ResponseBytes.ToArray());
    }

    [SnowflakeTheory]
    [InlineData(10)]
    [InlineData(13)] // ~8k
    [InlineData(17)] // ~131k — sync calls don't spawn threads
    public void SyncCalls_Repeated_NoNativeBufferLeaks(int dopExp)
    {
        var dop = 1 << dopExp;

        // Warm up to exclude JIT / type-init allocations from the baseline.
        for (var i = 0; i < 3; i++)
            Transport.HandleMessage("svc", "echo_increment", HelloBytes);

        var allocsBefore = GC.GetTotalMemory(false);
        var memBefore = Environment.WorkingSet;
        var leaksBefore = StubNativeMethods.sf_stub_leaked_alloc_count();

        for (var i = 0; i < dop; i++)
            Transport.HandleMessage("svc", "echo_increment", HelloBytes);

        GC.Collect(2, GCCollectionMode.Forced, blocking: true, compacting: true);
        GC.WaitForPendingFinalizers();
        GC.Collect(2, GCCollectionMode.Forced, blocking: true, compacting: true);

        var leaksAfter = StubNativeMethods.sf_stub_leaked_alloc_count();
        var allocsAfter = GC.GetTotalMemory(false);
        var memAfter = Environment.WorkingSet;

        // Native: every allocated buffer must have been freed.
        Assert.Equal(leaksBefore, leaksAfter);

        // Managed: per-iteration allocation should be bounded (no unbounded growth).
        var allocPerCall = (double)(allocsAfter - allocsBefore) / dop;
        Assert.True(allocPerCall < 512, $"Managed allocations per sync call too high: {allocPerCall:F0} bytes ");

        // Working set
        var memGrowth = memAfter - memBefore;
        Assert.True(memGrowth < 2 * 1024 * dop, $"Working set grew by {memGrowth / 1024} KB over {dop} sync calls ");
    }
}
