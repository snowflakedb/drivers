using log4net.Core;
using Snowflake.Data.Logging;
using Snowflake.Data.Tests.Wiremock;
using ILogger = Microsoft.Extensions.Logging.ILogger;

namespace Snowflake.Data.Tests.Logging;

[Collection("CustomLogger")]
[Trait("Category", "Interop")]
public sealed class CoreHttpLoggingTest : IDisposable
{
    private readonly ILogger _previous;

    public CoreHttpLoggingTest()
    {
        _previous = CustomLoggerHolder.Current;
    }

    public void Dispose() => CustomLoggerHolder.Set(_previous);

    [SnowflakeFact]
    public async Task CapturesCoreOutboundHttpCallAtInfoAsync()
    {
        using var session = Log4NetSession.Start(Level.Info);
        SnowflakeDbLoggerConfig.SetCustomLogger(session.Logger);

        using var wiremock = new WiremockClient();
        await wiremock.StartAsync().ConfigureAwait(false);
        await wiremock.AddMappingAsync("auth/login_success_any.json").ConfigureAwait(false);

        var connectionString = string.Join(";",
            "account=test_account",
            "user=test_user",
            "password=test_password",
            $"server_url=http://localhost:{wiremock.HttpPort}",
            "host=localhost",
            $"port={wiremock.HttpPort}",
            "scheme=http");

        using var connection = new SnowflakeDbConnection(connectionString);
        connection.Open();
        Assert.Equal(ConnectionState.Open, connection.State);

        Assert.Contains(session.Events, evt =>
            evt.Level == Level.Info &&
            Log4NetSession.MessageOf(evt).Contains("outbound HTTP call", StringComparison.Ordinal));
    }
}
