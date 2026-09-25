using Snowflake.Data.Tests.Compatibility;

namespace Snowflake.Data.Tests;

[Trait("Category", "E2E")]
public class NumberTest : IClassFixture<ITFixture>
{
    private static readonly string[] NumberTypeSynonyms =
        ["NUMBER", "NUMERIC", "DECIMAL", "DEC"];

    protected readonly ITestOutputHelper Output;
    protected readonly ITFixture Fixture;

    public NumberTest(ITFixture fixture, ITestOutputHelper output)
    {
        Fixture = fixture;
        Output = output;
    }

    // Scenario: should cast number values to appropriate type for number and synonyms
    [SnowflakeTheory]
    [InlineData("NUMBER")]
    [InlineData("NUMERIC")]
    [InlineData("DECIMAL")]
    [InlineData("DEC")]
    public void ShouldCastNumberValuesToAppropriateTypeForNumberAndSynonyms(string numberType)
    {
        // Given Snowflake client is logged in
        using var connection = Fixture.Factory.Create(Output);
        connection.Open();

        // When Query "SELECT 0::<type>(10,0), 123::<type>(10,0), 0.00::<type>(10,2), 123.45::<type>(10,2)" is executed
        using var cmd = connection.CreateCommand();
        cmd.CommandText = $"SELECT 0::{numberType}(10,0), 123::{numberType}(10,0), 0.00::{numberType}(10,2), 123.45::{numberType}(10,2)";
        using var reader = cmd.ExecuteReader();

        // Then All values should be returned as appropriate type matching [0, 123, 0.00, 123.45]
        Assert.True(reader.Read(), "Expected one row");
        Assert.Equal(typeof(long), reader.GetFieldType(0));
        Assert.Equal(0L, reader.GetInt64(0));
        Assert.Equal(typeof(long), reader.GetFieldType(1));
        Assert.Equal(123L, reader.GetInt64(1));
        Assert.Equal(typeof(decimal), reader.GetFieldType(2));
        Assert.Equal(0.00m, reader.GetDecimal(2));
        Assert.Equal(typeof(decimal), reader.GetFieldType(3));
        Assert.Equal(123.45m, reader.GetDecimal(3));
        Assert.False(reader.Read(), "Expected exactly one row");
    }

    // Scenario: should select number literals for number and synonyms
    [SnowflakeTheory]
    [InlineData("NUMBER")]
    [InlineData("NUMERIC")]
    [InlineData("DECIMAL")]
    [InlineData("DEC")]
    public void ShouldSelectNumberLiteralsForNumberAndSynonyms(string numberType)
    {
        // Given Snowflake client is logged in
        using var connection = Fixture.Factory.Create(Output);
        connection.Open();

        // When Query "SELECT 0::<type>(10,0), -456::<type>(10,0), 1.50::<type>(10,2), -123.45::<type>(10,2), 123.456::<type>(15,3), -789.012::<type>(15,3)" is executed
        using var cmd = connection.CreateCommand();
        cmd.CommandText = $"SELECT 0::{numberType}(10,0), -456::{numberType}(10,0), 1.50::{numberType}(10,2), -123.45::{numberType}(10,2), 123.456::{numberType}(15,3), -789.012::{numberType}(15,3)";
        using var reader = cmd.ExecuteReader();

        // Then Result should contain [0, -456, 1.50, -123.45, 123.456, -789.012]
        Assert.True(reader.Read(), "Expected one row");
        Assert.Equal(0L, reader.GetInt64(0));
        Assert.Equal(-456L, reader.GetInt64(1));
        Assert.Equal(1.50m, reader.GetDecimal(2));
        Assert.Equal(-123.45m, reader.GetDecimal(3));
        Assert.Equal(123.456m, reader.GetDecimal(4));
        Assert.Equal(-789.012m, reader.GetDecimal(5));
        Assert.False(reader.Read(), "Expected exactly one row");
    }

    // Scenario: should handle scale and precision boundaries from literals for number and synonyms
    [SnowflakeTheory]
    [InlineData("NUMBER")]
    [InlineData("NUMERIC")]
    [InlineData("DECIMAL")]
    [InlineData("DEC")]
    public void ShouldHandleScaleAndPrecisionBoundariesFromLiteralsForNumberAndSynonyms(string numberType)
    {
        // Given Snowflake client is logged in
        using var connection = Fixture.Factory.Create(Output);
        connection.Open();

        // When Query "SELECT 999.99::<type>(5,2), -999.99::<type>(5,2), 99999999::<type>(8,0), -99999999::<type>(8,0)" is executed
        using var cmd = connection.CreateCommand();
        cmd.CommandText = $"SELECT 999.99::{numberType}(5,2), -999.99::{numberType}(5,2), 99999999::{numberType}(8,0), -99999999::{numberType}(8,0)";
        using var reader = cmd.ExecuteReader();

        // Then Result should contain [999.99, -999.99, 99999999, -99999999]
        Assert.True(reader.Read(), "Expected one row");
        Assert.Equal(999.99m, reader.GetDecimal(0));
        Assert.Equal(-999.99m, reader.GetDecimal(1));
        Assert.Equal(99999999L, reader.GetInt64(2));
        Assert.Equal(-99999999L, reader.GetInt64(3));
        Assert.False(reader.Read(), "Expected exactly one row");
    }

    // Scenario: should handle NULL values from literals for number and synonyms
    [SnowflakeTheory]
    [InlineData("NUMBER")]
    [InlineData("NUMERIC")]
    [InlineData("DECIMAL")]
    [InlineData("DEC")]
    public void ShouldHandleNullValuesFromLiteralsForNumberAndSynonyms(string numberType)
    {
        // Given Snowflake client is logged in
        using var connection = Fixture.Factory.Create(Output);
        connection.Open();

        // When Query "SELECT NULL::<type>(10,0), 42::<type>(10,0), NULL::<type>(10,2), 42.50::<type>(10,2)" is executed
        using var cmd = connection.CreateCommand();
        cmd.CommandText = $"SELECT NULL::{numberType}(10,0), 42::{numberType}(10,0), NULL::{numberType}(10,2), 42.50::{numberType}(10,2)";
        using var reader = cmd.ExecuteReader();

        // Then Result should contain [NULL, 42, NULL, 42.50]
        Assert.True(reader.Read(), "Expected one row");
        Assert.True(reader.IsDBNull(0), "Expected NULL for column 0");
        Assert.Equal(42L, reader.GetInt64(1));
        Assert.True(reader.IsDBNull(2), "Expected NULL for column 2");
        Assert.Equal(42.50m, reader.GetDecimal(3));
        Assert.False(reader.Read(), "Expected exactly one row");
    }

    // Scenario: should download large result set with multiple chunks from GENERATOR for number and synonyms
    [SnowflakeTheory]
    [InlineData("NUMBER")]
    [InlineData("NUMERIC")]
    [InlineData("DECIMAL")]
    [InlineData("DEC")]
    public void ShouldDownloadLargeResultSetWithMultipleChunksFromGeneratorForNumberAndSynonyms(string numberType)
    {
        const int rowCount = 30000;

        // Given Snowflake client is logged in
        using var connection = Fixture.Factory.Create(Output);
        connection.Open();

        // When Query "SELECT seq8()::<type>(38,0), (seq8() + 0.12345)::<type>(20,5) FROM TABLE(GENERATOR(ROWCOUNT => 30000)) v" is executed
        using var cmd = connection.CreateCommand();
        cmd.CommandText = $"SELECT seq8()::{numberType}(38,0), (seq8() + 0.12345)::{numberType}(20,5) FROM TABLE(GENERATOR(ROWCOUNT => {rowCount})) v";
        using var reader = cmd.ExecuteReader();

        // Then Result should contain 30000 rows with sequential integers in column 1 and sequential decimals starting from 0.12345 in column 2
        var count = 0;
        while (reader.Read())
        {
            var col1 = reader.GetInt64(0);
            var col2 = reader.GetDecimal(1);
            Assert.True(col1 >= 0 && col1 < rowCount, $"Expected integer in [0,{rowCount}), got {col1}");
            Assert.Equal(col1 + 0.12345m, col2);
            count++;
        }
        Assert.Equal(rowCount, count);
    }

    // Scenario: should select numbers from table with multiple scales for number and synonyms
    [SnowflakeTheory]
    [InlineData("NUMBER")]
    [InlineData("NUMERIC")]
    [InlineData("DECIMAL")]
    [InlineData("DEC")]
    public void ShouldSelectNumbersFromTableWithMultipleScalesForNumberAndSynonyms(string numberType)
    {
        // Given Snowflake client is logged in
        using var connection = Fixture.Factory.Create(Output);
        connection.Open();

        // And Table with columns (<type>(10,0), <type>(10,2), <type>(15,3), <type>(20,5)) exists
        var tableName = $"UD_NUM_TBL_{Guid.NewGuid():N}".Substring(0, 30);
        using var createCmd = connection.CreateCommand();
        createCmd.CommandText = $"CREATE TEMPORARY TABLE {tableName} (col1 {numberType}(10,0), col2 {numberType}(10,2), col3 {numberType}(15,3), col4 {numberType}(20,5))";
        createCmd.ExecuteNonQuery();

        // And Row (123, 123.45, 123.456, 12345.67890) is inserted
        var insertSql = $"INSERT INTO {tableName} (col1, col2, col3, col4) VALUES " +
            "(123, 123.45, 123.456, 12345.67890), ";

        // And Row (-456, -67.89, -789.012, -98765.43210) is inserted
        insertSql += "(-456, -67.89, -789.012, -98765.43210), ";

        // And Row (0, 0.00, 0.000, 0.00000) is inserted
        insertSql += "(0, 0.00, 0.000, 0.00000), ";

        // And Row (999999, 999.99, 1000.500, 123456.78901) is inserted
        insertSql += "(999999, 999.99, 1000.500, 123456.78901)";
        using var insertCmd = connection.CreateCommand();
        insertCmd.CommandText = insertSql;
        insertCmd.ExecuteNonQuery();

        // When Query "SELECT * FROM <table>" is executed
        using var selectCmd = connection.CreateCommand();
        selectCmd.CommandText = $"SELECT * FROM {tableName} ORDER BY col1";
        using var reader = selectCmd.ExecuteReader();

        // Then Result should contain 4 rows with expected values
        Assert.True(reader.Read(), "Expected row 0");
        // Row order after ORDER BY col1: -456, 0, 123, 999999
        Assert.Equal(-456L, reader.GetInt64(0));
        Assert.Equal(-67.89m, reader.GetDecimal(1));
        Assert.Equal(-789.012m, reader.GetDecimal(2));
        Assert.Equal(-98765.43210m, reader.GetDecimal(3));

        Assert.True(reader.Read(), "Expected row 1");
        Assert.Equal(0L, reader.GetInt64(0));
        Assert.Equal(0.00m, reader.GetDecimal(1));
        Assert.Equal(0.000m, reader.GetDecimal(2));
        Assert.Equal(0.00000m, reader.GetDecimal(3));

        Assert.True(reader.Read(), "Expected row 2");
        Assert.Equal(123L, reader.GetInt64(0));
        Assert.Equal(123.45m, reader.GetDecimal(1));
        Assert.Equal(123.456m, reader.GetDecimal(2));
        Assert.Equal(12345.67890m, reader.GetDecimal(3));

        Assert.True(reader.Read(), "Expected row 3");
        Assert.Equal(999999L, reader.GetInt64(0));
        Assert.Equal(999.99m, reader.GetDecimal(1));
        Assert.Equal(1000.500m, reader.GetDecimal(2));
        Assert.Equal(123456.78901m, reader.GetDecimal(3));

        Assert.False(reader.Read(), "Expected exactly 4 rows");
    }

    // Scenario: should handle scale and precision boundaries from table for number and synonyms
    [SnowflakeTheory]
    [InlineData("NUMBER")]
    [InlineData("NUMERIC")]
    [InlineData("DECIMAL")]
    [InlineData("DEC")]
    public void ShouldHandleScaleAndPrecisionBoundariesFromTableForNumberAndSynonyms(string numberType)
    {
        // Given Snowflake client is logged in
        using var connection = Fixture.Factory.Create(Output);
        connection.Open();

        // And Table with columns (<type>(5,2), <type>(8,0)) exists
        var tableName = $"UD_NUM_BNDRY_{Guid.NewGuid():N}".Substring(0, 30);
        using var createCmd = connection.CreateCommand();
        createCmd.CommandText = $"CREATE TEMPORARY TABLE {tableName} (col1 {numberType}(5,2), col2 {numberType}(8,0))";
        createCmd.ExecuteNonQuery();

        // And Row (999.99, 99999999) is inserted
        var insertSql = $"INSERT INTO {tableName} (col1, col2) VALUES " +
            "(999.99, 99999999), ";

        // And Row (-999.99, -99999999) is inserted
        insertSql += "(-999.99, -99999999), ";

        // And Row (123.45, 12345678) is inserted
        insertSql += "(123.45, 12345678), ";

        // And Row (0.01, 0) is inserted
        insertSql += "(0.01, 0)";
        using var insertCmd = connection.CreateCommand();
        insertCmd.CommandText = insertSql;
        insertCmd.ExecuteNonQuery();

        // When Query "SELECT * FROM <table>" is executed
        using var selectCmd = connection.CreateCommand();
        selectCmd.CommandText = $"SELECT * FROM {tableName} ORDER BY col2";
        using var reader = selectCmd.ExecuteReader();

        // Then Result should contain 4 rows with expected boundary values
        Assert.True(reader.Read(), "Expected row 0");
        // Order by col2: -99999999, 0, 12345678, 99999999
        Assert.Equal(-999.99m, reader.GetDecimal(0));
        Assert.Equal(-99999999L, reader.GetInt64(1));

        Assert.True(reader.Read(), "Expected row 1");
        Assert.Equal(0.01m, reader.GetDecimal(0));
        Assert.Equal(0L, reader.GetInt64(1));

        Assert.True(reader.Read(), "Expected row 2");
        Assert.Equal(123.45m, reader.GetDecimal(0));
        Assert.Equal(12345678L, reader.GetInt64(1));

        Assert.True(reader.Read(), "Expected row 3");
        Assert.Equal(999.99m, reader.GetDecimal(0));
        Assert.Equal(99999999L, reader.GetInt64(1));

        Assert.False(reader.Read(), "Expected exactly 4 rows");
    }

    // Scenario: should handle NULL values from table with multiple scales for number and synonyms
    [SnowflakeTheory]
    [InlineData("NUMBER")]
    [InlineData("NUMERIC")]
    [InlineData("DECIMAL")]
    [InlineData("DEC")]
    public void ShouldHandleNullValuesFromTableWithMultipleScalesForNumberAndSynonyms(string numberType)
    {
        Skip.FutureMilestone();

        // Given Snowflake client is logged in
        using var connection = Fixture.Factory.Create(Output);
        connection.Open();

        // And Table with columns (<type>(10,0), <type>(10,2), <type>(15,3)) exists
        var tableName = $"UD_NUM_NULL_{Guid.NewGuid():N}".Substring(0, 30);
        using var createCmd = connection.CreateCommand();
        createCmd.CommandText = $"CREATE TEMPORARY TABLE {tableName} (id INT, col1 {numberType}(10,0), col2 {numberType}(10,2), col3 {numberType}(15,3))";
        createCmd.ExecuteNonQuery();

        // And Row (NULL, NULL, NULL) is inserted
        var insertSql = $"INSERT INTO {tableName} (id, col1, col2, col3) VALUES " +
            "(1, NULL, NULL, NULL), ";

        // And Row (123, 123.45, 123.456) is inserted
        insertSql += "(2, 123, 123.45, 123.456), ";

        // And Row (NULL, NULL, NULL) is inserted
        insertSql += "(3, NULL, NULL, NULL), ";

        // And Row (-456, -67.89, -789.012) is inserted
        insertSql += "(-456, -456, -67.89, -789.012)";
        using var insertCmd = connection.CreateCommand();
        insertCmd.CommandText = insertSql;
        insertCmd.ExecuteNonQuery();

        // When Query "SELECT * FROM <table>" is executed
        using var selectCmd = connection.CreateCommand();
        selectCmd.CommandText = $"SELECT col1, col2, col3 FROM {tableName} ORDER BY id";
        using var reader = selectCmd.ExecuteReader();

        // Then Result should contain 4 rows with 2 NULL rows and 2 non-NULL rows with expected values
        Assert.True(reader.Read(), "Expected row 0 (NULL row)");
        Assert.True(reader.IsDBNull(1), "Expected NULL col1");
        Assert.True(reader.IsDBNull(2), "Expected NULL col2");
        Assert.True(reader.IsDBNull(3), "Expected NULL col3");

        Assert.True(reader.Read(), "Expected row 1 (non-NULL)");
        Assert.Equal(123L, reader.GetInt64(1));
        Assert.Equal(123.45m, reader.GetDecimal(2));
        Assert.Equal(123.456m, reader.GetDecimal(3));

        Assert.True(reader.Read(), "Expected row 2 (NULL row)");
        Assert.True(reader.IsDBNull(1), "Expected NULL col1");
        Assert.True(reader.IsDBNull(2), "Expected NULL col2");
        Assert.True(reader.IsDBNull(3), "Expected NULL col3");

        Assert.True(reader.Read(), "Expected row 3 (non-NULL)");
        Assert.Equal(-456L, reader.GetInt64(1));
        Assert.Equal(-67.89m, reader.GetDecimal(2));
        Assert.Equal(-789.012m, reader.GetDecimal(3));

        Assert.False(reader.Read(), "Expected exactly 4 rows");
    }

    // Scenario: should download large result set from table for number and synonyms
    [SnowflakeTheory]
    [InlineData("NUMBER")]
    [InlineData("NUMERIC")]
    [InlineData("DECIMAL")]
    [InlineData("DEC")]
    public void ShouldDownloadLargeResultSetFromTableForNumberAndSynonyms(string numberType)
    {
        const int rowCount = 30000;

        // Given Snowflake client is logged in
        using var connection = Fixture.Factory.Create(Output);
        connection.Open();

        // And Table with columns (<type>(38,0), <type>(20,5)) exists with 30000 sequential rows, from 0 to 29999 in the first column and from 0.12345 to 29999.12345 in the second column
        var tableName = $"UD_NUM_LRG_{Guid.NewGuid():N}".Substring(0, 30);
        using var createCmd = connection.CreateCommand();
        createCmd.CommandText = $"CREATE TEMPORARY TABLE {tableName} (col1 {numberType}(38,0), col2 {numberType}(20,5))";
        createCmd.ExecuteNonQuery();

        using var insertCmd = connection.CreateCommand();
        insertCmd.CommandText = $"INSERT INTO {tableName} SELECT seq8()::{numberType}(38,0), (seq8() + 0.12345)::{numberType}(20,5) FROM TABLE(GENERATOR(ROWCOUNT => {rowCount})) v";
        insertCmd.ExecuteNonQuery();

        // When Query "SELECT * FROM <table>" is executed
        using var selectCmd = connection.CreateCommand();
        selectCmd.CommandText = $"SELECT * FROM {tableName} ORDER BY col1";
        using var reader = selectCmd.ExecuteReader();

        // Then Result should contain 30000 rows with sequential integers in column 1 and sequential decimals starting from 0.12345 in column 2
        var count = 0;
        while (reader.Read())
        {
            var col1 = reader.GetInt64(0);
            var col2 = reader.GetDecimal(1);
            Assert.Equal(count, col1);
            Assert.Equal(count + 0.12345m, col2);
            count++;
        }
        Assert.Equal(rowCount, count);
    }

    // Scenario: should select number using parameter binding for number and synonyms
    [SnowflakeTheory]
    [InlineData("NUMBER")]
    [InlineData("NUMERIC")]
    [InlineData("DECIMAL")]
    [InlineData("DEC")]
    public void ShouldSelectNumberUsingParameterBindingForNumberAndSynonyms(string numberType)
    {
        Skip.FutureMilestone();

        // Given Snowflake client is logged in
        using var connection = Fixture.Factory.Create(Output);
        connection.Open();

        // When Query "SELECT ?::<type>(10,0), ?::<type>(10,0), ?::<type>(10,2), ?::<type>(10,2), ?::<type>(10,0)" is executed with bound values [123, -456, 12.34, -56.78, NULL]
        using var cmd = connection.CreateCommand();
        cmd.CommandText = $"SELECT ?::{numberType}(10,0), ?::{numberType}(10,0), ?::{numberType}(10,2), ?::{numberType}(10,2), ?::{numberType}(10,0)";

        var param1 = cmd.CreateParameter();
        param1.ParameterName = "1";
        param1.DbType = DbType.Int64;
        param1.Value = 123L;
        cmd.Parameters.Add(param1);

        var param2 = cmd.CreateParameter();
        param2.ParameterName = "2";
        param2.DbType = DbType.Int64;
        param2.Value = -456L;
        cmd.Parameters.Add(param2);

        var param3 = cmd.CreateParameter();
        param3.ParameterName = "3";
        param3.DbType = DbType.Decimal;
        param3.Value = 12.34m;
        cmd.Parameters.Add(param3);

        var param4 = cmd.CreateParameter();
        param4.ParameterName = "4";
        param4.DbType = DbType.Decimal;
        param4.Value = -56.78m;
        cmd.Parameters.Add(param4);

        var param5 = cmd.CreateParameter();
        param5.ParameterName = "5";
        param5.DbType = DbType.Int64;
        param5.Value = DBNull.Value;
        cmd.Parameters.Add(param5);

        using var reader = cmd.ExecuteReader();

        // Then Result should contain [123, -456, 12.34, -56.78, NULL]
        Assert.True(reader.Read(), "Expected one row");
        Assert.Equal(123L, reader.GetInt64(0));
        Assert.Equal(-456L, reader.GetInt64(1));
        Assert.Equal(12.34m, reader.GetDecimal(2));
        Assert.Equal(-56.78m, reader.GetDecimal(3));
        Assert.True(reader.IsDBNull(4), "Expected NULL for column 4");
        Assert.False(reader.Read(), "Expected exactly one row");
    }

    // Scenario: should insert number using parameter binding for number and synonyms
    [SnowflakeTheory]
    [InlineData("NUMBER")]
    [InlineData("NUMERIC")]
    [InlineData("DECIMAL")]
    [InlineData("DEC")]
    public void ShouldInsertNumberUsingParameterBindingForNumberAndSynonyms(string numberType)
    {
        Skip.FutureMilestone();

        // Given Snowflake client is logged in
        using var connection = Fixture.Factory.Create(Output);
        connection.Open();

        // And Table with columns (<type>(10,0), <type>(10,2)) exists
        var tableName = $"UD_NUM_BIND_{Guid.NewGuid():N}".Substring(0, 30);
        using var createCmd = connection.CreateCommand();
        createCmd.CommandText = $"CREATE TEMPORARY TABLE {tableName} (col1 {numberType}(10,0), col2 {numberType}(10,2))";
        createCmd.ExecuteNonQuery();

        // When Rows (0, 0.00), (123, 123.45), (-456, -67.89), (999999, 999.99), (NULL, NULL) are inserted using binding
        (long? col1, decimal? col2)[] rows =
        [
            (0L, 0.00m),
            (123L, 123.45m),
            (-456L, -67.89m),
            (999999L, 999.99m),
            (null, null)
        ];

        foreach (var (col1, col2) in rows)
        {
            using var insertCmd = connection.CreateCommand();
            insertCmd.CommandText = $"INSERT INTO {tableName} (col1, col2) VALUES (?, ?)";

            var p1 = insertCmd.CreateParameter();
            p1.ParameterName = "1";
            p1.DbType = DbType.Int64;
            p1.Value = col1.HasValue ? (object)col1.Value : DBNull.Value;
            insertCmd.Parameters.Add(p1);

            var p2 = insertCmd.CreateParameter();
            p2.ParameterName = "2";
            p2.DbType = DbType.Decimal;
            p2.Value = col2.HasValue ? (object)col2.Value : DBNull.Value;
            insertCmd.Parameters.Add(p2);

            insertCmd.ExecuteNonQuery();
        }

        // Then Result should contain 5 rows with expected values
        using var selectCmd = connection.CreateCommand();
        selectCmd.CommandText = $"SELECT col1, col2 FROM {tableName} ORDER BY col1 NULLS LAST";
        using var reader = selectCmd.ExecuteReader();

        // Order: -456, 0, 123, 999999, NULL
        Assert.True(reader.Read(), "Expected row 0");
        Assert.Equal(-456L, reader.GetInt64(0));
        Assert.Equal(-67.89m, reader.GetDecimal(1));

        Assert.True(reader.Read(), "Expected row 1");
        Assert.Equal(0L, reader.GetInt64(0));
        Assert.Equal(0.00m, reader.GetDecimal(1));

        Assert.True(reader.Read(), "Expected row 2");
        Assert.Equal(123L, reader.GetInt64(0));
        Assert.Equal(123.45m, reader.GetDecimal(1));

        Assert.True(reader.Read(), "Expected row 3");
        Assert.Equal(999999L, reader.GetInt64(0));
        Assert.Equal(999.99m, reader.GetDecimal(1));

        Assert.True(reader.Read(), "Expected row 4 (NULL)");
        Assert.True(reader.IsDBNull(0), "Expected NULL for col1");
        Assert.True(reader.IsDBNull(1), "Expected NULL for col2");

        Assert.False(reader.Read(), "Expected exactly 5 rows");
    }

    // Scenario: should handle high precision values from literals as strings for number and synonyms
    [SnowflakeFact]
    public void ShouldHandleHighPrecisionValuesFromLiteralsAsStringsForNumberAndSynonyms()
    {
        Skip.FutureMilestone();

        // Given Snowflake client is logged in
        using var connection = Fixture.Factory.Create(Output);
        connection.Open();

        // When Query "SELECT 12345678901234567890123456789012345678::NUMBER(38,0), 123456789012345678901234567890123456.78::NUMBER(38,2)" is executed
        using var cmd = connection.CreateCommand();
        cmd.CommandText = "SELECT 12345678901234567890123456789012345678::NUMBER(38,0), 123456789012345678901234567890123456.78::NUMBER(38,2)";
        using var reader = cmd.ExecuteReader();

        // Then Result should contain string values ["12345678901234567890123456789012345678", "123456789012345678901234567890123456.78"]
        Assert.True(reader.Read(), "Expected one row");
        Assert.Equal("12345678901234567890123456789012345678", reader.GetString(0));
        Assert.Equal("123456789012345678901234567890123456.78", reader.GetString(1));
        Assert.False(reader.Read(), "Expected exactly one row");
    }

    // Scenario: should handle high precision boundaries from literals as strings for number and synonyms
    [SnowflakeFact]
    public void ShouldHandleHighPrecisionBoundariesFromLiteralsAsStringsForNumberAndSynonyms()
    {
        Skip.FutureMilestone();

        // Given Snowflake client is logged in
        using var connection = Fixture.Factory.Create(Output);
        connection.Open();

        // When Query "SELECT 99999999999999999999999999999999999999::NUMBER(38,0), -99999999999999999999999999999999999999::NUMBER(38,0)" is executed
        using var cmd = connection.CreateCommand();
        cmd.CommandText = "SELECT 99999999999999999999999999999999999999::NUMBER(38,0), -99999999999999999999999999999999999999::NUMBER(38,0)";
        using var reader = cmd.ExecuteReader();

        // Then Result should contain string values ["99999999999999999999999999999999999999", "-99999999999999999999999999999999999999"]
        Assert.True(reader.Read(), "Expected one row");
        Assert.Equal("99999999999999999999999999999999999999", reader.GetString(0));
        Assert.Equal("-99999999999999999999999999999999999999", reader.GetString(1));
        Assert.False(reader.Read(), "Expected exactly one row");
    }

    // Scenario: should handle high precision values from table as strings for number and synonyms
    [SnowflakeFact]
    public void ShouldHandleHighPrecisionValuesFromTableAsStringsForNumberAndSynonyms()
    {
        Skip.FutureMilestone();

        // Given Snowflake client is logged in
        using var connection = Fixture.Factory.Create(Output);
        connection.Open();

        // And Table with NUMBER(38,0) column exists with value 12345678901234567890123456789012345678
        var tableName = $"UD_NUM_HP_{Guid.NewGuid():N}".Substring(0, 30);
        using var createCmd = connection.CreateCommand();
        createCmd.CommandText = $"CREATE TEMPORARY TABLE {tableName} (col NUMBER(38,0))";
        createCmd.ExecuteNonQuery();

        using var insertCmd = connection.CreateCommand();
        insertCmd.CommandText = $"INSERT INTO {tableName} (col) VALUES (12345678901234567890123456789012345678)";
        insertCmd.ExecuteNonQuery();

        // When Query "SELECT * FROM <table>" is executed
        using var selectCmd = connection.CreateCommand();
        selectCmd.CommandText = $"SELECT * FROM {tableName}";
        using var reader = selectCmd.ExecuteReader();

        // Then Result should contain string value "12345678901234567890123456789012345678"
        Assert.True(reader.Read(), "Expected one row");
        Assert.Equal("12345678901234567890123456789012345678", reader.GetString(0));
        Assert.False(reader.Read(), "Expected exactly one row");
    }

    // Scenario: should handle high precision boundaries from table as strings for number and synonyms
    [SnowflakeFact]
    public void ShouldHandleHighPrecisionBoundariesFromTableAsStringsForNumberAndSynonyms()
    {
        Skip.FutureMilestone();

        // Given Snowflake client is logged in
        using var connection = Fixture.Factory.Create(Output);
        connection.Open();

        // And Table with NUMBER(38,0) column exists with values [99999999999999999999999999999999999999, -99999999999999999999999999999999999999]
        var tableName = $"UD_NUM_HPB_{Guid.NewGuid():N}".Substring(0, 30);
        using var createCmd = connection.CreateCommand();
        createCmd.CommandText = $"CREATE TEMPORARY TABLE {tableName} (col NUMBER(38,0))";
        createCmd.ExecuteNonQuery();

        using var insertCmd = connection.CreateCommand();
        insertCmd.CommandText = $"INSERT INTO {tableName} (col) VALUES (99999999999999999999999999999999999999), (-99999999999999999999999999999999999999)";
        insertCmd.ExecuteNonQuery();

        // When Query "SELECT * FROM <table> ORDER BY col" is executed
        using var selectCmd = connection.CreateCommand();
        selectCmd.CommandText = $"SELECT * FROM {tableName} ORDER BY col";
        using var reader = selectCmd.ExecuteReader();

        // Then Result should contain string values ["-99999999999999999999999999999999999999", "99999999999999999999999999999999999999"]
        Assert.True(reader.Read(), "Expected row 0");
        Assert.Equal("-99999999999999999999999999999999999999", reader.GetString(0));
        Assert.True(reader.Read(), "Expected row 1");
        Assert.Equal("99999999999999999999999999999999999999", reader.GetString(0));
        Assert.False(reader.Read(), "Expected exactly 2 rows");
    }
}
