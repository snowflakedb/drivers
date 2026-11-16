// The ZLogger package is referenced only for modern .NET, so this file is empty on .NET Framework.
#if !NETFRAMEWORK
using System.Collections.Concurrent;
using Microsoft.Extensions.Logging;
using ZLogger;

namespace Snowflake.Data.Tests.Logging;

public sealed class ZLoggerSessionFactory : ILogSessionFactory
{
    public ICapturedLogSession Start(LogLevel minimum)
    {
        var events = new ConcurrentQueue<CapturedLogEvent>();
        var factory = LoggerFactory.Create(builder =>
        {
            builder.SetMinimumLevel(minimum);
            builder.AddZLoggerLogProcessor(new CapturingProcessor(events));
        });
        return new LoggerFactorySession(factory, factory.CreateLogger("Snowflake.Data"), events);
    }

    private sealed class CapturingProcessor : IAsyncLogProcessor
    {
        private readonly ConcurrentQueue<CapturedLogEvent> _events;

        public CapturingProcessor(ConcurrentQueue<CapturedLogEvent> events)
        {
            _events = events;
        }

        public void Post(IZLoggerEntry log)
        {
            _events.Enqueue(new CapturedLogEvent(log.LogInfo.LogLevel, log.ToString()));
            log.Return();
        }

        public ValueTask DisposeAsync() => default;
    }
}
#endif
