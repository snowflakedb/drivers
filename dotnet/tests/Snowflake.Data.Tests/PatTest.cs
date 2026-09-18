using Snowflake.Data.Tests.Compatibility;
using Snowflake.Data.Tests.Config;
using Snowflake.Data.Tests.Utilities;

namespace Snowflake.Data.Tests;

[Trait("Category", "E2E")]
public class PatTest : IClassFixture<ITFixture>
{
    protected readonly ITestOutputHelper Output;
    protected readonly ITFixture Fixture;

    public PatTest(ITFixture fixture, ITestOutputHelper output)
    {
        Fixture = fixture;
        Output = output;
    }

    // Scenario: should authenticate using PAT as password
    [SnowflakeFact]
    public void ShouldAuthenticateUsingPatAsPassword()
    {
        // Given Authentication is set to password and valid PAT token is provided
        ParametersReader.Init(Output);
        var pat = ParametersReader.Get("SNOWFLAKE_TEST_PAT");
        Assert.NotNull(pat);
        var connectionString = new ConnectionStringBuilder()
            .WithAccount(ParametersReader.Get("SNOWFLAKE_TEST_ACCOUNT"))
            .WithUser(ParametersReader.Get("SNOWFLAKE_TEST_USER"))
            .WithWarehouse(ParametersReader.Get("SNOWFLAKE_TEST_WAREHOUSE"))
            .WithDatabase(ParametersReader.Get("SNOWFLAKE_TEST_DATABASE"))
            .WithSchema(ParametersReader.Get("SNOWFLAKE_TEST_SCHEMA"))
            .WithRole(ParametersReader.Get("SNOWFLAKE_TEST_ROLE"))
            .WithPassword(pat)
            .Build();

        using var connection = Fixture.Factory.Create(Output, connectionString);

        // When Trying to Connect
        connection.Open();

        // Then Login is successful and simple query can be executed
        Assert.Equal(ConnectionState.Open, connection.State);
        using var cmd = connection.CreateCommand();
        cmd.CommandText = "SELECT 1";
        var result = cmd.ExecuteScalar();
        Assert.Equal(1, Convert.ToInt32(result));
    }

    // Scenario: should authenticate using PAT as token
    [SnowflakeFact]
    public void ShouldAuthenticateUsingPatAsToken()
    {
        // Given Authentication is set to Programmatic Access Token and valid PAT token is provided
        ParametersReader.Init(Output);
        var pat = ParametersReader.Get("SNOWFLAKE_TEST_PAT");
        Assert.NotNull(pat);

        var connectionString = new ConnectionStringBuilder()
            .WithAccount(ParametersReader.Get("SNOWFLAKE_TEST_ACCOUNT"))
            .WithUser(ParametersReader.Get("SNOWFLAKE_TEST_USER"))
            .WithWarehouse(ParametersReader.Get("SNOWFLAKE_TEST_WAREHOUSE"))
            .WithDatabase(ParametersReader.Get("SNOWFLAKE_TEST_DATABASE"))
            .WithSchema(ParametersReader.Get("SNOWFLAKE_TEST_SCHEMA"))
            .WithRole(ParametersReader.Get("SNOWFLAKE_TEST_ROLE"))
            .WithPat(pat)
            .Build();

        using var connection = Fixture.Factory.Create(Output, connectionString);

        // When Trying to Connect
        connection.Open();

        // Then Login is successful and simple query can be executed
        Assert.Equal(ConnectionState.Open, connection.State);
        using var cmd = connection.CreateCommand();
        cmd.CommandText = "SELECT 1";
        var result = cmd.ExecuteScalar();
        Assert.Equal(1, Convert.ToInt32(result));
    }

    // Scenario: should authenticate using PAT as token with lowercase authenticator
    [SnowflakeFact]
    public void ShouldAuthenticateUsingPatAsTokenWithLowercaseAuthenticator()
    {
        // Given Authentication is set to lowercase programmatic_access_token and valid PAT token is provided
        ParametersReader.Init(Output);
        var pat = ParametersReader.Get("SNOWFLAKE_TEST_PAT");
        Assert.NotNull(pat);

        var connectionString = new ConnectionStringBuilder()
            .WithAccount(ParametersReader.Get("SNOWFLAKE_TEST_ACCOUNT"))
            .WithUser(ParametersReader.Get("SNOWFLAKE_TEST_USER"))
            .WithWarehouse(ParametersReader.Get("SNOWFLAKE_TEST_WAREHOUSE"))
            .WithDatabase(ParametersReader.Get("SNOWFLAKE_TEST_DATABASE"))
            .WithSchema(ParametersReader.Get("SNOWFLAKE_TEST_SCHEMA"))
            .WithRole(ParametersReader.Get("SNOWFLAKE_TEST_ROLE"))
            .WithPat(pat)
            .WithExplicitlySetAuthenticator("programmatic_access_token")
            .Build();
        using var connection = Fixture.Factory.Create(Output, connectionString);

        // When Trying to Connect
        connection.Open();

        // Then Login is successful and simple query can be executed
        Assert.Equal(ConnectionState.Open, connection.State);
        using var cmd = connection.CreateCommand();
        cmd.CommandText = "SELECT 1";
        var result = cmd.ExecuteScalar();
        Assert.Equal(1, Convert.ToInt32(result));
    }

    // Scenario: should authenticate using PAT token from token_file_path
    [SnowflakeFact]
    public void ShouldAuthenticateUsingPatTokenFromTokenFilePath()
    {
        Skip.FutureMilestone();

        // Given Authentication is set to Programmatic Access Token and a valid PAT token is stored in a file
        ParametersReader.Init(Output);
        var pat = ParametersReader.Get("SNOWFLAKE_TEST_PAT");
        Assert.NotNull(pat);

        var tokenFilePath = Path.GetTempFileName();
        try
        {
            File.WriteAllText(tokenFilePath, pat);

            var connectionString = new ConnectionStringBuilder()
                .WithAccount(ParametersReader.Get("SNOWFLAKE_TEST_ACCOUNT"))
                .WithUser(ParametersReader.Get("SNOWFLAKE_TEST_USER"))
                .WithWarehouse(ParametersReader.Get("SNOWFLAKE_TEST_WAREHOUSE"))
                .WithDatabase(ParametersReader.Get("SNOWFLAKE_TEST_DATABASE"))
                .WithSchema(ParametersReader.Get("SNOWFLAKE_TEST_SCHEMA"))
                .WithRole(ParametersReader.Get("SNOWFLAKE_TEST_ROLE"))
                .WithPat(null)
                .WithExplicitlySetAuthenticator("programmatic_access_token")
                .WithTokenFilePath(tokenFilePath)
                .Build();

            using var connection = Fixture.Factory.Create(Output, connectionString);

            // When Trying to Connect
            connection.Open();

            // Then Login is successful and simple query can be executed
            Assert.Equal(ConnectionState.Open, connection.State);
            using var cmd = connection.CreateCommand();
            cmd.CommandText = "SELECT 1";
            var result = cmd.ExecuteScalar();
            Assert.Equal(1, Convert.ToInt32(result));
        }
        finally
        {
            File.Delete(tokenFilePath);
        }
    }

    // Scenario: should fail PAT authentication when invalid token provided
    [SnowflakeFact]
    public void ShouldFailPatAuthenticationWhenInvalidTokenProvided()
    {
        // Given Authentication is set to Programmatic Access Token and invalid PAT token is provided
        ParametersReader.Init(Output);
        var connectionString = new ConnectionStringBuilder()
            .WithAccount(ParametersReader.Get("SNOWFLAKE_TEST_ACCOUNT"))
            .WithUser(ParametersReader.Get("SNOWFLAKE_TEST_USER"))
            .WithWarehouse(ParametersReader.Get("SNOWFLAKE_TEST_WAREHOUSE"))
            .WithDatabase(ParametersReader.Get("SNOWFLAKE_TEST_DATABASE"))
            .WithSchema(ParametersReader.Get("SNOWFLAKE_TEST_SCHEMA"))
            .WithRole(ParametersReader.Get("SNOWFLAKE_TEST_ROLE"))
            .WithPat("invalidToken")
            .Build();

        using var connection = Fixture.Factory.Create(Output, connectionString);

        // When Trying to Connect
        var exception = Assert.ThrowsAny<Exception>(connection.Open);

        // Then There is error returned
        Assert.NotNull(exception);
    }
}
