using Microsoft.Extensions.Logging;
using Snowflake.Data.Logging;

namespace Snowflake.Data.Tests.Logging;

[Collection("CustomLogger")]
[Trait("Category", "Interop")]
[Trait("Category", "Logging")]
public abstract class LoggingProviderCoreScenarios : IDisposable
{
    private readonly MockedLoginServer _server;
    private readonly ILogger _previous;

    protected LoggingProviderCoreScenarios(MockedLoginServer server)
    {
        _server = server;
        _previous = CustomLoggerHolder.Current;
    }

    protected abstract ILogSessionFactory Factory { get; }

    public void Dispose() => CustomLoggerHolder.Set(_previous);

    [SnowflakeFact]
    public async Task DeliversCoreOutboundHttpCallAsync()
    {
        using var session = Factory.Start(LogLevel.Information);
        SnowflakeDbLoggerConfig.SetCustomLogger(session.Logger);

        await _server.OpenConnectionAsync(TestContext.Current.CancellationToken).ConfigureAwait(false);

        Assert.Contains(
            session.GetEvents(),
            evt => evt.Level == LogLevel.Information
                   && evt.Message.Contains("outbound HTTP call", StringComparison.Ordinal));
    }

    [SnowflakeFact]
    public async Task TraceCaptureNeverContainsCredentialsAsync()
    {
        using var session = Factory.Start(LogLevel.Trace);
        SnowflakeDbLoggerConfig.SetCustomLogger(session.Logger);

        await _server.OpenConnectionAsync(TestContext.Current.CancellationToken).ConfigureAwait(false);

        var events = session.GetEvents();
        Assert.Contains(events, evt => evt.Level == LogLevel.Trace);
        Assert.DoesNotContain(
            events,
            evt => MockedLoginServer.ForbiddenSecrets.Any(secret => evt.Message.Contains(secret, StringComparison.Ordinal)));
    }
}
