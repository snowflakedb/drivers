using System.Diagnostics;
using System.Runtime.Versioning;
using Microsoft.Extensions.Logging;
using Microsoft.Extensions.Logging.EventLog;

namespace Snowflake.Data.Tests.Logging;

[SupportedOSPlatform("windows")]
public sealed class EventLogSession : ICapturedLogSession
{
    private const string LogName = "Application";
    private const string SourceName = ".NET Runtime";
    private readonly ILoggerFactory _factory;
    private readonly string _marker;
    private readonly DateTime _startedUtc;

    private EventLogSession(ILoggerFactory factory, ILogger logger, string marker, DateTime startedUtc)
    {
        _factory = factory;
        _marker = marker;
        _startedUtc = startedUtc;
        Logger = logger;
    }

    public ILogger Logger { get; }

    public IReadOnlyList<CapturedLogEvent> GetEvents()
    {
        _factory.Dispose();
        using var log = new EventLog(LogName);
        var events = new List<CapturedLogEvent>();
        var entries = log.Entries;
        var earliest = _startedUtc.AddSeconds(-2);
        for (var i = entries.Count - 1; i >= 0; i--)
        {
            var entry = entries[i];
            if (entry.TimeGenerated.ToUniversalTime() < earliest)
                break;

            if (!entry.Message.Contains(_marker, StringComparison.Ordinal))
                continue;

            events.Add(new CapturedLogEvent(ToLogLevel(entry.EntryType), StripMarker(entry.Message)));
        }

        events.Reverse();
        return events;
    }

    public static EventLogSession Start(LogLevel minimum)
    {
        var marker = $"sf-ud-{Guid.NewGuid():N}";
        var settings = new EventLogSettings
        {
            LogName = LogName,
            SourceName = SourceName,
            Filter = (_, level) => level >= minimum,
        };
        var factory = new LoggerFactory([new EventLogLoggerProvider(settings)], new LoggerFilterOptions { MinLevel = minimum });
        var inner = factory.CreateLogger("Snowflake.Data");
        return new EventLogSession(factory, new MarkedLogger(inner, marker), marker, DateTime.UtcNow);
    }

    public void Dispose() => _factory.Dispose();

    private string StripMarker(string message)
    {
        var markerPrefix = _marker + " ";
        var index = message.IndexOf(markerPrefix, StringComparison.Ordinal);
        return index < 0 ? message : message.Substring(index + markerPrefix.Length);
    }

    private static LogLevel ToLogLevel(EventLogEntryType entryType) => entryType switch
    {
        EventLogEntryType.Error => LogLevel.Error,
        EventLogEntryType.Warning => LogLevel.Warning,
        _ => LogLevel.Information,
    };

    private sealed class MarkedLogger : ILogger
    {
        private readonly ILogger _inner;
        private readonly string _marker;

        public MarkedLogger(ILogger inner, string marker)
        {
            _inner = inner;
            _marker = marker;
        }

        public IDisposable? BeginScope<TState>(TState state)
            where TState : notnull
            => _inner.BeginScope(state);

        public bool IsEnabled(LogLevel logLevel) => _inner.IsEnabled(logLevel);

        public void Log<TState>(
            LogLevel logLevel,
            EventId eventId,
            TState state,
            Exception? exception,
            Func<TState, Exception?, string> formatter)
        {
            _inner.Log(logLevel, eventId, state, exception, (s, ex) => _marker + " " + formatter(s, ex));
        }
    }
}

[SupportedOSPlatform("windows")]
public sealed class EventLogSessionFactory : ILogSessionFactory
{
    public ICapturedLogSession Start(LogLevel minimum) => EventLogSession.Start(minimum);
}
