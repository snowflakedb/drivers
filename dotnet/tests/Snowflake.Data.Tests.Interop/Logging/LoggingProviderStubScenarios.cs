using Microsoft.Extensions.Logging;
using Snowflake.Data.Logging;
using Snowflake.Data.Tests.Logging;

namespace Snowflake.Data.Tests.Interop.Logging;

[Collection("Interop")]
[Trait("Category", "Interop")]
[Trait("Category", "Logging")]
public abstract class LoggingProviderStubScenarios : IDisposable
{
    private static readonly (uint WireLevel, LogLevel Expected)[] DistinctWireLevels =
    [
        (0, LogLevel.Error),
        (1, LogLevel.Warning),
        (2, LogLevel.Information),
        (3, LogLevel.Debug),
        (4, LogLevel.Trace),
    ];

    private readonly ILogger _previous;

    protected LoggingProviderStubScenarios()
    {
        _previous = CustomLoggerHolder.Current;
    }

    protected abstract ILogSessionFactory Factory { get; }

    public void Dispose() => CustomLoggerHolder.Set(_previous);

    [SnowflakeFact]
    public void DeliversEveryWireLevelDistinctly()
    {
        using var session = Factory.Start(LogLevel.Trace);
        SnowflakeDbLoggerConfig.SetCustomLogger(session.Logger);

        foreach (var (wireLevel, _) in DistinctWireLevels)
            Assert.Equal(0u, Emit(wireLevel, "payload"));

        Assert.Equal(
            DistinctWireLevels.Select(pair => (pair.Expected, "[core.rs:10] payload")).ToArray(),
            session.GetEvents().Select(evt => (evt.Level, evt.Message)).ToArray());
    }

    [SnowflakeFact]
    public void DebugMinimumKeepsDebugAndDropsTrace()
    {
        using var session = Factory.Start(LogLevel.Debug);
        SnowflakeDbLoggerConfig.SetCustomLogger(session.Logger);

        Assert.Equal(0u, Emit(3, "debug-event"));
        Assert.Equal(0u, Emit(4, "trace-event"));

        var evt = Assert.Single(session.GetEvents());
        Assert.Equal(LogLevel.Debug, evt.Level);
        Assert.Equal("[core.rs:10] debug-event", evt.Message);
    }

    [SnowflakeFact]
    public void InformationMinimumDropsDebugAndTrace()
    {
        using var session = Factory.Start(LogLevel.Information);
        SnowflakeDbLoggerConfig.SetCustomLogger(session.Logger);

        Assert.Equal(0u, Emit(2, "info-event"));
        Assert.Equal(0u, Emit(3, "debug-event"));
        Assert.Equal(0u, Emit(4, "trace-event"));

        var evt = Assert.Single(session.GetEvents());
        Assert.Equal(LogLevel.Information, evt.Level);
        Assert.Equal("[core.rs:10] info-event", evt.Message);
    }

    [SnowflakeFact]
    public void PassesBracePlaceholdersThroughUnchanged()
    {
        using var session = Factory.Start(LogLevel.Trace);
        SnowflakeDbLoggerConfig.SetCustomLogger(session.Logger);

        Assert.Equal(0u, Emit(2, "user {user} id {0}"));

        Assert.Equal("[core.rs:10] user {user} id {0}", Assert.Single(session.GetEvents()).Message);
    }

    [SnowflakeFact]
    public void RoundTripsUtf8Text()
    {
        using var session = Factory.Start(LogLevel.Trace);
        SnowflakeDbLoggerConfig.SetCustomLogger(session.Logger);

        const string payload = "café 日本語";
        Assert.Equal(0u, Emit(2, payload));

        Assert.Equal($"[core.rs:10] {payload}", Assert.Single(session.GetEvents()).Message);
    }

    [SnowflakeFact]
    public void FormatsCoreEventsWithFileAndLineAndLeavesNamedLoggersUnprefixed()
    {
        using var session = Factory.Start(LogLevel.Trace);
        SnowflakeDbLoggerConfig.SetCustomLogger(session.Logger);

        Assert.Equal(0u, Emit(2, "outbound HTTP call", file: "retry.rs", line: 229));
        Assert.Equal(0u, Emit(2, "wrapper payload", loggerName: "Snowflake.Data.Client"));

        Assert.Equal(
            ["[retry.rs:229] outbound HTTP call", "wrapper payload"],
            session.GetEvents().Select(evt => evt.Message).ToArray());
    }

    [SnowflakeFact]
    public void ReplacingLoggerRedirectsDelivery()
    {
        using var first = Factory.Start(LogLevel.Trace);
        using var second = Factory.Start(LogLevel.Trace);
        SnowflakeDbLoggerConfig.SetCustomLogger(first.Logger);
        Assert.Equal(0u, Emit(2, "first"));

        SnowflakeDbLoggerConfig.SetCustomLogger(second.Logger);
        Assert.Equal(0u, Emit(2, "second"));

        Assert.Equal("[core.rs:10] first", Assert.Single(first.GetEvents()).Message);
        Assert.Equal("[core.rs:10] second", Assert.Single(second.GetEvents()).Message);
    }

    [SnowflakeFact]
    public void ResetStopsDelivery()
    {
        using var session = Factory.Start(LogLevel.Trace);
        SnowflakeDbLoggerConfig.SetCustomLogger(session.Logger);

        Assert.Equal(0u, Emit(2, "before reset"));
        SnowflakeDbLoggerConfig.ResetCustomLogger();
        Assert.Equal(0u, Emit(2, "after reset"));

        Assert.Equal("[core.rs:10] before reset", Assert.Single(session.GetEvents()).Message);
    }

    private static uint Emit(
        uint level,
        string message,
        string file = "core.rs",
        uint line = 10,
        string loggerName = "") =>
        StubNativeMethods.sf_stub_emit_log(level, message, file, line, "emit", loggerName);
}
