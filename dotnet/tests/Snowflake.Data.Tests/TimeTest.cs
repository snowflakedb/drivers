using Snowflake.Data.Tests.Compatibility;

namespace Snowflake.Data.Tests;

[Trait("Category", "E2E")]
public class TimeTest : IClassFixture<ITFixture>
{
    protected readonly ITestOutputHelper Output;
    protected readonly ITFixture Fixture;

    public TimeTest(ITFixture fixture, ITestOutputHelper output)
    {
        Fixture = fixture;
        Output = output;
    }

    // Scenario: should cast time values to appropriate type
    [SnowflakeFact]
    public void ShouldCastTimeValuesToAppropriateType()
    {
        // Given Snowflake client is logged in
        using var connection = Fixture.Factory.Create(Output);
        connection.Open();

        // When Query "SELECT '10:30:00'::TIME, '00:00:00'::TIME, '23:59:59'::TIME" is executed
        using var cmd = connection.CreateCommand();
        cmd.CommandText = "SELECT '10:30:00'::TIME, '00:00:00'::TIME, '23:59:59'::TIME";
        using var reader = cmd.ExecuteReader();

        // Then All values should be returned as appropriate type
        Assert.True(reader.Read(), "Expected one row");
        Assert.Equal(new DateTime(1970, 1, 1, 10, 30, 0), reader.GetDateTime(0));
        Assert.Equal(new DateTime(1970, 1, 1, 0, 0, 0), reader.GetDateTime(1));
        Assert.Equal(new DateTime(1970, 1, 1, 23, 59, 59), reader.GetDateTime(2));
        Assert.False(reader.Read(), "Expected exactly one row");
    }

    // Scenario Outline: should select time <values>
    [SnowflakeTheory]
    [MemberData(nameof(TimeValuesData))]
    public void ShouldSelectTimeValues(string values, string queryValues, string[] expectedValues)
    {
        Skip.FutureMilestone();
        _ = values;
        var typedExpectedValues = expectedValues.Select(s => DateTime.Parse($"1970-01-01T{s}")).ToArray();

        // Given Snowflake client is logged in
        using var connection = Fixture.Factory.Create(Output);
        connection.Open();

        // When Query "SELECT <query_values>" is executed
        using var cmd = connection.CreateCommand();
        cmd.CommandText = $"SELECT {queryValues}";
        using var reader = cmd.ExecuteReader();

        // Then Result should contain times <expected_values>
        Assert.True(reader.Read(), "Expected one row");
        for (var i = 0; i < typedExpectedValues.Length; i++)
        {
            Assert.Equal(typedExpectedValues[i], reader.GetDateTime(i));
        }

        Assert.False(reader.Read(), "Expected exactly one row");
    }

    public static IEnumerable<object[]> TimeValuesData()
    {
        yield return
        [
            "basic", "'10:30:00'::TIME, '14:45:30'::TIME, '23:59:59'::TIME",
            new[] { "10:30:00", "14:45:30", "23:59:59" }
        ];
        yield return ["midnight", "'00:00:00'::TIME", new[] { "00:00:00" }];
        yield return ["microseconds", "'10:30:00.123456'::TIME", new[] { "10:30:00.123456" }];
    }

    // Scenario Outline: should handle time precision <scale>
    [SnowflakeTheory]
    [MemberData(nameof(TimePrecisionData))]
    public void ShouldHandleTimePrecisionScale(int scale, DateTime expected)
    {
        Skip.FutureMilestone();
        // Given Snowflake client is logged in
        using var connection = Fixture.Factory.Create(Output);
        connection.Open();

        // When Query "SELECT '10:30:00.123456789'::TIME(<scale>)" is executed
        using var cmd = connection.CreateCommand();
        cmd.CommandText = $"SELECT '10:30:00.123456789'::TIME({scale})";
        using var reader = cmd.ExecuteReader();

        // Then Result should contain [<expected>]
        Assert.True(reader.Read(), "Expected one row");
        Assert.Equal(expected, reader.GetDateTime(0));
        Assert.False(reader.Read(), "Expected exactly one row");
    }

    public static IEnumerable<object[]> TimePrecisionData()
    {
        yield return [0, new DateTime(1970, 1, 1, 10, 30, 0)];
        yield return [3, DateTime.Parse("1970-01-01T10:30:00.123")];
        yield return [6, DateTime.Parse("1970-01-01T10:30:00.123456")];
    }

    // Scenario: should preserve nanosecond precision for time
    [SnowflakeFact]
    public void ShouldPreserveNanosecondPrecisionForTime()
    {
        Skip.FutureMilestone();

        // Given Snowflake client is logged in
        using var connection = Fixture.Factory.Create(Output);
        connection.Open();

        // When Query "SELECT '10:30:00.123456789'::TIME" is executed
        using var cmd = connection.CreateCommand();
        cmd.CommandText = "SELECT '10:30:00.123456789'::TIME";
        using var reader = cmd.ExecuteReader();

        // Then Result should contain [10:30:00.123456789]
        Assert.True(reader.Read(), "Expected one row");
        Assert.Equal("10:30:00.123456789", reader.GetDateTime(0).TimeOfDay.ToString());
        Assert.False(reader.Read(), "Expected exactly one row");
    }

    // Scenario: should handle NULL values for time
    [SnowflakeFact]
    public void ShouldHandleNullValuesForTime()
    {
        // Given Snowflake client is logged in
        using var connection = Fixture.Factory.Create(Output);
        connection.Open();

        // When Query "SELECT '10:30:00'::TIME, NULL::TIME, '23:59:59'::TIME" is executed
        using var cmd = connection.CreateCommand();
        cmd.CommandText = "SELECT '10:30:00'::TIME, NULL::TIME, '23:59:59'::TIME";
        using var reader = cmd.ExecuteReader();

        // Then Result should contain [10:30:00, NULL, 23:59:59]
        Assert.True(reader.Read(), "Expected one row");
        Assert.Equal(new DateTime(1970, 1, 1, 10, 30, 0), reader.GetDateTime(0));
        Assert.True(reader.IsDBNull(1), "Expected NULL for column 1");
        Assert.Equal(new DateTime(1970, 1, 1, 23, 59, 59), reader.GetDateTime(2));
        Assert.False(reader.Read(), "Expected exactly one row");
    }

    // Scenario: should download large result set with multiple chunks for time
    [SnowflakeFact]
    public void ShouldDownloadLargeResultSetWithMultipleChunksForTime()
    {
        const int rowCount = 100000;

        // Given Snowflake client is logged in
        using var connection = Fixture.Factory.Create(Output);
        connection.Open();

        // When Query "SELECT TIMEADD(millisecond, ROW_NUMBER() OVER (ORDER BY seq4()) - 1, '00:00:00'::TIME) as t FROM TABLE(GENERATOR(ROWCOUNT => 100000)) ORDER BY t" is executed
        using var cmd = connection.CreateCommand();
        cmd.CommandText =
            "SELECT TIMEADD(millisecond, ROW_NUMBER() OVER (ORDER BY seq4()) - 1, '00:00:00'::TIME) as t FROM TABLE(GENERATOR(ROWCOUNT => 100000)) ORDER BY t";
        using var reader = cmd.ExecuteReader();

        // Then Result should contain 100000 sequentially increasing time values from 00:00:00
        var count = 0;
        var epoch = new DateTime(1970, 1, 1);
        var previous = epoch.AddMilliseconds(-1);
        while (reader.Read())
        {
            var value = reader.GetDateTime(0);
            Assert.True(value > previous, $"Expected strictly increasing at row {count}: {previous} -> {value}");
            if (count == 0)
                Assert.Equal(epoch, value);
            previous = value;
            count++;
        }

        Assert.Equal(rowCount, count);
    }

    // Scenario Outline: should select <values> from table for time
    [SnowflakeTheory]
    [MemberData(nameof(TimeTableValuesData))]
    public void ShouldSelectValuesFromTableForTime(string values, string insertValues, string?[] expectedValues)
    {
        Skip.FutureMilestone();
        _ = values; // used for test display name only


        // Given Snowflake client is logged in
        using var connection = Fixture.Factory.Create(Output);
        connection.Open();

        // And Table with TIME column exists with values <insert_values>
        var tableName = $"UD_TIME_TBL_{Guid.NewGuid():N}".Substring(0, 30);
        using var createCmd = connection.CreateCommand();
        createCmd.CommandText = $"CREATE TEMPORARY TABLE {tableName} (col TIME)";
        createCmd.ExecuteNonQuery();

        using var insertCmd = connection.CreateCommand();
        insertCmd.CommandText = $"INSERT INTO {tableName} (col) VALUES {insertValues}";
        insertCmd.ExecuteNonQuery();

        // When Query "SELECT * FROM <table> ORDER BY col NULLS LAST" is executed
        using var selectCmd = connection.CreateCommand();
        selectCmd.CommandText = $"SELECT * FROM {tableName} ORDER BY col NULLS LAST";
        using var reader = selectCmd.ExecuteReader();

        // Then Result should contain times <expected_values>
        var rowIndex = 0;
        while (reader.Read())
        {
            if (expectedValues[rowIndex] is null)
            {
                Assert.True(reader.IsDBNull(0), $"Expected NULL at row {rowIndex}");
            }
            else
            {
                Assert.Equal(DateTime.Parse($"1970-01-01T{expectedValues[rowIndex]}"), reader.GetDateTime(0));
            }

            rowIndex++;
        }

        Assert.Equal(expectedValues.Length, rowIndex);
    }

    public static IEnumerable<object[]> TimeTableValuesData()
    {
        yield return
            ["basic", "('10:30:00'), ('14:45:30'), ('23:59:59')", new string?[] { "10:30:00", "14:45:30", "23:59:59" }];
        yield return
        [
            "midnight", "('00:00:00'), ('12:00:00'), ('23:59:59')", new string?[] { "00:00:00", "12:00:00", "23:59:59" }
        ];
        yield return
            ["microseconds", "('10:30:00'), ('10:30:00.123456')", new string?[] { "10:30:00", "10:30:00.123456" }];
        yield return ["null", "(NULL), ('10:30:00')", new string?[] { "10:30:00", null }];
    }

    // Scenario: should download large result set with multiple chunks from table for time
    [SnowflakeFact]
    public void ShouldDownloadLargeResultSetWithMultipleChunksFromTableForTime()
    {
        const int rowCount = 100000;

        // Given Snowflake client is logged in
        using var connection = Fixture.Factory.Create(Output);
        connection.Open();

        // And Table with TIME column exists with 100000 sequential time values starting from 00:00:00
        var tableName = $"UD_TIME_LRG_{Guid.NewGuid():N}".Substring(0, 30);
        using var createCmd = connection.CreateCommand();
        createCmd.CommandText = $"CREATE TEMPORARY TABLE {tableName} (col TIME)";
        createCmd.ExecuteNonQuery();

        using var insertCmd = connection.CreateCommand();
        insertCmd.CommandText =
            $"INSERT INTO {tableName} SELECT TIMEADD(millisecond, ROW_NUMBER() OVER (ORDER BY seq4()) - 1, '00:00:00'::TIME) FROM TABLE(GENERATOR(ROWCOUNT => {rowCount}))";
        insertCmd.ExecuteNonQuery();

        // When Query "SELECT * FROM <table> ORDER BY col" is executed
        using var selectCmd = connection.CreateCommand();
        selectCmd.CommandText = $"SELECT * FROM {tableName} ORDER BY col";
        using var reader = selectCmd.ExecuteReader();

        // Then Result should contain 100000 sequentially increasing time values from 00:00:00
        var count = 0;
        var epoch = new DateTime(1970, 1, 1);
        var previous = epoch.AddMilliseconds(-1);
        while (reader.Read())
        {
            var value = reader.GetDateTime(0);
            Assert.True(value > previous, $"Expected strictly increasing at row {count}: {previous} -> {value}");
            if (count == 0)
                Assert.Equal(epoch, value);
            previous = value;
            count++;
        }

        Assert.Equal(rowCount, count);
    }

    // Scenario: should select time using parameter binding
    [SnowflakeFact]
    public void ShouldSelectTimeUsingParameterBinding()
    {
        Skip.FutureMilestone();
        // Given Snowflake client is logged in
        using var connection = Fixture.Factory.Create(Output);
        connection.Open();

        // When Query "SELECT ?::TIME, ?::TIME, ?::TIME" is executed with bound time values [10:30:00, 14:45:30, 23:59:59]
        using var cmd = connection.CreateCommand();
        cmd.CommandText = "SELECT ?::TIME, ?::TIME, ?::TIME";
        var param1 = cmd.CreateParameter();
        param1.ParameterName = "1";
        param1.DbType = DbType.Time;
        param1.Value = new DateTime(1970, 1, 1, 10, 30, 0);
        cmd.Parameters.Add(param1);
        var param2 = cmd.CreateParameter();
        param2.ParameterName = "2";
        param2.DbType = DbType.Time;
        param2.Value = new DateTime(1970, 1, 1, 14, 45, 30);
        cmd.Parameters.Add(param2);
        var param3 = cmd.CreateParameter();
        param3.ParameterName = "3";
        param3.DbType = DbType.Time;
        param3.Value = new DateTime(1970, 1, 1, 23, 59, 59);
        cmd.Parameters.Add(param3);
        using var reader = cmd.ExecuteReader();

        // Then Result should contain times [10:30:00, 14:45:30, 23:59:59]
        Assert.True(reader.Read(), "Expected one row");
        Assert.Equal(new DateTime(1970, 1, 1, 10, 30, 0), reader.GetDateTime(0));
        Assert.Equal(new DateTime(1970, 1, 1, 14, 45, 30), reader.GetDateTime(1));
        Assert.Equal(new DateTime(1970, 1, 1, 23, 59, 59), reader.GetDateTime(2));
        Assert.False(reader.Read(), "Expected exactly one row");
    }

    // Scenario: should select null time using parameter binding
    [SnowflakeFact]
    public void ShouldSelectNullTimeUsingParameterBinding()
    {
        Skip.FutureMilestone();
        // Given Snowflake client is logged in
        using var connection = Fixture.Factory.Create(Output);
        connection.Open();

        // When Query "SELECT ?::TIME" is executed with bound NULL value
        using var cmd = connection.CreateCommand();
        cmd.CommandText = "SELECT ?::TIME";
        var param = cmd.CreateParameter();
        param.ParameterName = "1";
        param.DbType = DbType.Time;
        param.Value = DBNull.Value;
        cmd.Parameters.Add(param);
        using var reader = cmd.ExecuteReader();

        // Then Result should contain [NULL]
        Assert.True(reader.Read(), "Expected one row");
        Assert.True(reader.IsDBNull(0), "Expected NULL for column 0");
        Assert.False(reader.Read(), "Expected exactly one row");
    }

    // Scenario: should insert time using parameter binding
    [SnowflakeFact]
    public void ShouldInsertTimeUsingParameterBinding()
    {
        Skip.FutureMilestone();
        // Given Snowflake client is logged in
        using var connection = Fixture.Factory.Create(Output);
        connection.Open();

        // And Table with TIME column exists
        var tableName = $"UD_TIME_BND_{Guid.NewGuid():N}".Substring(0, 30);
        using var createCmd = connection.CreateCommand();
        createCmd.CommandText = $"CREATE TEMPORARY TABLE {tableName} (col TIME)";
        createCmd.ExecuteNonQuery();

        // When Time values [00:00:00, 10:30:00, 14:45:30, 23:59:59] are inserted using binding
        DateTime[] testValues =
        [
            new(1970, 1, 1, 0, 0, 0),
            new(1970, 1, 1, 10, 30, 0),
            new(1970, 1, 1, 14, 45, 30),
            new(1970, 1, 1, 23, 59, 59)
        ];
        foreach (var value in testValues)
        {
            using var insertCmd = connection.CreateCommand();
            insertCmd.CommandText = $"INSERT INTO {tableName} (col) VALUES (?)";
            var param = insertCmd.CreateParameter();
            param.ParameterName = "1";
            param.DbType = DbType.Time;
            param.Value = value;
            insertCmd.Parameters.Add(param);
            insertCmd.ExecuteNonQuery();
        }

        // And Query "SELECT * FROM <table> ORDER BY col" is executed
        using var selectCmd = connection.CreateCommand();
        selectCmd.CommandText = $"SELECT col FROM {tableName} ORDER BY col";
        using var reader = selectCmd.ExecuteReader();

        // Then Result should contain times [00:00:00, 10:30:00, 14:45:30, 23:59:59]
        Assert.True(reader.Read());
        Assert.Equal(new DateTime(1970, 1, 1, 0, 0, 0), reader.GetDateTime(0));
        Assert.True(reader.Read());
        Assert.Equal(new DateTime(1970, 1, 1, 10, 30, 0), reader.GetDateTime(0));
        Assert.True(reader.Read());
        Assert.Equal(new DateTime(1970, 1, 1, 14, 45, 30), reader.GetDateTime(0));
        Assert.True(reader.Read());
        Assert.Equal(new DateTime(1970, 1, 1, 23, 59, 59), reader.GetDateTime(0));
        Assert.False(reader.Read(), "Expected exactly 4 rows");
    }

    // Scenario: should insert time with fractional seconds using parameter binding
    [SnowflakeFact]
    public void ShouldInsertTimeWithFractionalSecondsUsingParameterBinding()
    {
        Skip.FutureMilestone();
        // Given Snowflake client is logged in
        using var connection = Fixture.Factory.Create(Output);
        connection.Open();

        // And Table with TIME column exists
        var tableName = $"UD_TIME_FRC_{Guid.NewGuid():N}".Substring(0, 30);
        using var createCmd = connection.CreateCommand();
        createCmd.CommandText = $"CREATE TEMPORARY TABLE {tableName} (col TIME)";
        createCmd.ExecuteNonQuery();

        // When Time values [10:30:00.123456, 14:45:30.654321] are bulk-inserted using multirow binding
        DateTime[] testValues =
            [DateTime.Parse("1970-01-01T10:30:00.123456"), DateTime.Parse("1970-01-01T14:45:30.654321")];
        foreach (var value in testValues)
        {
            using var insertCmd = connection.CreateCommand();
            insertCmd.CommandText = $"INSERT INTO {tableName} (col) VALUES (?)";
            var param = insertCmd.CreateParameter();
            param.ParameterName = "1";
            param.DbType = DbType.Time;
            param.Value = value;
            insertCmd.Parameters.Add(param);
            insertCmd.ExecuteNonQuery();
        }

        // And Query "SELECT * FROM <table> ORDER BY col" is executed
        using var selectCmd = connection.CreateCommand();
        selectCmd.CommandText = $"SELECT col FROM {tableName} ORDER BY col";
        using var reader = selectCmd.ExecuteReader();

        // Then Result should contain times [10:30:00.123456, 14:45:30.654321]
        Assert.True(reader.Read());
        Assert.Equal(DateTime.Parse("1970-01-01T10:30:00.123456"), reader.GetDateTime(0));
        Assert.True(reader.Read());
        Assert.Equal(DateTime.Parse("1970-01-01T14:45:30.654321"), reader.GetDateTime(0));
        Assert.False(reader.Read(), "Expected exactly 2 rows");
    }
}
