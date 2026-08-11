using System.Buffers;
using Snowflake.Data.Interop.Callback;
using Snowflake.Data.Proto;

namespace Snowflake.Data.Interop;

/// <summary>
/// P/Invoke-based transport that calls sf_core's C FFI directly.
/// Delegates to component which has platform-specific implementations:
/// - NativeInteropModern (NET7_0_OR_GREATER): LibraryImport + function pointers
/// - NativeInteropLegacy (NETFRAMEWORK): DllImport + delegates
/// </summary>
internal sealed class SfCoreTransport : ICoreTransport
{
    internal static readonly SfCoreTransport Instance = new();

    private static readonly Lazy<ISfCoreInterop> SFCore = new(() =>
    {
        SfCoreInteropProvider.SfCore.Initialize();
        return SfCoreInteropProvider.SfCore;
    });

    private SfCoreTransport()
    {
    }

    public TransportResponse HandleMessage(string service, string method, byte[] request)
    {
        EnsureInitialized();

        var response = CallProto(service, method, request);
        return new TransportResponse(response.Code, response.Response, response.Buffer);
    }

    public async Task<TransportResponse> HandleMessageAsync(string service, string method, byte[] request, CancellationToken cancellationToken)
    {
        EnsureInitialized();

        var result = await CallProtoAsync(service, method, request, cancellationToken).ConfigureAwait(false);
        return new TransportResponse(result.Code, result.Response, result.Buffer);
    }

    private static unsafe SfCoreResponseData CallProto(string api, string method, byte[] request)
    {
        byte* responsePtr = null;
        nuint responseLen = 0;
        var requestLength = (nuint)request.Length;

        nuint resultCode;
        fixed (byte* requestPtr = request)
        {
            resultCode = SFCore.Value.CallProto(api, method, requestPtr, requestLength, &responsePtr, &responseLen);
        }

        if (responseLen > int.MaxValue)
            throw new NotSupportedException("TODO");

        var responseLen32 = (int)responseLen;
        var responseBytes = ArrayPool<byte>.Shared.Rent(responseLen32);
        try
        {
            new ReadOnlySpan<byte>(responsePtr, responseLen32).CopyTo(responseBytes);
            var responseSegment = new ArraySegment<byte>(responseBytes, 0, responseLen32);

            var result = new SfCoreResponseData((int)resultCode, responseSegment, responseBytes);

            SFCore.Value.CallFreeBuffer(responsePtr, responseLen);
            return result;
        }
        catch
        {
            ArrayPool<byte>.Shared.Return(responseBytes);
            throw;
        }
    }

    private static unsafe Task<SfCoreResponseData> CallProtoAsync(string api, string method, byte[] request, CancellationToken cancelToken)
    {
        if (cancelToken.IsCancellationRequested)
            throw new TaskCanceledException("Operation cancelled before calling the sf_core.");

        var requestLength = (nuint)request.Length;
        var tcs = new TaskCompletionSource<SfCoreResponseData>(TaskCreationOptions.RunContinuationsAsynchronously);
        var callData = new SfCoreAsyncCallData(tcs, request);
        try
        {
            var callDataRef = callData.SelfPin();
            var asyncHandle = SFCore.Value.CallProtoAsync(api, method, callData.GetRequestPtr(), requestLength, callDataRef);
            var registration = cancelToken.Register(() =>
            {
                SFCore.Value.CallProtoCancel(asyncHandle);
            });

            callData.SetCancelRegistration(registration);
            return tcs.Task;
        }
        catch
        {
            callData.Dispose();
            throw;
        }
    }

    private static void EnsureInitialized() => _ = SFCore.Value;
}
