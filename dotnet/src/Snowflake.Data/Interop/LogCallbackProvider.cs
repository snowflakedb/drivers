namespace Snowflake.Data.Interop;

///  TODO this is PoC, will be subject to refactoring in the future
internal static unsafe class LogCallbackProvider
{
    private static readonly PtrToStringUtf8Delegate PtrToStringUtf8;
    private delegate string PtrToStringUtf8Delegate(byte* ptr);

    static LogCallbackProvider()
    {
        PtrToStringUtf8 = NativeInteropProvider.Interop.PtrToStringUtf8;
    }

    internal static uint LogCallback(uint level, byte* message, byte* filename, uint line, byte* function)
    {
        try
        {
            var msg = PtrToStringUtf8(message);
            var file = PtrToStringUtf8(filename);
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
