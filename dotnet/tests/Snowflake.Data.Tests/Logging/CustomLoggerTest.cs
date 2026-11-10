using System.Text;
using Microsoft.Extensions.Logging;
using Snowflake.Data.Interop.Callback;
using Snowflake.Data.Logging;

namespace Snowflake.Data.Tests.Logging;

[Collection("CustomLogger")]
[Trait("Category", "Unit")]
public sealed unsafe class CustomLoggerTest : IDisposable
{
    private readonly ILogger _previous;

    public CustomLoggerTest()
    {
        _previous = CustomLoggerHolder.Current;
    }

    public void Dispose() => CustomLoggerHolder.Set(_previous);

    [SnowflakeTheory]
    [InlineData(0u, LogLevel.Error)]
    [InlineData(1u, LogLevel.Warning)]
    [InlineData(2u, LogLevel.Information)]
    [InlineData(3u, LogLevel.Debug)]
    [InlineData(4u, LogLevel.Trace)]
    [InlineData(5u, LogLevel.Trace)]
    public void MapsWireLevelOntoILoggerLevel(uint wireLevel, LogLevel expected)
    {
        var logger = new RecordingLogger { MinLevel = LogLevel.Trace };
        SnowflakeDbLoggerConfig.SetCustomLogger(logger);

        Assert.Equal(0u, Emit(wireLevel, "payload"));

        Assert.Equal([(expected, "[core.rs:10] payload")], logger.Events);
    }

    [SnowflakeFact]
    public void DisabledLevelDoesNotCallLog()
    {
        var logger = new RecordingLogger { MinLevel = LogLevel.Information };
        SnowflakeDbLoggerConfig.SetCustomLogger(logger);

        Assert.Equal(0u, Emit(3, "debug-only"));
        Assert.Equal(0u, Emit(4, "trace-only"));

        Assert.Empty(logger.Events);
        Assert.False(logger.LogCalled);
    }

    [SnowflakeFact]
    public void FormatsCoreEventsWithFileAndLine()
    {
        var logger = new RecordingLogger { MinLevel = LogLevel.Trace };
        SnowflakeDbLoggerConfig.SetCustomLogger(logger);

        Assert.Equal(0u, Emit(2, "outbound HTTP call", file: "retry.rs", line: 42));

        Assert.Equal([(LogLevel.Information, "[retry.rs:42] outbound HTTP call")], logger.Events);
    }

    [SnowflakeFact]
    public void PassesBracesThroughWithoutTemplateParsing()
    {
        var logger = new RecordingLogger { MinLevel = LogLevel.Trace };
        SnowflakeDbLoggerConfig.SetCustomLogger(logger);

        Assert.Equal(0u, Emit(2, "user {user} id {0}"));

        Assert.Equal([(LogLevel.Information, "[core.rs:10] user {user} id {0}")], logger.Events);
    }

    [SnowflakeFact]
    public void DeliversWrapperEventsWithoutFilePrefix()
    {
        var logger = new RecordingLogger { MinLevel = LogLevel.Trace };
        SnowflakeDbLoggerConfig.SetCustomLogger(logger);

        Assert.Equal(0u, Emit(2, "wrapper payload", loggerName: "Snowflake.Data.Client"));

        Assert.Equal([(LogLevel.Information, "wrapper payload")], logger.Events);
    }

    [SnowflakeFact]
    public void SwallowingLoggerExceptionsDoesNotEscapeTheCallback()
    {
        SnowflakeDbLoggerConfig.SetCustomLogger(new ThrowingLogger());

        Assert.Equal(0u, Emit(0, "must not throw"));
    }

    [SnowflakeFact]
    public void ResetStopsDelivery()
    {
        var logger = new RecordingLogger { MinLevel = LogLevel.Trace };
        SnowflakeDbLoggerConfig.SetCustomLogger(logger);

        Assert.Equal(0u, Emit(2, "before reset"));
        SnowflakeDbLoggerConfig.ResetCustomLogger();
        Assert.Equal(0u, Emit(2, "after reset"));

        Assert.Equal([(LogLevel.Information, "[core.rs:10] before reset")], logger.Events);
    }

    [SnowflakeFact]
    public void SetCustomLoggerRejectsNull()
    {
        Assert.Throws<ArgumentNullException>(() => SnowflakeDbLoggerConfig.SetCustomLogger(null!));
    }

    private static uint Emit(uint level, string message, string file = "core.rs", uint line = 10, string loggerName = "")
    {
        var messageBytes = Encoding.UTF8.GetBytes(message + '\0');
        var fileBytes = Encoding.UTF8.GetBytes(file + '\0');
        var loggerNameBytes = Encoding.UTF8.GetBytes(loggerName + '\0');
        fixed (byte* messagePtr = messageBytes)
        fixed (byte* filePtr = fileBytes)
        fixed (byte* loggerNamePtr = loggerNameBytes)
            return LogCallbackProvider.LogCallback(level, messagePtr, filePtr, line, null, loggerNamePtr);
    }

    private sealed class RecordingLogger : ILogger
    {
        public LogLevel MinLevel { get; set; }
        public List<(LogLevel Level, string Message)> Events { get; } = [];
        public bool LogCalled { get; private set; }

        public IDisposable? BeginScope<TState>(TState state) where TState : notnull => null;

        public bool IsEnabled(LogLevel logLevel) => logLevel != LogLevel.None && logLevel >= MinLevel;

        public void Log<TState>(LogLevel logLevel, EventId eventId, TState state, Exception? exception, Func<TState, Exception?, string> formatter)
        {
            LogCalled = true;
            Events.Add((logLevel, formatter(state, exception)));
        }
    }

    private sealed class ThrowingLogger : ILogger
    {
        public IDisposable? BeginScope<TState>(TState state) where TState : notnull => null;

        public bool IsEnabled(LogLevel logLevel) => true;

        public void Log<TState>(LogLevel logLevel, EventId eventId, TState state, Exception? exception, Func<TState, Exception?, string> formatter) =>
            throw new InvalidOperationException("logger failed");
    }
}
