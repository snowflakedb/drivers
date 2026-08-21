using Snowflake.Data.Interop;

namespace Snowflake.Data.Tests.Interop;

/// <summary>
/// Basic behavioral tests for the async native interop stack.
/// Verifies echo, threading model, error propagation, and pre-dispatch cancellation.
/// </summary>
[Collection("Interop")]
[Trait("Category", "Interop")]
[Trait("Category", "Unit")]
public sealed class SfCoreTransportAsyncTest
{
    private static readonly SfCoreTransport Transport = SfCoreTransport.Instance;
    private static readonly byte[] HelloBytes = "hello, stub!"u8.ToArray();
    private static readonly byte[] HelloBytesIncremented = HelloBytes.Select(b => (byte)(b + 1)).ToArray();

    [SnowflakeFact]
    public async Task AsyncCall_EchosRequestBytesAndStatusZeroAsync()
    {
        var resp = await Transport.HandleMessageAsync("svc", "echo_increment", HelloBytes, CancellationToken.None).ConfigureAwait(false);

        Assert.Equal(0, resp.Code);
        Assert.Equal(HelloBytesIncremented, resp.ResponseBytes.ToArray());
    }

    [SnowflakeFact]
    public async Task AsyncCall_CallbackFiresAsynchronouslyAsync()
    {
        // Proves the native callback completes the task without the caller driving it:
        // we launch the call, do NOT await immediately, and verify completion arrives
        // via the TCS set by the native callback on its own thread.
        var tcs = new TaskCompletionSource<int>();
        var task = Transport.HandleMessageAsync("svc", "delay_ms:50", HelloBytes, CancellationToken.None);
        _ = task.ContinueWith(_ => tcs.TrySetResult(0), TaskScheduler.Default);

        // If the callback never fires, this times out — proving the async mechanism works.
        await tcs.Task.WaitAsync(TimeSpan.FromSeconds(10), TestContext.Current.CancellationToken).ConfigureAwait(false);

        var resp = await task.ConfigureAwait(false);
        Assert.Equal(0, resp.Code);
    }

    [SnowflakeFact]
    public async Task AsyncCall_ErrorCode_PropagatesNonZeroCodeAsync()
    {
        var resp = await Transport.HandleMessageAsync("svc", "error:7", HelloBytes, CancellationToken.None).ConfigureAwait(false);

        Assert.Equal(7, resp.Code);
        Assert.Equal(HelloBytes, resp.ResponseBytes.ToArray());
    }

    [SnowflakeFact]
    public async Task AsyncCall_CancelBeforeDispatched_ThrowsImmediatelyAsync()
    {
        using var cts = new CancellationTokenSource();
        cts.Cancel();

        await Assert.ThrowsAsync<OperationCanceledException>(
            () => Transport.HandleMessageAsync("svc", "block", HelloBytes, cts.Token)).ConfigureAwait(false);
    }
}
