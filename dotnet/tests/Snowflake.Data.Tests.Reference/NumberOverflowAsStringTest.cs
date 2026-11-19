using System.Data.Common;
using Snowflake.Data.Tests.Compatibility;
using Snowflake.Data.Tests.Reference.Fixtures;
using NewSnowflakeDbCommand = Snowflake.Data.SnowflakeDbCommand;
using NewSnowflakeDbDataAdapter = Snowflake.Data.SnowflakeDbDataAdapter;
using OldSnowflakeDbCommand = Snowflake.Data.Client.SnowflakeDbCommand;
using OldSnowflakeDbDataAdapter = Snowflake.Data.Client.SnowflakeDbDataAdapter;

namespace Snowflake.Data.Tests.Reference;

public sealed class NumberOverflowAsStringTest : ReferenceTestBase
{
    private const string BigNumber = "99999999999999999999999999999999";
    private const string BigNonIntNumber = "9999999999999999999999999999999.9";

    public NumberOverflowAsStringTest(ReferenceITFixture fixture, ITestOutputHelper output)
        : base(fixture, output)
    {
    }

    [SnowflakeTheory]
    [InlineData("JSON", BigNumber, 38, 0)]
    [InlineData("JSON", BigNonIntNumber, 37, 1)]
    [InlineData("ARROW", BigNumber, 38, 0)]
    [InlineData("ARROW", BigNonIntNumber, 37, 1)]
    public async Task ShouldReturnStringFromGetValueForOverflowWhenFlagIsSetAsync(string resultFormat, string insert, int precision, int scale)
    {
        if (resultFormat == "JSON")
        {
            // TODO(SNOW-4382580): the new driver ignores DOTNET_QUERY_RESULT_FORMAT, so JSON results cannot be selected
            Skip.FutureMilestone();
        }
        // TODO(SNOW-4483010): ALLOW_NUMBER_OVERFLOW_AS_STRING result conversion
        Skip.FutureMilestone();

        var tableName = UniqueTableName();
        using var conn = OpenConnectionWithFlag(resultFormat);
        try
        {
            CreateNumberTable(conn, tableName, precision, scale);
            using var cmd1 = conn.CreateCommand();
            cmd1.CommandText = $"INSERT INTO {tableName} VALUES ('{insert}'::NUMBER({precision},{scale}))";
            await cmd1.ExecuteNonQueryAsync(TestContext.Current.CancellationToken).ConfigureAwait(false);

            using var cmd = conn.CreateCommand();
            cmd.CommandText = $"SELECT cola FROM {tableName}";
            using var reader = await cmd.ExecuteReaderAsync(TestContext.Current.CancellationToken).ConfigureAwait(false);
            Assert.True(await reader.ReadAsync(TestContext.Current.CancellationToken).ConfigureAwait(false));
            var value = reader.GetValue(0);
            Assert.IsType<string>(value);
            Assert.Equal(insert, (string)value);
        }
        finally
        {
            DropTableIfExists(conn, tableName);
        }
    }

    [SnowflakeTheory]
    [InlineData("JSON")]
    [InlineData("ARROW")]
    public async Task ShouldThrowOverflowFromGetValueWhenFlagIsNotSetAsync(string resultFormat)
    {
        if (resultFormat == "JSON")
        {
            // TODO(SNOW-4382580): the new driver ignores DOTNET_QUERY_RESULT_FORMAT, so JSON results cannot be selected
            Skip.FutureMilestone();
        }

        var tableName = UniqueTableName();
        using var conn = OpenConnectionWithoutFlag(resultFormat);
        try
        {
            CreateNumberTable(conn, tableName, 38, 0);
            using var cmd1 = conn.CreateCommand();
            cmd1.CommandText = $"INSERT INTO {tableName} VALUES ('{BigNumber}'::NUMBER(38,0))";
            await cmd1.ExecuteNonQueryAsync(TestContext.Current.CancellationToken).ConfigureAwait(false);

            using var cmd = conn.CreateCommand();
            cmd.CommandText = $"SELECT cola FROM {tableName}";
            using var reader = await cmd.ExecuteReaderAsync(TestContext.Current.CancellationToken).ConfigureAwait(false);
            Assert.True(await reader.ReadAsync(TestContext.Current.CancellationToken).ConfigureAwait(false));
            Assert.Throws<OverflowException>(() => reader.GetValue(0));
        }
        finally
        {
            DropTableIfExists(conn, tableName);
        }
    }

    [SnowflakeTheory]
    [InlineData("JSON")]
    [InlineData("ARROW")]
    public async Task ShouldReturnRawStringFromGetStringWhenFlagIsNotSetAsync(string resultFormat)
    {
        if (resultFormat == "JSON")
        {
            // TODO(SNOW-4382580): the new driver ignores DOTNET_QUERY_RESULT_FORMAT, so JSON results cannot be selected
            Skip.FutureMilestone();
        }
        // TODO(SNOW-4483010): NUMBER overflow as string via GetString
        Skip.FutureMilestone();

        using var conn = OpenConnectionWithoutFlag(resultFormat);
        var tableName = UniqueTableName();
        try
        {
            CreateNumberTable(conn, tableName, 38, 0);
            using var cmd1 = conn.CreateCommand();
            cmd1.CommandText = $"INSERT INTO {tableName} VALUES ('{BigNumber}'::NUMBER(38,0))";
            await cmd1.ExecuteNonQueryAsync(TestContext.Current.CancellationToken).ConfigureAwait(false);

            using var cmd = conn.CreateCommand();
            cmd.CommandText = $"SELECT cola FROM {tableName}";
            using var reader = await cmd.ExecuteReaderAsync(TestContext.Current.CancellationToken).ConfigureAwait(false);
            Assert.True(await reader.ReadAsync(TestContext.Current.CancellationToken).ConfigureAwait(false));
            Assert.Equal(BigNumber, reader.GetString(0));
        }
        finally
        {
            DropTableIfExists(conn, tableName);
        }
    }

    [SnowflakeTheory]
    [InlineData("JSON")]
    [InlineData("ARROW")]
    public async Task ShouldThrowOverflowFromDataAdapterFillWhenFlagIsNotSetAsync(string resultFormat)
    {
        if (resultFormat == "JSON")
        {
            // TODO(SNOW-4382580): the new driver ignores DOTNET_QUERY_RESULT_FORMAT, so JSON results cannot be selected
            Skip.FutureMilestone();
        }
        // TODO(SNOW-4382584): SnowflakeDbDataAdapter is a stub
        Skip.FutureMilestone();

        using var conn = OpenConnectionWithoutFlag(resultFormat);
        var tableName = UniqueTableName();
        try
        {
            CreateNumberTable(conn, tableName, 38, 0);
            using var cmd = conn.CreateCommand();
            cmd.CommandText = $"INSERT INTO {tableName} VALUES ('{BigNumber}'::NUMBER(38,0))";
            await cmd.ExecuteNonQueryAsync(TestContext.Current.CancellationToken).ConfigureAwait(false);

            using var select = conn.CreateCommand();
            select.CommandText = $"SELECT cola FROM {tableName}";
            using var adapter = CreateAdapter(select);
            var dataTable = new DataTable();
            Assert.Throws<OverflowException>(() => adapter.Fill(dataTable));
        }
        finally
        {
            DropTableIfExists(conn, tableName);
        }
    }

    [SnowflakeTheory]
    [InlineData("JSON")]
    [InlineData("ARROW")]
    public async Task ShouldStoreOverflowAsStringInPreTypedColumnOnDataAdapterFillAsync(string resultFormat)
    {
        if (resultFormat == "JSON")
        {
            // TODO(SNOW-4382580): the new driver ignores DOTNET_QUERY_RESULT_FORMAT, so JSON results cannot be selected
            Skip.FutureMilestone();
        }
        // TODO(SNOW-4382584): SnowflakeDbDataAdapter is a stub
        Skip.FutureMilestone();

        using var conn = OpenConnectionWithFlag(resultFormat);
        var tableName = UniqueTableName();
        try
        {
            CreateNumberTable(conn, tableName, 38, 0);
            using var cmd = conn.CreateCommand();
            cmd.CommandText = $"INSERT INTO {tableName} VALUES ('{BigNumber}'::NUMBER(38,0))";
            await cmd.ExecuteNonQueryAsync(TestContext.Current.CancellationToken).ConfigureAwait(false);

            var dataTable = new DataTable();
            dataTable.Columns.Add("COLA", typeof(string));

            using var select = conn.CreateCommand();
            select.CommandText = $"SELECT cola FROM {tableName}";
            using var adapter = CreateAdapter(select);
            adapter.Fill(dataTable);

            Assert.Equal(1, dataTable.Rows.Count);
            Assert.Equal(BigNumber, (string)dataTable.Rows[0]["COLA"]);
        }
        finally
        {
            DropTableIfExists(conn, tableName);
        }
    }

    [SnowflakeTheory]
    [InlineData("JSON")]
    [InlineData("ARROW")]
    public async Task ShouldReadMixedOverflowAndRegularRowsAsync(string resultFormat)
    {
        if (resultFormat == "JSON")
        {
            // TODO(SNOW-4382580): the new driver ignores DOTNET_QUERY_RESULT_FORMAT, so JSON results cannot be selected
            Skip.FutureMilestone();
        }
        // TODO(SNOW-4483010): mixed NUMBER overflow/non-overflow GetValue types
        Skip.FutureMilestone();

        const string SmallNumber = "42";
        var tableName = UniqueTableName();

        using var conn = OpenConnectionWithFlag(resultFormat);
        try
        {
            CreateNumberTable(conn, tableName, 38, 0);
            using var cmd = conn.CreateCommand();
            cmd.CommandText =
                $"INSERT INTO {tableName} VALUES ('{SmallNumber}'::NUMBER(38,0)), ('{BigNumber}'::NUMBER(38,0))";
            await cmd.ExecuteNonQueryAsync(TestContext.Current.CancellationToken).ConfigureAwait(false);

            cmd.CommandText = $"SELECT cola FROM {tableName} ORDER BY cola";
            using var reader = await cmd.ExecuteReaderAsync(TestContext.Current.CancellationToken).ConfigureAwait(false);
            Assert.True(await reader.ReadAsync(TestContext.Current.CancellationToken).ConfigureAwait(false));
            var smallVal = reader.GetValue(0);
            if (resultFormat == "JSON")
                Assert.Equal(42L, smallVal);
            else
                Assert.Equal(42m, smallVal);

            Assert.True(await reader.ReadAsync(TestContext.Current.CancellationToken).ConfigureAwait(false));
            Assert.IsType<string>(reader.GetValue(0));
            Assert.Equal(BigNumber, (string)reader.GetValue(0));

            Assert.False(await reader.ReadAsync(TestContext.Current.CancellationToken).ConfigureAwait(false));
        }
        finally
        {
            DropTableIfExists(conn, tableName);
        }
    }

    private ISnowflakeDbConnection OpenConnectionWithFlag(string resultFormat)
    {
        var baseConnectionString = Fixture.Factory.Create(Output).ConnectionString;
        var conn = Fixture.Factory.Create(
            Output,
            AppendParam(baseConnectionString, "ALLOW_NUMBER_OVERFLOW_AS_STRING=true;"));
        conn.Open();
        SetResultFormat(conn, resultFormat);
        return conn;
    }

    private ISnowflakeDbConnection OpenConnectionWithoutFlag(string resultFormat)
    {
        var conn = Fixture.Factory.Create(Output);
        conn.Open();
        SetResultFormat(conn, resultFormat);
        return conn;
    }

    private static void SetResultFormat(ISnowflakeDbConnection conn, string resultFormat)
    {
        using var command = conn.CreateCommand();
        command.CommandText = resultFormat == "ARROW"
            ? "ALTER SESSION SET DOTNET_QUERY_RESULT_FORMAT = ARROW_FORCE"
            : $"ALTER SESSION SET DOTNET_QUERY_RESULT_FORMAT = {resultFormat}";
        try
        {
            command.ExecuteNonQuery();
        }
        catch (DbException ex) when (ex.Message.Contains("invalid parameter"))
        {
        }
    }

    private static string UniqueTableName() => $"NUMBER_OVERFLOW_{Guid.NewGuid():N}";

    private static void CreateNumberTable(ISnowflakeDbConnection conn, string tableName, int precision, int scale)
    {
        using var command = conn.CreateCommand();
        command.CommandText = $"CREATE OR REPLACE TABLE {tableName} (cola NUMBER({precision},{scale}))";
        command.ExecuteNonQuery();
    }

    private static void DropTableIfExists(ISnowflakeDbConnection conn, string tableName)
    {
        using var command = conn.CreateCommand();
        command.CommandText = $"DROP TABLE IF EXISTS {tableName}";
        command.ExecuteNonQuery();
    }

    private static string AppendParam(string connectionString, string extra)
    {
        if (connectionString.EndsWith(";", StringComparison.Ordinal))
            return connectionString + extra;
        return connectionString + ";" + extra;
    }

    private static DbDataAdapter CreateAdapter(DbCommand selectCommand)
    {
        if (TestDriver.Current == TestDriver.Kind.Old)
            return new OldSnowflakeDbDataAdapter((OldSnowflakeDbCommand)selectCommand);
        return new NewSnowflakeDbDataAdapter((NewSnowflakeDbCommand)selectCommand);
    }
}
