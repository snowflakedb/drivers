using System.Globalization;
using System.Numerics;
using Snowflake.Data.Tests.Compatibility;

namespace Snowflake.Data.Tests.E2E;

[Trait("Category", "E2E")]
public class IntTest : IClassFixture<ITFixture>
{
    protected readonly ITestOutputHelper Output;
    protected readonly ITFixture Fixture;

    private static readonly string[] IntTypeSynonyms =
        ["INT", "INTEGER", "BIGINT", "SMALLINT", "TINYINT", "BYTEINT"];

    private static readonly BigInteger LargestNumber38 =
        BigInteger.Parse("99999999999999999999999999999999999999", CultureInfo.InvariantCulture);

    public IntTest(ITFixture fixture, ITestOutputHelper output)
    {
        Fixture = fixture;
        Output = output;
    }

    // Scenario: should cast integer values to appropriate type for int and synonyms
    [SnowflakeTheory]
    [InlineData("INT")]
    [InlineData("INTEGER")]
    [InlineData("BIGINT")]
    [InlineData("SMALLINT")]
    [InlineData("TINYINT")]
    [InlineData("BYTEINT")]
    public async Task ShouldCastIntegerValuesToAppropriateTypeForIntAndSynonymsAsync(string intType)
    {
        // Given Snowflake client is logged in
        using var connection = Fixture.Factory.Create(Output);
        await connection.OpenAsync().ConfigureAwait(false);

        // When Query "SELECT 0::<type>, 1000000::<type>, 9223372036854775807::<type>" is executed
        using var cmd = connection.CreateCommand();
        cmd.CommandText = $"SELECT 0::{intType}, 1000000::{intType}, 9223372036854775807::{intType}";
        using var reader = await cmd.ExecuteReaderAsync().ConfigureAwait(false);

        // Then All values should be returned as appropriate type with no precision loss
        Assert.True(await reader.ReadAsync().ConfigureAwait(false), "Expected one row");
        Assert.Equal(0L, reader.GetInt64(0));
        Assert.Equal(1000000L, reader.GetInt64(1));
        Assert.Equal(9223372036854775807L, reader.GetInt64(2));
        Assert.False(await reader.ReadAsync().ConfigureAwait(false), "Expected exactly one row");
    }

    // Scenario Outline: should select integer <values> for int and synonyms
    [SnowflakeTheory]
    [MemberData(nameof(BoundaryValuesData))]
    public async Task ShouldSelectIntegerValuesForIntAndSynonymsAsync(string intType, string values, long[] expectedValues)
    {
        _ = values; // used for test display name only

        // Given Snowflake client is logged in
        using var connection = Fixture.Factory.Create(Output);
        await connection.OpenAsync().ConfigureAwait(false);

        // When Query "SELECT <query_values>" is executed
        using var cmd = connection.CreateCommand();
        var castExpressions = expectedValues.Select(v => $"{v}::{intType}");
        cmd.CommandText = $"SELECT {string.Join(", ", castExpressions)}";
        using var reader = await cmd.ExecuteReaderAsync().ConfigureAwait(false);

        // Then Result should contain integers <expected_values>
        Assert.True(await reader.ReadAsync().ConfigureAwait(false), "Expected one row");
        for (var i = 0; i < expectedValues.Length; i++)
        {
            Assert.Equal(expectedValues[i], reader.GetInt64(i));
        }
        Assert.False(await reader.ReadAsync().ConfigureAwait(false), "Expected exactly one row");
    }

    public static IEnumerable<object[]> BoundaryValuesData()
    {
        foreach (var intType in IntTypeSynonyms)
        {
            yield return [intType, "zero", new long[] { 0 }];
            yield return [intType, "tinyint", new long[] { -128, 127, 255 }];
            yield return [intType, "smallint", new long[] { -32768, 32767, 65535 }];
            yield return [intType, "int", new long[] { -2147483648, 2147483647, 4294967295 }];
            yield return [intType, "bigint", new long[] { -9223372036854775808, 9223372036854775807 }];
        }
    }

    // Scenario: should handle large integer values for int and synonyms
    [SnowflakeTheory]
    [InlineData("INT")]
    [InlineData("INTEGER")]
    [InlineData("BIGINT")]
    [InlineData("SMALLINT")]
    [InlineData("TINYINT")]
    [InlineData("BYTEINT")]
    public async Task ShouldHandleLargeIntegerValuesForIntAndSynonymsAsync(string intType)
    {
        Skip.FutureMilestone("GetString on a NUMBER(38,0) value beyond the decimal range throws OverflowException (SNOW-4483010)");

        // Given Snowflake client is logged in
        using var connection = Fixture.Factory.Create(Output);
        await connection.OpenAsync().ConfigureAwait(false);

        // When Query "SELECT -99999999999999999999999999999999999999::<type>, 99999999999999999999999999999999999999::<type>" is executed
        using var cmd = connection.CreateCommand();
        cmd.CommandText = $"SELECT -99999999999999999999999999999999999999::{intType}, 99999999999999999999999999999999999999::{intType}";
        using var reader = await cmd.ExecuteReaderAsync().ConfigureAwait(false);

        // Then Result should contain integers [-99999999999999999999999999999999999999, 99999999999999999999999999999999999999]
        Assert.True(await reader.ReadAsync().ConfigureAwait(false), "Expected one row");
        Assert.Equal(-LargestNumber38, BigInteger.Parse(reader.GetString(0), CultureInfo.InvariantCulture));
        Assert.Equal(LargestNumber38, BigInteger.Parse(reader.GetString(1), CultureInfo.InvariantCulture));
        Assert.False(await reader.ReadAsync().ConfigureAwait(false), "Expected exactly one row");
    }

    // Scenario: should handle NULL values for int and synonyms
    [SnowflakeTheory]
    [InlineData("INT")]
    [InlineData("INTEGER")]
    [InlineData("BIGINT")]
    [InlineData("SMALLINT")]
    [InlineData("TINYINT")]
    [InlineData("BYTEINT")]
    public async Task ShouldHandleNullValuesForIntAndSynonymsAsync(string intType)
    {
        // Given Snowflake client is logged in
        using var connection = Fixture.Factory.Create(Output);
        await connection.OpenAsync().ConfigureAwait(false);

        // When Query "SELECT NULL::<type>, 42::<type>, NULL::<type>" is executed
        using var cmd = connection.CreateCommand();
        cmd.CommandText = $"SELECT NULL::{intType}, 42::{intType}, NULL::{intType}";
        using var reader = await cmd.ExecuteReaderAsync().ConfigureAwait(false);

        // Then Result should contain [NULL, 42, NULL]
        Assert.True(await reader.ReadAsync().ConfigureAwait(false), "Expected one row");
        Assert.True(reader.IsDBNull(0), "Expected NULL for column 0");
        Assert.Equal(42L, reader.GetInt64(1));
        Assert.True(reader.IsDBNull(2), "Expected NULL for column 2");
        Assert.False(await reader.ReadAsync().ConfigureAwait(false), "Expected exactly one row");
    }

    // Scenario: should download large result set with multiple chunks for int and synonyms
    [SnowflakeTheory]
    [InlineData("INT")]
    [InlineData("INTEGER")]
    [InlineData("BIGINT")]
    [InlineData("SMALLINT")]
    [InlineData("TINYINT")]
    [InlineData("BYTEINT")]
    public async Task ShouldDownloadLargeResultSetWithMultipleChunksForIntAndSynonymsAsync(string intType)
    {
        const int rowCount = 50000;

        // Given Snowflake client is logged in
        using var connection = Fixture.Factory.Create(Output);
        await connection.OpenAsync().ConfigureAwait(false);

        // When Query "SELECT seq8()::<type> as id FROM TABLE(GENERATOR(ROWCOUNT => 50000)) v ORDER BY id" is executed
        using var cmd = connection.CreateCommand();
        cmd.CommandText = $"SELECT (ROW_NUMBER() OVER (ORDER BY seq8()) - 1)::{intType} AS id FROM TABLE(GENERATOR(ROWCOUNT => {rowCount})) v ORDER BY 1";
        using var reader = await cmd.ExecuteReaderAsync().ConfigureAwait(false);

        // Then Result should contain 50000 sequentially numbered rows from 0 to 49999
        var count = 0;
        while (await reader.ReadAsync().ConfigureAwait(false))
        {
            Assert.Equal((long)count, reader.GetInt64(0));
            count++;
        }
        Assert.Equal(rowCount, count);
    }

    // Scenario Outline: should select <values> from table for int and synonyms
    [SnowflakeTheory]
    [MemberData(nameof(TableValuesData))]
    public async Task ShouldSelectValuesFromTableForIntAndSynonymsAsync(string intType, string values, string sqlInsertValues, long?[] expectedValues)
    {
        _ = values; // used for test display name only

        // Given Snowflake client is logged in
        using var connection = Fixture.Factory.Create(Output);
        await connection.OpenAsync().ConfigureAwait(false);

        // And Table with <type> column exists with values <insert_values>
        var tableName = $"UD_INT_TBL_{Guid.NewGuid():N}".Substring(0, 30);
        using var createCmd = connection.CreateCommand();
        createCmd.CommandText = $"CREATE TEMPORARY TABLE {tableName} (col {intType})";
        await createCmd.ExecuteNonQueryAsync().ConfigureAwait(false);

        using var insertCmd = connection.CreateCommand();
        insertCmd.CommandText = $"INSERT INTO {tableName} (col) VALUES {sqlInsertValues}";
        await insertCmd.ExecuteNonQueryAsync().ConfigureAwait(false);

        // When Query "SELECT * FROM <table> ORDER BY col" is executed
        using var selectCmd = connection.CreateCommand();
        selectCmd.CommandText = $"SELECT * FROM {tableName} ORDER BY col";
        using var reader = await selectCmd.ExecuteReaderAsync().ConfigureAwait(false);

        // Then Result should contain integers <expected_values>
        var rowIndex = 0;
        while (await reader.ReadAsync().ConfigureAwait(false))
        {
            if (expectedValues[rowIndex] is null)
            {
                Assert.True(reader.IsDBNull(0), $"Expected NULL at row {rowIndex}");
            }
            else
            {
                Assert.Equal(expectedValues[rowIndex]!.Value, reader.GetInt64(0));
            }
            rowIndex++;
        }
        Assert.Equal(expectedValues.Length, rowIndex);
    }

    public static IEnumerable<object[]> TableValuesData()
    {
        foreach (var intType in IntTypeSynonyms)
        {
            yield return [
                intType,
                "positive",
                "(0), (1), (127), (255), (32767), (65535), (2147483647), (4294967295), (9223372036854775807)",
                new long?[] { 0L, 1L, 127L, 255L, 32767L, 65535L, 2147483647L, 4294967295L, 9223372036854775807L }
            ];
            yield return [
                intType,
                "negative",
                "(-1), (-128), (-32768), (-2147483648), (-9223372036854775808)",
                new long?[] { -9223372036854775808L, -2147483648L, -32768L, -128L, -1L }
            ];
            yield return [
                intType,
                "null",
                "(0), (NULL), (42)",
                new long?[] { 0L, 42L, null }
            ];
        }
    }

    // Scenario: should select large integer values from table for int and synonyms
    [SnowflakeTheory]
    [InlineData("INT")]
    [InlineData("INTEGER")]
    [InlineData("BIGINT")]
    [InlineData("SMALLINT")]
    [InlineData("TINYINT")]
    [InlineData("BYTEINT")]
    public async Task ShouldSelectLargeIntegerValuesFromTableForIntAndSynonymsAsync(string intType)
    {
        Skip.FutureMilestone("GetString on a NUMBER(38,0) value beyond the decimal range throws OverflowException (SNOW-4483010)");

        // Given Snowflake client is logged in
        using var connection = Fixture.Factory.Create(Output);
        await connection.OpenAsync().ConfigureAwait(false);

        // And Table with <type> column exists with values [-99999999999999999999999999999999999999, 99999999999999999999999999999999999999]
        var tableName = $"UD_INT_LARGE_{Guid.NewGuid():N}".Substring(0, 30);
        using var createCmd = connection.CreateCommand();
        createCmd.CommandText = $"CREATE TEMPORARY TABLE {tableName} (col {intType})";
        await createCmd.ExecuteNonQueryAsync().ConfigureAwait(false);

        using var insertCmd = connection.CreateCommand();
        insertCmd.CommandText = $"INSERT INTO {tableName} (col) VALUES (-99999999999999999999999999999999999999), (99999999999999999999999999999999999999)";
        await insertCmd.ExecuteNonQueryAsync().ConfigureAwait(false);

        // When Query "SELECT * FROM <table> ORDER BY col" is executed
        using var selectCmd = connection.CreateCommand();
        selectCmd.CommandText = $"SELECT * FROM {tableName} ORDER BY col";
        using var reader = await selectCmd.ExecuteReaderAsync().ConfigureAwait(false);

        // Then Result should contain integers [-99999999999999999999999999999999999999, 99999999999999999999999999999999999999]
        Assert.True(await reader.ReadAsync().ConfigureAwait(false), "Expected first row");
        Assert.Equal(-LargestNumber38, BigInteger.Parse(reader.GetString(0), CultureInfo.InvariantCulture));
        Assert.True(await reader.ReadAsync().ConfigureAwait(false), "Expected second row");
        Assert.Equal(LargestNumber38, BigInteger.Parse(reader.GetString(0), CultureInfo.InvariantCulture));
        Assert.False(await reader.ReadAsync().ConfigureAwait(false), "Expected exactly two rows");
    }

    // Scenario: should handle server-side Arrow memory optimization for int columns on multiple chunks
    [SnowflakeFact]
    public async Task ShouldHandleServerSideArrowMemoryOptimizationForIntColumnsOnMultipleChunksAsync()
    {
        const int rowCount = 50000;

        // Given Snowflake client is logged in
        using var connection = Fixture.Factory.Create(Output);
        await connection.OpenAsync().ConfigureAwait(false);

        // And Table with four INT columns exists
        var tableName = $"UD_INT_ARROW_{Guid.NewGuid():N}".Substring(0, 30);
        using var createCmd = connection.CreateCommand();
        createCmd.CommandText = $"CREATE TEMPORARY TABLE {tableName} (col_int8 INT, col_int16 INT, col_int32 INT, col_int64 INT)";
        await createCmd.ExecuteNonQueryAsync().ConfigureAwait(false);

        // And Each column contains values of different magnitudes (50000 rows to span multiple Arrow chunks)
        using var insertCmd = connection.CreateCommand();
        insertCmd.CommandText = $"INSERT INTO {tableName} SELECT 100, 30000, 2000000000, 9000000000000000000 FROM TABLE(GENERATOR(ROWCOUNT => {rowCount}))";
        await insertCmd.ExecuteNonQueryAsync().ConfigureAwait(false);

        // When Query "SELECT * FROM <table>" is executed
        using var selectCmd = connection.CreateCommand();
        selectCmd.CommandText = $"SELECT * FROM {tableName}";
        using var reader = await selectCmd.ExecuteReaderAsync().ConfigureAwait(false);

        // Then Result should contain 50000 rows with all values equal to expected data
        var count = 0;
        while (await reader.ReadAsync().ConfigureAwait(false))
        {
            Assert.Equal(100L, reader.GetInt64(0));
            Assert.Equal(30000L, reader.GetInt64(1));
            Assert.Equal(2000000000L, reader.GetInt64(2));
            Assert.Equal(9000000000000000000L, reader.GetInt64(3));
            count++;
        }
        Assert.Equal(rowCount, count);
    }

    // Scenario: should insert integer using parameter binding for int and synonyms
    [SnowflakeTheory]
    [InlineData("INT")]
    [InlineData("INTEGER")]
    [InlineData("BIGINT")]
    [InlineData("SMALLINT")]
    [InlineData("TINYINT")]
    [InlineData("BYTEINT")]
    public async Task ShouldInsertIntegerUsingParameterBindingForIntAndSynonymsAsync(string intType)
    {
        Skip.FutureMilestone("Commands with bound parameters are not implemented (SNOW-4382583)");
        long[] testValues = [0, -2147483648, 2147483647, 9223372036854775807];

        // Given Snowflake client is logged in
        using var connection = Fixture.Factory.Create(Output);
        await connection.OpenAsync().ConfigureAwait(false);

        // And Table with <type> column exists
        var tableName = $"UD_INT_BIND_{Guid.NewGuid():N}".Substring(0, 30);
        using var createCmd = connection.CreateCommand();
        createCmd.CommandText = $"CREATE TEMPORARY TABLE {tableName} (col {intType})";
        await createCmd.ExecuteNonQueryAsync().ConfigureAwait(false);

        // When Integer values [0, -2147483648, 2147483647, 9223372036854775807] are inserted using binding
        foreach (var value in testValues)
        {
            using var insertCmd = connection.CreateCommand();
            insertCmd.CommandText = $"INSERT INTO {tableName} (col) VALUES (?)";
            var param = insertCmd.CreateParameter();
            param.ParameterName = "1";
            param.DbType = DbType.Int64;
            param.Value = value;
            insertCmd.Parameters.Add(param);
            await insertCmd.ExecuteNonQueryAsync().ConfigureAwait(false);
        }

        // And Query "SELECT * FROM <table>" is executed
        using var selectCmd = connection.CreateCommand();
        selectCmd.CommandText = $"SELECT col FROM {tableName} ORDER BY col";
        using var reader = await selectCmd.ExecuteReaderAsync().ConfigureAwait(false);

        // Then Result should contain integers [0, -2147483648, 2147483647, 9223372036854775807]
        var sorted = testValues.OrderBy(x => x).ToArray();
        for (var i = 0; i < sorted.Length; i++)
        {
            Assert.True(await reader.ReadAsync().ConfigureAwait(false), $"Expected row {i}");
            Assert.Equal(sorted[i], reader.GetInt64(0));
        }
        Assert.False(await reader.ReadAsync().ConfigureAwait(false), $"Expected exactly {sorted.Length} rows");
    }

    // Scenario: should insert and select integers from table using batch parameter binding for int and synonyms
    [SnowflakeTheory]
    [InlineData("INT")]
    [InlineData("INTEGER")]
    [InlineData("BIGINT")]
    [InlineData("SMALLINT")]
    [InlineData("TINYINT")]
    [InlineData("BYTEINT")]
    public async Task ShouldInsertAndSelectIntegersFromTableUsingBatchParameterBindingForIntAndSynonymsAsync(string intType)
    {
        Skip.FutureMilestone("Bound parameters, including array binding, are not implemented (SNOW-4382583, SNOW-4382600)");
        long[] testValues = [0, 42, -2147483648, 2147483647, 9223372036854775807];

        // Given Snowflake client is logged in
        using var connection = Fixture.Factory.Create(Output);
        await connection.OpenAsync().ConfigureAwait(false);

        // And Table with <type> column exists
        var tableName = $"UD_INT_BATCH_{Guid.NewGuid():N}".Substring(0, 30);
        using var createCmd = connection.CreateCommand();
        createCmd.CommandText = $"CREATE TEMPORARY TABLE {tableName} (col {intType})";
        await createCmd.ExecuteNonQueryAsync().ConfigureAwait(false);

        // When Integer values [0, 42, -2147483648, 2147483647, 9223372036854775807] are inserted using binding
        using var insertCmd = connection.CreateCommand();
        insertCmd.CommandText = $"INSERT INTO {tableName} (col) VALUES (?)";
        var param = insertCmd.CreateParameter();
        param.ParameterName = "1";
        param.DbType = DbType.Int64;
        param.Value = testValues;
        insertCmd.Parameters.Add(param);
        Assert.Equal(testValues.Length, await insertCmd.ExecuteNonQueryAsync().ConfigureAwait(false));

        // And Query "SELECT * FROM <table>" is executed
        using var selectCmd = connection.CreateCommand();
        selectCmd.CommandText = $"SELECT col FROM {tableName} ORDER BY col";
        using var reader = await selectCmd.ExecuteReaderAsync().ConfigureAwait(false);

        // Then Result should contain integers [0, 42, -2147483648, 2147483647, 9223372036854775807]
        var sorted = testValues.OrderBy(x => x).ToArray();
        for (var i = 0; i < sorted.Length; i++)
        {
            Assert.True(await reader.ReadAsync().ConfigureAwait(false), $"Expected row {i}");
            Assert.Equal(sorted[i], reader.GetInt64(0));
        }
        Assert.False(await reader.ReadAsync().ConfigureAwait(false), $"Expected exactly {sorted.Length} rows");
    }
}
