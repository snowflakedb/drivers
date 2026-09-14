using Snowflake.Data.Tests.Compatibility;

namespace Snowflake.Data.Tests;

[Trait("Category", "E2E")]
public class DateTest : IClassFixture<ITFixture>
{
    protected readonly ITestOutputHelper Output;
    protected readonly ITFixture Fixture;

    public DateTest(ITFixture fixture, ITestOutputHelper output)
    {
        Fixture = fixture;
        Output = output;
    }

    // Scenario: should cast date values to appropriate type
    [SnowflakeFact]
    public void ShouldCastDateValuesToAppropriateType()
    {
        // Given Snowflake client is logged in
        using var connection = Fixture.Factory.Create(Output);
        connection.Open();

        // When Query "SELECT '2024-01-15'::DATE, '1970-01-01'::DATE, '1999-12-31'::DATE" is executed
        using var cmd = connection.CreateCommand();
        cmd.CommandText = "SELECT '2024-01-15'::DATE, '1970-01-01'::DATE, '1999-12-31'::DATE";
        using var reader = cmd.ExecuteReader();

        // Then All values should be returned as DATE type
        Assert.True(reader.Read(), "Expected one row");
        Assert.Equal(new DateTime(2024, 1, 15), reader.GetDateTime(0));
        Assert.Equal(new DateTime(1970, 1, 1), reader.GetDateTime(1));
        Assert.Equal(new DateTime(1999, 12, 31), reader.GetDateTime(2));

        // And No precision loss should occur
        Assert.Equal(0, reader.GetDateTime(0).TimeOfDay.Ticks);
        Assert.Equal(0, reader.GetDateTime(1).TimeOfDay.Ticks);
        Assert.Equal(0, reader.GetDateTime(2).TimeOfDay.Ticks);
        Assert.False(reader.Read(), "Expected exactly one row");
    }

    // Scenario: should select date literals
    [SnowflakeFact]
    public void ShouldSelectDateLiterals()
    {
        Skip.FutureMilestone();

        // Given Snowflake client is logged in
        using var connection = Fixture.Factory.Create(Output);
        connection.Open();

        // When Query "SELECT '2024-01-15'::DATE, '1970-01-01'::DATE, '1999-12-31'::DATE" is executed
        using var cmd = connection.CreateCommand();
        cmd.CommandText = "SELECT '2024-01-15'::DATE, '1970-01-01'::DATE, '1999-12-31'::DATE";
        using var reader = cmd.ExecuteReader();

        // Then Result should contain dates [2024-01-15, 1970-01-01, 1999-12-31]
        Assert.True(reader.Read(), "Expected one row");
        Assert.Equal("2024-01-15", reader.GetString(0));
        Assert.Equal("1970-01-01", reader.GetString(1));
        Assert.Equal("1999-12-31", reader.GetString(2));
        Assert.False(reader.Read(), "Expected exactly one row");
    }

    // Scenario: should select epoch and pre-epoch dates
    [SnowflakeFact]
    public void ShouldSelectEpochAndPreEpochDates()
    {
        Skip.FutureMilestone();

        // Given Snowflake client is logged in
        using var connection = Fixture.Factory.Create(Output);
        connection.Open();

        // When Query "SELECT '1970-01-01'::DATE, '1969-12-31'::DATE, '1900-01-01'::DATE" is executed
        using var cmd = connection.CreateCommand();
        cmd.CommandText = "SELECT '1970-01-01'::DATE, '1969-12-31'::DATE, '1900-01-01'::DATE";
        using var reader = cmd.ExecuteReader();

        // Then Result should contain dates [1970-01-01, 1969-12-31, 1900-01-01]
        Assert.True(reader.Read(), "Expected one row");
        Assert.Equal("1970-01-01", reader.GetString(0));
        Assert.Equal("1969-12-31", reader.GetString(1));
        Assert.Equal("1900-01-01", reader.GetString(2));
        Assert.False(reader.Read(), "Expected exactly one row");
    }

    // Scenario: should select historical and boundary dates
    [SnowflakeFact]
    public void ShouldSelectHistoricalAndBoundaryDates()
    {
        Skip.FutureMilestone();

        // Given Snowflake client is logged in
        using var connection = Fixture.Factory.Create(Output);
        connection.Open();

        // When Query "SELECT '0001-01-01'::DATE, '1582-10-15'::DATE, '9999-12-31'::DATE" is executed
        using var cmd = connection.CreateCommand();
        cmd.CommandText = "SELECT '0001-01-01'::DATE, '1582-10-15'::DATE, '9999-12-31'::DATE";
        using var reader = cmd.ExecuteReader();

        // Then Result should contain dates [0001-01-01, 1582-10-15, 9999-12-31]
        Assert.True(reader.Read(), "Expected one row");
        Assert.Equal("0001-01-01", reader.GetString(0));
        Assert.Equal("1582-10-15", reader.GetString(1));
        Assert.Equal("9999-12-31", reader.GetString(2));
        Assert.False(reader.Read(), "Expected exactly one row");
    }

    // Scenario: should handle NULL values for date
    [SnowflakeFact]
    public void ShouldHandleNullValuesForDate()
    {
        Skip.FutureMilestone();

        // Given Snowflake client is logged in
        using var connection = Fixture.Factory.Create(Output);
        connection.Open();

        // When Query "SELECT NULL::DATE, '2024-01-15'::DATE, NULL::DATE" is executed
        using var cmd = connection.CreateCommand();
        cmd.CommandText = "SELECT NULL::DATE, '2024-01-15'::DATE, NULL::DATE";
        using var reader = cmd.ExecuteReader();

        // Then Result should contain [NULL, 2024-01-15, NULL]
        Assert.True(reader.Read(), "Expected one row");
        Assert.True(reader.IsDBNull(0), "Expected NULL for column 0");
        Assert.Equal("2024-01-15", reader.GetString(1));
        Assert.True(reader.IsDBNull(2), "Expected NULL for column 2");
        Assert.False(reader.Read(), "Expected exactly one row");
    }

    // Scenario: should download large result set for date
    [SnowflakeFact]
    public void ShouldDownloadLargeResultSetForDate()
    {
        Skip.FutureMilestone();

        const int rowCount = 100000;

        // Given Snowflake client is logged in
        using var connection = Fixture.Factory.Create(Output);
        connection.Open();

        // When Query "SELECT DATEADD(day, ROW_NUMBER() OVER (ORDER BY seq4()) - 1, '1970-01-01'::DATE) as d FROM TABLE(GENERATOR(ROWCOUNT => 100000)) ORDER BY d" is executed
        using var cmd = connection.CreateCommand();
        cmd.CommandText = "SELECT DATEADD(day, ROW_NUMBER() OVER (ORDER BY seq4()) - 1, '1970-01-01'::DATE) as d FROM TABLE(GENERATOR(ROWCOUNT => 100000)) ORDER BY d";
        using var reader = cmd.ExecuteReader();

        // Then Result should contain 100000 rows with sequential dates starting from 1970-01-01
        var count = 0;
        var baseDate = new DateTime(1970, 1, 1);
        while (reader.Read())
        {
            Assert.Equal(baseDate.AddDays(count).ToString("yyyy-MM-dd"), reader.GetString(0));
            count++;
        }
        Assert.Equal(rowCount, count);
    }

    // Scenario: should select dates from table
    [SnowflakeFact]
    public void ShouldSelectDatesFromTable()
    {
        Skip.FutureMilestone();

        // Given Snowflake client is logged in
        using var connection = Fixture.Factory.Create(Output);
        connection.Open();

        // And Table with DATE column exists with values ['2024-01-15', '1970-01-01', '1999-12-31']
        var tableName = $"UD_DATE_TBL_{Guid.NewGuid():N}".Substring(0, 30);
        using var createCmd = connection.CreateCommand();
        createCmd.CommandText = $"CREATE TEMPORARY TABLE {tableName} (col DATE)";
        createCmd.ExecuteNonQuery();

        using var insertCmd = connection.CreateCommand();
        insertCmd.CommandText = $"INSERT INTO {tableName} (col) VALUES ('2024-01-15'), ('1970-01-01'), ('1999-12-31')";
        insertCmd.ExecuteNonQuery();

        // When Query "SELECT * FROM <table> ORDER BY col" is executed
        using var selectCmd = connection.CreateCommand();
        selectCmd.CommandText = $"SELECT * FROM {tableName} ORDER BY col";
        using var reader = selectCmd.ExecuteReader();

        // Then Result should contain dates [1970-01-01, 1999-12-31, 2024-01-15]
        Assert.True(reader.Read());
        Assert.Equal("1970-01-01", reader.GetString(0));
        Assert.True(reader.Read());
        Assert.Equal("1999-12-31", reader.GetString(0));
        Assert.True(reader.Read());
        Assert.Equal("2024-01-15", reader.GetString(0));
        Assert.False(reader.Read(), "Expected exactly 3 rows");
    }

    // Scenario: should select dates with NULL from table
    [SnowflakeFact]
    public void ShouldSelectDatesWithNullFromTable()
    {
        Skip.FutureMilestone();

        // Given Snowflake client is logged in
        using var connection = Fixture.Factory.Create(Output);
        connection.Open();

        // And Table with DATE column exists with values ['2024-01-15', NULL, '1999-12-31']
        var tableName = $"UD_DATE_NUL_{Guid.NewGuid():N}".Substring(0, 30);
        using var createCmd = connection.CreateCommand();
        createCmd.CommandText = $"CREATE TEMPORARY TABLE {tableName} (col DATE)";
        createCmd.ExecuteNonQuery();

        using var insertCmd = connection.CreateCommand();
        insertCmd.CommandText = $"INSERT INTO {tableName} (col) VALUES ('2024-01-15'), (NULL), ('1999-12-31')";
        insertCmd.ExecuteNonQuery();

        // When Query "SELECT * FROM <table> ORDER BY col" is executed
        using var selectCmd = connection.CreateCommand();
        selectCmd.CommandText = $"SELECT * FROM {tableName} ORDER BY col";
        using var reader = selectCmd.ExecuteReader();

        // Then Result should contain [1999-12-31, 2024-01-15, NULL]
        Assert.True(reader.Read());
        Assert.Equal("1999-12-31", reader.GetString(0));
        Assert.True(reader.Read());
        Assert.Equal("2024-01-15", reader.GetString(0));
        Assert.True(reader.Read());
        Assert.True(reader.IsDBNull(0), "Expected NULL for last row");
        Assert.False(reader.Read(), "Expected exactly 3 rows");
    }

    // Scenario: should select historical and boundary dates from table
    [SnowflakeFact]
    public void ShouldSelectHistoricalAndBoundaryDatesFromTable()
    {
        Skip.FutureMilestone();

        // Given Snowflake client is logged in
        using var connection = Fixture.Factory.Create(Output);
        connection.Open();

        // And Table with DATE column exists with values ['0001-01-01', '0100-03-01', '1582-10-15', '9999-12-31']
        var tableName = $"UD_DATE_HIS_{Guid.NewGuid():N}".Substring(0, 30);
        using var createCmd = connection.CreateCommand();
        createCmd.CommandText = $"CREATE TEMPORARY TABLE {tableName} (col DATE)";
        createCmd.ExecuteNonQuery();

        using var insertCmd = connection.CreateCommand();
        insertCmd.CommandText = $"INSERT INTO {tableName} (col) VALUES ('0001-01-01'), ('0100-03-01'), ('1582-10-15'), ('9999-12-31')";
        insertCmd.ExecuteNonQuery();

        // When Query "SELECT * FROM <table> ORDER BY col" is executed
        using var selectCmd = connection.CreateCommand();
        selectCmd.CommandText = $"SELECT * FROM {tableName} ORDER BY col";
        using var reader = selectCmd.ExecuteReader();

        // Then Result should contain dates [0001-01-01, 0100-03-01, 1582-10-15, 9999-12-31]
        Assert.True(reader.Read());
        Assert.Equal("0001-01-01", reader.GetString(0));
        Assert.True(reader.Read());
        Assert.Equal("0100-03-01", reader.GetString(0));
        Assert.True(reader.Read());
        Assert.Equal("1582-10-15", reader.GetString(0));
        Assert.True(reader.Read());
        Assert.Equal("9999-12-31", reader.GetString(0));
        Assert.False(reader.Read(), "Expected exactly 4 rows");
    }

    // Scenario: should download large result set for date from table
    [SnowflakeFact]
    public void ShouldDownloadLargeResultSetForDateFromTable()
    {
        Skip.FutureMilestone();

        const int rowCount = 100000;

        // Given Snowflake client is logged in
        using var connection = Fixture.Factory.Create(Output);
        connection.Open();

        // And Table with DATE column exists with 100000 sequential dates starting from 1970-01-01
        var tableName = $"UD_DATE_LRG_{Guid.NewGuid():N}".Substring(0, 30);
        using var createCmd = connection.CreateCommand();
        createCmd.CommandText = $"CREATE TEMPORARY TABLE {tableName} (col DATE)";
        createCmd.ExecuteNonQuery();

        using var insertCmd = connection.CreateCommand();
        insertCmd.CommandText = $"INSERT INTO {tableName} SELECT DATEADD(day, ROW_NUMBER() OVER (ORDER BY seq4()) - 1, '1970-01-01'::DATE) FROM TABLE(GENERATOR(ROWCOUNT => {rowCount}))";
        insertCmd.ExecuteNonQuery();

        // When Query "SELECT * FROM <table> ORDER BY col" is executed
        using var selectCmd = connection.CreateCommand();
        selectCmd.CommandText = $"SELECT * FROM {tableName} ORDER BY col";
        using var reader = selectCmd.ExecuteReader();

        // Then Result should contain 100000 rows with sequential dates starting from 1970-01-01
        var count = 0;
        var baseDate = new DateTime(1970, 1, 1);
        while (reader.Read())
        {
            Assert.Equal(baseDate.AddDays(count).ToString("yyyy-MM-dd"), reader.GetString(0));
            count++;
        }
        Assert.Equal(rowCount, count);
    }

    // Scenario: should select date using parameter binding
    [SnowflakeFact]
    public void ShouldSelectDateUsingParameterBinding()
    {
        Skip.FutureMilestone();

        // Given Snowflake client is logged in
        using var connection = Fixture.Factory.Create(Output);
        connection.Open();

        // When Query "SELECT ?::DATE, ?::DATE, ?::DATE" is executed with bound date values [2024-01-15, 1970-01-01, 1999-12-31]
        using var cmd = connection.CreateCommand();
        cmd.CommandText = "SELECT ?::DATE, ?::DATE, ?::DATE";
        var param1 = cmd.CreateParameter();
        param1.ParameterName = "1";
        param1.DbType = DbType.Date;
        param1.Value = new DateTime(2024, 1, 15);
        cmd.Parameters.Add(param1);
        var param2 = cmd.CreateParameter();
        param2.ParameterName = "2";
        param2.DbType = DbType.Date;
        param2.Value = new DateTime(1970, 1, 1);
        cmd.Parameters.Add(param2);
        var param3 = cmd.CreateParameter();
        param3.ParameterName = "3";
        param3.DbType = DbType.Date;
        param3.Value = new DateTime(1999, 12, 31);
        cmd.Parameters.Add(param3);
        using var reader = cmd.ExecuteReader();

        // Then Result should contain [2024-01-15, 1970-01-01, 1999-12-31]
        Assert.True(reader.Read(), "Expected one row");
        Assert.Equal("2024-01-15", reader.GetString(0));
        Assert.Equal("1970-01-01", reader.GetString(1));
        Assert.Equal("1999-12-31", reader.GetString(2));
        Assert.False(reader.Read(), "Expected exactly one row");
    }

    // Scenario: should select null date using parameter binding
    [SnowflakeFact]
    public void ShouldSelectNullDateUsingParameterBinding()
    {
        Skip.FutureMilestone();

        // Given Snowflake client is logged in
        using var connection = Fixture.Factory.Create(Output);
        connection.Open();

        // When Query "SELECT ?::DATE" is executed with bound NULL value
        using var cmd = connection.CreateCommand();
        cmd.CommandText = "SELECT ?::DATE";
        var param = cmd.CreateParameter();
        param.ParameterName = "1";
        param.DbType = DbType.Date;
        param.Value = DBNull.Value;
        cmd.Parameters.Add(param);
        using var reader = cmd.ExecuteReader();

        // Then Result should contain [NULL]
        Assert.True(reader.Read(), "Expected one row");
        Assert.True(reader.IsDBNull(0), "Expected NULL for column 0");
        Assert.False(reader.Read(), "Expected exactly one row");
    }

    // Scenario: should insert date using parameter binding
    [SnowflakeFact]
    public void ShouldInsertDateUsingParameterBinding()
    {
        Skip.FutureMilestone();

        // Given Snowflake client is logged in
        using var connection = Fixture.Factory.Create(Output);
        connection.Open();

        // And Table with DATE column exists
        var tableName = $"UD_DATE_BND_{Guid.NewGuid():N}".Substring(0, 30);
        using var createCmd = connection.CreateCommand();
        createCmd.CommandText = $"CREATE TEMPORARY TABLE {tableName} (col DATE)";
        createCmd.ExecuteNonQuery();

        // When Date values [2024-01-15, 1970-01-01, 1999-12-31] are inserted using parameter binding
        DateTime[] testValues = [new DateTime(2024, 1, 15), new DateTime(1970, 1, 1), new DateTime(1999, 12, 31)];
        foreach (var value in testValues)
        {
            using var insertCmd = connection.CreateCommand();
            insertCmd.CommandText = $"INSERT INTO {tableName} (col) VALUES (?)";
            var param = insertCmd.CreateParameter();
            param.ParameterName = "1";
            param.DbType = DbType.Date;
            param.Value = value;
            insertCmd.Parameters.Add(param);
            insertCmd.ExecuteNonQuery();
        }

        // And Query "SELECT * FROM <table> ORDER BY col" is executed
        using var selectCmd = connection.CreateCommand();
        selectCmd.CommandText = $"SELECT col FROM {tableName} ORDER BY col";
        using var reader = selectCmd.ExecuteReader();

        // Then Result should contain dates [1970-01-01, 1999-12-31, 2024-01-15]
        Assert.True(reader.Read());
        Assert.Equal("1970-01-01", reader.GetString(0));
        Assert.True(reader.Read());
        Assert.Equal("1999-12-31", reader.GetString(0));
        Assert.True(reader.Read());
        Assert.Equal("2024-01-15", reader.GetString(0));
        Assert.False(reader.Read(), "Expected exactly 3 rows");
    }
}
