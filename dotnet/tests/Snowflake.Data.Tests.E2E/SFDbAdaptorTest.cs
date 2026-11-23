extern alias OldDriver;
using System.Data.Common;
using System.Runtime.InteropServices;
using Snowflake.Data.Tests.Compatibility;
using Snowflake.Data.Tests.E2E.Fixtures;
using NewSnowflakeDbCommand = Snowflake.Data.Client.SnowflakeDbCommand;
using NewSnowflakeDbConnection = Snowflake.Data.Client.SnowflakeDbConnection;
using NewSnowflakeDbDataAdapter = Snowflake.Data.Client.SnowflakeDbDataAdapter;
using OldSnowflakeDbCommand = OldDriver::Snowflake.Data.Client.SnowflakeDbCommand;
using OldSnowflakeDbConnection = OldDriver::Snowflake.Data.Client.SnowflakeDbConnection;
using OldSnowflakeDbDataAdapter = OldDriver::Snowflake.Data.Client.SnowflakeDbDataAdapter;

namespace Snowflake.Data.Tests.E2E;

public sealed class SFDbAdaptorTest : LiveE2ETestBase
{
    public SFDbAdaptorTest(E2EITFixture fixture, ITestOutputHelper output)
        : base(fixture, output)
    {
    }

    [SnowflakeFact]
    public void ShouldCreateDataAdapterWithSelectCommand()
    {
        // TODO(SNOW-4382584): SnowflakeDbDataAdapter
        Skip.FutureMilestone();

        var command = CreateDriverCommand();
        command.CommandText = "select 1 as col1, 2 AS col2";
        var adapter = CreateDataAdapter(command);

        Assert.Equal(command.CommandText, adapter.SelectCommand!.CommandText);
    }

    [SnowflakeFact]
    public void ShouldCreateDataAdapterWithSelectCommandTextAndConnection()
    {
        // TODO(SNOW-4382584): SnowflakeDbDataAdapter
        Skip.FutureMilestone();

        var command = CreateDriverCommand();
        command.CommandText = "select 1 as col1, 2 AS col2";
        using var conn = CreateDriverConnection();
        var adapter = CreateDataAdapter(command.CommandText, conn);

        Assert.Equal(command.CommandText, adapter.SelectCommand!.CommandText);
        Assert.Equal(conn, adapter.SelectCommand.Connection);
    }

    [SnowflakeFact]
    public async Task ShouldFillDataSetFromSelectStatementAsync()
    {
        // TODO(SNOW-4382584): SnowflakeDbDataAdapter
        Skip.FutureMilestone();

        var ds = new DataSet("ds");
        using (var conn = CreateDriverConnection())
        {
            await conn.OpenAsync(TestContext.Current.CancellationToken).ConfigureAwait(false);

            var adapter = CreateDataAdapter("select 1 as col1, 2 AS col2", conn);
            adapter.Fill(ds);
            await conn.CloseAsync().ConfigureAwait(false);
        }

        Assert.Equal("Table", ds.Tables[0].TableName);
        Assert.Equal(ds.Tables[0].Rows[0].ItemArray[0], 1L);
        Assert.Equal(ds.Tables[0].Rows[0].ItemArray[1], 2L);

        if (RuntimeInformation.IsOSPlatform(OSPlatform.Windows))
        {
            Assert.Equal("1", ds.Tables[0].Rows[0]["col1"].ToString());
            Assert.Equal("2", ds.Tables[0].Rows[0]["col2"].ToString());
        }
    }

    [SnowflakeFact]
    public void ShouldSetDeleteCommandOnDataAdapter()
    {
        // TODO(SNOW-4382584): SnowflakeDbDataAdapter
        Skip.FutureMilestone();

        var adapter = CreateDataAdapter();
        var command = CreateDriverCommand();
        command.CommandText = "delete from table";
        adapter.DeleteCommand = command;

        Assert.Equal(command, adapter.DeleteCommand);
        Assert.Equal(command.CommandText, adapter.DeleteCommand.CommandText);
    }

    [SnowflakeFact]
    public void ShouldSetInsertCommandOnDataAdapter()
    {
        // TODO(SNOW-4382584): SnowflakeDbDataAdapter
        Skip.FutureMilestone();

        var adapter = CreateDataAdapter();
        var command = CreateDriverCommand();
        command.CommandText = "insert into table values (1, 2, 3)";
        adapter.InsertCommand = command;

        Assert.Equal(command, adapter.InsertCommand);
        Assert.Equal(command.CommandText, adapter.InsertCommand.CommandText);
    }

    [SnowflakeFact]
    public void ShouldSetSelectCommandOnDataAdapter()
    {
        // TODO(SNOW-4382584): SnowflakeDbDataAdapter
        Skip.FutureMilestone();

        var adapter = CreateDataAdapter();
        var command = CreateDriverCommand();
        command.CommandText = "select 1 as col1, 2 AS col2";
        adapter.SelectCommand = command;

        Assert.Equal(command, adapter.SelectCommand);
        Assert.Equal(command.CommandText, adapter.SelectCommand.CommandText);
    }

    [SnowflakeFact]
    public void ShouldSetUpdateCommandOnDataAdapter()
    {
        // TODO(SNOW-4382584): SnowflakeDbDataAdapter
        Skip.FutureMilestone();

        var adapter = CreateDataAdapter();
        var command = CreateDriverCommand();
        command.CommandText = "update table set col = 1 where col = 0";
        adapter.UpdateCommand = command;

        Assert.Equal(command, adapter.UpdateCommand);
        Assert.Equal(command.CommandText, adapter.UpdateCommand.CommandText);
    }

    private static DbCommand CreateDriverCommand() =>
        TestDriver.Current == TestDriver.Kind.Old ? new OldSnowflakeDbCommand() : new NewSnowflakeDbCommand();

    private DbConnection CreateDriverConnection()
    {
        var connectionString = Fixture.Factory.Create(Output).ConnectionString;
        return TestDriver.Current == TestDriver.Kind.Old
            ? new OldSnowflakeDbConnection(connectionString)
            : new NewSnowflakeDbConnection(connectionString);
    }

    private static IDbDataAdapter CreateDataAdapter() =>
        TestDriver.Current == TestDriver.Kind.Old ? new OldSnowflakeDbDataAdapter() : new NewSnowflakeDbDataAdapter();

    private static IDbDataAdapter CreateDataAdapter(DbCommand selectCommand) =>
        TestDriver.Current == TestDriver.Kind.Old
            ? new OldSnowflakeDbDataAdapter((OldSnowflakeDbCommand)selectCommand)
            : new NewSnowflakeDbDataAdapter((NewSnowflakeDbCommand)selectCommand);

    private static IDbDataAdapter CreateDataAdapter(string selectCommandText, DbConnection selectConnection) =>
        TestDriver.Current == TestDriver.Kind.Old
            ? new OldSnowflakeDbDataAdapter(selectCommandText, (OldSnowflakeDbConnection)selectConnection)
            : new NewSnowflakeDbDataAdapter(selectCommandText, (NewSnowflakeDbConnection)selectConnection);
}
