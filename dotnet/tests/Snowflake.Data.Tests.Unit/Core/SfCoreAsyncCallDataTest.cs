using System.Runtime.CompilerServices;
using Snowflake.Data.Interop;
using Snowflake.Data.Interop.Callback;

namespace Snowflake.Data.Tests.Core;

[Trait("Category", "Unit")]
public sealed class SfCoreAsyncCallDataTest
{
    private readonly TaskCompletionSource<SfCoreResponseData> _completion = new(TaskCreationOptions.RunContinuationsAsynchronously);

    [SnowflakeFact(RetriesCount = RetriesCount.NoRetries)]
    public void DisposingDisposesTheCancelRegistration()
    {
        using var source = new CancellationTokenSource();
        var cancelCalls = 0;
        var callData = Pinned();
        callData.SetCancelRegistration(source.Token.Register(() => cancelCalls++));

        callData.Dispose();
        source.Cancel();

        Assert.Equal(0, cancelCalls);
    }

    [SnowflakeFact(RetriesCount = RetriesCount.NoRetries)]
    public void ARegistrationSetAfterDisposeIsDisposedAtOnce()
    {
        using var source = new CancellationTokenSource();
        var cancelCalls = 0;
        var callData = Pinned();
        callData.Dispose();

        callData.SetCancelRegistration(source.Token.Register(() => cancelCalls++));
        source.Cancel();

        Assert.Equal(0, cancelCalls);
    }

    [SnowflakeFact(RetriesCount = RetriesCount.NoRetries)]
    public void ARegistrationSetBeforeDisposeStaysActive()
    {
        using var source = new CancellationTokenSource();
        var cancelCalls = 0;
        using var callData = Pinned();

        callData.SetCancelRegistration(source.Token.Register(() => cancelCalls++));
        source.Cancel();

        Assert.Equal(1, cancelCalls);
    }

    [SnowflakeFact(RetriesCount = RetriesCount.NoRetries)]
    public void DisposingTwiceDoesNotThrow()
    {
        var callData = Pinned();
        callData.Dispose();

        var exception = Record.Exception(callData.Dispose);

        Assert.Null(exception);
    }

    [SnowflakeFact(RetriesCount = RetriesCount.NoRetries)]
    public void PinnedInstanceStaysAliveUntilDisposed()
    {
        var reference = PinnedWeakReference(dispose: false);

        CollectGarbage();

        Assert.True(reference.IsAlive);
    }

    [SnowflakeFact(RetriesCount = RetriesCount.NoRetries)]
    public void DisposeReleasesPinsSoInstanceCanBeCollected()
    {
        var reference = PinnedWeakReference(dispose: true);

        CollectGarbage();

        Assert.False(reference.IsAlive);
    }

    [MethodImpl(MethodImplOptions.NoInlining)]
    private WeakReference PinnedWeakReference(bool dispose)
    {
        var callData = Pinned();
        if (dispose)
            callData.Dispose();
        return new WeakReference(callData);
    }

    private static void CollectGarbage()
    {
        GC.Collect();
        GC.WaitForPendingFinalizers();
        GC.Collect();
    }

    private unsafe SfCoreAsyncCallData Pinned()
    {
        var callData = new SfCoreAsyncCallData(_completion, [1, 2, 3]);
        callData.SelfPin();
        return callData;
    }
}
