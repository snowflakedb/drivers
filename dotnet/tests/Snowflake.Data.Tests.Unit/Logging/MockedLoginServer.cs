using Snowflake.Data.Tests.Wiremock;

namespace Snowflake.Data.Tests.Logging;

public sealed class MockedLoginServer : IDisposable
{
    public static readonly string[] ForbiddenSecrets =
    [
        "test_password",
        "mock_session_token",
        "mock_master_token",
    ];

    private readonly SemaphoreSlim _startLock = new(1, 1);
    private WiremockClient? _wiremock;

    public async Task OpenConnectionAsync(CancellationToken cancellationToken)
    {
        var port = await EnsureStartedAsync(cancellationToken).ConfigureAwait(false);
        var connectionString = string.Join(";",
            "account=test_account",
            "user=test_user",
            "password=test_password",
            $"server_url=http://localhost:{port}",
            "host=localhost",
            $"port={port}",
            "scheme=http");

        using var connection = new SnowflakeDbConnection(connectionString);
        await connection.OpenAsync(cancellationToken).ConfigureAwait(false);
        Assert.Equal(ConnectionState.Open, connection.State);
    }

    public void Dispose()
    {
        _wiremock?.Dispose();
        _startLock.Dispose();
    }

    private async Task<int> EnsureStartedAsync(CancellationToken cancellationToken)
    {
        await _startLock.WaitAsync(cancellationToken).ConfigureAwait(false);
        try
        {
            if (_wiremock is null)
            {
                var wiremock = new WiremockClient();
                try
                {
                    await wiremock.StartAsync(cancellationToken: cancellationToken).ConfigureAwait(false);
                    await wiremock.AddMappingAsync("auth/login_success_any.json", cancellationToken: cancellationToken).ConfigureAwait(false);
                }
                catch
                {
                    wiremock.Dispose();
                    throw;
                }

                _wiremock = wiremock;
            }

            return _wiremock.HttpPort;
        }
        finally
        {
            _startLock.Release();
        }
    }
}
