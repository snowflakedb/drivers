using Microsoft.Extensions.Logging;

namespace Snowflake.Data.Tests.ArchitectureInvariantTests.Dummies;

// do not move/remove/rename/modify this.
public static class LoggingDummy
{
    public static void LogFailure(ILogger logger, Action action)
    {
        try
        {
            action();
        }
        catch (InvalidOperationException ex)
        {
            // Violation: exception message passed as a placeholder value
            logger.LogError("Operation failed: {Error}", ex.Message);

            // Violation: exception rendered with ToString()
            logger.LogWarning("Operation failed: {Details}", ex.ToString());

            // Violation: exception passed as a placeholder value
            logger.LogError("Operation failed: {Error}", ex);

            // Violation: exception interpolated into an argument
            logger.LogDebug("Operation failed: {Error}", $"{ex}");

            logger.LogError(ex, "Operation failed");
            logger.Log(LogLevel.Error, ex, "Operation failed");
            logger.LogError(ex, "Operation failed with {ExceptionType}", ex.GetType().Name);
        }
    }
}
