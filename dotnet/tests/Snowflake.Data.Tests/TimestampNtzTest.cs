using System.Data.Common;
using Snowflake.Data.Tests.Compatibility;

namespace Snowflake.Data.Tests;

[Trait("Category", "E2E")]
public class TimestampNtzTest : IClassFixture<ITFixture>
{
    protected readonly ITestOutputHelper Output;
    protected readonly ITFixture Fixture;

    public TimestampNtzTest(ITFixture fixture, ITestOutputHelper output)
    {
        Fixture = fixture;
        Output = output;
    }

    // Scenario: should cast timestamp_ntz values to appropriate type
    [SnowflakeFact]
    public void ShouldCastTimestampNtzValuesToAppropriateType()
    {
        Skip.FutureMilestone();

        // Given Snowflake client is logged in
        using var connection = Fixture.Factory.Create(Output);
        connection.Open();

        // When Query "SELECT '2024-01-15 10:30:00'::TIMESTAMP_NTZ" is executed
        using var cmd = connection.CreateCommand();
        cmd.CommandText = "SELECT '2024-01-15 10:30:00'::TIMESTAMP_NTZ";
        using var reader = cmd.ExecuteReader();

        // Then All values should be returned as appropriate type
        Assert.True(reader.Read(), "Expected one row");
        var value = reader.GetDateTime(0);

        // And Values should not have timezone info
        Assert.Equal(DateTimeKind.Unspecified, value.Kind);
        Assert.False(reader.Read(), "Expected exactly one row");
    }

    // Scenario Outline: should select timestamp_ntz <values>
    [SnowflakeTheory]
    [MemberData(nameof(SelectTimestampNtzData))]
    public void ShouldSelectTimestampNtzValues(string values, string queryValues, DateTime[] expectedValues)
    {
        _ = values; // used for test display name only
        Skip.FutureMilestone();

        // Given Snowflake client is logged in
        using var connection = Fixture.Factory.Create(Output);
        connection.Open();

        // When Query "SELECT <query_values>" is executed
        using var cmd = connection.CreateCommand();
        cmd.CommandText = $"SELECT {queryValues}";
        using var reader = cmd.ExecuteReader();

        // Then Result should contain timestamps <expected_values>
        Assert.True(reader.Read(), "Expected one row");
        for (var i = 0; i < expectedValues.Length; i++)
        {
            var value = reader.GetDateTime(i);
            Assert.Equal(expectedValues[i], value);

            // And Values should not have timezone info
            Assert.Equal(DateTimeKind.Unspecified, value.Kind);
        }
        Assert.False(reader.Read(), "Expected exactly one row");
    }

    public static IEnumerable<object[]> SelectTimestampNtzData()
    {
        yield return [
            "basic",
            "'2024-01-15 10:30:00'::TIMESTAMP_NTZ, '2024-06-20 14:45:30'::TIMESTAMP_NTZ",
            new[] { new DateTime(2024, 1, 15, 10, 30, 0), new DateTime(2024, 6, 20, 14, 45, 30) }
        ];
        yield return [
            "epoch",
            "'1970-01-01 00:00:00'::TIMESTAMP_NTZ",
            new[] { new DateTime(1970, 1, 1, 0, 0, 0) }
        ];
        yield return [
            "microseconds",
            "'2024-01-15 10:30:00.123456'::TIMESTAMP_NTZ",
            new[] { new DateTime(2024, 1, 15, 10, 30, 0).AddTicks(1234560) }
        ];
    }

    // Scenario: should handle NULL values for timestamp_ntz
    [SnowflakeFact]
    public void ShouldHandleNullValuesForTimestampNtz()
    {
        Skip.FutureMilestone();

        // Given Snowflake client is logged in
        using var connection = Fixture.Factory.Create(Output);
        connection.Open();

        // When Query "SELECT '2024-01-15 10:30:00'::TIMESTAMP_NTZ, NULL::TIMESTAMP_NTZ" is executed
        using var cmd = connection.CreateCommand();
        cmd.CommandText = "SELECT '2024-01-15 10:30:00'::TIMESTAMP_NTZ, NULL::TIMESTAMP_NTZ";
        using var reader = cmd.ExecuteReader();

        // Then Result should contain [2024-01-15 10:30:00, NULL]
        Assert.True(reader.Read(), "Expected one row");
        Assert.Equal(new DateTime(2024, 1, 15, 10, 30, 0), reader.GetDateTime(0));
        Assert.True(reader.IsDBNull(1), "Expected NULL for column 1");
        Assert.False(reader.Read(), "Expected exactly one row");
    }

    // Scenario: should download large result set with multiple chunks for timestamp_ntz
    [SnowflakeFact]
    public void ShouldDownloadLargeResultSetWithMultipleChunksForTimestampNtz()
    {
        Skip.FutureMilestone();

        const int rowCount = 50000;

        // Given Snowflake client is logged in
        using var connection = Fixture.Factory.Create(Output);
        connection.Open();

        // When Query "SELECT DATEADD(second, ROW_NUMBER() OVER (ORDER BY seq8()) - 1, '2024-01-01 00:00:00'::TIMESTAMP_NTZ) as ts FROM TABLE(GENERATOR(ROWCOUNT => 50000)) ORDER BY ts" is executed
        using var cmd = connection.CreateCommand();
        cmd.CommandText = "SELECT DATEADD(second, ROW_NUMBER() OVER (ORDER BY seq8()) - 1, '2024-01-01 00:00:00'::TIMESTAMP_NTZ) as ts FROM TABLE(GENERATOR(ROWCOUNT => 50000)) ORDER BY ts";
        using var reader = cmd.ExecuteReader();

        // Then Result should contain 50000 sequentially increasing timestamps from 2024-01-01 00:00:00
        var baseTs = new DateTime(2024, 1, 1, 0, 0, 0);
        var count = 0;
        while (reader.Read())
        {
            Assert.Equal(baseTs.AddSeconds(count), reader.GetDateTime(0));
            count++;
        }
        Assert.Equal(rowCount, count);
    }

    // Scenario Outline: should select <values> from table for timestamp_ntz
    [SnowflakeTheory]
    [MemberData(nameof(TableTimestampNtzData))]
    public void ShouldSelectValuesFromTableForTimestampNtz(string values, string insertValues, DateTime?[] expectedValues)
    {
        _ = values; // used for test display name only
        Skip.FutureMilestone();

        // Given Snowflake client is logged in
        using var connection = Fixture.Factory.Create(Output);
        connection.Open();

        // And Table with TIMESTAMP_NTZ column exists with values <insert_values>
        var tableName = $"UD_TSNTZ_TBL_{Guid.NewGuid():N}".Substring(0, 30);
        using var createCmd = connection.CreateCommand();
        createCmd.CommandText = $"CREATE TEMPORARY TABLE {tableName} (col TIMESTAMP_NTZ)";
        createCmd.ExecuteNonQuery();

        using var insertCmd = connection.CreateCommand();
        insertCmd.CommandText = $"INSERT INTO {tableName} (col) VALUES {insertValues}";
        insertCmd.ExecuteNonQuery();

        // When Query "SELECT * FROM <table> ORDER BY col NULLS LAST" is executed
        using var selectCmd = connection.CreateCommand();
        selectCmd.CommandText = $"SELECT * FROM {tableName} ORDER BY col NULLS LAST";
        using var reader = selectCmd.ExecuteReader();

        // Then Result should contain timestamps <expected_values>
        var rowIndex = 0;
        while (reader.Read())
        {
            if (expectedValues[rowIndex] is null)
            {
                Assert.True(reader.IsDBNull(0), $"Expected NULL at row {rowIndex}");
            }
            else
            {
                var value = reader.GetDateTime(0);
                Assert.Equal(expectedValues[rowIndex]!.Value, value);

                // And Values should not have timezone info
                Assert.Equal(DateTimeKind.Unspecified, value.Kind);
            }
            rowIndex++;
        }
        Assert.Equal(expectedValues.Length, rowIndex);
    }

    public static IEnumerable<object[]> TableTimestampNtzData()
    {
        yield return [
            "basic",
            "('2024-01-15 10:30:00'), ('2024-06-20 14:45:30')",
            new DateTime?[] { new DateTime(2024, 1, 15, 10, 30, 0), new DateTime(2024, 6, 20, 14, 45, 30) }
        ];
        yield return [
            "epoch",
            "('1970-01-01 00:00:00'), ('2024-01-15 10:30:00')",
            new DateTime?[] { new DateTime(1970, 1, 1, 0, 0, 0), new DateTime(2024, 1, 15, 10, 30, 0) }
        ];
        yield return [
            "microseconds",
            "('2024-01-15 10:30:00'), ('2024-01-15 10:30:00.123456')",
            new DateTime?[] { new DateTime(2024, 1, 15, 10, 30, 0), new DateTime(2024, 1, 15, 10, 30, 0).AddTicks(1234560) }
        ];
        yield return [
            "null",
            "(NULL), ('2024-01-15 10:30:00')",
            new DateTime?[] { new DateTime(2024, 1, 15, 10, 30, 0), null }
        ];
    }

    // Scenario: should download large result set with multiple chunks from table for timestamp_ntz
    [SnowflakeFact]
    public void ShouldDownloadLargeResultSetWithMultipleChunksFromTableForTimestampNtz()
    {
        Skip.FutureMilestone();

        const int rowCount = 50000;

        // Given Snowflake client is logged in
        using var connection = Fixture.Factory.Create(Output);
        connection.Open();

        // And Table with TIMESTAMP_NTZ column exists with 50000 sequential timestamp values
        var tableName = $"UD_TSNTZ_LRG_{Guid.NewGuid():N}".Substring(0, 30);
        using var createCmd = connection.CreateCommand();
        createCmd.CommandText = $"CREATE TEMPORARY TABLE {tableName} (col TIMESTAMP_NTZ)";
        createCmd.ExecuteNonQuery();

        using var insertCmd = connection.CreateCommand();
        insertCmd.CommandText = $"INSERT INTO {tableName} SELECT DATEADD(second, ROW_NUMBER() OVER (ORDER BY seq8()) - 1, '2024-01-01 00:00:00'::TIMESTAMP_NTZ) FROM TABLE(GENERATOR(ROWCOUNT => {rowCount}))";
        insertCmd.ExecuteNonQuery();

        // When Query "SELECT * FROM <table> ORDER BY col NULLS LAST" is executed
        using var selectCmd = connection.CreateCommand();
        selectCmd.CommandText = $"SELECT * FROM {tableName} ORDER BY col NULLS LAST";
        using var reader = selectCmd.ExecuteReader();

        // Then Result should contain 50000 sequentially increasing timestamps from 2024-01-01 00:00:00
        var baseTs = new DateTime(2024, 1, 1, 0, 0, 0);
        var count = 0;
        while (reader.Read())
        {
            Assert.Equal(baseTs.AddSeconds(count), reader.GetDateTime(0));
            count++;
        }
        Assert.Equal(rowCount, count);
    }

    // Scenario: should select timestamp_ntz using parameter binding
    [SnowflakeFact]
    public void ShouldSelectTimestampNtzUsingParameterBinding()
    {
        Skip.FutureMilestone();

        // Given Snowflake client is logged in
        using var connection = Fixture.Factory.Create(Output);
        connection.Open();

        // When Query "SELECT ?::TIMESTAMP_NTZ, ?::TIMESTAMP_NTZ" is executed with bound timestamp values
        using var cmd = connection.CreateCommand();
        cmd.CommandText = "SELECT ?::TIMESTAMP_NTZ, ?::TIMESTAMP_NTZ";

        var param1 = cmd.CreateParameter();
        param1.ParameterName = "1";
        param1.DbType = DbType.DateTime;
        param1.Value = new DateTime(2024, 1, 15, 10, 30, 0);
        cmd.Parameters.Add(param1);

        var param2 = cmd.CreateParameter();
        param2.ParameterName = "2";
        param2.DbType = DbType.DateTime;
        param2.Value = new DateTime(2024, 6, 20, 14, 45, 30);
        cmd.Parameters.Add(param2);

        using var reader = cmd.ExecuteReader();

        // Then Result should contain [2024-01-15 10:30:00, 2024-06-20 14:45:30]
        Assert.True(reader.Read(), "Expected one row");
        var val1 = reader.GetDateTime(0);
        Assert.Equal(new DateTime(2024, 1, 15, 10, 30, 0), val1);
        var val2 = reader.GetDateTime(1);
        Assert.Equal(new DateTime(2024, 6, 20, 14, 45, 30), val2);

        // And Values should not have timezone info
        Assert.Equal(DateTimeKind.Unspecified, val1.Kind);
        Assert.Equal(DateTimeKind.Unspecified, val2.Kind);
        Assert.False(reader.Read(), "Expected exactly one row");
    }

    // Scenario: should return NULL when selecting timestamp_ntz using parameter binding with NULL value
    [SnowflakeFact]
    public void ShouldReturnNullWhenSelectingTimestampNtzUsingParameterBindingWithNullValue()
    {
        Skip.FutureMilestone();

        // Given Snowflake client is logged in
        using var connection = Fixture.Factory.Create(Output);
        connection.Open();

        // When Query "SELECT ?::TIMESTAMP_NTZ" is executed with bound NULL value
        using var cmd = connection.CreateCommand();
        cmd.CommandText = "SELECT ?::TIMESTAMP_NTZ";

        var param = cmd.CreateParameter();
        param.ParameterName = "1";
        param.DbType = DbType.DateTime;
        param.Value = DBNull.Value;
        cmd.Parameters.Add(param);

        using var reader = cmd.ExecuteReader();

        // Then Result should contain [NULL]
        Assert.True(reader.Read(), "Expected one row");
        Assert.True(reader.IsDBNull(0), "Expected NULL for column 0");
        Assert.False(reader.Read(), "Expected exactly one row");
    }

    // Scenario Outline: should store UTC equivalent when binding timezone-aware datetime to timestamp_ntz
    [SnowflakeTheory]
    [MemberData(nameof(TimezoneAwareBindingData))]
    public void ShouldStoreUtcEquivalentWhenBindingTimezoneAwareDatetimeToTimestampNtz(string input, DateTime expected)
    {
        _ = input; // used for test display name only
        Skip.FutureMilestone();

        // Given Snowflake client is logged in
        using var connection = Fixture.Factory.Create(Output);
        connection.Open();

        // When Query "SELECT ?::TIMESTAMP_NTZ" is executed with bound aware datetime <input>
        using var cmd = connection.CreateCommand();
        cmd.CommandText = "SELECT ?::TIMESTAMP_NTZ";

        var param = cmd.CreateParameter();
        param.ParameterName = "1";
        param.DbType = DbType.DateTimeOffset;
        param.Value = DateTimeOffset.Parse(input);
        cmd.Parameters.Add(param);

        using var reader = cmd.ExecuteReader();

        // Then Result should contain [<expected>]
        Assert.True(reader.Read(), "Expected one row");
        var value = reader.GetDateTime(0);
        Assert.Equal(expected, value);

        // And Values should not have timezone info
        Assert.Equal(DateTimeKind.Unspecified, value.Kind);
        Assert.False(reader.Read(), "Expected exactly one row");
    }

    public static IEnumerable<object[]> TimezoneAwareBindingData()
    {
        yield return ["2024-01-15 10:30:00+00:00", new DateTime(2024, 1, 15, 10, 30, 0)];
        yield return ["2024-01-15 12:30:00+02:00", new DateTime(2024, 1, 15, 10, 30, 0)];
        yield return ["2024-01-15 10:30:00-05:00", new DateTime(2024, 1, 15, 15, 30, 0)];
    }

    // Scenario: should insert timestamp_ntz using parameter binding
    [SnowflakeFact]
    public void ShouldInsertTimestampNtzUsingParameterBinding()
    {
        Skip.FutureMilestone();

        DateTime[] testValues =
        [
            new DateTime(2024, 1, 15, 10, 30, 0),
            new DateTime(2024, 6, 20, 14, 45, 30),
            new DateTime(1970, 1, 1, 0, 0, 0)
        ];

        // Given Snowflake client is logged in
        using var connection = Fixture.Factory.Create(Output);
        connection.Open();

        // And Table with TIMESTAMP_NTZ column exists
        var tableName = $"UD_TSNTZ_INS_{Guid.NewGuid():N}".Substring(0, 30);
        using var createCmd = connection.CreateCommand();
        createCmd.CommandText = $"CREATE TEMPORARY TABLE {tableName} (col TIMESTAMP_NTZ)";
        createCmd.ExecuteNonQuery();

        // When Timestamp values are bulk-inserted using multirow binding
        foreach (var value in testValues)
        {
            using var insertCmd = connection.CreateCommand();
            insertCmd.CommandText = $"INSERT INTO {tableName} (col) VALUES (?)";
            var param = insertCmd.CreateParameter();
            param.ParameterName = "1";
            param.DbType = DbType.DateTime;
            param.Value = value;
            insertCmd.Parameters.Add(param);
            insertCmd.ExecuteNonQuery();
        }

        // And Query "SELECT * FROM <table> ORDER BY col NULLS LAST" is executed
        using var selectCmd = connection.CreateCommand();
        selectCmd.CommandText = $"SELECT col FROM {tableName} ORDER BY col NULLS LAST";
        using var reader = selectCmd.ExecuteReader();

        // Then SELECT should return the inserted values in ascending order
        var sorted = testValues.OrderBy(x => x).ToArray();
        for (var i = 0; i < sorted.Length; i++)
        {
            Assert.True(reader.Read(), $"Expected row {i}");
            Assert.Equal(sorted[i], reader.GetDateTime(0));
        }
        Assert.False(reader.Read(), $"Expected exactly {sorted.Length} rows");
    }

    // Scenario Outline: should return naive datetime for <type_name> alias when session mapping is TIMESTAMP_NTZ
    [SnowflakeTheory]
    [InlineData("TIMESTAMP")]
    [InlineData("DATETIME")]
    public void ShouldReturnNaiveDatetimeForTypeNameAliasWhenSessionMappingIsTimestampNtz(string typeName)
    {
        Skip.FutureMilestone();

        // Given Snowflake client is logged in
        using var connection = Fixture.Factory.Create(Output);
        connection.Open();

        // And Session TIMESTAMP_TYPE_MAPPING is set to TIMESTAMP_NTZ
        using var sessionCmd = connection.CreateCommand();
        sessionCmd.CommandText = "ALTER SESSION SET TIMESTAMP_TYPE_MAPPING = TIMESTAMP_NTZ";
        sessionCmd.ExecuteNonQuery();

        // When Query "SELECT '2024-01-15 10:30:00'::<type_name>" is executed
        using var cmd = connection.CreateCommand();
        cmd.CommandText = $"SELECT '2024-01-15 10:30:00'::{typeName}";
        using var reader = cmd.ExecuteReader();

        // Then All values should be returned as appropriate type
        Assert.True(reader.Read(), "Expected one row");
        var value = reader.GetDateTime(0);
        Assert.Equal(new DateTime(2024, 1, 15, 10, 30, 0), value);

        // And Values should not have timezone info
        Assert.Equal(DateTimeKind.Unspecified, value.Kind);
        Assert.False(reader.Read(), "Expected exactly one row");
    }

    // Scenario: should return aware datetime for TIMESTAMP alias when session mapping is TIMESTAMP_LTZ
    [SnowflakeFact]
    public async Task ShouldReturnAwareDatetimeForTimestampAliasWhenSessionMappingIsTimestampLtzAsync()
    {
        // Given Snowflake client is logged in
        using var connection = Fixture.Factory.Create(Output);
        await connection.OpenAsync().ConfigureAwait(false);
        await SetTimezoneToNonUtcAsync(connection, CancellationToken.None).ConfigureAwait(false);

        // And Session TIMESTAMP_TYPE_MAPPING is set to TIMESTAMP_LTZ
        using var sessionCmd = connection.CreateCommand();
        sessionCmd.CommandText = "ALTER SESSION SET TIMESTAMP_TYPE_MAPPING = TIMESTAMP_LTZ";
        await sessionCmd.ExecuteNonQueryAsync().ConfigureAwait(false);

        // When Query "SELECT '2024-01-15 10:30:00'::TIMESTAMP" is executed
        using var cmd = connection.CreateCommand();
        cmd.CommandText = "SELECT '2024-01-15 10:30:00'::TIMESTAMP";
        using var reader = await cmd.ExecuteReaderAsync().ConfigureAwait(false);

        // Then All values should be returned as appropriate type
        Assert.True(await reader.ReadAsync().ConfigureAwait(false), "Expected one row");
        var value = reader.GetValue(0);
        Assert.IsType<DateTimeOffset>(value);

        // And Values should have timezone info
        Assert.Equal(new DateTimeOffset(2024, 01, 15, 4, 45, 0, TimeSpan.Zero), (DateTimeOffset)value);
        Assert.False(await reader.ReadAsync().ConfigureAwait(false), "Expected exactly one row");
    }

    private static async Task SetTimezoneToNonUtcAsync(ISnowflakeDbConnection connection, CancellationToken cancelToken)
    {
        using var cmd1 = connection.CreateCommand();
        cmd1.CommandText = "ALTER SESSION SET TIMEZONE = 'Asia/Kathmandu'";
        await cmd1.ExecuteNonQueryAsync(cancelToken).ConfigureAwait(false);
    }
}
