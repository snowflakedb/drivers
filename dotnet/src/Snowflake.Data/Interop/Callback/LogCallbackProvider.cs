using Microsoft.Extensions.Logging;
using Snowflake.Data.Interop;
using Snowflake.Data.Logging;

namespace Snowflake.Data.Interop.Callback;

internal static unsafe class LogCallbackProvider
{
    private static readonly PtrToStringUtf8Delegate PtrToStringUtf8 = SfCoreInteropProvider.StringHelper.PtrToStringUtf8;
    private delegate string PtrToStringUtf8Delegate(byte* ptr);

    internal static uint LogCallback(uint level, byte* message, byte* filename, uint line, byte* function, byte* loggerName)
    {
        try
        {
            var logLevel = ToLogLevel(level);
            var logger = CustomLoggerHolder.Current;
            if (!logger.IsEnabled(logLevel))
                return 0;

            var msg = PtrToStringUtf8(message);
            var loggerNameStr = PtrToStringUtf8(loggerName);
            var text = loggerNameStr.Length == 0
                ? $"[{PtrToStringUtf8(filename)}:{line}] {msg}"
                : msg;

            logger.Log(logLevel, default, text, null, static (s, _) => s);
        }
        catch
        {
            // Exceptions must not unwind across the native callback.
        }

        return 0;
    }

    private static LogLevel ToLogLevel(uint level) => level switch
    {
        0 => LogLevel.Error,
        1 => LogLevel.Warning,
        2 => LogLevel.Information,
        3 => LogLevel.Debug,
        _ => LogLevel.Trace,
    };
}
