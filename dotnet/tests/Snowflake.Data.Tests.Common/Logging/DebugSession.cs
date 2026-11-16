using System.Collections.Concurrent;
using Microsoft.Extensions.Logging;
using Microsoft.Extensions.Logging.Debug;

namespace Snowflake.Data.Tests.Logging;

public sealed class DebugSessionFactory : ILogSessionFactory
{
    public ICapturedLogSession Start(LogLevel minimum)
    {
        var factory = new LoggerFactory([new DebugLoggerProvider()], new LoggerFilterOptions { MinLevel = minimum });
        return new LoggerFactorySession(factory, factory.CreateLogger("Snowflake.Data"), new ConcurrentQueue<CapturedLogEvent>());
    }
}
