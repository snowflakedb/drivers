using System.Data.Common;
using Snowflake.Data.Tests.Compatibility;

namespace Snowflake.Data.Tests;

[Trait("Category", "E2E")]
public class TimestampLtzTest : IClassFixture<ITFixture>
{
    protected readonly ITestOutputHelper Output;
    protected readonly ITFixture Fixture;

    public TimestampLtzTest(ITFixture fixture, ITestOutputHelper output)
    {
        Fixture = fixture;
        Output = output;
    }

    // Scenario: should cast timestamp_ltz values to appropriate type
    [SnowflakeFact]
    public async Task ShouldCastTimestampLtzValuesToAppropriateTypeAsync()
    {
        // Given Snowflake client is logged in
        using var connection = Fixture.Factory.Create(Output);
        await connection.OpenAsync().ConfigureAwait(false);
        await SetTimezoneToNonUtcAsync(connection, CancellationToken.None).ConfigureAwait(false);

        // When Query "SELECT '2024-01-15 10:30:00 +00:00'::TIMESTAMP_LTZ" is executed
        using var cmd = connection.CreateCommand();
        cmd.CommandText = "SELECT '2024-01-15 10:30:00 +00:00'::TIMESTAMP_LTZ";
        using var reader = await cmd.ExecuteReaderAsync().ConfigureAwait(false);

        // Then All values should be returned as appropriate type
        Assert.True(await reader.ReadAsync().ConfigureAwait(false), "Expected one row");
        var value = reader.GetValue(0);
        Assert.IsType<DateTimeOffset>(value);

        // And Values should have timezone info
        var dto = (DateTimeOffset)value;
        Assert.Equal(new DateTime(2024, 1, 15, 10, 30, 0, DateTimeKind.Utc), dto);
        Assert.False(await reader.ReadAsync().ConfigureAwait(false), "Expected exactly one row");
    }

    // Scenario Outline: should select timestamp_ltz <values>
    [SnowflakeFact]
    public async Task ShouldSelectTimestampLtzValuesAsync()
    {
        Skip.FutureMilestone();

        // Given Snowflake client is logged in
        using var connection = Fixture.Factory.Create(Output);
        await connection.OpenAsync().ConfigureAwait(false);
        await SetTimezoneToNonUtcAsync(connection, CancellationToken.None).ConfigureAwait(false);

        // When Query "SELECT <query_values>" is executed
        using var cmd = connection.CreateCommand();
        cmd.CommandText = "SELECT '2024-01-15 10:30:00 +00:00'::TIMESTAMP_LTZ, '2024-06-20 14:45:30 +00:00'::TIMESTAMP_LTZ";
        using var reader = await cmd.ExecuteReaderAsync().ConfigureAwait(false);

        // Then Result should contain timestamps <expected_values>
        Assert.True(await reader.ReadAsync().ConfigureAwait(false), "Expected one row");
        var ts1 = reader.GetDateTime(0);
        var ts2 = reader.GetDateTime(1);
        Assert.Equal(new DateTime(2024, 1, 15, 10, 30, 0, DateTimeKind.Utc), ts1.ToUniversalTime());
        Assert.Equal(new DateTime(2024, 6, 20, 14, 45, 30, DateTimeKind.Utc), ts2.ToUniversalTime());
        Assert.False(await reader.ReadAsync().ConfigureAwait(false), "Expected exactly one row");
    }

    // Scenario Outline: should select timestamp_ltz epoch
    [SnowflakeFact]
    public async Task ShouldSelectTimestampLtzEpochAsync()
    {
        Skip.FutureMilestone();

        // Given Snowflake client is logged in
        using var connection = Fixture.Factory.Create(Output);
        await connection.OpenAsync().ConfigureAwait(false);
        await SetTimezoneToNonUtcAsync(connection, CancellationToken.None).ConfigureAwait(false);

        // When Query "SELECT '1970-01-01 00:00:00 +00:00'::TIMESTAMP_LTZ" is executed
        using var cmd = connection.CreateCommand();
        cmd.CommandText = "SELECT '1970-01-01 00:00:00 +00:00'::TIMESTAMP_LTZ";
        using var reader = await cmd.ExecuteReaderAsync().ConfigureAwait(false);

        // Then Result should contain timestamps 1970-01-01 00:00:00 UTC
        Assert.True(await reader.ReadAsync().ConfigureAwait(false), "Expected one row");
        var ts = reader.GetDateTime(0);
        Assert.Equal(new DateTime(1970, 1, 1, 0, 0, 0, DateTimeKind.Utc), ts.ToUniversalTime());
        Assert.False(await reader.ReadAsync().ConfigureAwait(false), "Expected exactly one row");
    }

    // Scenario Outline: should select timestamp_ltz microseconds
    [SnowflakeFact]
    public async Task ShouldSelectTimestampLtzMicrosecondsAsync()
    {
        Skip.FutureMilestone();

        // Given Snowflake client is logged in
        using var connection = Fixture.Factory.Create(Output);
        await connection.OpenAsync().ConfigureAwait(false);
        await SetTimezoneToNonUtcAsync(connection, CancellationToken.None).ConfigureAwait(false);

        // When Query "SELECT '2024-01-15 10:30:00.123456 +00:00'::TIMESTAMP_LTZ" is executed
        using var cmd = connection.CreateCommand();
        cmd.CommandText = "SELECT '2024-01-15 10:30:00.123456 +00:00'::TIMESTAMP_LTZ";
        using var reader = await cmd.ExecuteReaderAsync().ConfigureAwait(false);

        // Then Result should contain timestamps 2024-01-15 10:30:00.123456 UTC
        Assert.True(await reader.ReadAsync().ConfigureAwait(false), "Expected one row");
        var ts = reader.GetDateTime(0);
        var utc = ts.ToUniversalTime();
        var expected = new DateTime(2024, 1, 15, 10, 30, 0, 123, DateTimeKind.Utc);
#if NET6_0_OR_GREATER
        Assert.InRange(ts.ToUniversalTime(), expected, expected.AddMicroseconds(1_000 - 1));
        Assert.Equal(456, utc.Microsecond);
#endif
        Assert.False(await reader.ReadAsync().ConfigureAwait(false), "Expected exactly one row");
    }

    // Scenario: should select sub-second timestamp_ltz values before epoch
    [SnowflakeFact]
    public async Task ShouldSelectSubSecondTimestampLtzValuesBeforeEpochAsync()
    {
        Skip.FutureMilestone();
        // TODO future milestone - reference driver doesn't support this scenario well.

        // Given Snowflake client is logged in
        using var connection = Fixture.Factory.Create(Output);
        await connection.OpenAsync().ConfigureAwait(false);
        await SetTimezoneToNonUtcAsync(connection, CancellationToken.None).ConfigureAwait(false);

        // When Sub-second timestamp_ltz values before the epoch are selected
        using var cmd1 = connection.CreateCommand();
        cmd1.CommandText = "SELECT '1969-12-31 23:59:59.999999999 +00:00'::TIMESTAMP_LTZ";
        using var reader1 = await cmd1.ExecuteReaderAsync().ConfigureAwait(false);

        // Then Result should contain the expected sub-second values before the epoch
        Assert.True(await reader1.ReadAsync().ConfigureAwait(false), "Expected one row");
        var ts1 = reader1.GetDateTime(0);
        Assert.Equal(1969, ts1.Year);
        Assert.Equal(12, ts1.Month);
        Assert.Equal(31, ts1.Day);
        Assert.Equal(23, ts1.Hour);
        Assert.Equal(59, ts1.Minute);
        Assert.Equal(59, ts1.Second);
#if NET6_0_OR_GREATER
        Assert.Equal(999, ts1.Millisecond);
        Assert.Equal(999, ts1.Microsecond);
#endif
        Assert.False(await reader1.ReadAsync().ConfigureAwait(false), "Expected exactly one row");

        using var cmd2 = connection.CreateCommand();
        cmd2.CommandText = "SELECT '1969-12-31 23:59:58.5 +00:00'::TIMESTAMP_LTZ(3)";
        using var reader2 = await cmd2.ExecuteReaderAsync().ConfigureAwait(false);
        Assert.True(await reader2.ReadAsync().ConfigureAwait(false), "Expected one row");
        var ts2 = reader2.GetDateTime(0).ToUniversalTime();
        Assert.Equal(new DateTime(1969, 12, 31, 23, 59, 58, 500, DateTimeKind.Utc), ts2);
        Assert.False(await reader2.ReadAsync().ConfigureAwait(false), "Expected exactly one row");
    }

    // Scenario: should handle NULL values for timestamp_ltz
    [SnowflakeFact]
    public async Task ShouldHandleNullValuesForTimestampLtzAsync()
    {
        Skip.FutureMilestone();

        // Given Snowflake client is logged in
        using var connection = Fixture.Factory.Create(Output);
        await connection.OpenAsync().ConfigureAwait(false);
        await SetTimezoneToNonUtcAsync(connection, CancellationToken.None).ConfigureAwait(false);

        // When Query "SELECT '2024-01-15 10:30:00 +00:00'::TIMESTAMP_LTZ, NULL::TIMESTAMP_LTZ" is executed
        using var cmd = connection.CreateCommand();
        cmd.CommandText = "SELECT '2024-01-15 10:30:00 +00:00'::TIMESTAMP_LTZ, NULL::TIMESTAMP_LTZ";
        using var reader = await cmd.ExecuteReaderAsync().ConfigureAwait(false);

        // Then Result should contain [2024-01-15 10:30:00 UTC, NULL]
        Assert.True(await reader.ReadAsync().ConfigureAwait(false), "Expected one row");
        Assert.False(reader.IsDBNull(0), "Expected non-NULL for column 0");
        Assert.Equal(new DateTime(2024, 1, 15, 10, 30, 0, DateTimeKind.Utc), reader.GetDateTime(0).ToUniversalTime());
        Assert.True(reader.IsDBNull(1), "Expected NULL for column 1");
        Assert.False(await reader.ReadAsync().ConfigureAwait(false), "Expected exactly one row");
    }

    // Scenario: should download large result set with multiple chunks for timestamp_ltz
    [SnowflakeFact]
    public async Task ShouldDownloadLargeResultSetWithMultipleChunksForTimestampLtzAsync()
    {
        Skip.FutureMilestone();

        const int rowCount = 50000;

        // Given Snowflake client is logged in
        using var connection = Fixture.Factory.Create(Output);
        await connection.OpenAsync().ConfigureAwait(false);
        await SetTimezoneToNonUtcAsync(connection, CancellationToken.None).ConfigureAwait(false);

        // When Query "SELECT DATEADD(second, ROW_NUMBER() OVER (ORDER BY seq8()) - 1, '2024-01-01 00:00:00 +00:00'::TIMESTAMP_LTZ) as ts FROM TABLE(GENERATOR(ROWCOUNT => 50000)) ORDER BY ts" is executed
        using var cmd = connection.CreateCommand();
        cmd.CommandText = $"SELECT DATEADD(second, ROW_NUMBER() OVER (ORDER BY seq8()) - 1, '2024-01-01 00:00:00 +00:00'::TIMESTAMP_LTZ) as ts FROM TABLE(GENERATOR(ROWCOUNT => {rowCount})) ORDER BY ts";
        using var reader = await cmd.ExecuteReaderAsync().ConfigureAwait(false);

        // Then Result should contain 50000 sequentially increasing timestamps from 2024-01-01 00:00:00 UTC
        var baseTs = new DateTime(2024, 1, 1, 0, 0, 0, DateTimeKind.Utc);
        var count = 0;
        while (await reader.ReadAsync().ConfigureAwait(false))
        {
            var ts = reader.GetDateTime(0).ToUniversalTime();
            Assert.Equal(baseTs.AddSeconds(count), ts);
            count++;
        }
        Assert.Equal(rowCount, count);
    }

    // Scenario Outline: should select <values> from table for timestamp_ltz
    [SnowflakeFact]
    public async Task ShouldSelectValuesFromTableForTimestampLtzAsync()
    {
        Skip.FutureMilestone();

        // Given Snowflake client is logged in
        using var connection = Fixture.Factory.Create(Output);
        await connection.OpenAsync().ConfigureAwait(false);
        await SetTimezoneToNonUtcAsync(connection, CancellationToken.None).ConfigureAwait(false);

        // And Table with TIMESTAMP_LTZ column exists with values <insert_values>
        var tableName = $"UD_TSLTZ_TBL_{Guid.NewGuid():N}".Substring(0, 30);
        using var createCmd = connection.CreateCommand();
        createCmd.CommandText = $"CREATE TEMPORARY TABLE {tableName} (col TIMESTAMP_LTZ)";
        await createCmd.ExecuteNonQueryAsync().ConfigureAwait(false);

        using var insertCmd = connection.CreateCommand();
        insertCmd.CommandText = $"INSERT INTO {tableName} (col) VALUES ('2024-01-15 10:30:00 +00:00'), ('2024-06-20 14:45:30 +00:00')";
        await insertCmd.ExecuteNonQueryAsync().ConfigureAwait(false);

        // When Query "SELECT * FROM <table> ORDER BY col" is executed
        using var selectCmd = connection.CreateCommand();
        selectCmd.CommandText = $"SELECT * FROM {tableName} ORDER BY col";
        using var reader = await selectCmd.ExecuteReaderAsync().ConfigureAwait(false);

        // Then Result should contain timestamps <expected_values>
        Assert.True(await reader.ReadAsync().ConfigureAwait(false), "Expected row 0");
        Assert.Equal(new DateTime(2024, 1, 15, 10, 30, 0, DateTimeKind.Utc), reader.GetDateTime(0).ToUniversalTime());
        Assert.True(await reader.ReadAsync().ConfigureAwait(false), "Expected row 1");
        Assert.Equal(new DateTime(2024, 6, 20, 14, 45, 30, DateTimeKind.Utc), reader.GetDateTime(0).ToUniversalTime());
        Assert.False(await reader.ReadAsync().ConfigureAwait(false), "Expected exactly two rows");
    }

    // Scenario Outline: should select epoch from table for timestamp_ltz
    [SnowflakeFact]
    public async Task ShouldSelectEpochFromTableForTimestampLtzAsync()
    {
        Skip.FutureMilestone();

        // Given Snowflake client is logged in
        using var connection = Fixture.Factory.Create(Output);
        await connection.OpenAsync().ConfigureAwait(false);
        await SetTimezoneToNonUtcAsync(connection, CancellationToken.None).ConfigureAwait(false);

        // And Table with TIMESTAMP_LTZ column exists with values '1970-01-01 00:00:00 +00:00', '2024-01-15 10:30:00 +00:00'
        var tableName = $"UD_TSLTZ_EPO_{Guid.NewGuid():N}".Substring(0, 30);
        using var createCmd = connection.CreateCommand();
        createCmd.CommandText = $"CREATE TEMPORARY TABLE {tableName} (col TIMESTAMP_LTZ)";
        await createCmd.ExecuteNonQueryAsync().ConfigureAwait(false);

        using var insertCmd = connection.CreateCommand();
        insertCmd.CommandText = $"INSERT INTO {tableName} (col) VALUES ('1970-01-01 00:00:00 +00:00'), ('2024-01-15 10:30:00 +00:00')";
        await insertCmd.ExecuteNonQueryAsync().ConfigureAwait(false);

        // When Query "SELECT * FROM <table> ORDER BY col" is executed
        using var selectCmd = connection.CreateCommand();
        selectCmd.CommandText = $"SELECT * FROM {tableName} ORDER BY col";
        using var reader = await selectCmd.ExecuteReaderAsync().ConfigureAwait(false);

        // Then Result should contain timestamps 1970-01-01 00:00:00 UTC, 2024-01-15 10:30:00 UTC
        Assert.True(await reader.ReadAsync().ConfigureAwait(false), "Expected row 0");
        Assert.Equal(new DateTime(1970, 1, 1, 0, 0, 0, DateTimeKind.Utc), reader.GetDateTime(0).ToUniversalTime());
        Assert.True(await reader.ReadAsync().ConfigureAwait(false), "Expected row 1");
        Assert.Equal(new DateTime(2024, 1, 15, 10, 30, 0, DateTimeKind.Utc), reader.GetDateTime(0).ToUniversalTime());
        Assert.False(await reader.ReadAsync().ConfigureAwait(false), "Expected exactly two rows");
    }

    // Scenario Outline: should select null from table for timestamp_ltz
    [SnowflakeFact]
    public async Task ShouldSelectNullFromTableForTimestampLtzAsync()
    {
        Skip.FutureMilestone();

        // Given Snowflake client is logged in
        using var connection = Fixture.Factory.Create(Output);
        await connection.OpenAsync().ConfigureAwait(false);
        await SetTimezoneToNonUtcAsync(connection, CancellationToken.None).ConfigureAwait(false);

        // And Table with TIMESTAMP_LTZ column exists with values NULL, '2024-01-15 10:30:00 +00:00'
        var tableName = $"UD_TSLTZ_NUL_{Guid.NewGuid():N}".Substring(0, 30);
        using var createCmd = connection.CreateCommand();
        createCmd.CommandText = $"CREATE TEMPORARY TABLE {tableName} (col TIMESTAMP_LTZ)";
        await createCmd.ExecuteNonQueryAsync().ConfigureAwait(false);

        using var insertCmd = connection.CreateCommand();
        insertCmd.CommandText = $"INSERT INTO {tableName} (col) VALUES (NULL), ('2024-01-15 10:30:00 +00:00')";
        await insertCmd.ExecuteNonQueryAsync().ConfigureAwait(false);

        // When Query "SELECT * FROM <table> ORDER BY col" is executed
        using var selectCmd = connection.CreateCommand();
        selectCmd.CommandText = $"SELECT * FROM {tableName} ORDER BY col";
        using var reader = await selectCmd.ExecuteReaderAsync().ConfigureAwait(false);

        // Then Result should contain timestamps 2024-01-15 10:30:00 UTC, NULL
        Assert.True(await reader.ReadAsync().ConfigureAwait(false), "Expected row 0");
        Assert.Equal(new DateTime(2024, 1, 15, 10, 30, 0, DateTimeKind.Utc), reader.GetDateTime(0).ToUniversalTime());
        Assert.True(await reader.ReadAsync().ConfigureAwait(false), "Expected row 1");
        Assert.True(reader.IsDBNull(0), "Expected NULL for row 1");
        Assert.False(await reader.ReadAsync().ConfigureAwait(false), "Expected exactly two rows");
    }

    // Scenario: should download large result set with multiple chunks from table for timestamp_ltz
    [SnowflakeFact]
    public async Task ShouldDownloadLargeResultSetWithMultipleChunksFromTableForTimestampLtzAsync()
    {
        Skip.FutureMilestone();

        const int rowCount = 50000;

        // Given Snowflake client is logged in
        using var connection = Fixture.Factory.Create(Output);
        await connection.OpenAsync().ConfigureAwait(false);
        await SetTimezoneToNonUtcAsync(connection, CancellationToken.None).ConfigureAwait(false);

        // And Table with TIMESTAMP_LTZ column exists with 50000 sequential timestamp values
        var tableName = $"UD_TSLTZ_CHK_{Guid.NewGuid():N}".Substring(0, 30);
        using var createCmd = connection.CreateCommand();
        createCmd.CommandText = $"CREATE TEMPORARY TABLE {tableName} (col TIMESTAMP_LTZ)";
        await createCmd.ExecuteNonQueryAsync().ConfigureAwait(false);

        using var insertCmd = connection.CreateCommand();
        insertCmd.CommandText = $"INSERT INTO {tableName} SELECT DATEADD(second, ROW_NUMBER() OVER (ORDER BY seq8()) - 1, '2024-01-01 00:00:00 +00:00'::TIMESTAMP_LTZ) FROM TABLE(GENERATOR(ROWCOUNT => {rowCount}))";
        await insertCmd.ExecuteNonQueryAsync().ConfigureAwait(false);

        // When Query "SELECT * FROM <table> ORDER BY col" is executed
        using var selectCmd = connection.CreateCommand();
        selectCmd.CommandText = $"SELECT * FROM {tableName} ORDER BY col";
        using var reader = await selectCmd.ExecuteReaderAsync().ConfigureAwait(false);

        // Then Result should contain 50000 sequentially increasing timestamps from 2024-01-01 00:00:00 UTC
        var baseTs = new DateTime(2024, 1, 1, 0, 0, 0, DateTimeKind.Utc);
        var count = 0;
        while (await reader.ReadAsync().ConfigureAwait(false))
        {
            var ts = reader.GetDateTime(0).ToUniversalTime();
            Assert.Equal(baseTs.AddSeconds(count), ts);
            count++;
        }
        Assert.Equal(rowCount, count);
    }

    // Scenario: should select timestamp_ltz using parameter binding
    [SnowflakeFact]
    public async Task ShouldSelectTimestampLtzUsingParameterBindingAsync()
    {
        Skip.FutureMilestone();

        var ts1 = new DateTimeOffset(2024, 1, 15, 10, 30, 0, TimeSpan.Zero);
        var ts2 = new DateTimeOffset(2024, 6, 20, 14, 45, 30, TimeSpan.Zero);

        // Given Snowflake client is logged in
        using var connection = Fixture.Factory.Create(Output);
        await connection.OpenAsync().ConfigureAwait(false);
        await SetTimezoneToNonUtcAsync(connection, CancellationToken.None).ConfigureAwait(false);

        // When Query "SELECT ?::TIMESTAMP_LTZ, ?::TIMESTAMP_LTZ" is executed with bound timestamp values
        using var cmd = connection.CreateCommand();
        cmd.CommandText = "SELECT ?::TIMESTAMP_LTZ, ?::TIMESTAMP_LTZ";

        var param1 = cmd.CreateParameter();
        param1.ParameterName = "1";
        param1.DbType = DbType.DateTimeOffset;
        param1.Value = ts1;
        cmd.Parameters.Add(param1);

        var param2 = cmd.CreateParameter();
        param2.ParameterName = "2";
        param2.DbType = DbType.DateTimeOffset;
        param2.Value = ts2;
        cmd.Parameters.Add(param2);

        using var reader = await cmd.ExecuteReaderAsync().ConfigureAwait(false);

        // Then Result should contain the bound timestamps
        Assert.True(await reader.ReadAsync().ConfigureAwait(false), "Expected one row");
        Assert.Equal(ts1.UtcDateTime, reader.GetDateTime(0).ToUniversalTime());
        Assert.Equal(ts2.UtcDateTime, reader.GetDateTime(1).ToUniversalTime());
        Assert.False(await reader.ReadAsync().ConfigureAwait(false), "Expected exactly one row");
    }

    // Scenario: should select null timestamp_ltz using parameter binding
    [SnowflakeFact]
    public async Task ShouldSelectNullTimestampLtzUsingParameterBindingAsync()
    {
        Skip.FutureMilestone();

        // Given Snowflake client is logged in
        using var connection = Fixture.Factory.Create(Output);
        await connection.OpenAsync().ConfigureAwait(false);
        await SetTimezoneToNonUtcAsync(connection, CancellationToken.None).ConfigureAwait(false);

        // When Query "SELECT ?::TIMESTAMP_LTZ" is executed with bound NULL value
        using var cmd = connection.CreateCommand();
        cmd.CommandText = "SELECT ?::TIMESTAMP_LTZ";

        var param = cmd.CreateParameter();
        param.ParameterName = "1";
        param.DbType = DbType.DateTimeOffset;
        param.Value = DBNull.Value;
        cmd.Parameters.Add(param);

        using var reader = await cmd.ExecuteReaderAsync().ConfigureAwait(false);

        // Then Result should contain [NULL]
        Assert.True(await reader.ReadAsync().ConfigureAwait(false), "Expected one row");
        Assert.True(reader.IsDBNull(0), "Expected NULL for column 0");
        Assert.False(await reader.ReadAsync().ConfigureAwait(false), "Expected exactly one row");
    }



    private static async Task SetTimezoneToNonUtcAsync(ISnowflakeDbConnection connection, CancellationToken cancelToken)
    {
        using var cmd1 = connection.CreateCommand();
        cmd1.CommandText = "ALTER SESSION SET TIMEZONE = 'Asia/Kathmandu'";
        await cmd1.ExecuteNonQueryAsync(cancelToken).ConfigureAwait(false);
    }
}
