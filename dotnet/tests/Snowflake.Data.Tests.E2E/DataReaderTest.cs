extern alias OldDriver;
using System.Data.Common;
using System.Globalization;
using Snowflake.Data.Client;
using Snowflake.Data.Core;
using Snowflake.Data.Tests.Compatibility;
using Snowflake.Data.Tests.Utilities;
using OldSnowflakeDbDataReader = OldDriver::Snowflake.Data.Client.SnowflakeDbDataReader;
using OldSnowflakeDbException = OldDriver::Snowflake.Data.Client.SnowflakeDbException;

namespace Snowflake.Data.Tests.E2E;

[Trait("Category", "E2E")]
[Trait("Category", "Conversion")]
public class DataReaderTest : IClassFixture<ITFixture>
{
    protected readonly ITestOutputHelper Output;
    protected readonly ITFixture Fixture;

    public DataReaderTest(ITFixture fixture, ITestOutputHelper output)
    {
        Fixture = fixture;
        Output = output;
    }

    [SnowflakeFact]
    public async Task ReadsIntegerAndTextColumnsWithTheirMetadataAsync()
    {
        var ct = TestContext.Current.CancellationToken;
        using var connection = Fixture.Factory.Create(Output);
        await connection.OpenAsync(ct).ConfigureAwait(false);
        using var cmd = connection.CreateCommand();
        cmd.CommandText = "SELECT 42::NUMBER(38,0) AS ID, 'żółw'::VARCHAR AS NAME, NULL::NUMBER(38,0) AS NO_ID, NULL::VARCHAR AS NO_NAME";
        using var reader = await cmd.ExecuteReaderAsync(ct).ConfigureAwait(false);

        Assert.Equal(4, reader.FieldCount);
        Assert.Equal("ID", reader.GetName(0));
        Assert.Equal("FIXED", reader.GetDataTypeName(0));
        Assert.Equal("TEXT", reader.GetDataTypeName(1));
        Assert.Equal(typeof(long), reader.GetFieldType(0));
        Assert.Equal(typeof(string), reader.GetFieldType(1));
        Assert.True(reader.HasRows);
        Assert.True(await reader.ReadAsync(ct).ConfigureAwait(false));
        Assert.Equal(42L, Assert.IsType<long>(reader.GetValue(0)));
        Assert.Equal(42, reader.GetInt32(0));
        Assert.Equal("42", reader.GetString(0));
        Assert.Equal("żółw", Assert.IsType<string>(reader.GetValue(1)));
        Assert.True(reader.IsDBNull(2));
        Assert.Same(DBNull.Value, reader.GetValue(2));
        Assert.True(reader.IsDBNull(3));
        Assert.Null(reader.GetString(3));
        Assert.False(await reader.ReadAsync(ct).ConfigureAwait(false));
    }

    [SnowflakeFact]
    public async Task DescribesColumnsInSchemaTableAsync()
    {
        var ct = TestContext.Current.CancellationToken;
        using var connection = Fixture.Factory.Create(Output);
        await connection.OpenAsync(ct).ConfigureAwait(false);
        using var cmd = connection.CreateCommand();
        cmd.CommandText = "SELECT NULL::VARCHAR AS JUSTMAYBE, 'YOURE'::VARCHAR(100) AS GONNABETHE, 1::NUMBER(20,4) AS THATSAVESME, 1::NUMBER(38,0) AS ID, TRUE AS FLAG";
        using var reader = await cmd.ExecuteReaderAsync(ct).ConfigureAwait(false);

        var table = reader.GetSchemaTable();

        Assert.NotNull(table);
        Assert.Same(table, reader.GetSchemaTable());
        Assert.Equal(5, table.Rows.Count);
        var maybe = table.Rows[0];
        Assert.Equal("JUSTMAYBE", maybe[SchemaTableColumn.ColumnName]);
        var varchar = table.Rows[1];
        Assert.Equal("GONNABETHE", varchar[SchemaTableColumn.ColumnName]);
        Assert.Equal(1, varchar[SchemaTableColumn.ColumnOrdinal]);
        Assert.Equal(100, varchar[SchemaTableColumn.ColumnSize]);
        Assert.Equal(0, varchar[SchemaTableColumn.NumericPrecision]);
        Assert.Equal(0, varchar[SchemaTableColumn.NumericScale]);
        Assert.Equal(typeof(string), varchar[SchemaTableColumn.DataType]);
        Assert.Equal(SFDataType.TEXT, (SFDataType)varchar[SchemaTableColumn.ProviderType]);
        Assert.True((bool)maybe[SchemaTableColumn.AllowDBNull]);
        var num = table.Rows[2];
        Assert.Equal("THATSAVESME", num[SchemaTableColumn.ColumnName]);
        Assert.Equal(2, num[SchemaTableColumn.ColumnOrdinal]);
        Assert.Equal(20, num[SchemaTableColumn.NumericPrecision]);
        Assert.Equal(4, num[SchemaTableColumn.NumericScale]);
        Assert.Equal(0, num[SchemaTableColumn.ColumnSize]);
        Assert.Equal(typeof(decimal), num[SchemaTableColumn.DataType]);
        Assert.Equal(SFDataType.FIXED, (SFDataType)num[SchemaTableColumn.ProviderType]);

        var id = table.Rows[3];
        Assert.Equal(0, id[SchemaTableColumn.ColumnSize]);
        Assert.Equal(typeof(long), id[SchemaTableColumn.DataType]);
        Assert.Equal(0, id[SchemaTableColumn.NumericScale]);
        var flag = table.Rows[4];
        Assert.Equal(typeof(bool), flag[SchemaTableColumn.DataType]);
        Assert.Equal(SFDataType.BOOLEAN, (SFDataType)flag[SchemaTableColumn.ProviderType]);
    }

    [SnowflakeFact]
    public async Task SchemaTableOfSchemaOnlyReaderListsColumnsWithoutRowsAsync()
    {
        var ct = TestContext.Current.CancellationToken;
        using var connection = Fixture.Factory.Create(Output);
        await connection.OpenAsync(ct).ConfigureAwait(false);
        using var cmd = connection.CreateCommand();
        cmd.CommandText = "SELECT 1::NUMBER(38,0) AS ID, 'a'::VARCHAR AS NAME";
        using var reader = await cmd.ExecuteReaderAsync(CommandBehavior.SchemaOnly, ct).ConfigureAwait(false);

        var table = reader.GetSchemaTable();

        Assert.NotNull(table);
        Assert.Equal(["ID", "NAME"], table.Rows.Cast<DataRow>().Select(row => (string)row[SchemaTableColumn.ColumnName]));
        Assert.False(reader.HasRows);
    }

    [SnowflakeFact]
    public async Task SchemaTableStaysAvailableAfterReaderIsClosedAsync()
    {
        var ct = TestContext.Current.CancellationToken;
        using var connection = Fixture.Factory.Create(Output);
        await connection.OpenAsync(ct).ConfigureAwait(false);
        using var cmd = connection.CreateCommand();
        cmd.CommandText = "SELECT 1::NUMBER(38,0) AS ID";
        var reader = await cmd.ExecuteReaderAsync(ct).ConfigureAwait(false);
        reader.Close();

        var table = reader.GetSchemaTable();

        Assert.NotNull(table);
        Assert.Equal("ID", Assert.Single(table.Rows.Cast<DataRow>())[SchemaTableColumn.ColumnName]);
    }

    [SnowflakeFact]
    public async Task ReadsEveryRowOfAResultSpanningManyChunksAsync()
    {
        const int rowCount = 300_000;
        var ct = TestContext.Current.CancellationToken;
        using var connection = Fixture.Factory.Create(Output);
        await connection.OpenAsync(ct).ConfigureAwait(false);
        using var cmd = connection.CreateCommand();
        cmd.CommandText = $"SELECT SEQ8() AS N, RANDSTR(20, RANDOM()) AS S FROM TABLE(GENERATOR(ROWCOUNT => {rowCount}))";
        using var reader = await cmd.ExecuteReaderAsync(ct).ConfigureAwait(false);

        long rows = 0;
        long sum = 0;
        while (reader.Read())
        {
            rows++;
            sum += reader.GetInt64(0);
            Assert.Equal(20, reader.GetString(1).Length);
        }

        Assert.Equal(rowCount, rows);
        Assert.Equal((long)rowCount * (rowCount - 1) / 2, sum);
    }

    [SnowflakeFact]
    public async Task ReadAsyncReadsEveryRowOfAResultSpanningManyChunksAsync()
    {
        const int rowCount = 300_000;
        var ct = TestContext.Current.CancellationToken;
        using var connection = Fixture.Factory.Create(Output);
        await connection.OpenAsync(ct).ConfigureAwait(false);
        using var cmd = connection.CreateCommand();
        cmd.CommandText = $"SELECT ROW_NUMBER() OVER (ORDER BY SEQ8()) AS N FROM TABLE(GENERATOR(ROWCOUNT => {rowCount}))";
        using var reader = await cmd.ExecuteReaderAsync(ct).ConfigureAwait(false);

        long rows = 0;
        while (await reader.ReadAsync(ct).ConfigureAwait(false))
        {
            Assert.Equal(++rows, reader.GetInt64(0));
        }

        Assert.Equal(rowCount, rows);
    }

    [SnowflakeFact]
    public async Task ClosingAReaderBeforeTheLastRowLeavesTheConnectionUsableAsync()
    {
        var ct = TestContext.Current.CancellationToken;
        using var connection = Fixture.Factory.Create(Output);
        await connection.OpenAsync(ct).ConfigureAwait(false);
        using (var cmd = connection.CreateCommand())
        {
            cmd.CommandText = "SELECT SEQ8() FROM TABLE(GENERATOR(ROWCOUNT => 300000))";
            var reader = await cmd.ExecuteReaderAsync(ct).ConfigureAwait(false);
            Assert.True(await reader.ReadAsync(ct).ConfigureAwait(false));
            Assert.Equal(0L, reader.GetInt64(0));
            reader.Dispose();
            Assert.True(reader.IsClosed);
        }

        using var next = connection.CreateCommand();
        next.CommandText = "SELECT 'still open'";
        Assert.Equal("still open", await next.ExecuteScalarAsync(ct).ConfigureAwait(false));
    }

    [SnowflakeFact]
    public async Task HasRowsReportsWhetherTheResultHasRowsAsync()
    {
        var ct = TestContext.Current.CancellationToken;
        using var connection = Fixture.Factory.Create(Output);
        await connection.OpenAsync(ct).ConfigureAwait(false);

        using (var cmd = connection.CreateCommand())
        {
            cmd.CommandText = "SELECT 1 WHERE 1 = 0";
            using var reader = await cmd.ExecuteReaderAsync(ct).ConfigureAwait(false);
            Assert.False(reader.HasRows);
            Assert.False(await reader.ReadAsync(ct).ConfigureAwait(false));
        }

        using (var cmd = connection.CreateCommand())
        {
            cmd.CommandText = "SELECT 1";
            using var reader = await cmd.ExecuteReaderAsync(ct).ConfigureAwait(false);
            Assert.True(reader.HasRows);
            Assert.True(await reader.ReadAsync(ct).ConfigureAwait(false));
            Assert.True(reader.HasRows);
        }
    }

    [SnowflakeFact]
    public async Task GetOrdinalFindsAColumnIgnoringCaseAndRejectsAMissingOneAsync()
    {
        var ct = TestContext.Current.CancellationToken;
        using var connection = Fixture.Factory.Create(Output);
        await connection.OpenAsync(ct).ConfigureAwait(false);
        using var cmd = connection.CreateCommand();
        cmd.CommandText = "SELECT 1 AS FIRST_COL, 2 AS SECOND_COL";
        using var reader = await cmd.ExecuteReaderAsync(ct).ConfigureAwait(false);

        Assert.True(await reader.ReadAsync(ct).ConfigureAwait(false));

        Assert.Equal(1, reader.GetOrdinal("SECOND_COL"));
        if (BehaviorDifference.NewDriverOnly("BD#reader-get-ordinal-missing-column-throws"))
        {
            Assert.Equal(1, reader.GetOrdinal("second_col"));
            Assert.Equal(2L, reader["second_col"]);
            Assert.Equal(270002, Assert.Throws<SnowflakeDbException>(() => reader.GetOrdinal("MISSING")).ErrorCode);
            Assert.Equal(270002, Assert.Throws<SnowflakeDbException>(() => reader["MISSING"]).ErrorCode);
        }
        else
        {
            Assert.Equal(-1, reader.GetOrdinal("second_col"));
            Assert.Equal(-1, reader.GetOrdinal("MISSING"));
            Assert.Equal(270002, Assert.Throws<OldSnowflakeDbException>(() => reader["second_col"]).ErrorCode);
            Assert.Equal(270002, Assert.Throws<OldSnowflakeDbException>(() => reader["MISSING"]).ErrorCode);
        }
    }

    [SnowflakeTheory]
    [InlineData(false)]
    [InlineData(true)]
    public async Task ReadingAValueWithNoCurrentRowIsDataReaderNoCurrentRowAsync(bool readAsync)
    {
        Skip.OnOldDriver("BD#reader-access-without-current-row-error-code", "the old driver throws an exception that depends on the result format the server picks");
        var ct = TestContext.Current.CancellationToken;
        using var connection = Fixture.Factory.Create(Output);
        await connection.OpenAsync(ct).ConfigureAwait(false);
        using var cmd = connection.CreateCommand();
        cmd.CommandText = "SELECT 1";
        using var reader = await cmd.ExecuteReaderAsync(ct).ConfigureAwait(false);
        Func<Task<bool>> read = readAsync ? () => reader.ReadAsync(ct) : () => Task.FromResult(reader.Read());

        Assert.Equal(270073, Assert.Throws<SnowflakeDbException>(() => reader.GetValue(0)).ErrorCode);
        Assert.True(await read().ConfigureAwait(false));
        Assert.False(await read().ConfigureAwait(false));
        Assert.Equal(270073, Assert.Throws<SnowflakeDbException>(() => reader.GetInt64(0)).ErrorCode);
    }

    [SnowflakeFact]
    public async Task ConversionFailuresAreInvalidDataConversionAsync()
    {
        var ct = TestContext.Current.CancellationToken;
        using var connection = Fixture.Factory.Create(Output);
        await connection.OpenAsync(ct).ConfigureAwait(false);
        using var cmd = connection.CreateCommand();
        cmd.CommandText = "SELECT 3000000000::NUMBER(38,0), NULL::NUMBER(38,0), 'abc'::VARCHAR, ''::VARCHAR, 'ab'::VARCHAR";
        using var reader = await cmd.ExecuteReaderAsync(ct).ConfigureAwait(false);
        Assert.True(await reader.ReadAsync(ct).ConfigureAwait(false));

        if (BehaviorDifference.OldDriverOnly("BD#reader-conversion-failures-throw-invalid-data-conversion"))
        {
            Assert.Throws<OverflowException>(() => reader.GetInt32(0));
            Assert.Throws<InvalidCastException>(() => reader.GetInt64(1));
            Assert.Throws<FormatException>(() => reader.GetInt64(2));
            Assert.Throws<IndexOutOfRangeException>(() => reader.GetChar(3));
            Assert.Equal('a', reader.GetChar(4));
            var oldReader = Assert.IsType<OldSnowflakeDbDataReader>(reader);
            Assert.Equal(270003, Assert.Throws<OldSnowflakeDbException>(() => oldReader.GetTimeSpan(0)).ErrorCode);
            return;
        }

        var newReader = Assert.IsType<SnowflakeDbDataReader>(reader);
        Assert.Equal(270003, Assert.Throws<SnowflakeDbException>(() => reader.GetInt32(0)).ErrorCode);
        Assert.Equal(270003, Assert.Throws<SnowflakeDbException>(() => reader.GetInt64(1)).ErrorCode);
        Assert.Equal(270003, Assert.Throws<SnowflakeDbException>(() => reader.GetInt64(2)).ErrorCode);
        Assert.Equal(270003, Assert.Throws<SnowflakeDbException>(() => reader.GetBoolean(2)).ErrorCode);
        Assert.Equal(270003, Assert.Throws<SnowflakeDbException>(() => reader.GetChar(3)).ErrorCode);
        Assert.Equal(270003, Assert.Throws<SnowflakeDbException>(() => reader.GetChar(4)).ErrorCode);
        Assert.Equal(270003, Assert.Throws<SnowflakeDbException>(() => newReader.GetTimeSpan(0)).ErrorCode);
    }

    [SnowflakeFact]
    public async Task GetStringOnANegativeNumberIgnoresTheCurrentCultureAsync()
    {
        Skip.OnOldDriver("BD#reader-get-string-invariant-culture", "the old driver's text depends on the result format the server picks");
        var ct = TestContext.Current.CancellationToken;
        var original = CultureInfo.CurrentCulture;
        CultureInfo.CurrentCulture = TestCultures.UnicodeMinus();
        try
        {
            using var connection = Fixture.Factory.Create(Output);
            await connection.OpenAsync(ct).ConfigureAwait(false);
            using var cmd = connection.CreateCommand();
            cmd.CommandText = "SELECT -5";
            using var reader = await cmd.ExecuteReaderAsync(ct).ConfigureAwait(false);
            Assert.True(await reader.ReadAsync(ct).ConfigureAwait(false));

            Assert.Equal("-5", reader.GetString(0));
        }
        finally
        {
            CultureInfo.CurrentCulture = original;
        }
    }

    [SnowflakeFact]
    public async Task AnIntegerOutsideTheLongRangeIsADecimalAsync()
    {
        Skip.OnOldDriver("BD#reader-fixed-beyond-int64-returns-decimal-on-json", "the old driver's result depends on the result format the server picks");
        var ct = TestContext.Current.CancellationToken;
        using var connection = Fixture.Factory.Create(Output);
        await connection.OpenAsync(ct).ConfigureAwait(false);
        using var cmd = connection.CreateCommand();
        cmd.CommandText = "SELECT 12345678901234567890123::NUMBER(38,0)";
        using var reader = await cmd.ExecuteReaderAsync(ct).ConfigureAwait(false);
        Assert.True(await reader.ReadAsync(ct).ConfigureAwait(false));

        Assert.Equal(12345678901234567890123m, Assert.IsType<decimal>(reader.GetValue(0)));
        Assert.Equal("12345678901234567890123", reader.GetString(0));
    }

    [SnowflakeFact]
    public async Task AnIntegerOutsideTheDecimalRangeIsInvalidDataConversionAsync()
    {
        var ct = TestContext.Current.CancellationToken;
        using var connection = Fixture.Factory.Create(Output);
        await connection.OpenAsync(ct).ConfigureAwait(false);
        using var cmd = connection.CreateCommand();
        cmd.CommandText = "SELECT 99999999999999999999999999999999999999::NUMBER(38,0)";
        using var reader = await cmd.ExecuteReaderAsync(ct).ConfigureAwait(false);
        Assert.True(await reader.ReadAsync(ct).ConfigureAwait(false));

        Assert.Equal("99999999999999999999999999999999999999", reader.GetString(0));
        if (BehaviorDifference.OldDriverOnly("BD#reader-conversion-failures-throw-invalid-data-conversion"))
        {
            Assert.Throws<OverflowException>(() => reader.GetValue(0));
            return;
        }

        Assert.Equal(270003, Assert.Throws<SnowflakeDbException>(() => reader.GetValue(0)).ErrorCode);
    }
}
