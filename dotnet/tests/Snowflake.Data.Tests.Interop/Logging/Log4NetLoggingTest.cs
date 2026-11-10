using log4net.Core;
using Snowflake.Data.Logging;
using Snowflake.Data.Tests.Logging;
using static Snowflake.Data.Tests.Logging.Log4NetSession;
using ILogger = Microsoft.Extensions.Logging.ILogger;

namespace Snowflake.Data.Tests.Interop.Logging;

[Collection("Interop")]
[Trait("Category", "Interop")]
[Trait("Category", "Unit")]
public sealed class Log4NetLoggingTest : IDisposable
{
    private readonly ILogger _previous;

    public Log4NetLoggingTest()
    {
        _previous = CustomLoggerHolder.Current;
    }

    public void Dispose() => CustomLoggerHolder.Set(_previous);

    [SnowflakeTheory]
    [InlineData(0u, "ERROR")]
    [InlineData(1u, "WARN")]
    [InlineData(2u, "INFO")]
    [InlineData(3u, "DEBUG")]
    [InlineData(4u, "TRACE")]
    public void MapsWireLevelsOntoLog4NetLevels(uint wireLevel, string expectedLevel)
    {
        using var session = Log4NetSession.Start(Level.All);
        SnowflakeDbLoggerConfig.SetCustomLogger(session.Logger);

        Assert.Equal(0u, Emit(wireLevel, "payload"));

        var evt = Assert.Single(session.Events);
        Assert.Equal(expectedLevel, evt.Level?.Name);
        Assert.Equal("[core.rs:10] payload", MessageOf(evt));
    }

    [SnowflakeFact]
    public void InfoThresholdDropsDebugAndTrace()
    {
        using var session = Log4NetSession.Start(Level.Info);
        SnowflakeDbLoggerConfig.SetCustomLogger(session.Logger);

        Assert.Equal(0u, Emit(2, "info-event"));
        Assert.Equal(0u, Emit(3, "debug-event"));
        Assert.Equal(0u, Emit(4, "trace-event"));

        var evt = Assert.Single(session.Events);
        Assert.Equal("INFO", evt.Level?.Name);
        Assert.Equal("[core.rs:10] info-event", MessageOf(evt));
    }

    [SnowflakeFact]
    public void PassesBracePlaceholdersThroughUnchanged()
    {
        using var session = Log4NetSession.Start(Level.All);
        SnowflakeDbLoggerConfig.SetCustomLogger(session.Logger);

        Assert.Equal(0u, Emit(2, "user {user} id {0}"));

        Assert.Equal("[core.rs:10] user {user} id {0}", MessageOf(Assert.Single(session.Events)));
    }

    [SnowflakeFact]
    public void RoundTripsUtf8Text()
    {
        using var session = Log4NetSession.Start(Level.All);
        SnowflakeDbLoggerConfig.SetCustomLogger(session.Logger);

        const string payload = "café 日本語";
        Assert.Equal(0u, Emit(2, payload));

        Assert.Equal($"[core.rs:10] {payload}", MessageOf(Assert.Single(session.Events)));
    }

    [SnowflakeFact]
    public void FormatsCoreEventsWithFileAndLineAndLeavesNamedLoggersUnprefixed()
    {
        using var session = Log4NetSession.Start(Level.All);
        SnowflakeDbLoggerConfig.SetCustomLogger(session.Logger);

        Assert.Equal(0u, Emit(2, "outbound HTTP call", file: "retry.rs", line: 229));
        Assert.Equal(0u, Emit(2, "wrapper payload", loggerName: "Snowflake.Data.Client"));

        Assert.Equal(
            ["[retry.rs:229] outbound HTTP call", "wrapper payload"],
            session.Events.Select(MessageOf).ToArray());
    }

    [SnowflakeFact]
    public void ResetStopsDelivery()
    {
        using var session = Log4NetSession.Start(Level.All);
        SnowflakeDbLoggerConfig.SetCustomLogger(session.Logger);

        Assert.Equal(0u, Emit(2, "before reset"));
        SnowflakeDbLoggerConfig.ResetCustomLogger();
        Assert.Equal(0u, Emit(2, "after reset"));

        Assert.Equal("[core.rs:10] before reset", MessageOf(Assert.Single(session.Events)));
    }

    private static uint Emit(
        uint level,
        string message,
        string file = "core.rs",
        uint line = 10,
        string function = "emit",
        string loggerName = "") =>
        StubNativeMethods.sf_stub_emit_log(level, message, file, line, function, loggerName);
}
