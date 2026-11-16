using System.Collections.Concurrent;
using Serilog;
using Serilog.Core;
using Serilog.Events;
using Serilog.Extensions.Logging;
using LogLevel = Microsoft.Extensions.Logging.LogLevel;

namespace Snowflake.Data.Tests.Logging;

public sealed class SerilogSessionFactory : ILogSessionFactory
{
    public ICapturedLogSession Start(LogLevel minimum)
    {
        var sink = new RecordingSink();
        var serilog = new LoggerConfiguration()
            .MinimumLevel.Is(ToSerilogLevel(minimum))
            .WriteTo.Sink(sink)
            .CreateLogger();
        var factory = new SerilogLoggerFactory(serilog, dispose: true);
        return new LoggerFactorySession(factory, factory.CreateLogger("Snowflake.Data"), sink.Events);
    }

    private static LogEventLevel ToSerilogLevel(LogLevel minimum) => minimum switch
    {
        LogLevel.Critical => LogEventLevel.Fatal,
        LogLevel.Error => LogEventLevel.Error,
        LogLevel.Warning => LogEventLevel.Warning,
        LogLevel.Information => LogEventLevel.Information,
        LogLevel.Debug => LogEventLevel.Debug,
        _ => LogEventLevel.Verbose,
    };

    private sealed class RecordingSink : ILogEventSink
    {
        public ConcurrentQueue<CapturedLogEvent> Events { get; } = new();

        public void Emit(LogEvent logEvent) =>
            Events.Enqueue(new CapturedLogEvent(ToLogLevel(logEvent.Level), logEvent.RenderMessage()));

        private static LogLevel ToLogLevel(LogEventLevel level) => level switch
        {
            LogEventLevel.Fatal => LogLevel.Critical,
            LogEventLevel.Error => LogLevel.Error,
            LogEventLevel.Warning => LogLevel.Warning,
            LogEventLevel.Information => LogLevel.Information,
            LogEventLevel.Debug => LogLevel.Debug,
            _ => LogLevel.Trace,
        };
    }
}
