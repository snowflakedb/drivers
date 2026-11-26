extern alias OldDriver;
using System.Data.Common;
using Snowflake.Data.Client;
using Snowflake.Data.Tests.Compatibility;
using Snowflake.Data.Tests.E2E.Fixtures;
using OldSnowflakeDbFactory = OldDriver::Snowflake.Data.Client.SnowflakeDbFactory;

namespace Snowflake.Data.Tests.E2E;

public sealed class SFDbFactoryTest : LiveE2ETestBase
{
    public SFDbFactoryTest(E2EITFixture fixture, ITestOutputHelper output)
        : base(fixture, output)
    {
    }

    [SnowflakeFact]
    public async Task ShouldExecuteCommandCreatedByDbFactoryAsync()
    {
        var factory = GetFactory();
        using var command = factory.CreateCommand()!;
        using var connection = OpenConnection(factory);
        command.Connection = connection;
        command.CommandText = "select 1";

        var res = await command.ExecuteScalarAsync(TestContext.Current.CancellationToken).ConfigureAwait(false);

        Assert.Equal(1L, res);
    }

    [SnowflakeFact]
    public async Task ShouldBindParameterCreatedByDbFactoryAsync()
    {
        // TODO(SNOW-4382583): parameter bindings
        Skip.FutureMilestone();

        var expectedIntValue = 1;
        var factory = GetFactory();
        using var command = factory.CreateCommand()!;
        using var connection = OpenConnection(factory);

        var parameter = factory.CreateParameter()!;
        parameter.ParameterName = "1";
        parameter.Value = expectedIntValue;
        parameter.DbType = DbType.Int16;
        command.Parameters.Add(parameter);

        command.Connection = connection;
        command.CommandText = "select ?";

        var result = await command.ExecuteScalarAsync(TestContext.Current.CancellationToken).ConfigureAwait(false);

        Assert.Equal((long)expectedIntValue, result);
    }

    [SnowflakeFact]
    public async Task ShouldOpenConnectionWithDbFactoryConnectionStringBuilderAsync()
    {
        var factory = GetFactory();
        using var command = factory.CreateCommand()!;
        using var connection = OpenConnection(factory);
        var builder = factory.CreateConnectionStringBuilder()!;
        builder.ConnectionString = BaseConnectionString();

        connection.ConnectionString = builder.ConnectionString;
        await connection.OpenAsync(TestContext.Current.CancellationToken).ConfigureAwait(false);

        command.Connection = connection;
        command.CommandText = "select 1";

        var result = await command.ExecuteScalarAsync(TestContext.Current.CancellationToken).ConfigureAwait(false);

        Assert.Equal(1L, result);
    }

    [SnowflakeFact]
    public void ShouldFillDataSetWithDbFactoryDataAdapter()
    {
        var factory = GetFactory();
        using var command = factory.CreateCommand()!;
        using var connection = OpenConnection(factory);
        command.Connection = connection;
        command.CommandText = "select 1";

        using var adapter = factory.CreateDataAdapter()!;
        adapter.SelectCommand = command;
        using var builder = factory.CreateCommandBuilder()!;
        builder.DataAdapter = adapter;
        using var ds = new DataSet("ds");

        adapter.Fill(ds);

        Assert.Equal(1L, ds.Tables[0].Rows[0].ItemArray[0]);
    }

    private static DbProviderFactory GetFactory() =>
        TestDriver.Current == TestDriver.Kind.Old
            ? OldSnowflakeDbFactory.Instance
            : SnowflakeDbFactory.Instance;

    private DbConnection OpenConnection(DbProviderFactory factory)
    {
        var connection = factory.CreateConnection()!;
        connection.ConnectionString = BaseConnectionString();
        connection.Open();
        return connection;
    }

    private string BaseConnectionString()
    {
        using var connection = Fixture.Factory.Create(Output);
        return connection.ConnectionString;
    }
}
