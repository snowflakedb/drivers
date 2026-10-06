using Snowflake.Data.Tests.Wiremock;

namespace Snowflake.Data.Tests;

[Trait("Category", "E2E")]
public class TlsVersionTest : IClassFixture<ITFixture>
{
    protected readonly ITestOutputHelper Output;
    protected readonly ITFixture Fixture;

    public TlsVersionTest(ITFixture fixture, ITestOutputHelper output)
    {
        Fixture = fixture;
        Output = output;
    }

    // Scenario: should negotiate TLS when the server offers a version inside the window
    [SnowflakeFact(SkipCondition.SkipOnMacOS, "TLS 1.3 is not supported on macOS due to lack of support from the CoreTLS system library.")]
    public virtual async Task ShouldNegotiateTlsWhenTheServerOffersAVersionInsideTheWindowAsync()
    {
        // Given a TLS server that offers only TLS 1.3
        using var wiremock = new WiremockClient("tls13");
        await wiremock.StartAsync().ConfigureAwait(false);
        await wiremock.AddMappingAsync("auth/login_success_any.json").ConfigureAwait(false);
        // And a client configured with min_tls_version tls12 and max_tls_version tls13
        var connectionString = BuildHttpsConnectionString(wiremock,
            ("min_tls_version", "tls12"), ("mintls", "tls12"),
            ("max_tls_version", "tls13"), ("maxtls", "tls13"));
        // When a request is sent to the server
        using var connection = Fixture.Factory.Create(null, connectionStringOverride: connectionString);
        connection.Open();
        // Then the handshake succeeds
        Assert.Equal(ConnectionState.Open, connection.State);
    }

    // Scenario: should fail the handshake when the server only offers a version below the minimum
    [SnowflakeFact]
    public virtual async Task ShouldFailTheHandshakeWhenTheServerOnlyOffersAVersionBelowTheMinimumAsync()
    {
        // Given a TLS server that offers only TLS 1.2
        using var wiremock = new WiremockClient("tls12");
        await wiremock.StartAsync().ConfigureAwait(false);
        await wiremock.AddMappingAsync("auth/login_success_any.json").ConfigureAwait(false);
        // And a client configured with min_tls_version tls13
        var connectionString = BuildHttpsConnectionString(wiremock,
            ("min_tls_version", "tls13"), ("mintls", "tls13"));
        // When a request is sent to the server
        using var connection = Fixture.Factory.Create(null, connectionStringOverride: connectionString);
        // Then the handshake fails
        var ex = Assert.ThrowsAny<Exception>(() => connection.Open());
        Assert.Contains("received fatal alert: ProtocolVersion", ex.Message, StringComparison.OrdinalIgnoreCase);

        // Positive control: same server succeeds with a permissive window
        var permissiveConnectionString = BuildHttpsConnectionString(wiremock,
            ("min_tls_version", "tls12"), ("mintls", "tls12"),
            ("max_tls_version", "tls13"), ("maxtls", "tls13"));
        using var permissiveConnection = Fixture.Factory.Create(null, connectionStringOverride: permissiveConnectionString);
        permissiveConnection.Open();
        Assert.Equal(ConnectionState.Open, permissiveConnection.State);
    }

    // Scenario: should reject the configuration when the minimum exceeds the maximum
    [SnowflakeFact]
    public void ShouldRejectTheConfigurationWhenTheMinimumExceedsTheMaximum()
    {
        // Given settings with min_tls_version tls13 and max_tls_version tls12
        var connectionString = string.Join(";",
            "server_url=https://localhost",
            "account=test_account",
            "user=test_user",
            "password=test_password",
            "host=localhost",
            "scheme=https",
            "min_tls_version=tls13", "mintls=tls13",
            "max_tls_version=tls12", "maxtls=tls12");
        // When the TLS configuration is built from settings
        using var connection = Fixture.Factory.Create(null, connectionStringOverride: connectionString);
        var ex = Assert.ThrowsAny<Exception>(() => connection.Open());
        // Then a configuration error is returned
        Assert.Contains(ExpectedTlsRejectErrorMessage, ex.Message, StringComparison.OrdinalIgnoreCase);
    }

    protected virtual string ExpectedTlsRejectErrorMessage =>
        "max_tls_version (TLS 1.2) must be at least min_tls_version (TLS 1.3)";

    private static string BuildHttpsConnectionString(
        WiremockClient wiremock,
        params (string key, string value)[] extra)
    {
        if (wiremock.HttpsPort < 0)
            throw new InvalidOperationException("HttpsUrl() requires WiremockClient(\"tls12\") or WiremockClient(\"tls13\")");

        var parts = new List<string>
        {
            "account=test_account",
            "user=test_user",
            "password=test_password",
            // New driver (sf_core)
            $"server_url=https://localhost:{wiremock.HttpsPort}",
            $"custom_root_store_path={wiremock.WiremockCaPemPath}",
            // Old driver (NuGet): separate host/port/scheme
            $"host=localhost",
            $"port={wiremock.HttpsPort}",
            "scheme=https",
        };
        foreach (var (key, value) in extra)
            parts.Add($"{key}={value}");

        return string.Join(";", parts);
    }
}
