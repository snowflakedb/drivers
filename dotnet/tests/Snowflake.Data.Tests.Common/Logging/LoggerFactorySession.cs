using System.Collections.Concurrent;
using Microsoft.Extensions.Logging;

namespace Snowflake.Data.Tests.Logging;

internal sealed class LoggerFactorySession : ICapturedLogSession
{
    private readonly ILoggerFactory _factory;
    private readonly ConcurrentQueue<CapturedLogEvent> _events;

    public LoggerFactorySession(ILoggerFactory factory, ILogger logger, ConcurrentQueue<CapturedLogEvent> events)
    {
        _factory = factory;
        _events = events;
        Logger = logger;
    }

    public ILogger Logger { get; }

    public IReadOnlyList<CapturedLogEvent> GetEvents() => _events.ToArray();

    public void Dispose() => _factory.Dispose();
}
