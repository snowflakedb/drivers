using System.Collections.Concurrent;
using System.Diagnostics.Tracing;
using Microsoft.Extensions.Logging;
using Microsoft.Extensions.Logging.EventSource;

namespace Snowflake.Data.Tests.Logging;

public sealed class EventSourceSession : ICapturedLogSession
{
    private const string EventSourceName = "Microsoft-Extensions-Logging";
    private readonly LoggingEventListener _listener;
    private readonly ILoggerFactory _factory;

    private EventSourceSession(LoggingEventListener listener, ILoggerFactory factory, ILogger logger)
    {
        _listener = listener;
        _factory = factory;
        Logger = logger;
    }

    public ILogger Logger { get; }

    public static EventSourceSession Start(LogLevel minimum)
    {
        var category = "Snowflake.Data." + Guid.NewGuid().ToString("N");
        var listener = new LoggingEventListener(category);
        var factory = LoggerFactory.Create(builder =>
        {
            builder.SetMinimumLevel(minimum);
            builder.AddEventSourceLogger();
        });
        return new EventSourceSession(listener, factory, factory.CreateLogger(category));
    }

    public IReadOnlyList<CapturedLogEvent> GetEvents() => _listener.Events.ToArray();

    public void Dispose()
    {
        _factory.Dispose();
        _listener.Dispose();
    }

    private sealed class LoggingEventListener : EventListener
    {
        private readonly string _loggerName;

        public LoggingEventListener(string loggerName)
        {
            _loggerName = loggerName;
        }

        public ConcurrentQueue<CapturedLogEvent> Events { get; } = new();

        protected override void OnEventSourceCreated(EventSource eventSource)
        {
            if (eventSource.Name != EventSourceName)
                return;

            EnableEvents(
                eventSource,
                EventLevel.LogAlways,
                LoggingEventSource.Keywords.FormattedMessage,
                new Dictionary<string, string?> { ["FilterSpecs"] = "UseAppFilters" });
        }

        protected override void OnEventWritten(EventWrittenEventArgs eventData)
        {
            if (eventData.EventName != "FormattedMessage" || eventData.Payload is null || eventData.PayloadNames is null)
                return;

            if (PayloadString(eventData, "LoggerName") != _loggerName)
                return;

            var message = PayloadString(eventData, "FormattedMessage");
            if (string.IsNullOrEmpty(message))
                return;

            Events.Enqueue(new CapturedLogEvent(PayloadLogLevel(eventData), message));
        }

        private static string PayloadString(EventWrittenEventArgs eventData, string name)
        {
            var index = eventData.PayloadNames!.IndexOf(name);
            return index < 0 ? string.Empty : eventData.Payload?[index]?.ToString() ?? string.Empty;
        }

        private static LogLevel PayloadLogLevel(EventWrittenEventArgs eventData)
        {
            var index = eventData.PayloadNames!.IndexOf("Level");
            if (index < 0 || eventData.Payload?[index] is not { } value)
                return LogLevel.None;
            return value switch
            {
                LogLevel logLevel => logLevel,
                int numeric => (LogLevel)numeric,
                _ => Enum.TryParse(value.ToString(), out LogLevel parsed) ? parsed : LogLevel.None,
            };
        }
    }
}

public sealed class EventSourceSessionFactory : ILogSessionFactory
{
    public ICapturedLogSession Start(LogLevel minimum) => EventSourceSession.Start(minimum);
}
