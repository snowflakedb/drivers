using System.Buffers;
using Snowflake.Data.Interop;
using Snowflake.Data.Interop.Callback;

namespace Snowflake.Data.Tests.Core;

[Trait("Category", "Unit")]
public sealed class ProtoAsyncCallbackProviderTest
{
    private static readonly byte[] Payload = [1, 2, 3, 4, 5];

    private readonly TaskCompletionSource<SfCoreResponseData> _completion = new(TaskCreationOptions.RunContinuationsAsynchronously);

    [SnowflakeFact(RetriesCount = RetriesCount.NoRetries)]
    public async Task ResponseCompletesTheCallWithItsStatusAndACopyOfItsBytesAsync()
    {
        var payload = Payload.ToArray();

        var callData = Respond(status: 7, payload, (nuint)payload.Length);
        payload[0] = 0xFF;

        var response = await _completion.Task.ConfigureAwait(false);
        try
        {
            Assert.Equal(7, response.Code);
            Assert.Equal(Payload, response.Response.ToArray());
            Assert.Same(response.Buffer, response.Response.Array);
            AssertDisposed(callData);
        }
        finally
        {
            ArrayPool<byte>.Shared.Return(response.Buffer);
        }
    }

    [SnowflakeFact(RetriesCount = RetriesCount.NoRetries)]
    public async Task EmptyResponseCompletesTheCallWithNoBytesAsync()
    {
        Respond(status: 0, Payload, len: 0);

        var response = await _completion.Task.ConfigureAwait(false);

        Assert.Equal(0, response.Code);
        Assert.Equal(0, response.Response.Count);
        ArrayPool<byte>.Shared.Return(response.Buffer);
    }

    [SnowflakeFact(RetriesCount = RetriesCount.NoRetries)]
    public async Task NullResponsePointerFaultsTheCallAsync()
    {
        var callData = Respond(status: 0, payload: null, len: 0);

        await Assert.ThrowsAsync<InvalidOperationException>(() => _completion.Task).ConfigureAwait(false);
        AssertDisposed(callData);
    }

    [SnowflakeFact(RetriesCount = RetriesCount.NoRetries)]
    public async Task LengthAboveIntMaxValueFaultsTheCallWithoutReadingTheResponseAsync()
    {
        var callData = Respond(status: 0, Payload, len: (nuint)int.MaxValue + 1);

        await Assert.ThrowsAsync<NotSupportedException>(() => _completion.Task).ConfigureAwait(false);
        AssertDisposed(callData);
    }

    [SnowflakeFact(RetriesCount = RetriesCount.NoRetries)]
    public async Task LengthOfIntMaxValuePassesTheLengthCheckAndFaultsTheCallWithTheBufferFailureAsync()
    {
        var callData = Respond(status: 0, Payload, len: int.MaxValue);

        await Assert.ThrowsAsync<OutOfMemoryException>(() => _completion.Task).ConfigureAwait(false);
        AssertDisposed(callData);
    }

    [SnowflakeFact(RetriesCount = RetriesCount.NoRetries)]
    public async Task ResponseForACallThatAlreadyCompletedLeavesItsResultAndDoesNotThrowAsync()
    {
        var earlier = new SfCoreResponseData(3, new ArraySegment<byte>([9]), [9]);
        _completion.SetResult(earlier);

        var callData = Respond(status: 0, Payload, (nuint)Payload.Length);

        Assert.Equal(earlier, await _completion.Task.ConfigureAwait(false));
        AssertDisposed(callData);
    }

    private static void AssertDisposed(SfCoreAsyncCallData callData)
    {
        using var source = new CancellationTokenSource();
        var cancelCalls = 0;

        callData.SetCancelRegistration(source.Token.Register(() => cancelCalls++));
        source.Cancel();

        Assert.Equal(0, cancelCalls);
    }

    private unsafe SfCoreAsyncCallData Respond(nuint status, byte[]? payload, nuint len)
    {
        var callData = new SfCoreAsyncCallData(_completion, []);
        var userData = callData.SelfPin();

        fixed (byte* response = payload)
        {
            ProtoAsyncCallbackProvider.ResponseCallback(userData, status, (nuint)response, len);
        }

        return callData;
    }
}
