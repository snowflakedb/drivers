using System.Runtime.InteropServices;
using Snowflake.Data.Proto;

namespace Snowflake.Data;

/// <summary>
/// P/Invoke-based transport that calls sf_core's C FFI directly.
/// Lazily initializes sf_core on first use.
/// </summary>
internal sealed class NativeCoreTransport : ICoreTransport
{
    public static readonly NativeCoreTransport Instance = new();

    private static readonly Lazy<bool> Initialized = new(InitializeCore);

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
                var code = (int)NativeMethods.sf_core_api_call_proto(
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
                NativeMethods.sf_core_free_buffer(responsePtr, responseLen);

                return new TransportResponse(code, responseBytes);
            }
        }
    }

    public Task<TransportResponse> HandleMessageAsync(string service, string method, byte[] request, CancellationToken cancellationToken) =>
        throw new NotImplementedException("TODO");

    private static void EnsureInitialized() => _ = Initialized.Value;

    private static bool InitializeCore()
    {
        NativeLibraryLoader.Register();

        uint result;
        unsafe
        {
            result = NativeMethods.sf_core_init(&LogCallback);
        }

        if (result != 0)
            throw new InvalidOperationException($"sf_core_init failed with code {result}. Check stderr for details.");

        return true;
    }

    [UnmanagedCallersOnly(CallConvs = [typeof(System.Runtime.CompilerServices.CallConvCdecl)])]
    private static unsafe uint LogCallback(uint level, byte* message, byte* filename, uint line, byte* function)
    {
        // TODO Minimal logging: write to stderr. Replace with ILogger integration later.
        try
        {
            var msg = Marshal.PtrToStringUTF8((nint)message) ?? string.Empty;
            var file = Marshal.PtrToStringUTF8((nint)filename) ?? string.Empty;
            Console.Error.WriteLine($"[sf_core:{LevelName(level)}] {file}:{line} {msg}");
        }
        catch
        {
            // Never let exceptions propagate back across FFI boundary
        }

        return 0;
    }

    private static string LevelName(uint level) => level switch
    {
        0 => "ERROR",
        1 => "WARN",
        2 => "INFO",
        3 => "DEBUG",
        4 => "TRACE",
        _ => "UNKNOWN",
    };
}
