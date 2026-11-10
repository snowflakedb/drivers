using Microsoft.Extensions.Logging;
using Snowflake.Data.Logging;

namespace Snowflake.Data;

public sealed class SnowflakeDbLoggerConfig
{
    /// <summary>
    /// Restores the no-op logger so core events are no longer delivered to a previously registered logger.
    /// </summary>
    public static void ResetCustomLogger() => CustomLoggerHolder.Reset();

    /// <summary>
    /// Registers a custom <see cref="ILogger"/> that receives native core log events.
    /// </summary>
    /// <param name="customLogger">The logger instance. Must not be null.</param>
    /// <exception cref="ArgumentNullException">Thrown when <paramref name="customLogger"/> is null.</exception>
    public static void SetCustomLogger(ILogger customLogger)
    {
        if (customLogger is null)
            throw new ArgumentNullException(nameof(customLogger));
        CustomLoggerHolder.Set(customLogger);
    }
}
