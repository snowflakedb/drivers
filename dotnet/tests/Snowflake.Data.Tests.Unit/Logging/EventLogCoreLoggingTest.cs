using System.Runtime.Versioning;
using Microsoft.Extensions.Logging;
using Snowflake.Data.Logging;

namespace Snowflake.Data.Tests.Logging;

[Collection("CustomLogger")]
[Trait("Category", "Interop")]
[Trait("Category", "Logging")]
[SupportedOSPlatform("windows")]
public sealed class EventLogCoreLoggingTest : IDisposable
{
    private readonly MockedLoginServer _server;
    private readonly ILogger _previous;

    public EventLogCoreLoggingTest(MockedLoginServer server)
    {
        _server = server;
        _previous = CustomLoggerHolder.Current;
    }

    public void Dispose() => CustomLoggerHolder.Set(_previous);

    [SnowflakeFact(SkipCondition.RunOnlyOnWindows)]
    public async Task DeliversCoreOutboundHttpCallAsync()
    {
        using var session = EventLogSession.Start(LogLevel.Debug);
        SnowflakeDbLoggerConfig.SetCustomLogger(session.Logger);

        await _server.OpenConnectionAsync(TestContext.Current.CancellationToken).ConfigureAwait(false);

        Assert.Contains(
            session.GetEvents(),
            evt => evt.Message.Contains("outbound HTTP call", StringComparison.Ordinal));
    }

    [SnowflakeFact(SkipCondition.RunOnlyOnWindows)]
    public async Task CaptureNeverContainsCredentialsAsync()
    {
        using var session = EventLogSession.Start(LogLevel.Trace);
        SnowflakeDbLoggerConfig.SetCustomLogger(session.Logger);

        await _server.OpenConnectionAsync(TestContext.Current.CancellationToken).ConfigureAwait(false);

        var events = session.GetEvents();
        Assert.NotEmpty(events);
        Assert.DoesNotContain(
            events,
            evt => MockedLoginServer.ForbiddenSecrets.Any(secret => evt.Message.Contains(secret, StringComparison.Ordinal)));
    }
}
