using System.Collections.Concurrent;
using Microsoft.Extensions.Logging;
using Microsoft.Extensions.Logging.Abstractions;
using Microsoft.Extensions.Logging.Console;
using Microsoft.Extensions.Options;
using NSubstitute;

namespace Snowflake.Data.Tests.Logging;

public sealed class ConsoleSessionFactory : ILogSessionFactory
{
    private const string FormatterName = "capture";

    public ICapturedLogSession Start(LogLevel minimum)
    {
        var events = new ConcurrentQueue<CapturedLogEvent>();
        var options = Substitute.For<IOptionsMonitor<ConsoleLoggerOptions>>();
        options.CurrentValue.Returns(new ConsoleLoggerOptions { FormatterName = FormatterName });
        var provider = new ConsoleLoggerProvider(options, [new CapturingConsoleFormatter(events)]);
        var factory = new LoggerFactory([provider], new LoggerFilterOptions { MinLevel = minimum });
        return new ConsoleSession(factory, factory.CreateLogger("Snowflake.Data"), events);
    }

    private sealed class ConsoleSession : ICapturedLogSession
    {
        private readonly ILoggerFactory _factory;
        private readonly ConcurrentQueue<CapturedLogEvent> _events;

        public ConsoleSession(ILoggerFactory factory, ILogger logger, ConcurrentQueue<CapturedLogEvent> events)
        {
            _factory = factory;
            _events = events;
            Logger = logger;
        }

        public ILogger Logger { get; }

        public IReadOnlyList<CapturedLogEvent> GetEvents()
        {
            _factory.Dispose();
            return _events.ToArray();
        }

        public void Dispose() => _factory.Dispose();
    }

    private sealed class CapturingConsoleFormatter : ConsoleFormatter
    {
        private readonly ConcurrentQueue<CapturedLogEvent> _events;

        public CapturingConsoleFormatter(ConcurrentQueue<CapturedLogEvent> events)
            : base(FormatterName)
        {
            _events = events;
        }

        public override void Write<TState>(
            in LogEntry<TState> logEntry,
            IExternalScopeProvider? scopeProvider,
            TextWriter textWriter)
        {
            var message = logEntry.Formatter(logEntry.State, logEntry.Exception);
            if (message.Length == 0)
                return;

            _events.Enqueue(new CapturedLogEvent(logEntry.LogLevel, message));
        }
    }
}
