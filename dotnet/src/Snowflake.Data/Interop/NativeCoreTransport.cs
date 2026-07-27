using Snowflake.Data.Proto;

namespace Snowflake.Data.Interop;

/// <summary>
/// P/Invoke-based transport that calls sf_core's C FFI directly.
/// Delegates to INativeInterop which has platform-specific implementations:
/// - NativeInteropModern (NET7_0_OR_GREATER): LibraryImport + function pointers
/// - NativeInteropLegacy (NETFRAMEWORK): DllImport + delegates
/// </summary>
///  TODO this is PoC, will be subject to refactoring in the future
internal sealed class NativeCoreTransport : ICoreTransport
{
    public static readonly NativeCoreTransport Instance = new();

    private static readonly Lazy<bool> Initialized = new(() =>
    {
        NativeInteropProvider.Interop.Initialize();
        return true;
    });

    private NativeCoreTransport()
    {
    }

    public TransportResponse HandleMessage(string service, string method, byte[] request)
    {
        EnsureInitialized();

        unsafe
        {
            byte* responsePtr = null;
            nuint responseLen = 0;

            fixed (byte* requestPtr = request)
            {
                var code = (int)NativeInteropProvider.Interop.CallProto(
                    service,
                    method,
                    requestPtr,
                    (nuint)request.Length,
                    &responsePtr,
                    &responseLen);

                var responseBytes = new byte[responseLen];
                if (responsePtr == null)
                    return new TransportResponse(code, responseBytes);

                new ReadOnlySpan<byte>(responsePtr, (int)responseLen).CopyTo(responseBytes);
                NativeInteropProvider.Interop.FreeBuffer(responsePtr, responseLen);

                return new TransportResponse(code, responseBytes);
            }
        }
    }

    public Task<TransportResponse> HandleMessageAsync(string service, string method, byte[] request, CancellationToken cancellationToken) =>
        throw new NotImplementedException("TODO");

    private static void EnsureInitialized() => _ = Initialized.Value;
}
