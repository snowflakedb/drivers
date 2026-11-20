using System.Data.Common;
using System.Globalization;
using System.Reflection;
using System.Text;
using Newtonsoft.Json.Linq;
using Snowflake.Data.Tests.Compatibility;
using Snowflake.Data.Tests.Reference.Fixtures;
using TimeZoneConverter;
using NewSnowflakeDbException = Snowflake.Data.SnowflakeDbException;
using OldSFDataType = Snowflake.Data.Core.SFDataType;
using OldSnowflakeDbDataReader = Snowflake.Data.Client.SnowflakeDbDataReader;
using OldSnowflakeDbException = Snowflake.Data.Client.SnowflakeDbException;
using OldSnowflakeDbParameter = Snowflake.Data.Client.SnowflakeDbParameter;

namespace Snowflake.Data.Tests.Reference;

public sealed class SFDbDataReaderTest : ReferenceTestBase
{
    private static readonly string[] s_resultFormats = { "JSON", "ARROW" };

    public SFDbDataReaderTest(ReferenceITFixture fixture, ITestOutputHelper output)
        : base(fixture, output)
    {
    }

    public static TheoryData<string, string> HighScaleNumberCases =>
        Cross("NUMBER(18,10)", "NUMBER(18,12)", "NUMBER(38,20)", "NUMBER(38,28)");

    public static TheoryData<string, string?> DateCases =>
        Cross<string?>(
            null,
            "9999-12-31 00:00:00.0000000",
            "9999-12-30 00:00:00.0000000",
            "1982-01-18 00:00:00.0000000",
            "1969-07-21 00:00:00.0000000",
            "1900-09-03 00:00:00.0000000");

    public static TheoryData<string, string?, int?> TimeCases =>
        CrossPairs<string?, int?>(
            (null, null),
            (null, 3),
            ("9999-12-31 23:59:59.9999999", null),
            ("9999-12-31 23:59:59.9999999", 5),
            ("1982-01-18 16:20:00.6666666", null),
            ("1982-01-18 16:20:00.6666666", 3),
            ("1969-07-21 02:56:15.1234567", null),
            ("1969-07-21 02:56:15.1234567", 1),
            ("1900-09-03 12:12:12.1212121", null),
            ("1900-09-03 12:12:12.1212121", 1));

    public static TheoryData<string, string> TimeSpanCases =>
        Cross(
            "11:22:33.4455667",
            "23:59:59.9999999",
            "16:20:00.6666666",
            "00:00:00.0000000",
            "00:00:00",
            "23:59:59.1",
            "23:59:59.12",
            "23:59:59.123",
            "23:59:59.1234",
            "23:59:59.12345",
            "23:59:59.123456",
            "23:59:59.1234567",
            "23:59:59.12345678",
            "23:59:59.123456789");

    public static TheoryData<string, string?, int?> TimestampNtzCases =>
        CrossPairs<string?, int?>(
            (null, null),
            (null, 3),
            ("2100-12-31 23:59:59.9999999", null),
            ("2100-12-31 23:59:59.9999999", 5),
            ("9999-12-31 23:59:59.9999999", null),
            ("9999-12-31 23:59:59.9999999", 5),
            ("9999-12-30 23:59:59.9999999", null),
            ("9999-12-30 23:59:59.9999999", 5),
            ("1982-01-18 16:20:00.6666666", null),
            ("1982-01-18 16:20:00.6666666", 3),
            ("1969-07-21 02:56:15.1234567", null),
            ("1969-07-21 02:56:15.0000000", 1),
            ("1900-09-03 12:12:12.1212121", null),
            ("1900-09-03 12:12:12.0000000", 1));

    public static TheoryData<string, int> TimestampTzOffsetCases => Cross(0, 5, -5, 14, -14);

    public static TheoryData<string, bool> BooleanCases => Cross(false, true);

    public static TheoryData<string, string> NumericValueCases =>
        Cross(
            "99",
            "9.9",
            "999",
            "9.99",
            "99999999",
            "999999.99",
            "99999999999",
            "999999999.99",
            "999999999999999999999999999",
            "9999999999999999999999999.99");

    public static TheoryData<string, string, int> TimestampTzCases =>
        CrossPairs(
            ("2019-01-01 12:12:12.1234567 +0500", 7),
            ("2019-01-01 12:12:12.1234567 -0500", 7),
            ("2019-01-01 12:12:12.1234567 +1400", 7),
            ("2019-01-01 12:12:12.1234567 -1400", 7),
            ("0001-01-01 00:00:00.0000000 +0000", 9),
            ("9999-12-31 23:59:59.9999999 +0000", 9));

    public static TheoryData<string, string, int, string> TimestampLtzCases
    {
        get
        {
            var cases = new (string TestValue, int Scale, string ExpectedValue)[]
            {
                ("2019-01-01 12:12:12.1234567 +0200", 7, "2019-01-01 02:12:12.1234567 -08:00"),
                ("2019-01-01 12:12:12.1234567 +1400", 7, "2018-12-31 14:12:12.1234567 -08:00"),
                ("1900-01-15 00:00:00.0000000 +0000", 9, "1900-01-14 16:00:00.0000000 -08:00"),
                ("1883-11-19 00:00:00.0000000 +0000", 9, "1883-11-18 16:00:00.0000000 -08:00"),
                ("9999-12-31 23:59:59.9999999 +0000", 9, "9999-12-31 15:59:59.9999999 -08:00"),
                ("2019-01-01 12:12:12.1234567", 7, "2019-01-01 12:12:12.1234567 -08:00"),
            };
            var data = new TheoryData<string, string, int, string>();
            foreach (var resultFormat in s_resultFormats)
            {
                foreach (var (testValue, scale, expectedValue) in cases)
                    data.Add(resultFormat, testValue, scale, expectedValue);
            }

            return data;
        }
    }

    public static TheoryData<string, string, int> TimestampNtzValueCases =>
        CrossPairs(
            ("2019-01-01 12:12:12.1234567", 7),
            ("0001-01-01 00:00:00.0000000", 9),
            ("9999-12-31 23:59:59.9999999", 9));

    public static TheoryData<string, string> SemiStructuredTypeCases => Cross("array", "object", "variant");

    [SnowflakeTheory]
    [InlineData("JSON")]
    [InlineData("ARROW")]
    public async Task ShouldReportRecordsAffectedAsync(string resultFormat)
    {
        if (resultFormat == "JSON")
        {
            // TODO(SNOW-4382580): the new driver ignores DOTNET_QUERY_RESULT_FORMAT, so JSON results cannot be selected
            Skip.FutureMilestone();
        }

        var tableName = UniqueName();
        using var conn = await OpenConnectionAsync(resultFormat, TestContext.Current.CancellationToken).ConfigureAwait(false);
        try
        {
            CreateOrReplaceTable(conn, tableName, "cola NUMBER");

            using var cmd = conn.CreateCommand();
            cmd.CommandText = $"insert into {tableName} values (1),(1),(1)";
            var reader = cmd.ExecuteReader();
            Assert.Equal(3, reader.RecordsAffected);

            reader.Close();
            Assert.Equal(3, reader.RecordsAffected);

            cmd.CommandText = $"drop table if exists {tableName}";
            var count = cmd.ExecuteNonQuery();
            Assert.Equal(0, count);

            await conn.CloseAsync(TestContext.Current.CancellationToken).ConfigureAwait(false);
            Assert.Equal(3, reader.RecordsAffected);
        }
        finally
        {
            DropTableIfOpen(conn, tableName);
        }
    }

    [SnowflakeTheory]
    [InlineData("JSON")]
    [InlineData("ARROW")]
    public async Task ShouldReadInsertedIntegersAsync(string resultFormat)
    {
        if (resultFormat == "JSON")
        {
            // TODO(SNOW-4382580): the new driver ignores DOTNET_QUERY_RESULT_FORMAT, so JSON results cannot be selected
            Skip.FutureMilestone();
        }

        // TODO(SNOW-4382583): parameter bindings
        Skip.FutureMilestone();

        var tableName = UniqueName();
        using var conn = await OpenConnectionAsync(resultFormat, TestContext.Current.CancellationToken).ConfigureAwait(false);
        try
        {
            CreateOrReplaceTable(conn, tableName, "cola NUMBER");

            using var cmd = conn.CreateCommand();

            var numInt = 10000;
            var numLong = 1000000L;
            short numShort = 10;

            cmd.CommandText = $"insert into {tableName} values (?),(?),(?)";
            AddParameter(cmd, "1", DbType.Int32, numInt);
            AddParameter(cmd, "2", DbType.Int32, numLong);
            AddParameter(cmd, "3", DbType.Int16, numShort);

            var count = cmd.ExecuteNonQuery();
            Assert.Equal(3, count);

            cmd.CommandText = $"select * from {tableName}";
            using var reader = cmd.ExecuteReader();
            ValidateResultFormat(reader, resultFormat);

            Assert.True(reader.Read());
            Assert.Equal(numInt, reader.GetInt32(0));

            Assert.True(reader.Read());
            Assert.Equal(numLong, reader.GetInt64(0));

            Assert.True(reader.Read());
            Assert.Equal(numShort, reader.GetInt16(0));

            Assert.False(reader.Read());
            reader.Close();
        }
        finally
        {
            DropTableIfOpen(conn, tableName);
        }
    }

    [SnowflakeTheory]
    [MemberData(nameof(HighScaleNumberCases))]
    public async Task ShouldReadHighScaleNumberAsDecimalAsync(string resultFormat, string columnType)
    {
        if (resultFormat == "JSON")
        {
            // TODO(SNOW-4382580): the new driver ignores DOTNET_QUERY_RESULT_FORMAT, so JSON results cannot be selected
            Skip.FutureMilestone();
        }

        // TODO(SNOW-4382582): SnowflakeDbDataReader.GetDataTypeName/GetFieldType for FIXED
        Skip.FutureMilestone();

        var tableName = UniqueName();
        using var conn = await OpenConnectionAsync(resultFormat, TestContext.Current.CancellationToken).ConfigureAwait(false);
        try
        {
            CreateOrReplaceTable(conn, tableName, $"cola {columnType}");

            using var cmd = conn.CreateCommand();
            cmd.CommandText = $"INSERT INTO {tableName} SELECT 1.23";
            cmd.ExecuteNonQuery();

            cmd.CommandText = $"SELECT cola FROM {tableName}";
            using var reader = cmd.ExecuteReader();
            ValidateResultFormat(reader, resultFormat);
            Assert.Equal("FIXED", reader.GetDataTypeName(0));
            Assert.Equal(typeof(decimal), reader.GetFieldType(0));
            Assert.True(reader.Read());
            var rawValue = reader.GetValue(0);
            Assert.IsType<decimal>(rawValue);
            Assert.Equal(1.23m, (decimal)rawValue);
            Assert.Equal(1.23m, reader.GetDecimal(0));
            Assert.False(reader.Read());
        }
        finally
        {
            DropTableIfOpen(conn, tableName);
        }
    }

    [SnowflakeTheory]
    [InlineData("JSON")]
    [InlineData("ARROW")]
    public async Task ShouldReadHighPrecisionDecfloatAsStringAsync(string resultFormat)
    {
        if (resultFormat == "JSON")
        {
            // TODO(SNOW-4382580): the new driver ignores DOTNET_QUERY_RESULT_FORMAT, so JSON results cannot be selected
            Skip.FutureMilestone();
        }

        // TODO(SNOW-4382582): SnowflakeDbDataReader.GetDataTypeName/GetFieldType for DECFLOAT
        Skip.FutureMilestone();

        using var conn = await OpenConnectionAsync(resultFormat, TestContext.Current.CancellationToken).ConfigureAwait(false);

        using var cmd = conn.CreateCommand();
        cmd.CommandText = "SELECT 12345678901234567890.123456789::DECFLOAT";
        using var reader = cmd.ExecuteReader();
        ValidateResultFormat(reader, resultFormat);
        Assert.Equal("DECFLOAT", reader.GetDataTypeName(0));
        Assert.Equal(typeof(string), reader.GetFieldType(0));
        Assert.True(reader.Read());
        var value = reader.GetValue(0);
        Assert.IsType<string>(value);
        Assert.Contains("1234567890123456789", (string)value, StringComparison.Ordinal);
        Assert.Equal((string)value, reader.GetString(0));
        Assert.False(reader.Read());
    }

    [SnowflakeTheory]
    [InlineData("JSON")]
    [InlineData("ARROW")]
    public async Task ShouldReadInsertedFloatingPointValuesAsync(string resultFormat)
    {
        if (resultFormat == "JSON")
        {
            // TODO(SNOW-4382580): the new driver ignores DOTNET_QUERY_RESULT_FORMAT, so JSON results cannot be selected
            Skip.FutureMilestone();
        }

        // TODO(SNOW-4382583): parameter bindings
        Skip.FutureMilestone();

        var tableName = UniqueName();
        using var conn = await OpenConnectionAsync(resultFormat, TestContext.Current.CancellationToken).ConfigureAwait(false);
        try
        {
            CreateOrReplaceTable(conn, tableName, "cola DOUBLE");

            using var cmd = conn.CreateCommand();

            var numFloat = (float)1.23;
            var numDouble = (double)1.2345678;

            cmd.CommandText = $"insert into {tableName} values (?),(?)";
            AddParameter(cmd, "1", DbType.Double, numFloat);
            AddParameter(cmd, "2", DbType.Double, numDouble);

            var count = cmd.ExecuteNonQuery();
            Assert.Equal(2, count);

            cmd.CommandText = $"select * from {tableName}";
            using var reader = cmd.ExecuteReader();
            ValidateResultFormat(reader, resultFormat);

            Assert.True(reader.Read());
            Assert.Equal(numFloat, reader.GetFloat(0));
            Assert.Equal((decimal)numFloat, reader.GetDecimal(0));

            Assert.True(reader.Read());
            Assert.Equal(numDouble, reader.GetDouble(0));

            Assert.False(reader.Read());
            reader.Close();
        }
        finally
        {
            DropTableIfOpen(conn, tableName);
        }
    }

    [SnowflakeTheory]
    [MemberData(nameof(DateCases))]
    public async Task ShouldReadDateValuesAsync(string resultFormat, string? inputTimeStr)
    {
        if (resultFormat == "JSON")
        {
            // TODO(SNOW-4382580): the new driver ignores DOTNET_QUERY_RESULT_FORMAT, so JSON results cannot be selected
            Skip.FutureMilestone();
        }

        // TODO(SNOW-4382583): parameter bindings
        Skip.FutureMilestone();

        await AssertGetDateAndOrTimeAsync(resultFormat, inputTimeStr, null, "DATE", TestContext.Current.CancellationToken).ConfigureAwait(false);
    }

    [SnowflakeFact]
    public async Task ShouldApplyDateOutputFormatAsync()
    {
        // TODO(SNOW-4483010): DATE_OUTPUT_FORMAT applied to DATE values read as string
        Skip.FutureMilestone();

        using var conn = Fixture.Factory.Create(Output);
        await conn.OpenAsync(TestContext.Current.CancellationToken).ConfigureAwait(false);
        conn.PreventPooling();
        using var cmd = conn.CreateCommand();

        try
        {
            cmd.CommandText = "alter session set DATE_OUTPUT_FORMAT='MM/DD/YYYY'";
            cmd.ExecuteNonQuery();

            cmd.CommandText = "select TO_DATE('2013-05-17')";
            using var reader = cmd.ExecuteReader();

            Assert.True(reader.Read());
            Assert.Equal("05/17/2013", reader.GetString(0));

            reader.Close();
        }
        finally
        {
            cmd.CommandText = "alter session set DATE_OUTPUT_FORMAT='YYYY-MM-DD'";
            cmd.ExecuteNonQuery();
        }
    }

    [SnowflakeTheory]
    [MemberData(nameof(TimeCases))]
    public async Task ShouldReadTimeValuesAsync(string resultFormat, string? inputTimeStr, int? precision)
    {
        if (resultFormat == "JSON")
        {
            // TODO(SNOW-4382580): the new driver ignores DOTNET_QUERY_RESULT_FORMAT, so JSON results cannot be selected
            Skip.FutureMilestone();
        }

        // TODO(SNOW-4382583): parameter bindings
        Skip.FutureMilestone();

        await AssertGetDateAndOrTimeAsync(resultFormat, inputTimeStr, precision, "TIME", TestContext.Current.CancellationToken).ConfigureAwait(false);
    }

    [SnowflakeTheory]
    [MemberData(nameof(TimeSpanCases))]
    public async Task ShouldReadTimeAsTimeSpanAsync(string resultFormat, string inputTimeStr)
    {
        if (resultFormat == "JSON")
        {
            // TODO(SNOW-4382580): the new driver ignores DOTNET_QUERY_RESULT_FORMAT, so JSON results cannot be selected
            Skip.FutureMilestone();
        }

        // TODO(SNOW-4382582): SnowflakeDbDataReader.GetTimeSpan
        Skip.FutureMilestone();

        var tableName = UniqueName();
        using var conn = await OpenConnectionAsync(resultFormat, TestContext.Current.CancellationToken).ConfigureAwait(false);
        try
        {
            var fractionalPartIndex = inputTimeStr.IndexOf('.');
            var precision = fractionalPartIndex > 0 ? inputTimeStr.Length - (fractionalPartIndex + 1) : 0;
            CreateOrReplaceTable(conn, tableName, $"cola TIME{(precision > 0 ? string.Empty : $"({precision})")}");
            using var cmd = conn.CreateCommand();

            cmd.CommandText = $"insert into {tableName} values ('{inputTimeStr}')";
            var count = cmd.ExecuteNonQuery();
            Assert.Equal(1, count);

            cmd.CommandText = $"SELECT cola FROM {tableName}";
            using var reader = cmd.ExecuteReader();
            ValidateResultFormat(reader, resultFormat);

            Assert.True(reader.Read());

            var dateTimeTime = reader.GetDateTime(0);
            var timeSpanTime = GetTimeSpan(reader, 0);
            reader.Close();

            // TimeSpan precision only goes up to 7 fractional digits.
            var expected = TimeSpan.ParseExact(
                inputTimeStr.Length < 16 ? inputTimeStr : inputTimeStr.Substring(0, 16),
                "c",
                CultureInfo.InvariantCulture);
            Assert.Equal(expected, timeSpanTime);
            Assert.Equal(dateTimeTime.Hour, timeSpanTime.Hours);
            Assert.Equal(dateTimeTime.Minute, timeSpanTime.Minutes);
            Assert.Equal(dateTimeTime.Second, timeSpanTime.Seconds);
            Assert.Equal(dateTimeTime.Millisecond, timeSpanTime.Milliseconds);
        }
        finally
        {
            DropTableIfOpen(conn, tableName);
        }
    }

    [SnowflakeTheory]
    [InlineData("JSON")]
    [InlineData("ARROW")]
    public async Task ShouldThrowWhenReadingNonTimeColumnAsTimeSpanAsync(string resultFormat)
    {
        if (resultFormat == "JSON")
        {
            // TODO(SNOW-4382580): the new driver ignores DOTNET_QUERY_RESULT_FORMAT, so JSON results cannot be selected
            Skip.FutureMilestone();
        }

        // TODO(SNOW-4382582): SnowflakeDbDataReader.GetTimeSpan
        Skip.FutureMilestone();

        var tableName = UniqueName();
        using var conn = await OpenConnectionAsync(resultFormat, TestContext.Current.CancellationToken).ConfigureAwait(false);
        try
        {
            CreateOrReplaceTable(
                conn,
                tableName,
                "C1 NUMBER",
                "C2 FLOAT",
                "C3 VARCHAR(255)",
                "C4 BINARY(255)",
                "C5 BOOLEAN",
                "C6 DATE",
                "C7 TIMESTAMP_NTZ(9)",
                "C8 TIMESTAMP_LTZ(9)",
                "C9 TIMESTAMP_TZ(9)",
                "C10 VARIANT",
                "C11 OBJECT",
                "C12 ARRAY",
                "C13 VARCHAR(1)",
                "C14 TIME");

            using var cmd = conn.CreateCommand();

            cmd.CommandText = $"insert into {tableName}(C1, C10, C11, C12) select 1, " +
                              "PARSE_JSON('{ \"key1\": \"value1\", \"key2\": \"value2\" }')" +
                              ", PARSE_JSON(' { \"outer_key1\": { \"inner_key1A\": \"1a\", \"inner_key1B\": NULL }, '||' \"outer_key2\": { \"inner_key2\": 2 } '||' } ')," +
                              " ARRAY_CONSTRUCT(1, 2, 3, NULL)";
            var count = cmd.ExecuteNonQuery();
            Assert.Equal(1, count);

            cmd.CommandText = $"update {tableName} set C2 = 2.5, C3 = 'C3Val', C4 = TO_BINARY('C4'), C5 = true, C6 = '2021-01-01', " +
                              "C7 = '2017-01-01 12:00:00', C8 = '2017-01-01 12:00:00 +04:00', C9 = '2014-01-02 16:00:00 +10:00', C14 = '12:00:00' where C1 = 1";
            count = cmd.ExecuteNonQuery();
            Assert.Equal(1, count);

            cmd.CommandText = $"SELECT * FROM {tableName}";
            using var reader = cmd.ExecuteReader();
            ValidateResultFormat(reader, resultFormat);

            Assert.True(reader.Read());

            for (var i = 0; i < 12; i++)
            {
                var ordinal = i;
                AssertSnowflakeDbError(270003, () => GetTimeSpan(reader, ordinal));
            }

            // TimeSpan is a non-nullable type, so a NULL value cannot be converted to it.
            Assert.ThrowsAny<InvalidCastException>(() => GetTimeSpan(reader, 12));

            GetTimeSpan(reader, 13);

            reader.Close();
        }
        finally
        {
            DropTableIfOpen(conn, tableName);
        }
    }

    [SnowflakeTheory]
    [MemberData(nameof(TimestampNtzCases))]
    public async Task ShouldReadInsertedTimestampNtzValuesAsync(string resultFormat, string? inputTimeStr, int? precision)
    {
        if (resultFormat == "JSON")
        {
            // TODO(SNOW-4382580): the new driver ignores DOTNET_QUERY_RESULT_FORMAT, so JSON results cannot be selected
            Skip.FutureMilestone();
        }

        // TODO(SNOW-4382583): parameter bindings
        Skip.FutureMilestone();

        await AssertGetDateAndOrTimeAsync(resultFormat, inputTimeStr, precision, "TIMESTAMP_NTZ", TestContext.Current.CancellationToken).ConfigureAwait(false);
    }

    [SnowflakeTheory]
    [MemberData(nameof(TimestampTzOffsetCases))]
    public async Task ShouldPreserveTimestampTzOffsetAsync(string resultFormat, int timezoneOffsetInHours)
    {
        if (resultFormat == "JSON")
        {
            // TODO(SNOW-4382580): the new driver ignores DOTNET_QUERY_RESULT_FORMAT, so JSON results cannot be selected
            Skip.FutureMilestone();
        }

        // TODO(SNOW-4382583): parameter bindings
        Skip.FutureMilestone();

        var tableName = UniqueName();
        using var conn = await OpenConnectionAsync(resultFormat, TestContext.Current.CancellationToken).ConfigureAwait(false);
        try
        {
            CreateOrReplaceTable(conn, tableName, "cola TIMESTAMP_TZ");

            var now = DateTimeOffset.Now.ToOffset(TimeSpan.FromHours(timezoneOffsetInHours));

            using var cmd = conn.CreateCommand();
            cmd.CommandText = $"insert into {tableName} values (?)";
            AddParameter(cmd, "1", DbType.DateTimeOffset, now);

            var count = cmd.ExecuteNonQuery();
            Assert.Equal(1, count);

            cmd.CommandText = $"select * from {tableName}";
            using var reader = cmd.ExecuteReader();
            ValidateResultFormat(reader, resultFormat);

            Assert.True(reader.Read());
            var dtOffset = (DateTimeOffset)reader.GetValue(0);
            reader.Close();

            Assert.Equal(now, dtOffset);
            Assert.Equal(now.Offset, dtOffset.Offset);
        }
        finally
        {
            DropTableIfOpen(conn, tableName);
        }
    }

    [SnowflakeTheory]
    [InlineData("JSON")]
    [InlineData("ARROW")]
    public async Task ShouldPreserveTimestampLtzInstantAsync(string resultFormat)
    {
        if (resultFormat == "JSON")
        {
            // TODO(SNOW-4382580): the new driver ignores DOTNET_QUERY_RESULT_FORMAT, so JSON results cannot be selected
            Skip.FutureMilestone();
        }

        // TODO(SNOW-4382583): parameter bindings
        Skip.FutureMilestone();

        var tableName = UniqueName();
        using var conn = await OpenConnectionWithHonorSessionTimezoneAsync(resultFormat, TestContext.Current.CancellationToken).ConfigureAwait(false);
        try
        {
            ExecuteNonQuery(conn, "ALTER SESSION SET TIMEZONE = 'America/Los_Angeles'");

            CreateOrReplaceTable(conn, tableName, "cola TIMESTAMP_LTZ");

            var insertValue = new DateTimeOffset(2024, 1, 15, 18, 30, 45, 123, TimeSpan.Zero);

            using var cmd = conn.CreateCommand();
            cmd.CommandText = $"insert into {tableName} values (?)";

            var p1 = AddParameter(cmd, "1", DbType.DateTimeOffset, insertValue);
            MarkOldParameterAsTimestampLtz(p1);

            var count = cmd.ExecuteNonQuery();
            Assert.Equal(1, count);

            cmd.CommandText = $"select * from {tableName}";
            using var reader = cmd.ExecuteReader();
            ValidateResultFormat(reader, resultFormat);

            Assert.True(reader.Read());
            var dtOffset = (DateTimeOffset)reader.GetValue(0);
            reader.Close();

            Assert.Equal(insertValue.UtcDateTime, dtOffset.UtcDateTime);
        }
        finally
        {
            DropTableIfOpen(conn, tableName);
        }
    }

    [SnowflakeTheory]
    [MemberData(nameof(BooleanCases))]
    public async Task ShouldReadBooleanValuesAsync(string resultFormat, bool value)
    {
        if (resultFormat == "JSON")
        {
            // TODO(SNOW-4382580): the new driver ignores DOTNET_QUERY_RESULT_FORMAT, so JSON results cannot be selected
            Skip.FutureMilestone();
        }

        // TODO(SNOW-4382583): parameter bindings
        Skip.FutureMilestone();

        var tableName = UniqueName();
        using var conn = await OpenConnectionAsync(resultFormat, TestContext.Current.CancellationToken).ConfigureAwait(false);
        try
        {
            CreateOrReplaceTable(conn, tableName, "cola BOOLEAN");

            using var cmd = conn.CreateCommand();
            cmd.CommandText = $"insert into {tableName} values (?)";
            AddParameter(cmd, "1", DbType.Boolean, value);

            var count = cmd.ExecuteNonQuery();
            Assert.Equal(1, count);

            cmd.CommandText = $"select * from {tableName}";
            using var reader = cmd.ExecuteReader();
            ValidateResultFormat(reader, resultFormat);

            Assert.True(reader.Read());
            Assert.Equal(value, reader.GetBoolean(0));
            reader.Close();
        }
        finally
        {
            DropTableIfOpen(conn, tableName);
        }
    }

    [SnowflakeFact]
    public async Task ShouldReadByteValuesAsync()
    {
        // TODO(SNOW-4382583): parameter bindings
        Skip.FutureMilestone();

        var tableName = UniqueName();
        using var conn = Fixture.Factory.Create(Output);
        await conn.OpenAsync(TestContext.Current.CancellationToken).ConfigureAwait(false);
        try
        {
            CreateOrReplaceTable(conn, tableName, "col1 NUMBER(3)");

            short[] testBytes = { 0, 10, 150, 200, 255 };

            using var cmd = conn.CreateCommand();
            AddParameter(cmd, "1", DbType.Byte, testBytes);
            cmd.CommandText = $"insert into {tableName} values (?)";
            cmd.ExecuteNonQuery();
            cmd.CommandText = $"select * from {tableName} order by 1";

            using var reader = cmd.ExecuteReader();
            var index = 0;
            while (reader.Read())
                Assert.Equal(testBytes[index++], reader.GetByte(0));
        }
        finally
        {
            DropTableIfOpen(conn, tableName);
        }
    }

    [SnowflakeTheory]
    [InlineData("JSON")]
    [InlineData("ARROW")]
    public async Task ShouldReadBytesFromColumnsAsync(string resultFormat)
    {
        if (resultFormat == "JSON")
        {
            // TODO(SNOW-4382580): the new driver ignores DOTNET_QUERY_RESULT_FORMAT, so JSON results cannot be selected
            Skip.FutureMilestone();
        }

        // TODO(SNOW-4382583): parameter bindings
        Skip.FutureMilestone();

        var tableName = UniqueName();
        using var conn = await OpenConnectionAsync(resultFormat, TestContext.Current.CancellationToken).ConfigureAwait(false);
        try
        {
            CreateOrReplaceTable(conn, tableName, "col1 BINARY", "col2 VARCHAR(50)", "col3 DOUBLE");

            var testBytes = Encoding.UTF8.GetBytes("TEST_GET_BINARAY");
            var testChars = "TEST_GET_CHARS";
            var testDouble = 1.2345678;
            using var cmd = conn.CreateCommand();
            cmd.CommandText =
                $"insert into {tableName} values (?, '{testChars}',{testDouble.ToString(CultureInfo.InvariantCulture)})";
            AddParameter(cmd, "1", DbType.Binary, testBytes);

            var count = cmd.ExecuteNonQuery();
            Assert.Equal(1, count);

            cmd.CommandText = $"select * from {tableName}";
            using var reader = cmd.ExecuteReader();
            ValidateResultFormat(reader, resultFormat);

            Assert.True(reader.Read());
            Assert.Equal(testBytes, (byte[])reader.GetValue(0));
            Assert.Equal(testChars, reader.GetValue(1));
            Assert.Equal(testDouble, reader.GetValue(2));

            var toReadLength = testBytes.Length;
            var sub = new byte[toReadLength];
            var read = reader.GetBytes(0, 0, sub, 0, toReadLength);
            Assert.Equal(toReadLength, read);
            Assert.Equal(testBytes, sub);

            toReadLength = 11;
            var testSubBytes = Encoding.UTF8.GetBytes("GET_BINARAY");
            sub = new byte[toReadLength];
            read = reader.GetBytes(0, 5, sub, 0, toReadLength);
            Assert.Equal(toReadLength, read);
            Assert.Equal(testSubBytes, sub);

            toReadLength = 9;
            testSubBytes = Encoding.UTF8.GetBytes("GET_CHARS");
            sub = new byte[toReadLength];
            read = reader.GetBytes(1, 5, sub, 0, toReadLength);
            Assert.Equal(toReadLength, read);
            Assert.Equal(testSubBytes, sub);

            toReadLength = 4;
            testSubBytes = Encoding.UTF8.GetBytes("5678");
            sub = new byte[toReadLength];
            read = reader.GetBytes(2, 5, sub, 0, toReadLength);
            Assert.Equal(toReadLength, read);
            Assert.Equal(testSubBytes, sub);

            toReadLength = 11;
            var testSubBytesWithTargetOffset = Encoding.UTF8.GetBytes("OFFSET GET_BINARAY EXTRA");
            sub = Encoding.UTF8.GetBytes("OFFSET XXXXXXXXXXX EXTRA");
            read = reader.GetBytes(0, 5, sub, 7, toReadLength);
            Assert.Equal(toReadLength, read);
            Assert.Equal(testSubBytesWithTargetOffset, sub);

            var dataOffset = 10;
            read = reader.GetBytes(0, dataOffset, sub, 0, toReadLength);
            Assert.Equal(testBytes.Length - dataOffset, read);

            Assert.Equal(
                "dataOffset",
                Assert.ThrowsAny<ArgumentException>(() => reader.GetBytes(0, 25, sub, 7, toReadLength)).ParamName);
            Assert.Equal(
                "dataOffset",
                Assert.ThrowsAny<ArgumentException>(() => reader.GetBytes(0, -1, sub, 7, toReadLength)).ParamName);
            Assert.Equal(
                "buffer",
                Assert.ThrowsAny<ArgumentException>(() => reader.GetBytes(0, 6, sub, 25, toReadLength)).ParamName);
            Assert.Equal(
                "bufferOffset",
                Assert.ThrowsAny<ArgumentException>(() => reader.GetBytes(0, 6, sub, -1, toReadLength)).ParamName);

            // A null buffer makes GetBytes return the size required to fit all of the data.
            read = reader.GetBytes(0, 6, null, 0, toReadLength);
            Assert.Equal(testBytes.Length, read);

            reader.Close();
        }
        finally
        {
            DropTableIfOpen(conn, tableName);
        }
    }

    [SnowflakeFact]
    public async Task ShouldReadSingleCharacterAsync()
    {
        // TODO(SNOW-4382582): SnowflakeDbDataReader.GetChar
        Skip.FutureMilestone();

        var tableName = UniqueName();
        using var conn = Fixture.Factory.Create(Output);
        await conn.OpenAsync(TestContext.Current.CancellationToken).ConfigureAwait(false);
        try
        {
            CreateOrReplaceTable(conn, tableName, "col1 VARCHAR(50)");

            var testChar = 'T';

            using var cmd = conn.CreateCommand();
            cmd.CommandText = $"insert into {tableName} values ('{testChar}')";
            cmd.ExecuteNonQuery();
            cmd.CommandText = $"select * from {tableName}";

            using var reader = cmd.ExecuteReader();
            Assert.True(reader.Read());
            Assert.Equal(testChar, reader.GetChar(0));
        }
        finally
        {
            DropTableIfOpen(conn, tableName);
        }
    }

    [SnowflakeTheory]
    [InlineData("JSON")]
    [InlineData("ARROW")]
    public async Task ShouldReadCharactersFromColumnsAsync(string resultFormat)
    {
        if (resultFormat == "JSON")
        {
            // TODO(SNOW-4382580): the new driver ignores DOTNET_QUERY_RESULT_FORMAT, so JSON results cannot be selected
            Skip.FutureMilestone();
        }

        // TODO(SNOW-4382583): parameter bindings
        Skip.FutureMilestone();

        var tableName = UniqueName();
        using var conn = await OpenConnectionAsync(resultFormat, TestContext.Current.CancellationToken).ConfigureAwait(false);
        try
        {
            CreateOrReplaceTable(conn, tableName, "col1 VARCHAR(50)", "col2 BINARY", "col3 DOUBLE");

            var testChars = "TEST_GET_CHARS";
            var testBytes = Encoding.UTF8.GetBytes("TEST_GET_BINARY");
            var testDouble = 1.2345678;
            using var cmd = conn.CreateCommand();
            cmd.CommandText =
                $"insert into {tableName} values ('{testChars}', ?, {testDouble.ToString(CultureInfo.InvariantCulture)})";
            AddParameter(cmd, "1", DbType.Binary, testBytes);

            var count = cmd.ExecuteNonQuery();
            Assert.Equal(1, count);

            cmd.CommandText = $"select * from {tableName}";
            using var reader = cmd.ExecuteReader();
            ValidateResultFormat(reader, resultFormat);

            Assert.True(reader.Read());
            Assert.Equal(testChars, reader.GetValue(0));
            Assert.Equal(testBytes, (byte[])reader.GetValue(1));
            Assert.Equal(testDouble, reader.GetValue(2));

            var toReadLength = 14;
            var testSubChars = testChars.ToCharArray();
            var sub = new char[toReadLength];
            var read = reader.GetChars(0, 0, sub, 0, toReadLength);
            Assert.Equal(toReadLength, read);
            Assert.Equal(testSubChars, sub);

            toReadLength = 9;
            testSubChars = "GET_CHARS".ToCharArray();
            sub = new char[toReadLength];
            read = reader.GetChars(0, 5, sub, 0, toReadLength);
            Assert.Equal(toReadLength, read);
            Assert.Equal(testSubChars, sub);

            toReadLength = 10;
            testSubChars = "GET_BINARY".ToCharArray();
            sub = new char[toReadLength];
            read = reader.GetChars(1, 5, sub, 0, toReadLength);
            Assert.Equal(toReadLength, read);
            Assert.Equal(testSubChars, sub);

            toReadLength = 4;
            testSubChars = "5678".ToCharArray();
            sub = new char[toReadLength];
            read = reader.GetChars(2, 5, sub, 0, toReadLength);
            Assert.Equal(toReadLength, read);
            Assert.Equal(testSubChars, sub);

            var testSubCharsWithTargetOffset = "OFFSET GET_CHARS EXTRA".ToCharArray();
            toReadLength = 9;
            sub = "OFFSET XXXXXXXXX EXTRA".ToCharArray();
            read = reader.GetChars(0, 5, sub, 7, toReadLength);
            Assert.Equal(toReadLength, read);
            Assert.Equal(testSubCharsWithTargetOffset, sub);

            var dataOffset = 10;
            read = reader.GetChars(0, dataOffset, sub, 0, toReadLength);
            Assert.Equal(testChars.Length - dataOffset, read);

            Assert.Equal(
                "dataOffset",
                Assert.ThrowsAny<ArgumentException>(() => reader.GetChars(0, 25, sub, 7, toReadLength)).ParamName);
            Assert.Equal(
                "dataOffset",
                Assert.ThrowsAny<ArgumentException>(() => reader.GetChars(0, -1, sub, 7, toReadLength)).ParamName);
            Assert.Equal(
                "buffer",
                Assert.ThrowsAny<ArgumentException>(() => reader.GetChars(0, 6, sub, 25, toReadLength)).ParamName);
            Assert.Equal(
                "bufferOffset",
                Assert.ThrowsAny<ArgumentException>(() => reader.GetChars(0, 6, sub, -1, toReadLength)).ParamName);

            // A null buffer makes GetChars return the size required to fit all of the data.
            read = reader.GetChars(0, 6, null, 0, toReadLength);
            Assert.Equal(testChars.Length, read);

            reader.Close();
        }
        finally
        {
            DropTableIfOpen(conn, tableName);
        }
    }

    [SnowflakeFact]
    public async Task ShouldReportSnowflakeDataTypeNamesAsync()
    {
        // TODO(SNOW-4382583): parameter bindings
        Skip.FutureMilestone();

        var tableName = UniqueName();
        using var conn = Fixture.Factory.Create(Output);
        await conn.OpenAsync(TestContext.Current.CancellationToken).ConfigureAwait(false);
        try
        {
            CreateOrReplaceTable(conn, tableName, "col1 VARCHAR(50)", "col2 BINARY", "col3 DOUBLE");

            var testChars = "TEST_GET_CHARS";
            var testBytes = Encoding.UTF8.GetBytes("TEST_GET_BINARY");
            var testDouble = 1.2345678;

            using var cmd = conn.CreateCommand();
            AddParameter(cmd, "1", DbType.Binary, testBytes);
            cmd.CommandText =
                $"insert into {tableName} values ('{testChars}', ?, {testDouble.ToString(CultureInfo.InvariantCulture)})";
            cmd.ExecuteNonQuery();
            cmd.CommandText = $"select * from {tableName}";

            using var reader = cmd.ExecuteReader();
            Assert.True(reader.Read());
            Assert.Equal("TEXT", reader.GetDataTypeName(0));
            Assert.Equal("BINARY", reader.GetDataTypeName(1));
            Assert.Equal("REAL", reader.GetDataTypeName(2));
        }
        finally
        {
            DropTableIfOpen(conn, tableName);
        }
    }

    [SnowflakeTheory]
    [InlineData("JSON")]
    [InlineData("ARROW")]
    public async Task ShouldReadColumnsAsStreamsAsync(string resultFormat)
    {
        if (resultFormat == "JSON")
        {
            // TODO(SNOW-4382580): the new driver ignores DOTNET_QUERY_RESULT_FORMAT, so JSON results cannot be selected
            Skip.FutureMilestone();
        }

        // TODO(SNOW-4382583): parameter bindings
        Skip.FutureMilestone();

        var tableName = UniqueName();
        using var conn = await OpenConnectionAsync(resultFormat, TestContext.Current.CancellationToken).ConfigureAwait(false);

        // The old driver converts Arrow REAL values to text with the current culture.
        var originalCulture = CultureInfo.CurrentCulture;
        CultureInfo.CurrentCulture = CultureInfo.InvariantCulture;
        try
        {
            CreateOrReplaceTable(conn, tableName, "col1 VARCHAR(50)", "col2 BINARY", "col3 DOUBLE");

            var testChars = "TEST_GET_CHARS";
            var testBytes = Encoding.UTF8.GetBytes("TEST_GET_BINARY");
            var testDouble = 1.2345678;
            using var cmd = conn.CreateCommand();
            cmd.CommandText =
                $"insert into {tableName} values ('{testChars}', ?, {testDouble.ToString(CultureInfo.InvariantCulture)})";
            AddParameter(cmd, "1", DbType.Binary, testBytes);

            var count = cmd.ExecuteNonQuery();
            Assert.Equal(1, count);

            cmd.CommandText = $"select * from {tableName}";
            using var reader = cmd.ExecuteReader();
            ValidateResultFormat(reader, resultFormat);

            Assert.True(reader.Read());

            Assert.Equal(testChars, reader.GetValue(0));
            Assert.Equal(testBytes, (byte[])reader.GetValue(1));
            Assert.Equal(testDouble, reader.GetValue(2));

            using (var stream = reader.GetStream(0))
            {
                var col1ToBytes = Encoding.UTF8.GetBytes(testChars);
                var buf = new byte[col1ToBytes.Length];
                var readBytes = stream.Read(buf, 0, col1ToBytes.Length);
                Assert.Equal(col1ToBytes.Length, readBytes);
                Assert.Equal(-1, stream.ReadByte());
                Assert.Equal(col1ToBytes, buf);
            }

            using (var stream = reader.GetStream(1))
            {
                var buf = new byte[testBytes.Length];
                var readBytes = stream.Read(buf, 0, testBytes.Length);
                Assert.Equal(testBytes.Length, readBytes);
                Assert.Equal(-1, stream.ReadByte());
                Assert.Equal(testBytes, buf);
            }

            using (var stream = reader.GetStream(2))
            {
                var col3ToBytes = Encoding.UTF8.GetBytes(testDouble.ToString(CultureInfo.InvariantCulture));
                var buf = new byte[col3ToBytes.Length];
                var readBytes = stream.Read(buf, 0, col3ToBytes.Length);
                Assert.Equal(col3ToBytes.Length, readBytes);
                Assert.Equal(-1, stream.ReadByte());
                Assert.Equal(col3ToBytes, buf);
            }

            reader.Close();
        }
        finally
        {
            CultureInfo.CurrentCulture = originalCulture;
            DropTableIfOpen(conn, tableName);
        }
    }

    [SnowflakeTheory]
    [InlineData("JSON")]
    [InlineData("ARROW")]
    public async Task ShouldThrowWhenOrdinalIsOutOfRangeAsync(string resultFormat)
    {
        if (resultFormat == "JSON")
        {
            // TODO(SNOW-4382580): the new driver ignores DOTNET_QUERY_RESULT_FORMAT, so JSON results cannot be selected
            Skip.FutureMilestone();
        }

        // TODO(SNOW-4382591): SnowflakeDbException with vendor code for an out-of-range ordinal
        Skip.FutureMilestone();

        using var conn = await OpenConnectionAsync(resultFormat, TestContext.Current.CancellationToken).ConfigureAwait(false);

        using var cmd = conn.CreateCommand();
        cmd.CommandText = "select 1";
        using var reader = cmd.ExecuteReader();
        ValidateResultFormat(reader, resultFormat);

        Assert.True(reader.Read());

        AssertSnowflakeDbError(270002, () => reader.GetInt16(-1));
        AssertSnowflakeDbError(270002, () => reader.GetInt16(1));

        reader.Close();
    }

    [SnowflakeTheory]
    [InlineData("JSON")]
    [InlineData("ARROW")]
    public async Task ShouldExposeBasicReaderMetadataAsync(string resultFormat)
    {
        if (resultFormat == "JSON")
        {
            // TODO(SNOW-4382580): the new driver ignores DOTNET_QUERY_RESULT_FORMAT, so JSON results cannot be selected
            Skip.FutureMilestone();
        }

        // TODO(SNOW-4382582): SnowflakeDbDataReader RecordsAffected/GetOrdinal/closed-reader errors
        Skip.FutureMilestone();

        using var conn = await OpenConnectionAsync(resultFormat, TestContext.Current.CancellationToken).ConfigureAwait(false);

        using var cmd = conn.CreateCommand();
        cmd.CommandText = "select 1 as colone, 2 as coltwo";
        using var reader = cmd.ExecuteReader();
        ValidateResultFormat(reader, resultFormat);
        Assert.Equal(2, reader.FieldCount);
        Assert.Equal(0, reader.Depth);
        Assert.True(reader.HasRows);
        Assert.False(reader.IsClosed);
        Assert.Equal("COLONE", reader.GetName(0));
        Assert.Equal("COLTWO", reader.GetName(1));

        Assert.Equal(typeof(long), reader.GetFieldType(0));
        Assert.Equal(typeof(long), reader.GetFieldType(1));

        Assert.False(reader.NextResult());
        Assert.Equal(-1, reader.RecordsAffected);

        Assert.Equal(0, reader.GetOrdinal("COLONE"));
        Assert.Equal(0, reader.GetOrdinal("COLONE"));
        Assert.Equal(0, reader.GetOrdinal("COLONE"));
        Assert.Equal(1, reader.GetOrdinal("COLTWO"));
        Assert.Equal(-1, reader.GetOrdinal("COL_NOT_EXISTS"));

        reader.Close();
        Assert.True(reader.IsClosed);

        AssertSnowflakeDbError(270010, () => reader.Read());
        AssertSnowflakeDbError(270010, () => reader.GetInt16(0));
    }

    [SnowflakeTheory]
    [InlineData("JSON")]
    [InlineData("ARROW")]
    public async Task ShouldReadNullValuesAsDbNullAsync(string resultFormat)
    {
        if (resultFormat == "JSON")
        {
            // TODO(SNOW-4382580): the new driver ignores DOTNET_QUERY_RESULT_FORMAT, so JSON results cannot be selected
            Skip.FutureMilestone();
        }

        var tableName = UniqueName();
        using var conn = await OpenConnectionAsync(resultFormat, TestContext.Current.CancellationToken).ConfigureAwait(false);
        try
        {
            CreateOrReplaceTable(conn, tableName, "a INTEGER", "b STRING");

            using var cmd = conn.CreateCommand();
            cmd.CommandText = $"insert into {tableName} values(null, null)";
            cmd.ExecuteNonQuery();

            cmd.CommandText = $"select * from {tableName}";
            using var reader = cmd.ExecuteReader();
            ValidateResultFormat(reader, resultFormat);
            reader.Read();
            var nullVal = reader.GetValue(0);
            Assert.Equal(DBNull.Value, nullVal);
            Assert.True(reader.IsDBNull(0));
            Assert.True(reader.IsDBNull(1));

            reader.Close();
        }
        finally
        {
            DropTableIfOpen(conn, tableName);
        }
    }

    [SnowflakeTheory]
    [InlineData("JSON")]
    [InlineData("ARROW")]
    public async Task ShouldReadGuidFromStringColumnAsync(string resultFormat)
    {
        if (resultFormat == "JSON")
        {
            // TODO(SNOW-4382580): the new driver ignores DOTNET_QUERY_RESULT_FORMAT, so JSON results cannot be selected
            Skip.FutureMilestone();
        }

        // TODO(SNOW-4382583): parameter bindings
        Skip.FutureMilestone();

        var tableName = UniqueName();
        using var conn = await OpenConnectionAsync(resultFormat, TestContext.Current.CancellationToken).ConfigureAwait(false);
        try
        {
            CreateOrReplaceTable(conn, tableName, "cola STRING");

            using var cmd = conn.CreateCommand();
            cmd.CommandText = $"insert into {tableName} values (?)";

            var val = Guid.NewGuid();
            AddParameter(cmd, "1", DbType.Guid, val);

            var count = cmd.ExecuteNonQuery();
            Assert.Equal(1, count);

            cmd.CommandText = $"select * from {tableName}";
            using var reader = cmd.ExecuteReader();
            ValidateResultFormat(reader, resultFormat);

            Assert.True(reader.Read());
            Assert.Equal(val, reader.GetGuid(0));

            Assert.Equal(val.ToString(), reader[0]);
            Assert.Equal(val.ToString(), reader["COLA"]);

            var values = new object[1];
            Assert.Equal(1, reader.GetValues(values));
            Assert.Equal(val.ToString(), values[0]);

            reader.Close();
        }
        finally
        {
            DropTableIfOpen(conn, tableName);
        }
    }

    [SnowflakeTheory]
    [InlineData("JSON")]
    [InlineData("ARROW")]
    public async Task ShouldReturnLoadedRowCountForCopyIntoTableAsync(string resultFormat)
    {
        Skip.OnNewDriver(
            "BD#copy-into-table-rows-affected-counts-status-rows",
            "The new driver returns the number of COPY status rows, not the number of loaded rows");

        var tableName = UniqueName();
        var stageName = UniqueName();
        using var conn = await OpenConnectionAsync(resultFormat, TestContext.Current.CancellationToken).ConfigureAwait(false);
        try
        {
            CreateOrReplaceTable(conn, tableName, "cola STRING");

            using var cmd = conn.CreateCommand();
            cmd.CommandText = $"create or replace stage {stageName}";
            cmd.ExecuteNonQuery();

            cmd.CommandText = $"copy into {tableName} from @{stageName}";
            var updateCount = cmd.ExecuteNonQuery();
            Assert.Equal(0, updateCount);

            cmd.CommandText = $"copy into @%{tableName} from (select 'test_string')";
            cmd.ExecuteNonQuery();

            cmd.CommandText = $"copy into {tableName}";
            updateCount = cmd.ExecuteNonQuery();
            Assert.Equal(1, updateCount);
        }
        finally
        {
            DropStageIfOpen(conn, stageName);
            DropTableIfOpen(conn, tableName);
        }
    }

    [SnowflakeTheory]
    [InlineData("JSON")]
    [InlineData("ARROW")]
    public async Task ShouldReturnStatusRowCountForCopyIntoTableAsync(string resultFormat)
    {
        Skip.OnOldDriver(
            "BD#copy-into-table-rows-affected-counts-status-rows",
            "The old driver returns the number of loaded rows, not the number of COPY status rows");
        if (resultFormat == "JSON")
        {
            // TODO(SNOW-4382580): the new driver ignores DOTNET_QUERY_RESULT_FORMAT, so JSON results cannot be selected
            Skip.FutureMilestone();
        }

        var tableName = UniqueName();
        var stageName = UniqueName();
        using var conn = await OpenConnectionAsync(resultFormat, TestContext.Current.CancellationToken).ConfigureAwait(false);
        try
        {
            CreateOrReplaceTable(conn, tableName, "cola STRING");

            using var cmd = conn.CreateCommand();
            cmd.CommandText = $"create or replace stage {stageName}";
            cmd.ExecuteNonQuery();

            cmd.CommandText = $"copy into {tableName} from @{stageName}";
            var updateCount = cmd.ExecuteNonQuery();
            Assert.Equal(1, updateCount);

            cmd.CommandText = $"copy into @%{tableName} from (select 'test_string')";
            cmd.ExecuteNonQuery();

            cmd.CommandText = $"copy into {tableName}";
            updateCount = cmd.ExecuteNonQuery();
            Assert.Equal(1, updateCount);
        }
        finally
        {
            DropStageIfOpen(conn, stageName);
            DropTableIfOpen(conn, tableName);
        }
    }

    [SnowflakeTheory]
    [InlineData("JSON")]
    [InlineData("ARROW")]
    public async Task ShouldReturnResultSetForCopyCommandsAsync(string resultFormat)
    {
        if (resultFormat == "JSON")
        {
            // TODO(SNOW-4382580): the new driver ignores DOTNET_QUERY_RESULT_FORMAT, so JSON results cannot be selected
            Skip.FutureMilestone();
        }

        var tableName = UniqueName();
        var stageName = UniqueName();
        using var conn = await OpenConnectionAsync(resultFormat, TestContext.Current.CancellationToken).ConfigureAwait(false);
        try
        {
            CreateOrReplaceTable(conn, tableName, "cola STRING");

            using var cmd = conn.CreateCommand();
            cmd.CommandText = $"create or replace stage {stageName}";
            cmd.ExecuteNonQuery();

            cmd.CommandText = $"copy into {tableName} from @{stageName}";
            using (var rdr = cmd.ExecuteReader())
                Assert.True(rdr.Read());

            cmd.CommandText = $"copy into @%{tableName} from (select 'test_string')";
            using (var rdr = cmd.ExecuteReader())
                Assert.True(rdr.Read());

            cmd.CommandText = $"copy into {tableName}";
            using (var rdr = cmd.ExecuteReader())
                Assert.True(rdr.Read());
        }
        finally
        {
            DropStageIfOpen(conn, stageName);
            DropTableIfOpen(conn, tableName);
        }
    }

    [SnowflakeTheory]
    [InlineData("JSON")]
    [InlineData("ARROW")]
    public async Task ShouldReadSemiStructuredValuesAsStringsAsync(string resultFormat)
    {
        if (resultFormat == "JSON")
        {
            // TODO(SNOW-4382580): the new driver ignores DOTNET_QUERY_RESULT_FORMAT, so JSON results cannot be selected
            Skip.FutureMilestone();
        }

        var tableName = UniqueName();
        using var conn = await OpenConnectionAsync(resultFormat, TestContext.Current.CancellationToken).ConfigureAwait(false);
        try
        {
            ExecuteNonQuery(
                conn,
                $"CREATE OR REPLACE TABLE {tableName}(cola VARIANT, colb ARRAY, colc OBJECT) " +
                "as select '[\"1\", \"2\"]', '[\"1\", \"2\"]', '{\"key\": \"value\"}'");

            using var cmd = conn.CreateCommand();
            cmd.CommandText = $"select * from {tableName}";
            using var reader = cmd.ExecuteReader();
            ValidateResultFormat(reader, resultFormat);
            Assert.True(reader.Read());
            Assert.Equal("[\n  \"1\",\n  \"2\"\n]", reader.GetString(0));
            Assert.Equal("[\n  \"1\",\n  \"2\"\n]", reader.GetString(1));
            Assert.Equal("{\n  \"key\": \"value\"\n}", reader.GetString(2));
        }
        finally
        {
            DropTableIfOpen(conn, tableName);
        }
    }

    [SnowflakeTheory]
    [InlineData("JSON")]
    [InlineData("ARROW")]
    public async Task ShouldReadVariantInsertedFromJsonParameterAsync(string resultFormat)
    {
        if (resultFormat == "JSON")
        {
            // TODO(SNOW-4382580): the new driver ignores DOTNET_QUERY_RESULT_FORMAT, so JSON results cannot be selected
            Skip.FutureMilestone();
        }

        // TODO(SNOW-4382583): parameter bindings
        Skip.FutureMilestone();

        var tableName = UniqueName();
        using var conn = await OpenConnectionAsync(resultFormat, TestContext.Current.CancellationToken).ConfigureAwait(false);
        try
        {
            CreateOrReplaceTable(conn, tableName, "cola VARIANT");

            using var cmd = conn.CreateCommand();
            cmd.CommandText = $"insert into {tableName} (cola) select parse_json( (?) )";

            var val = "    {\"FieldB\":21,\"FieldA\":37}   ";
            var expectedVal = "{\n  \"FieldA\": 37,\n  \"FieldB\": 21\n}";

            var p1 = cmd.CreateParameter();
            p1.ParameterName = "1";
            MarkOldParameterAsText(p1);
            p1.Value = val;
            cmd.Parameters.Add(p1);

            var count = cmd.ExecuteNonQuery();
            Assert.Equal(1, count);

            cmd.CommandText = $"select * from {tableName}";
            using var reader = cmd.ExecuteReader();
            ValidateResultFormat(reader, resultFormat);

            Assert.True(reader.Read());
            Assert.Equal(expectedVal, reader.GetString(0));

            reader.Close();
        }
        finally
        {
            DropTableIfOpen(conn, tableName);
        }
    }

    [SnowflakeTheory]
    [InlineData("JSON")]
    [InlineData("ARROW")]
    public async Task ShouldReturnSchemaTableMetadataAsync(string resultFormat)
    {
        if (resultFormat == "JSON")
        {
            // TODO(SNOW-4382580): the new driver ignores DOTNET_QUERY_RESULT_FORMAT, so JSON results cannot be selected
            Skip.FutureMilestone();
        }

        // TODO(SNOW-4478021): SnowflakeDbDataReader.GetSchemaTable
        Skip.FutureMilestone();

        var tableName = UniqueName();
        using var conn = await OpenConnectionAsync(resultFormat, TestContext.Current.CancellationToken).ConfigureAwait(false);
        try
        {
            CreateOrReplaceTable(
                conn,
                tableName,
                "c1 NUMBER(20, 4)",
                "c2 STRING(100)",
                "c3 DOUBLE",
                "c4 TIMESTAMP_NTZ",
                "c5 VARIANT not null",
                "c6 BOOLEAN");

            using var cmd = conn.CreateCommand();
            cmd.CommandText = $"select * from {tableName}";
            using var reader = cmd.ExecuteReader();
            ValidateResultFormat(reader, resultFormat);
            var dataTable = reader.GetSchemaTable()!;
            dataTable.DefaultView.Sort = SchemaTableColumn.ColumnName;
            dataTable = dataTable.DefaultView.ToTable();

            var row = dataTable.Rows[0];
            Assert.Equal("C1", row[SchemaTableColumn.ColumnName]);
            Assert.Equal(0, row[SchemaTableColumn.ColumnOrdinal]);
            Assert.Equal(20, row[SchemaTableColumn.NumericPrecision]);
            Assert.Equal(4, row[SchemaTableColumn.NumericScale]);
            Assert.Equal("FIXED", ProviderTypeName(row[SchemaTableColumn.ProviderType]));
            Assert.True((bool)row[SchemaTableColumn.AllowDBNull]);

            row = dataTable.Rows[1];
            Assert.Equal("C2", row[SchemaTableColumn.ColumnName]);
            Assert.Equal(1, row[SchemaTableColumn.ColumnOrdinal]);
            Assert.Equal(100, row[SchemaTableColumn.ColumnSize]);
            Assert.Equal("TEXT", ProviderTypeName(row[SchemaTableColumn.ProviderType]));
            Assert.True((bool)row[SchemaTableColumn.AllowDBNull]);

            row = dataTable.Rows[2];
            Assert.Equal("C3", row[SchemaTableColumn.ColumnName]);
            Assert.Equal(2, row[SchemaTableColumn.ColumnOrdinal]);
            Assert.Equal("REAL", ProviderTypeName(row[SchemaTableColumn.ProviderType]));
            Assert.True((bool)row[SchemaTableColumn.AllowDBNull]);

            row = dataTable.Rows[3];
            Assert.Equal("C4", row[SchemaTableColumn.ColumnName]);
            Assert.Equal(3, row[SchemaTableColumn.ColumnOrdinal]);
            Assert.Equal(0, row[SchemaTableColumn.NumericPrecision]);
            Assert.Equal(9, row[SchemaTableColumn.NumericScale]);
            Assert.Equal("TIMESTAMP_NTZ", ProviderTypeName(row[SchemaTableColumn.ProviderType]));
            Assert.True((bool)row[SchemaTableColumn.AllowDBNull]);

            row = dataTable.Rows[4];
            Assert.Equal("C5", row[SchemaTableColumn.ColumnName]);
            Assert.Equal(4, row[SchemaTableColumn.ColumnOrdinal]);
            Assert.Equal("VARIANT", ProviderTypeName(row[SchemaTableColumn.ProviderType]));
            Assert.False((bool)row[SchemaTableColumn.AllowDBNull]);

            row = dataTable.Rows[5];
            Assert.Equal("C6", row[SchemaTableColumn.ColumnName]);
            Assert.Equal(5, row[SchemaTableColumn.ColumnOrdinal]);
            Assert.Equal("BOOLEAN", ProviderTypeName(row[SchemaTableColumn.ProviderType]));
            Assert.True((bool)row[SchemaTableColumn.AllowDBNull]);
        }
        finally
        {
            DropTableIfOpen(conn, tableName);
        }
    }

    [SnowflakeTheory]
    [InlineData("JSON")]
    [InlineData("ARROW")]
    public async Task ShouldReportNoRowsForEmptyResultAsync(string resultFormat)
    {
        if (resultFormat == "JSON")
        {
            // TODO(SNOW-4382580): the new driver ignores DOTNET_QUERY_RESULT_FORMAT, so JSON results cannot be selected
            Skip.FutureMilestone();
        }

        using var conn = await OpenConnectionAsync(resultFormat, TestContext.Current.CancellationToken).ConfigureAwait(false);

        using var cmd = conn.CreateCommand();
        cmd.CommandText = "select 1 where 1=2";

        using var reader = cmd.ExecuteReader();
        ValidateResultFormat(reader, resultFormat);

        Assert.False(reader.HasRows);
        reader.Close();
    }

    [SnowflakeTheory]
    [InlineData("JSON")]
    [InlineData("ARROW")]
    public async Task ShouldReportHasRowsForEachStatementResultAsync(string resultFormat)
    {
        if (resultFormat == "JSON")
        {
            // TODO(SNOW-4382580): the new driver ignores DOTNET_QUERY_RESULT_FORMAT, so JSON results cannot be selected
            Skip.FutureMilestone();
        }

        // TODO(SNOW-4382583): parameter bindings
        Skip.FutureMilestone();

        using var conn = await OpenConnectionAsync(resultFormat, TestContext.Current.CancellationToken).ConfigureAwait(false);

        ExecuteNonQuery(conn, "ALTER SESSION SET ENABLE_FIX_1758055_ADD_ARROW_SUPPORT_FOR_MULTI_STMTS = TRUE");

        using var cmd = conn.CreateCommand();
        cmd.CommandText = "select 1;" +
                          "select 1 where 1=2;" +
                          "select 1;" +
                          "select 1 where 1=2;";
        AddParameter(cmd, "MULTI_STATEMENT_COUNT", DbType.Int16, 4);

        using var reader = cmd.ExecuteReader();
        ValidateResultFormat(reader, resultFormat);

        Assert.True(reader.HasRows);
        reader.Read();
        Assert.True(reader.HasRows);
        reader.NextResult();

        Assert.False(reader.HasRows);
        reader.NextResult();

        Assert.True(reader.HasRows);
        reader.Read();
        Assert.True(reader.HasRows);
        reader.NextResult();

        Assert.False(reader.HasRows);
        reader.NextResult();
        Assert.False(reader.HasRows);

        reader.Close();
    }

    [SnowflakeTheory(RetriesCount = RetriesCount.Thrice)]
    [MemberData(nameof(NumericValueCases))]
    public async Task ShouldReadNumericValuesAsync(string resultFormat, string testValue)
    {
        if (resultFormat == "JSON")
        {
            // TODO(SNOW-4382580): the new driver ignores DOTNET_QUERY_RESULT_FORMAT, so JSON results cannot be selected
            Skip.FutureMilestone();
        }

        // TODO(SNOW-4483010): FIXED to double/float/byte conversion
        Skip.FutureMilestone();

        using var conn = await OpenConnectionAsync(resultFormat, TestContext.Current.CancellationToken).ConfigureAwait(false);

        using var cmd = conn.CreateCommand();
        cmd.CommandText = "select " + testValue;
        using var reader = cmd.ExecuteReader();
        ValidateResultFormat(reader, resultFormat);
        while (reader.Read())
        {
            Assert.Equal(Convert.ToDecimal(testValue, CultureInfo.InvariantCulture), reader.GetDecimal(0));
            Assert.Equal(Convert.ToDouble(testValue, CultureInfo.InvariantCulture), reader.GetDouble(0));
            Assert.Equal(Convert.ToSingle(testValue, CultureInfo.InvariantCulture), reader.GetFloat(0));
            if (testValue.IndexOf('.') < 0)
            {
                var value = decimal.Parse(testValue, CultureInfo.InvariantCulture);
                if (value >= long.MinValue && value <= long.MaxValue)
                    Assert.Equal(Convert.ToInt64(testValue, CultureInfo.InvariantCulture), reader.GetInt64(0));
                else
                    Assert.Throws<OverflowException>(() => reader.GetInt64(0));
                if (value >= int.MinValue && value <= int.MaxValue)
                    Assert.Equal(Convert.ToInt32(testValue, CultureInfo.InvariantCulture), reader.GetInt32(0));
                else
                    Assert.Throws<OverflowException>(() => reader.GetInt32(0));
                if (value >= short.MinValue && value <= short.MaxValue)
                    Assert.Equal(Convert.ToInt16(testValue, CultureInfo.InvariantCulture), reader.GetInt16(0));
                else
                    Assert.Throws<OverflowException>(() => reader.GetInt16(0));
                if (value >= 0 && value <= 255)
                    Assert.Equal(Convert.ToByte(testValue, CultureInfo.InvariantCulture), reader.GetByte(0));
                else
                    Assert.Throws<OverflowException>(() => reader.GetByte(0));
            }
        }
    }

    [SnowflakeTheory]
    [MemberData(nameof(TimestampTzCases))]
    public async Task ShouldReadTimestampTzAsync(string resultFormat, string testValue, int scale)
    {
        if (resultFormat == "JSON")
        {
            // TODO(SNOW-4382580): the new driver ignores DOTNET_QUERY_RESULT_FORMAT, so JSON results cannot be selected
            Skip.FutureMilestone();
        }

        // TODO(SNOW-4483010): TIMESTAMP_TZ conversion
        Skip.FutureMilestone();

        using var conn = await OpenConnectionAsync(resultFormat, TestContext.Current.CancellationToken).ConfigureAwait(false);

        using var cmd = conn.CreateCommand();
        cmd.CommandText = $"select '{testValue}'::TIMESTAMP_TZ({scale})";
        using var reader = cmd.ExecuteReader();
        ValidateResultFormat(reader, resultFormat);
        reader.Read();

        var expectedValue = DateTimeOffset.Parse(testValue, CultureInfo.InvariantCulture);

        Assert.Equal<object>(expectedValue, reader.GetValue(0));
    }

    [SnowflakeTheory(RetriesCount = RetriesCount.Once)]
    [MemberData(nameof(TimestampLtzCases))]
    public async Task ShouldReadTimestampLtzAsync(string resultFormat, string testValue, int scale, string expectedValue)
    {
        if (resultFormat == "JSON")
        {
            // TODO(SNOW-4382580): the new driver ignores DOTNET_QUERY_RESULT_FORMAT, so JSON results cannot be selected
            Skip.FutureMilestone();
        }

        // TODO(SNOW-4483010): TIMESTAMP_LTZ conversion
        Skip.FutureMilestone();

        using var conn = await OpenConnectionWithHonorSessionTimezoneAsync(resultFormat, TestContext.Current.CancellationToken).ConfigureAwait(false);

        ExecuteNonQuery(conn, "ALTER SESSION SET TIMEZONE = 'America/Los_Angeles'");

        using var cmd = conn.CreateCommand();
        cmd.CommandText = $"select '{testValue}'::TIMESTAMP_LTZ({scale})";
        using var reader = cmd.ExecuteReader();
        ValidateResultFormat(reader, resultFormat);
        reader.Read();

        var expected = DateTimeOffset.Parse(expectedValue, CultureInfo.InvariantCulture);

        Assert.Equal<object>(expected, reader.GetValue(0));
    }

    [SnowflakeTheory]
    [MemberData(nameof(TimestampNtzValueCases))]
    public async Task ShouldReadTimestampNtzAsync(string resultFormat, string testValue, int scale)
    {
        if (resultFormat == "JSON")
        {
            // TODO(SNOW-4382580): the new driver ignores DOTNET_QUERY_RESULT_FORMAT, so JSON results cannot be selected
            Skip.FutureMilestone();
        }

        // TODO(SNOW-4483010): TIMESTAMP_NTZ conversion
        Skip.FutureMilestone();

        using var conn = await OpenConnectionAsync(resultFormat, TestContext.Current.CancellationToken).ConfigureAwait(false);

        using var cmd = conn.CreateCommand();
        cmd.CommandText = $"select '{testValue}'::TIMESTAMP_NTZ({scale})";
        using var reader = cmd.ExecuteReader();
        ValidateResultFormat(reader, resultFormat);
        reader.Read();

        var expectedValue = DateTime.Parse(testValue, CultureInfo.InvariantCulture);

        Assert.Equal<object>(expectedValue, reader.GetValue(0));
    }

    [SnowflakeTheory(RetriesCount = RetriesCount.Thrice)]
    [MemberData(nameof(SemiStructuredTypeCases))]
    public async Task ShouldLoadDataTableOnSemiStructuredColumnAsync(string resultFormat, string type)
    {
        if (resultFormat == "JSON")
        {
            // TODO(SNOW-4382580): the new driver ignores DOTNET_QUERY_RESULT_FORMAT, so JSON results cannot be selected
            Skip.FutureMilestone();
        }

        // TODO(SNOW-4478021): SnowflakeDbDataReader.GetSchemaTable
        Skip.FutureMilestone();

        var tableName = UniqueName();
        using var conn = await OpenConnectionAsync(resultFormat, TestContext.Current.CancellationToken).ConfigureAwait(false);
        try
        {
            var colName = "c1";
            var expectedVal = "id:1";
            CreateOrReplaceTable(conn, tableName, $"{colName} {type}");

            using var cmd = conn.CreateCommand();
            cmd.CommandText = $"insert into {tableName} select parse_json('{{{expectedVal}}}')";

            var count = cmd.ExecuteNonQuery();
            Assert.Equal(1, count);

            cmd.CommandText = $"select {colName} from {tableName}";
            using var reader = cmd.ExecuteReader();
            ValidateResultFormat(reader, resultFormat);
            using var dt = new DataTable();
            dt.Locale = CultureInfo.InvariantCulture;
            dt.Load(reader);
            Assert.Equal(expectedVal, GetFirstRowValue(dt, colName));
        }
        finally
        {
            DropTableIfOpen(conn, tableName);
        }
    }

    [SnowflakeTheory]
    [InlineData("JSON")]
    [InlineData("ARROW")]
    public async Task ShouldConvertTimestampLtzToSessionTimezoneAsync(string resultFormat)
    {
        if (resultFormat == "JSON")
        {
            // TODO(SNOW-4382580): the new driver ignores DOTNET_QUERY_RESULT_FORMAT, so JSON results cannot be selected
            Skip.FutureMilestone();
        }

        // TODO(SNOW-4483010): TIMESTAMP_LTZ conversion honoring HonorSessionTimezone
        Skip.FutureMilestone();

        var tableName = UniqueName();
        using var conn = await OpenConnectionWithHonorSessionTimezoneAsync(resultFormat, TestContext.Current.CancellationToken).ConfigureAwait(false);
        try
        {
            CreateOrReplaceTable(conn, tableName, "val TIMESTAMP_LTZ");

            using var cmd = conn.CreateCommand();
            cmd.CommandText = "ALTER SESSION SET TIMEZONE = 'Europe/Warsaw'";
            cmd.ExecuteNonQuery();

            cmd.CommandText = $"INSERT INTO {tableName} VALUES('2023-08-09 10:00:00')";
            cmd.ExecuteNonQuery();

            var warsawTz = TZConvert.GetTimeZoneInfo("Europe/Warsaw");

            cmd.CommandText = $"SELECT * FROM {tableName}";
            using (var reader = cmd.ExecuteReader())
            {
                Assert.True(reader.Read(), "Should read a record");
                var timestamp1 = reader.GetDateTime(0);

                var expectedTime1 = new DateTime(2023, 8, 9, 10, 0, 0, DateTimeKind.Unspecified);
                var expectedUtc1 = TimeZoneInfo.ConvertTimeToUtc(expectedTime1, warsawTz);
                var expectedInWarsaw = TimeZoneInfo.ConvertTimeFromUtc(expectedUtc1, warsawTz);

                Assert.Equal(expectedInWarsaw, timestamp1);
            }

            cmd.CommandText = "ALTER SESSION SET TIMEZONE = 'Pacific/Honolulu'";
            cmd.ExecuteNonQuery();

            cmd.CommandText = $"SELECT * FROM {tableName}";
            using (var reader = cmd.ExecuteReader())
            {
                Assert.True(reader.Read());
                var timestamp2 = reader.GetDateTime(0);

                var honoluluTz = TZConvert.GetTimeZoneInfo("Pacific/Honolulu");

                var originalTimeInWarsaw = new DateTime(2023, 8, 9, 10, 0, 0, DateTimeKind.Unspecified);
                var utcTime = TimeZoneInfo.ConvertTimeToUtc(originalTimeInWarsaw, warsawTz);
                var expectedInHonolulu = TimeZoneInfo.ConvertTimeFromUtc(utcTime, honoluluTz);

                Assert.Equal(expectedInHonolulu, timestamp2);
            }
        }
        finally
        {
            DropTableIfOpen(conn, tableName);
        }
    }

    [SnowflakeTheory]
    [InlineData("JSON")]
    [InlineData("ARROW")]
    public async Task ShouldReadTimestampLtzWithMultipleSessionTimezonesAsync(string resultFormat)
    {
        if (resultFormat == "JSON")
        {
            // TODO(SNOW-4382580): the new driver ignores DOTNET_QUERY_RESULT_FORMAT, so JSON results cannot be selected
            Skip.FutureMilestone();
        }

        // TODO(SNOW-4483010): TIMESTAMP_LTZ conversion honoring HonorSessionTimezone
        Skip.FutureMilestone();

        var tableName = UniqueName();
        using var conn = await OpenConnectionWithHonorSessionTimezoneAsync(resultFormat, TestContext.Current.CancellationToken).ConfigureAwait(false);
        try
        {
            CreateOrReplaceTable(conn, tableName, "val TIMESTAMP_LTZ");

            using var cmd = conn.CreateCommand();
            cmd.CommandText = "ALTER SESSION SET TIMEZONE = 'UTC'";
            cmd.ExecuteNonQuery();

            cmd.CommandText = $"INSERT INTO {tableName} VALUES('2024-01-01 00:00:00')";
            cmd.ExecuteNonQuery();

            var utcBase = new DateTime(2024, 1, 1, 0, 0, 0, DateTimeKind.Utc);

            foreach (var tzName in new[] { "Europe/Warsaw", "Asia/Tokyo", "America/Los_Angeles" })
            {
                cmd.CommandText = $"ALTER SESSION SET TIMEZONE = '{tzName}'";
                cmd.ExecuteNonQuery();

                cmd.CommandText = $"SELECT val FROM {tableName}";
                using var reader = cmd.ExecuteReader();
                Assert.True(reader.Read());
                var timestamp = reader.GetDateTime(0);

                var tz = TZConvert.GetTimeZoneInfo(tzName);
                var expected = TimeZoneInfo.ConvertTimeFromUtc(utcBase, tz);

                Assert.Equal(expected, timestamp);
            }
        }
        finally
        {
            DropTableIfOpen(conn, tableName);
        }
    }

    [SnowflakeTheory]
    [InlineData("JSON")]
    [InlineData("ARROW")]
    public async Task ShouldConvertTimestampLtzToLocalTimezoneByDefaultAsync(string resultFormat)
    {
        if (resultFormat == "JSON")
        {
            // TODO(SNOW-4382580): the new driver ignores DOTNET_QUERY_RESULT_FORMAT, so JSON results cannot be selected
            Skip.FutureMilestone();
        }

        // TODO(SNOW-4483010): TIMESTAMP_LTZ conversion to local time zone
        Skip.FutureMilestone();

        var tableName = UniqueName();
        using var conn = await OpenConnectionAsync(resultFormat, TestContext.Current.CancellationToken).ConfigureAwait(false);
        try
        {
            CreateOrReplaceTable(conn, tableName, "val TIMESTAMP_LTZ");

            using var cmd = conn.CreateCommand();
            cmd.CommandText = "ALTER SESSION SET TIMEZONE = 'UTC'";
            cmd.ExecuteNonQuery();
            cmd.CommandText = $"INSERT INTO {tableName} VALUES('2024-06-15 12:00:00')";
            cmd.ExecuteNonQuery();

            cmd.CommandText = "ALTER SESSION SET TIMEZONE = 'Pacific/Auckland'";
            cmd.ExecuteNonQuery();

            cmd.CommandText = $"SELECT val FROM {tableName}";
            using var reader = cmd.ExecuteReader();
            Assert.True(reader.Read());
            var timestamp = (DateTimeOffset)reader.GetValue(0);

            var utcTime = new DateTime(2024, 6, 15, 12, 0, 0, DateTimeKind.Utc);
            var expectedLocal = TimeZoneInfo.ConvertTimeFromUtc(utcTime, TimeZoneInfo.Local);
            var expectedOffset = TimeZoneInfo.Local.GetUtcOffset(expectedLocal);

            Assert.Equal(expectedLocal, timestamp.DateTime);
            Assert.Equal(expectedOffset, timestamp.Offset);

            var aucklandTz = TZConvert.GetTimeZoneInfo("Pacific/Auckland");
            var aucklandOffset = aucklandTz.GetUtcOffset(utcTime);
            if (aucklandOffset != expectedOffset)
                Assert.NotEqual(aucklandOffset, timestamp.Offset);
        }
        finally
        {
            DropTableIfOpen(conn, tableName);
        }
    }

    private static TheoryData<string, T> Cross<T>(params T[] values)
    {
        var data = new TheoryData<string, T>();
        foreach (var resultFormat in s_resultFormats)
        {
            foreach (var value in values)
                data.Add(resultFormat, value);
        }

        return data;
    }

    private static TheoryData<string, T1, T2> CrossPairs<T1, T2>(params (T1, T2)[] values)
    {
        var data = new TheoryData<string, T1, T2>();
        foreach (var resultFormat in s_resultFormats)
        {
            foreach (var (first, second) in values)
                data.Add(resultFormat, first, second);
        }

        return data;
    }

    private static string UniqueName() => $"DOTNET_TEST_DATA_READER_{Guid.NewGuid():N}";

    private async Task AssertGetDateAndOrTimeAsync(
        string resultFormat,
        string? inputTimeStr,
        int? precision,
        string dataType,
        CancellationToken cancellationToken)
    {
        var tableName = UniqueName();
        var inputTime = inputTimeStr == null
            ? dataType == "DATE" ? DateTime.Today : DateTime.Now
            : DateTime.ParseExact(inputTimeStr, "yyyy-MM-dd HH:mm:ss.fffffff", CultureInfo.InvariantCulture);

        using var conn = await OpenConnectionAsync(resultFormat, cancellationToken).ConfigureAwait(false);
        try
        {
            CreateOrReplaceTable(conn, tableName, $"cola {dataType}{(precision == null ? string.Empty : $"({precision})")}");

            using var cmd = conn.CreateCommand();
            cmd.CommandText = $"insert into {tableName} values (?)";
            var dbType = dataType switch
            {
                "TIME" => DbType.Time,
                "DATE" => DbType.Date,
                _ => DbType.DateTime,
            };
            AddParameter(cmd, "1", dbType, inputTime);

            var count = cmd.ExecuteNonQuery();
            Assert.Equal(1, count);

            cmd.CommandText = $"select * from {tableName}";
            using var reader = cmd.ExecuteReader();
            ValidateResultFormat(reader, resultFormat);

            Assert.True(reader.Read());

            var actualTime = reader.GetDateTime(0);

            if (dataType == "DATE")
            {
                Assert.Equal(inputTime.Date, actualTime);
                Assert.Equal(inputTime.Date.ToString("yyyy-MM-dd", CultureInfo.InvariantCulture), reader.GetString(0));
            }

            if (dataType != "DATE")
            {
                var inputTimeTicksOfTheDay = inputTime.Ticks - inputTime.Date.Ticks;
                var actualTimeTicksOfTheDay = actualTime.Ticks - actualTime.Date.Ticks;
                var allowedPrecisionLossInTicks = precision < 7 ? Math.Pow(10, (double)(7 - precision)) - 1 : 0d;
                Assert.Equal(inputTimeTicksOfTheDay, actualTimeTicksOfTheDay, allowedPrecisionLossInTicks);
            }

            if (dataType == "TIMESTAMP_NTZ")
            {
                if (precision == 9)
                    Assert.Equal(inputTime, actualTime);
                else
                    Assert.Equal(inputTime.Date, actualTime.Date);
            }

            Assert.Equal(DateTimeKind.Unspecified, actualTime.Kind);

            reader.Close();
        }
        finally
        {
            DropTableIfOpen(conn, tableName);
        }
    }

    private Task<ISnowflakeDbConnection> OpenConnectionAsync(string resultFormat, CancellationToken cancellationToken) =>
        OpenConnectionAsync(resultFormat, null, cancellationToken);

    private Task<ISnowflakeDbConnection> OpenConnectionWithHonorSessionTimezoneAsync(
        string resultFormat,
        CancellationToken cancellationToken)
    {
        var baseConnectionString = Fixture.Factory.Create(Output).ConnectionString.TrimEnd(';');
        return OpenConnectionAsync(resultFormat, baseConnectionString + ";HonorSessionTimezone=true", cancellationToken);
    }

    private async Task<ISnowflakeDbConnection> OpenConnectionAsync(
        string resultFormat,
        string? connectionStringOverride,
        CancellationToken cancellationToken)
    {
        var conn = Fixture.Factory.Create(Output, connectionStringOverride);
        await conn.OpenAsync(cancellationToken).ConfigureAwait(false);
        conn.PreventPooling();
        ExecuteNonQuery(conn, $"alter session set DOTNET_QUERY_RESULT_FORMAT = {(resultFormat == "ARROW" ? "ARROW_FORCE" : resultFormat)}");
        return conn;
    }

    private static void ValidateResultFormat(DbDataReader reader, string resultFormat)
    {
        // The new driver's reader exposes no result format; the old driver's property is internal.
        if (reader is not OldSnowflakeDbDataReader)
            return;
        var resultFormatProperty = typeof(OldSnowflakeDbDataReader).GetProperty(
            "ResultFormat",
            BindingFlags.Instance | BindingFlags.NonPublic)!;
        Assert.Equal(resultFormat, resultFormatProperty.GetValue(reader)!.ToString());
    }

    private static string ProviderTypeName(object providerType) =>
        providerType is int code ? ((OldSFDataType)code).ToString() : providerType.ToString()!;

    private static TimeSpan GetTimeSpan(DbDataReader reader, int ordinal) =>
        reader is OldSnowflakeDbDataReader oldReader
            ? oldReader.GetTimeSpan(ordinal)
            : throw new NotImplementedException("SnowflakeDbDataReader.GetTimeSpan");

    private static void AssertSnowflakeDbError(int expectedVendorCode, Action action)
    {
        var exception = Assert.ThrowsAny<Exception>(action);
        Assert.Equal(expectedVendorCode, SnowflakeDbErrorCode(exception));
    }

    private static int SnowflakeDbErrorCode(Exception exception)
    {
        if (TestDriver.Current == TestDriver.Kind.Old)
        {
            Assert.Equal(typeof(OldSnowflakeDbException), exception.GetType());
            return ((OldSnowflakeDbException)exception).ErrorCode;
        }

        return Assert.IsType<NewSnowflakeDbException>(exception).ErrorCode;
    }

    private static void MarkOldParameterAsTimestampLtz(DbParameter parameter)
    {
        if (parameter is OldSnowflakeDbParameter oldParameter)
            oldParameter.SFDataType = OldSFDataType.TIMESTAMP_LTZ;
    }

    private static void MarkOldParameterAsText(DbParameter parameter)
    {
        if (parameter is OldSnowflakeDbParameter oldParameter)
            oldParameter.SFDataType = OldSFDataType.TEXT;
    }

    private static string GetFirstRowValue(DataTable dt, string colName)
    {
        var jsonString = dt.Rows[0][colName].ToString()!;
        var token = JToken.Parse(jsonString);

        if (token.Type == JTokenType.Object)
            return ParseKeyValue(token);

        if (token.Type == JTokenType.Array)
        {
            var element = token.First!;
            if (element.Type == JTokenType.Object)
                return ParseKeyValue(element);

            return string.Join(",", token.ToObject<List<string>>()!);
        }

        return jsonString;
    }

    private static string ParseKeyValue(JToken token)
    {
        var keyValue = token.ToObject<Dictionary<string, string>>()!;
        var key = keyValue.Keys.First();
        return $"{key}:{keyValue[key]}";
    }

    private static DbParameter AddParameter(DbCommand cmd, string name, DbType dbType, object value)
    {
        var parameter = cmd.CreateParameter();
        parameter.ParameterName = name;
        parameter.DbType = dbType;
        parameter.Value = value;
        cmd.Parameters.Add(parameter);
        return parameter;
    }

    private static void CreateOrReplaceTable(ISnowflakeDbConnection conn, string tableName, params string[] columns) =>
        ExecuteNonQuery(conn, $"CREATE OR REPLACE TABLE {tableName}({string.Join(", ", columns)})");

    private static void DropTableIfOpen(ISnowflakeDbConnection conn, string tableName)
    {
        if (conn.State == ConnectionState.Open)
            ExecuteNonQuery(conn, $"DROP TABLE IF EXISTS {tableName}");
    }

    private static void DropStageIfOpen(ISnowflakeDbConnection conn, string stageName)
    {
        if (conn.State == ConnectionState.Open)
            ExecuteNonQuery(conn, $"DROP STAGE IF EXISTS {stageName}");
    }

    private static void ExecuteNonQuery(ISnowflakeDbConnection conn, string commandText)
    {
        using var cmd = conn.CreateCommand();
        cmd.CommandText = commandText;
        cmd.ExecuteNonQuery();
    }
}
