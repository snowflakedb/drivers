using System.Data.Common;
using System.Reflection;
using System.Text.Json;
using Snowflake.Data.Tests.Compatibility;
using Snowflake.Data.Tests.E2E.Fixtures;
using Snowflake.Data.Tests.Utilities;
using OldSnowflakeDbConnection = Snowflake.Data.Client.SnowflakeDbConnection;

namespace Snowflake.Data.Tests.E2E;

public sealed class SFConnectionTest : LiveE2ETestBase
{
    private const string RestrictedPatSession =
        "The test account authenticates with a PAT, whose restricted session does not activate secondary roles";

    public SFConnectionTest(E2EITFixture fixture, ITestOutputHelper output)
        : base(fixture, output)
    {
    }

    [SnowflakeFact]
    public void ShouldTrackExplicitTransactionOperations()
    {
        // TODO(SNOW-4382589): explicit transactions
        Skip.FutureMilestone();

        using var conn = CreateDriverConnection();
        conn.Open();
        Assert.False(HasActiveExplicitTransaction(conn));

        var trans = conn.BeginTransaction();
        Assert.True(HasActiveExplicitTransaction(conn));
        trans.Rollback();
        Assert.False(HasActiveExplicitTransaction(conn));

        conn.BeginTransaction().Rollback();
        Assert.False(HasActiveExplicitTransaction(conn));

        conn.BeginTransaction().Commit();
        Assert.False(HasActiveExplicitTransaction(conn));
    }

    [SnowflakeFact]
    public async Task ShouldActivateSecondaryRolesWhenSecondaryRolesAllAsync()
    {
        Skip.When(UsesPatAuthentication(), RestrictedPatSession);

        using var conn = Fixture.Factory.Create(
            Output,
            WithConnectionParameters(BaseConnectionString(), "secondary_roles=all;"));
        await conn.OpenAsync(TestContext.Current.CancellationToken).ConfigureAwait(false);

        using var command = conn.CreateCommand();
        command.CommandText = "SELECT CURRENT_SECONDARY_ROLES()";
        var result = (await command.ExecuteScalarAsync(TestContext.Current.CancellationToken).ConfigureAwait(false))!.ToString()!;

        var rootElement = JsonDocument.Parse(result).RootElement;
        Assert.True(rootElement.TryGetProperty("roles", out _));
        Assert.Equal("ALL", rootElement.GetProperty("value").GetString());
    }

    [SnowflakeFact]
    public async Task ShouldDisableSecondaryRolesWhenSecondaryRolesNoneAsync()
    {
        using var conn = Fixture.Factory.Create(
            Output,
            WithConnectionParameters(BaseConnectionString(), "secondary_roles=none;"));
        await conn.OpenAsync(TestContext.Current.CancellationToken).ConfigureAwait(false);

        using var command = conn.CreateCommand();
        command.CommandText = "SELECT CURRENT_SECONDARY_ROLES()";
        var result = (await command.ExecuteScalarAsync(TestContext.Current.CancellationToken).ConfigureAwait(false))!.ToString()!;

        var rootElement = JsonDocument.Parse(result).RootElement;
        Assert.True(rootElement.TryGetProperty("roles", out _));
        Assert.Empty(rootElement.GetProperty("value").GetString()!);
    }

    [SnowflakeFact]
    public async Task ShouldUseDefaultSecondaryRolesBehaviorWhenOmittedAsync()
    {
        Skip.When(UsesPatAuthentication(), RestrictedPatSession);

        using var conn = Fixture.Factory.Create(
            Output,
            WithConnectionParameters(BaseConnectionString(), "secondary_roles=none;"));
        await conn.OpenAsync(TestContext.Current.CancellationToken).ConfigureAwait(false);

        using var command = conn.CreateCommand();
        command.CommandText = "SELECT CURRENT_SECONDARY_ROLES()";
        var result = (await command.ExecuteScalarAsync(TestContext.Current.CancellationToken).ConfigureAwait(false))!.ToString()!;
        var rootElement = JsonDocument.Parse(result).RootElement;
        Assert.True(rootElement.TryGetProperty("roles", out _));
        Assert.Empty(rootElement.GetProperty("value").GetString()!);

        command.CommandText = "USE SECONDARY ROLES ALL";
        await command.ExecuteNonQueryAsync(TestContext.Current.CancellationToken).ConfigureAwait(false);

        command.CommandText = "SELECT CURRENT_SECONDARY_ROLES()";
        result = (await command.ExecuteScalarAsync(TestContext.Current.CancellationToken).ConfigureAwait(false))!.ToString()!;

        Assert.Contains("\"roles\":", result);
    }

    private string BaseConnectionString() => Fixture.Factory.Create(Output).ConnectionString;

    private bool UsesPatAuthentication()
    {
        ParametersReader.Init(Output);
        return !string.IsNullOrEmpty(ParametersReader.Get("SNOWFLAKE_TEST_PAT"));
    }

    private static string WithConnectionParameters(string baseConnectionString, string parameters) =>
        $"{baseConnectionString.TrimEnd(';')};{parameters}";

    private DbConnection CreateDriverConnection() =>
        TestDriver.Current == TestDriver.Kind.Old
            ? new OldSnowflakeDbConnection(BaseConnectionString())
            : (DbConnection)Fixture.Factory.Create(Output);

    private static bool HasActiveExplicitTransaction(DbConnection connection)
    {
        var method = connection.GetType().GetMethod(
            "HasActiveExplicitTransaction",
            BindingFlags.Instance | BindingFlags.Public | BindingFlags.NonPublic);
        Assert.NotNull(method);
        return (bool)method.Invoke(connection, null)!;
    }
}
