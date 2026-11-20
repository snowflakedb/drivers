using System.Runtime.Versioning;
using Microsoft.Extensions.Logging;
using Snowflake.Data.Client;
using Snowflake.Data.Logging;
using Snowflake.Data.Tests.Logging;

namespace Snowflake.Data.Tests.Interop.Logging;

[Collection("Interop")]
[Trait("Category", "Interop")]
[Trait("Category", "Logging")]
[SupportedOSPlatform("windows")]
public sealed class EventLogStubLoggingTest : IDisposable
{
    private readonly ILogger _previous;

    public EventLogStubLoggingTest()
    {
        _previous = CustomLoggerHolder.Current;
    }

    public void Dispose() => CustomLoggerHolder.Set(_previous);

    [SnowflakeFact(SkipCondition.RunOnlyOnWindows)]
    public void DeliversInformationAfterReplaceAndReset()
    {
        using var first = EventLogSession.Start(LogLevel.Information);
        using var second = EventLogSession.Start(LogLevel.Information);
        SnowflakeDbLoggerConfig.SetCustomLogger(first.Logger);
        Assert.Equal(0u, Emit(2, "first"));

        SnowflakeDbLoggerConfig.SetCustomLogger(second.Logger);
        Assert.Equal(0u, Emit(2, "second"));
        SnowflakeDbLoggerConfig.ResetCustomLogger();
        Assert.Equal(0u, Emit(2, "after reset"));

        var firstEvents = first.GetEvents();
        var secondEvents = second.GetEvents();
        Assert.Contains(firstEvents, evt => evt.Message.Contains("first", StringComparison.Ordinal));
        Assert.Contains(secondEvents, evt => evt.Message.Contains("second", StringComparison.Ordinal));
        Assert.DoesNotContain(firstEvents, evt => evt.Message.Contains("second", StringComparison.Ordinal));
        Assert.DoesNotContain(secondEvents, evt => evt.Message.Contains("after reset", StringComparison.Ordinal));
    }

    private static uint Emit(uint level, string message) =>
        StubNativeMethods.sf_stub_emit_log(level, message, "core.rs", 10, "emit", string.Empty);
}
