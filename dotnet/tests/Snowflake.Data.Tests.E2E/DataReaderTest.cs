using System.Data.Common;
using System.Globalization;
using Snowflake.Data.Tests.Compatibility;

namespace Snowflake.Data.Tests.E2E;

[Trait("Category", "E2E")]
public class DataReaderTest : IClassFixture<ITFixture>
{
    protected readonly ITestOutputHelper Output;
    protected readonly ITFixture Fixture;

    public DataReaderTest(ITFixture fixture, ITestOutputHelper output)
    {
        Fixture = fixture;
        Output = output;
    }

    [SnowflakeTheory]
    [InlineData("ARROW")]
    [InlineData("JSON")]
    public async Task ReadsIntegerAndTextColumnsWithTheirMetadataAsync(string resultFormat)
    {
        using var connection = await OpenAsync(resultFormat, CancellationToken.None).ConfigureAwait(false);
        using var cmd = connection.CreateCommand();
        cmd.CommandText = "SELECT 42::NUMBER(38,0) AS ID, 'żółw'::VARCHAR AS NAME, NULL::NUMBER(38,0) AS NO_ID, NULL::VARCHAR AS NO_NAME";
        using var reader = await cmd.ExecuteReaderAsync().ConfigureAwait(false);

        Assert.Equal(4, reader.FieldCount);
        Assert.Equal("ID", reader.GetName(0));
        Assert.Equal("FIXED", reader.GetDataTypeName(0));
        Assert.Equal("TEXT", reader.GetDataTypeName(1));
        Assert.Equal(typeof(long), reader.GetFieldType(0));
        Assert.Equal(typeof(string), reader.GetFieldType(1));
        Assert.True(reader.HasRows);
        Assert.True(await reader.ReadAsync().ConfigureAwait(false));
        Assert.Equal(42L, Assert.IsType<long>(reader.GetValue(0)));
        Assert.Equal(42, reader.GetInt32(0));
        Assert.Equal("42", reader.GetString(0));
        Assert.Equal("żółw", Assert.IsType<string>(reader.GetValue(1)));
        Assert.True(reader.IsDBNull(2));
        Assert.Same(DBNull.Value, reader.GetValue(2));
        Assert.True(reader.IsDBNull(3));
        Assert.Null(reader.GetString(3));
        Assert.False(await reader.ReadAsync().ConfigureAwait(false));
    }

    [SnowflakeTheory]
    [InlineData("ARROW")]
    [InlineData("JSON")]
    public void ReadsEveryRowOfAResultSpanningManyChunks(string resultFormat)
    {
        const int rowCount = 300_000;
        using var connection = Open(resultFormat);
        using var cmd = connection.CreateCommand();
        cmd.CommandText = $"SELECT SEQ8() AS N, RANDSTR(20, RANDOM()) AS S FROM TABLE(GENERATOR(ROWCOUNT => {rowCount}))";
        using var reader = cmd.ExecuteReader();

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

    [SnowflakeTheory]
    [InlineData("ARROW")]
    [InlineData("JSON")]
    public async Task ReadAsyncReadsEveryRowOfAResultSpanningManyChunksAsync(string resultFormat)
    {
        const int rowCount = 300_000;
        using var connection = await OpenAsync(resultFormat, CancellationToken.None).ConfigureAwait(false);
        using var cmd = connection.CreateCommand();
        cmd.CommandText = $"SELECT SEQ8() AS N FROM TABLE(GENERATOR(ROWCOUNT => {rowCount}))";
        using var reader = await cmd.ExecuteReaderAsync().ConfigureAwait(false);

        long rows = 0;
        while (await reader.ReadAsync().ConfigureAwait(false))
        {
            Assert.Equal(rows, reader.GetInt64(0));
            rows++;
        }

        Assert.Equal(rowCount, rows);
    }

    [SnowflakeFact]
    public void ClosingAReaderBeforeTheLastRowLeavesTheConnectionUsable()
    {
        using var connection = Open("ARROW");
        using (var cmd = connection.CreateCommand())
        {
            cmd.CommandText = "SELECT SEQ8() FROM TABLE(GENERATOR(ROWCOUNT => 300000))";
            var reader = cmd.ExecuteReader();
            Assert.True(reader.Read());
            Assert.Equal(0L, reader.GetInt64(0));
            reader.Dispose();
            Assert.True(reader.IsClosed);
        }

        using var next = connection.CreateCommand();
        next.CommandText = "SELECT 'still open'";
        Assert.Equal("still open", next.ExecuteScalar());
    }

    [SnowflakeTheory]
    [InlineData("ARROW")]
    [InlineData("JSON")]
    public void HasRowsReportsWhetherTheResultHasRows(string resultFormat)
    {
        using var connection = Open(resultFormat);

        using (var reader = ExecuteReader(connection, "SELECT 1 WHERE 1 = 0"))
        {
            Assert.False(reader.HasRows);
            Assert.False(reader.Read());
        }

        using (var reader = ExecuteReader(connection, "SELECT 1"))
        {
            Assert.True(reader.HasRows);
            Assert.True(reader.Read());
            Assert.True(reader.HasRows);
        }
    }

    [SnowflakeFact]
    public void GetOrdinalFindsAColumnIgnoringCaseAndRejectsAMissingOne()
    {
        using var connection = Open("ARROW");
        using var reader = ExecuteReader(connection, "SELECT 1 AS FIRST_COL, 2 AS SECOND_COL");

        Assert.Equal(1, reader.GetOrdinal("SECOND_COL"));
        if (BehaviorDifference.NewDriverOnly("BD#reader-get-ordinal-missing-column-throws"))
        {
            Assert.Equal(1, reader.GetOrdinal("second_col"));
            SkipUntilPublicExceptionsExist();
            Assert.Equal(270002, Assert.Throws<SnowflakeDbException>(() => reader.GetOrdinal("MISSING")).ErrorCode);
        }
        else
        {
            Assert.Equal(-1, reader.GetOrdinal("second_col"));
            Assert.Equal(-1, reader.GetOrdinal("MISSING"));
        }
    }

    [SnowflakeFact]
    public void ReadingAValueWithNoCurrentRowIsDataReaderNoCurrentRow()
    {
        Skip.OnOldDriver("BD#reader-access-without-current-row-error-code", "the old driver throws a format-dependent exception");
        SkipUntilPublicExceptionsExist();
        using var connection = Open("ARROW");
        using var reader = ExecuteReader(connection, "SELECT 1");

        Assert.Equal(270073, Assert.Throws<SnowflakeDbException>(() => reader.GetValue(0)).ErrorCode);
        Assert.True(reader.Read());
        Assert.False(reader.Read());
        Assert.Equal(270073, Assert.Throws<SnowflakeDbException>(() => reader.GetInt64(0)).ErrorCode);
    }

    [SnowflakeTheory]
    [InlineData("ARROW")]
    [InlineData("JSON")]
    public void ConversionFailuresAreInvalidDataConversion(string resultFormat)
    {
        Skip.OnOldDriver("BD#reader-conversion-failures-throw-invalid-data-conversion", "the old driver throws format-dependent BCL exceptions");
        SkipUntilPublicExceptionsExist();
        using var connection = Open(resultFormat);
        using var reader = ExecuteReader(connection, "SELECT 3000000000::NUMBER(38,0), NULL::NUMBER(38,0), 'abc'::VARCHAR, ''::VARCHAR");
        Assert.True(reader.Read());

        Assert.Equal(270003, Assert.Throws<SnowflakeDbException>(() => reader.GetInt32(0)).ErrorCode);
        Assert.Equal(270003, Assert.Throws<SnowflakeDbException>(() => reader.GetInt64(1)).ErrorCode);
        Assert.Equal(270003, Assert.Throws<SnowflakeDbException>(() => reader.GetInt64(2)).ErrorCode);
        Assert.Equal(270003, Assert.Throws<SnowflakeDbException>(() => reader.GetBoolean(2)).ErrorCode);
        Assert.Equal(270003, Assert.Throws<SnowflakeDbException>(() => reader.GetChar(3)).ErrorCode);
    }

    [SnowflakeFact]
    public void GetStringOnANegativeNumberIgnoresTheCurrentCulture()
    {
        Skip.OnOldDriver("BD#reader-get-string-invariant-culture", "the old driver's text depends on the result format the server picks");
        var original = CultureInfo.CurrentCulture;
        // sv-SE formats negative numbers with U+2212 MINUS SIGN instead of '-'.
        CultureInfo.CurrentCulture = CultureInfo.GetCultureInfo("sv-SE");
        try
        {
            using var connection = Open("ARROW");
            using var reader = ExecuteReader(connection, "SELECT -5");
            Assert.True(reader.Read());

            Assert.Equal("-5", reader.GetString(0));
        }
        finally
        {
            CultureInfo.CurrentCulture = original;
        }
    }

    [SnowflakeTheory]
    [InlineData("ARROW")]
    [InlineData("JSON")]
    public void AnIntegerOutsideTheLongRangeIsADecimal(string resultFormat)
    {
        using var connection = Open(resultFormat);
        using var reader = ExecuteReader(connection, "SELECT 12345678901234567890123::NUMBER(38,0)");
        Assert.True(reader.Read());

        if (resultFormat == "JSON" && BehaviorDifference.OldDriverOnly("BD#reader-fixed-beyond-int64-returns-decimal-on-json"))
        {
            Assert.Throws<OverflowException>(() => reader.GetValue(0));
            return;
        }

        Assert.Equal(12345678901234567890123m, Assert.IsType<decimal>(reader.GetValue(0)));
        Assert.Equal("12345678901234567890123", reader.GetString(0));
    }

    // TODO SNOW-4382591: remove once SnowflakeDbException can be constructed.
    private static void SkipUntilPublicExceptionsExist() =>
        Skip.When(TestDriver.Current == TestDriver.Kind.New, "every SnowflakeDbException constructor throws NotImplementedException until SNOW-4382591");

    private ISnowflakeDbConnection Open(string resultFormat)
    {
        var connection = Fixture.Factory.Create(Output);
        connection.Open();
        using var cmd = connection.CreateCommand();
        cmd.CommandText = $"ALTER SESSION SET DOTNET_QUERY_RESULT_FORMAT = {resultFormat}";
        cmd.ExecuteNonQuery();
        return connection;
    }

    private async Task<ISnowflakeDbConnection> OpenAsync(string resultFormat, CancellationToken cancellationToken)
    {
        var connection = Fixture.Factory.Create(Output);
        await connection.OpenAsync(cancellationToken).ConfigureAwait(false);
        using var cmd = connection.CreateCommand();
        cmd.CommandText = $"ALTER SESSION SET DOTNET_QUERY_RESULT_FORMAT = {resultFormat}";
        await cmd.ExecuteNonQueryAsync(cancellationToken).ConfigureAwait(false);
        return connection;
    }

    private static DbDataReader ExecuteReader(ISnowflakeDbConnection connection, string sql)
    {
        using var cmd = connection.CreateCommand();
        cmd.CommandText = sql;
        return cmd.ExecuteReader();
    }
}
