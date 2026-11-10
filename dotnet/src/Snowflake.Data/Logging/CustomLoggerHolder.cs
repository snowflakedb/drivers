using Microsoft.Extensions.Logging;
using Microsoft.Extensions.Logging.Abstractions;

namespace Snowflake.Data.Logging;

internal static class CustomLoggerHolder
{
    private static ILogger s_logger = NullLogger.Instance;

    internal static ILogger Current => Volatile.Read(ref s_logger)!;

    internal static void Set(ILogger logger) => Volatile.Write(ref s_logger, logger);

    internal static void Reset() => Volatile.Write(ref s_logger, NullLogger.Instance);
}
