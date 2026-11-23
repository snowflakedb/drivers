extern alias OldDriver;
using System.Data.Common;
using System.Globalization;
using System.Text;
using Snowflake.Data.Interfaces;
using Snowflake.Data.Tests.Compatibility;
using Snowflake.Data.Tests.E2E.Fixtures;
using OldSnowflakeDbDataReader = OldDriver::Snowflake.Data.Client.SnowflakeDbDataReader;

namespace Snowflake.Data.Tests.E2E;

public sealed class StructuredTypesTest : LiveE2ETestBase
{
    private const string AllTypesObjectDefinition = @"::OBJECT(
                        StringValue VARCHAR,
                        CharValue CHAR,
                        ByteValue SMALLINT,
                        SByteValue SMALLINT,
                        ShortValue SMALLINT,
                        UShortValue INTEGER,
                        IntValue INTEGER,
                        UIntValue INTEGER,
                        LongValue BIGINT,
                        ULongValue BIGINT,
                        FloatValue FLOAT,
                        DoubleValue DOUBLE,
                        DecimalValue REAL,
                        BooleanValue BOOLEAN,
                        GuidValue TEXT,
                        DateTimeValue TIMESTAMP_NTZ,
                        DateTimeOffsetValue TIMESTAMP_LTZ,
                        TimeSpanValue TIME,
                        BinaryValue BINARY,
                        SemiStructuredValue OBJECT
                    )";

    public StructuredTypesTest(E2EITFixture fixture, ITestOutputHelper output)
        : base(fixture, output)
    {
    }

    public static TheoryData<string, string, DateTime?, DateTime> DateTimeConversionCases => new()
    {
        { "2024-07-11 14:20:05", "TIMESTAMP_NTZ", ParseDateTime("2024-07-11 14:20:05"), ParseDateTime("2024-07-11 14:20:05") },
        { "2024-07-11 14:20:05 +5:00", "TIMESTAMP_TZ", null, ParseDateTime("2024-07-11 09:20:05", DateTimeKind.Utc) },
        { "2024-07-11 14:20:05", "TIMESTAMP_LTZ", null, ParseDateTime("2024-07-11 14:20:05", DateTimeKind.Local) },
        { "2024-07-11", "DATE", ParseDateTime("2024-07-11"), ParseDateTime("2024-07-11") },
        { "2024-07-11 14:20:05.123456789", "TIMESTAMP_NTZ", ParseDateTime("2024-07-11 14:20:05.1234567"), ParseDateTime("2024-07-11 14:20:05.1234568") },
        { "2024-07-11 14:20:05.123456789 +5:00", "TIMESTAMP_TZ", null, ParseDateTime("2024-07-11 09:20:05.1234568", DateTimeKind.Utc) },
        { "2024-07-11 14:20:05.123456789 -2:00", "TIMESTAMP_LTZ", null, ParseDateTime("2024-07-11 09:20:05.1234568", DateTimeKind.Local) },
        { "9999-12-31 23:59:59.999999", "TIMESTAMP_NTZ", ParseDateTime("9999-12-31 23:59:59.999999"), ParseDateTime("9999-12-31 23:59:59.999999") },
        { "9999-12-31 23:59:59.999999 +1:00", "TIMESTAMP_TZ", null, ParseDateTime("9999-12-31 22:59:59.999999", DateTimeKind.Utc) },
        { "1883-11-19 00:00:00.000000 -5:00", "TIMESTAMP_LTZ", null, ParseDateTime("1883-11-18 21:00:00.000000", DateTimeKind.Local) },
        { "1900-01-15 00:00:00.000000 +0:00", "TIMESTAMP_LTZ", null, ParseDateTime("1900-01-14 16:00:00.000000", DateTimeKind.Local) },
        { "0001-01-01 00:00:00", "TIMESTAMP_NTZ", ParseDateTime("0001-01-01 00:00:00"), ParseDateTime("0001-01-01 00:00:00") },
        { "0001-01-01 00:00:00 -1:00", "TIMESTAMP_TZ", null, ParseDateTime("0001-01-01 01:00:00", DateTimeKind.Utc) },
    };

    public static TheoryData<string, string, DateTime?, DateTimeOffset> DateTimeOffsetConversionCases => new()
    {
        { "2024-07-11 14:20:05", "TIMESTAMP_NTZ", ParseDateTime("2024-07-11 14:20:05"), ParseDateTimeOffset("2024-07-11 14:20:05Z") },
        { "2024-07-11 14:20:05 +5:00", "TIMESTAMP_TZ", null, ParseDateTimeOffset("2024-07-11 14:20:05 +5:00") },
        { "2024-07-11 14:20:05 -7:00", "TIMESTAMP_LTZ", null, ParseDateTimeOffset("2024-07-11 14:20:05 -7:00") },
        { "2024-07-11", "DATE", ParseDateTime("2024-07-11"), ParseDateTimeOffset("2024-07-11Z") },
        { "2024-07-11 14:20:05.123456789", "TIMESTAMP_NTZ", ParseDateTime("2024-07-11 14:20:05.1234567"), ParseDateTimeOffset("2024-07-11 14:20:05.1234568Z") },
        { "2024-07-11 14:20:05.123456789 +5:00", "TIMESTAMP_TZ", null, ParseDateTimeOffset("2024-07-11 14:20:05.1234568 +5:00") },
        { "2024-07-11 14:20:05.123456789 -2:00", "TIMESTAMP_LTZ", null, ParseDateTimeOffset("2024-07-11 09:20:05.1234568 -7:00") },
        { "9999-12-31 23:59:59.999999", "TIMESTAMP_NTZ", ParseDateTime("9999-12-31 23:59:59.999999"), ParseDateTimeOffset("9999-12-31 23:59:59.999999Z") },
        { "9999-12-31 23:59:59.999999 +1:00", "TIMESTAMP_TZ", null, ParseDateTimeOffset("9999-12-31 23:59:59.999999 +1:00") },
        { "9999-12-31 23:59:59.999999 +13:00", "TIMESTAMP_LTZ", null, ParseDateTimeOffset("9999-12-31 02:59:59.999999 -08:00") },
        { "1900-01-15 00:00:00.000000 +0:00", "TIMESTAMP_LTZ", null, ParseDateTimeOffset("1900-01-14 16:00:00.000000 -08:00") },
        { "0001-01-01 00:00:00", "TIMESTAMP_NTZ", ParseDateTime("0001-01-01 00:00:00"), ParseDateTimeOffset("0001-01-01 00:00:00Z") },
        { "0001-01-01 00:00:00 -1:00", "TIMESTAMP_TZ", null, ParseDateTimeOffset("0001-01-01 00:00:00 -1:00") },
    };

    [SnowflakeFact]
    public async Task ShouldSelectAllUnstructuredTypesObjectAsync()
    {
        // TODO(SNOW-4382580): the new driver ignores DOTNET_QUERY_RESULT_FORMAT, so JSON results cannot be selected
        Skip.FutureMilestone();

        // TODO(SNOW-4382584): SnowflakeDbDataReader.GetObject<T> for structured types
        Skip.FutureMilestone();

        var ct = TestContext.Current.CancellationToken;
        using var connection = Fixture.Factory.Create(Output);
        await connection.OpenAsync(ct).ConfigureAwait(false);
        await EnableStructuredTypesAsync(connection, ct).ConfigureAwait(false);
        var timeZone = await GetTimeZoneAsync(connection, ct).ConfigureAwait(false);
        var expectedOffset = timeZone.GetUtcOffset(ParseDateTime("2024-07-11 14:20:05"));
        var expectedOffsetString = ToOffsetString(expectedOffset);
        var allTypesObjectAsSFString = @"OBJECT_CONSTRUCT(
                        'StringValue', 'abcąęśźń',
                        'CharValue', 'x',
                        'ByteValue', 15,
                        'SByteValue', -14,
                        'ShortValue', 1200,
                        'UShortValue', 65000,
                        'IntValue', 150150,
                        'UIntValue', 151151,
                        'LongValue', 9111222333444555666,
                        'ULongValue', 9111222333444555666,
                        'FloatValue', 1.23,
                        'DoubleValue', 1.23,
                        'DecimalValue', 1.23,
                        'BooleanValue', true,
                        'GuidValue', '57af59a1-f010-450a-8c37-8fdc78e6ee93',
                        'DateTimeValue', '2024-07-11 14:20:05'::TIMESTAMP_NTZ,
                        'DateTimeOffsetValue', '2024-07-11 14:20:05'::TIMESTAMP_LTZ,
                        'TimeSpanValue', '14:20:05'::TIME,
                        'BinaryValue', TO_BINARY('this is binary data', 'UTF-8'),
                        'SemiStructuredValue', OBJECT_CONSTRUCT('a', 'b')
                    )" + AllTypesObjectDefinition + ", '2024-07-11 14:20:05'::TIMESTAMP_LTZ";
        var bytesForBinary = Encoding.UTF8.GetBytes("this is binary data");
        using var command = connection.CreateCommand();
        command.CommandText = $"SELECT {allTypesObjectAsSFString}";
        using var reader = await command.ExecuteReaderAsync(ct).ConfigureAwait(false);
        Assert.True(reader.Read());

        var allUnstructuredTypesObject = GetObject<AllUnstructuredTypesClass>(reader, 0);

        Assert.NotNull(allUnstructuredTypesObject);
        Assert.Equal("abcąęśźń", allUnstructuredTypesObject.StringValue);
        Assert.Equal('x', allUnstructuredTypesObject.CharValue);
        Assert.Equal(15, allUnstructuredTypesObject.ByteValue);
        Assert.Equal(-14, allUnstructuredTypesObject.SByteValue);
        Assert.Equal(1200, allUnstructuredTypesObject.ShortValue);
        Assert.Equal(65000, allUnstructuredTypesObject.UShortValue);
        Assert.Equal(150150, allUnstructuredTypesObject.IntValue);
        Assert.Equal(151151, allUnstructuredTypesObject.UIntValue);
        Assert.Equal(9111222333444555666, allUnstructuredTypesObject.LongValue);
        Assert.Equal(9111222333444555666, allUnstructuredTypesObject.ULongValue);
        Assert.Equal(1.23f, allUnstructuredTypesObject.FloatValue);
        Assert.Equal(1.23d, allUnstructuredTypesObject.DoubleValue);
        Assert.Equal(1.23m, allUnstructuredTypesObject.DecimalValue);
        Assert.True(allUnstructuredTypesObject.BooleanValue);
        Assert.Equal(Guid.Parse("57af59a1-f010-450a-8c37-8fdc78e6ee93"), allUnstructuredTypesObject.GuidValue);
        Assert.Equal(ParseDateTime("2024-07-11 14:20:05"), allUnstructuredTypesObject.DateTimeValue);
        Assert.Equal(
            ParseDateTimeOffset($"2024-07-11 14:20:05 {expectedOffsetString}"),
            allUnstructuredTypesObject.DateTimeOffsetValue);
        Assert.Equal(TimeSpan.Parse("14:20:05", CultureInfo.InvariantCulture), allUnstructuredTypesObject.TimeSpanValue);
        Assert.Equal(bytesForBinary, allUnstructuredTypesObject.BinaryValue);
        Assert.Equal(
            RemoveWhiteSpaces("{\"a\": \"b\"}"),
            RemoveWhiteSpaces(allUnstructuredTypesObject.SemiStructuredValue!));
    }

    [SnowflakeFact]
    public async Task ShouldSelectAllUnstructuredTypesObjectIntoNullableFieldsAsync()
    {
        // TODO(SNOW-4382580): the new driver ignores DOTNET_QUERY_RESULT_FORMAT, so JSON results cannot be selected
        Skip.FutureMilestone();

        // TODO(SNOW-4382584): SnowflakeDbDataReader.GetObject<T> for structured types
        Skip.FutureMilestone();

        var ct = TestContext.Current.CancellationToken;
        using var connection = Fixture.Factory.Create(Output);
        await connection.OpenAsync(ct).ConfigureAwait(false);
        await EnableStructuredTypesAsync(connection, ct).ConfigureAwait(false);
        var timeZone = await GetTimeZoneAsync(connection, ct).ConfigureAwait(false);
        var expectedOffset = timeZone.GetUtcOffset(ParseDateTime("2024-07-11 14:20:05"));
        var expectedOffsetString = ToOffsetString(expectedOffset);
        var allTypesObjectAsSFString = @"OBJECT_CONSTRUCT(
                        'StringValue', 'abc',
                        'CharValue', 'x',
                        'ByteValue', 15,
                        'SByteValue', -14,
                        'ShortValue', 1200,
                        'UShortValue', 65000,
                        'IntValue', 150150,
                        'UIntValue', 151151,
                        'LongValue', 9111222333444555666,
                        'ULongValue', 9111222333444555666,
                        'FloatValue', 1.23,
                        'DoubleValue', 1.23,
                        'DecimalValue', 1.23,
                        'BooleanValue', true,
                        'GuidValue', '57af59a1-f010-450a-8c37-8fdc78e6ee93',
                        'DateTimeValue', '2024-07-11 14:20:05'::TIMESTAMP_NTZ,
                        'DateTimeOffsetValue', '2024-07-11 14:20:05'::TIMESTAMP_LTZ,
                        'TimeSpanValue', '14:20:05'::TIME,
                        'BinaryValue', TO_BINARY('this is binary data', 'UTF-8'),
                        'SemiStructuredValue', OBJECT_CONSTRUCT('a', 'b')
                    )" + AllTypesObjectDefinition;
        var bytesForBinary = Encoding.UTF8.GetBytes("this is binary data");
        using var command = connection.CreateCommand();
        command.CommandText = $"SELECT {allTypesObjectAsSFString}";
        using var reader = await command.ExecuteReaderAsync(ct).ConfigureAwait(false);
        Assert.True(reader.Read());

        var allUnstructuredTypesObject = GetObject<AllUnstructuredTypesClass>(reader, 0);

        Assert.NotNull(allUnstructuredTypesObject);
        Assert.Equal("abc", allUnstructuredTypesObject.StringValue);
        Assert.Equal('x', allUnstructuredTypesObject.CharValue);
        Assert.Equal(15, allUnstructuredTypesObject.ByteValue);
        Assert.Equal(-14, allUnstructuredTypesObject.SByteValue);
        Assert.Equal(1200, allUnstructuredTypesObject.ShortValue);
        Assert.Equal(65000, allUnstructuredTypesObject.UShortValue);
        Assert.Equal(150150, allUnstructuredTypesObject.IntValue);
        Assert.Equal(151151, allUnstructuredTypesObject.UIntValue);
        Assert.Equal(9111222333444555666, allUnstructuredTypesObject.LongValue);
        Assert.Equal(9111222333444555666, allUnstructuredTypesObject.ULongValue);
        Assert.Equal(1.23f, allUnstructuredTypesObject.FloatValue);
        Assert.Equal(1.23d, allUnstructuredTypesObject.DoubleValue);
        Assert.Equal(1.23m, allUnstructuredTypesObject.DecimalValue);
        Assert.True(allUnstructuredTypesObject.BooleanValue);
        Assert.Equal(Guid.Parse("57af59a1-f010-450a-8c37-8fdc78e6ee93"), allUnstructuredTypesObject.GuidValue);
        Assert.Equal(ParseDateTime("2024-07-11 14:20:05"), allUnstructuredTypesObject.DateTimeValue);
        Assert.Equal(
            ParseDateTimeOffset($"2024-07-11 14:20:05 {expectedOffsetString}"),
            allUnstructuredTypesObject.DateTimeOffsetValue);
        Assert.Equal(TimeSpan.Parse("14:20:05", CultureInfo.InvariantCulture), allUnstructuredTypesObject.TimeSpanValue);
        Assert.Equal(bytesForBinary, allUnstructuredTypesObject.BinaryValue);
        Assert.Equal(
            RemoveWhiteSpaces("{\"a\": \"b\"}"),
            RemoveWhiteSpaces(allUnstructuredTypesObject.SemiStructuredValue!));
    }

    [SnowflakeFact]
    public async Task ShouldSelectNullIntoUnstructuredTypesObjectAsync()
    {
        // TODO(SNOW-4382580): the new driver ignores DOTNET_QUERY_RESULT_FORMAT, so JSON results cannot be selected
        Skip.FutureMilestone();

        // TODO(SNOW-4382584): SnowflakeDbDataReader.GetObject<T> for structured types
        Skip.FutureMilestone();

        var ct = TestContext.Current.CancellationToken;
        using var connection = Fixture.Factory.Create(Output);
        await connection.OpenAsync(ct).ConfigureAwait(false);
        await EnableStructuredTypesAsync(connection, ct).ConfigureAwait(false);
        var allTypesObjectAsSFString = @"OBJECT_CONSTRUCT_KEEP_NULL(
                        'StringValue', NULL,
                        'CharValue', NULL,
                        'ByteValue', NULL,
                        'SByteValue', NULL,
                        'ShortValue', NULL,
                        'UShortValue', NULL,
                        'IntValue', NULL,
                        'UIntValue', NULL,
                        'LongValue', NULL,
                        'ULongValue', NULL,
                        'FloatValue', NULL,
                        'DoubleValue', NULL,
                        'DecimalValue', NULL,
                        'BooleanValue', NULL,
                        'GuidValue', NULL,
                        'DateTimeValue', NULL,
                        'DateTimeOffsetValue', NULL,
                        'TimeSpanValue', NULL,
                        'BinaryValue', NULL,
                        'SemiStructuredValue', NULL
                    )" + AllTypesObjectDefinition;
        using var command = connection.CreateCommand();
        command.CommandText = $"SELECT {allTypesObjectAsSFString}";
        using var reader = await command.ExecuteReaderAsync(ct).ConfigureAwait(false);
        Assert.True(reader.Read());

        var allUnstructuredTypesObject = GetObject<AllNullableUnstructuredTypesClass>(reader, 0);

        Assert.NotNull(allUnstructuredTypesObject);
        Assert.Null(allUnstructuredTypesObject.StringValue);
        Assert.Null(allUnstructuredTypesObject.CharValue);
        Assert.Null(allUnstructuredTypesObject.ByteValue);
        Assert.Null(allUnstructuredTypesObject.SByteValue);
        Assert.Null(allUnstructuredTypesObject.ShortValue);
        Assert.Null(allUnstructuredTypesObject.UShortValue);
        Assert.Null(allUnstructuredTypesObject.IntValue);
        Assert.Null(allUnstructuredTypesObject.UIntValue);
        Assert.Null(allUnstructuredTypesObject.LongValue);
        Assert.Null(allUnstructuredTypesObject.ULongValue);
        Assert.Null(allUnstructuredTypesObject.FloatValue);
        Assert.Null(allUnstructuredTypesObject.DoubleValue);
        Assert.Null(allUnstructuredTypesObject.DecimalValue);
        Assert.Null(allUnstructuredTypesObject.BooleanValue);
        Assert.Null(allUnstructuredTypesObject.GuidValue);
        Assert.Null(allUnstructuredTypesObject.DateTimeValue);
        Assert.Null(allUnstructuredTypesObject.DateTimeOffsetValue);
        Assert.Null(allUnstructuredTypesObject.TimeSpanValue);
        Assert.Null(allUnstructuredTypesObject.BinaryValue);
        Assert.Null(allUnstructuredTypesObject.SemiStructuredValue);
    }

    [SnowflakeTheory]
    [MemberData(nameof(DateTimeConversionCases))]
    public async Task ShouldSelectDateTimeAsync(string dbValue, string dbType, DateTime? expectedRaw, DateTime expected)
    {
        // TODO(SNOW-4382580): the new driver ignores DOTNET_QUERY_RESULT_FORMAT, so JSON results cannot be selected
        Skip.FutureMilestone();

        // TODO(SNOW-4382584): SnowflakeDbDataReader.GetObject<T> for structured types
        Skip.FutureMilestone();

        var ct = TestContext.Current.CancellationToken;
        using var connection = await OpenConnectionHonoringSessionTimezoneAsync(ct).ConfigureAwait(false);
        using var reader = await QueryRawAndWrappedValueAsync(connection, dbValue, dbType, ct).ConfigureAwait(false);

        AssertRawDateTime(reader, expectedRaw);
        var wrappedValue = GetObject<DateTimeWrapper>(reader, 1);
        Assert.Equal(expected, wrappedValue.Value);
        Assert.Equal(expected.Kind, wrappedValue.Value.Kind);
    }

    [SnowflakeTheory]
    [MemberData(nameof(DateTimeOffsetConversionCases))]
    public async Task ShouldSelectDateTimeOffsetAsync(
        string dbValue,
        string dbType,
        DateTime? expectedRaw,
        DateTimeOffset expected)
    {
        // TODO(SNOW-4382580): the new driver ignores DOTNET_QUERY_RESULT_FORMAT, so JSON results cannot be selected
        Skip.FutureMilestone();

        // TODO(SNOW-4382584): SnowflakeDbDataReader.GetObject<T> for structured types
        Skip.FutureMilestone();

        var ct = TestContext.Current.CancellationToken;
        using var connection = await OpenConnectionHonoringSessionTimezoneAsync(ct).ConfigureAwait(false);
        using var reader = await QueryRawAndWrappedValueAsync(connection, dbValue, dbType, ct).ConfigureAwait(false);

        AssertRawDateTime(reader, expectedRaw);
        var wrappedValue = GetObject<DateTimeOffsetWrapper>(reader, 1);
        Assert.Equal(expected, wrappedValue.Value);
    }

    private static async Task<DbDataReader> QueryRawAndWrappedValueAsync(
        ISnowflakeDbConnection connection,
        string dbValue,
        string dbType,
        CancellationToken cancellationToken)
    {
        var rawValueString = $"'{dbValue}'::{dbType}";
        var objectValueString = $"OBJECT_CONSTRUCT('Value', {rawValueString})::OBJECT(Value {dbType})";
        var command = connection.CreateCommand();
        command.CommandText = $"SELECT {rawValueString}, {objectValueString}";
        var reader = await command.ExecuteReaderAsync(cancellationToken).ConfigureAwait(false);
        Assert.True(reader.Read());
        return reader;
    }

    private static void AssertRawDateTime(DbDataReader reader, DateTime? expectedRaw)
    {
        if (expectedRaw == null)
            return;
        var rawValue = reader.GetDateTime(0);
        Assert.Equal(expectedRaw, rawValue);
        Assert.Equal(expectedRaw.Value.Kind, rawValue.Kind);
    }

    private async Task<ISnowflakeDbConnection> OpenConnectionHonoringSessionTimezoneAsync(CancellationToken ct)
    {
        var baseConnectionString = Fixture.Factory.Create(Output).ConnectionString;
        var connection = Fixture.Factory.Create(Output, $"{baseConnectionString};HonorSessionTimezone=true");
        await connection.OpenAsync(ct).ConfigureAwait(false);
        using var command = connection.CreateCommand();
        command.CommandText = "ALTER SESSION SET TIMEZONE = 'America/Los_Angeles'";
        await command.ExecuteNonQueryAsync(ct).ConfigureAwait(false);
        await EnableStructuredTypesAsync(connection, ct).ConfigureAwait(false);
        await SetTimePrecisionAsync(connection, 9, ct).ConfigureAwait(false);
        return connection;
    }

    private static async Task EnableStructuredTypesAsync(ISnowflakeDbConnection connection, CancellationToken ct)
    {
        using var command = connection.CreateCommand();
        command.CommandText = "alter session set ENABLE_STRUCTURED_TYPES_IN_CLIENT_RESPONSE = true";
        await command.ExecuteNonQueryAsync(ct).ConfigureAwait(false);
        command.CommandText = "alter session set IGNORE_CLIENT_VESRION_IN_STRUCTURED_TYPES_RESPONSE = true";
        await command.ExecuteNonQueryAsync(ct).ConfigureAwait(false);
        command.CommandText = "ALTER SESSION SET DOTNET_QUERY_RESULT_FORMAT = JSON";
        await command.ExecuteNonQueryAsync(ct).ConfigureAwait(false);
    }

    private static async Task SetTimePrecisionAsync(ISnowflakeDbConnection connection, int precision, CancellationToken ct)
    {
        using var command = connection.CreateCommand();
        command.CommandText = $"ALTER SESSION SET TIMESTAMP_NTZ_OUTPUT_FORMAT = 'YYYY-MM-DD HH24:MI:SS.FF{precision}'";
        await command.ExecuteNonQueryAsync(ct).ConfigureAwait(false);
        command.CommandText = $"ALTER SESSION SET TIMESTAMP_OUTPUT_FORMAT = 'YYYY-MM-DD HH24:MI:SS.FF{precision} TZHTZM'";
        await command.ExecuteNonQueryAsync(ct).ConfigureAwait(false);
    }

    private static async Task<TimeZoneInfo> GetTimeZoneAsync(ISnowflakeDbConnection connection, CancellationToken ct)
    {
        using var command = connection.CreateCommand();
        command.CommandText = "show parameters like 'timezone'";
        using var reader = await command.ExecuteReaderAsync(ct).ConfigureAwait(false);
        Assert.True(reader.Read());
        return FindSystemTimeZoneById(reader.GetString(1));
    }

    private static TimeZoneInfo FindSystemTimeZoneById(string timeZoneString)
    {
        try
        {
            return TimeZoneInfo.FindSystemTimeZoneById(timeZoneString);
        }
        catch (TimeZoneNotFoundException)
        {
            var alternativeTimeZoneString = timeZoneString switch
            {
                "America/Los_Angeles" => "Pacific Standard Time",
                "Europe/Warsaw" => "Central European Standard Time",
                "Asia/Tokyo" => "Tokyo Standard Time",
                _ => throw new TimeZoneNotFoundException($"Could not recognize time zone: {timeZoneString}"),
            };
            return TimeZoneInfo.FindSystemTimeZoneById(alternativeTimeZoneString);
        }
    }

    private static string ToOffsetString(TimeSpan timeSpan)
    {
        var offsetString = timeSpan.ToString(null, CultureInfo.InvariantCulture);
        var secondsIndex = offsetString.LastIndexOf(':');
        var offsetWithoutSeconds = offsetString.Substring(0, secondsIndex);
        return offsetWithoutSeconds.StartsWith("+", StringComparison.Ordinal)
               || offsetWithoutSeconds.StartsWith("-", StringComparison.Ordinal)
            ? offsetWithoutSeconds
            : "+" + offsetWithoutSeconds;
    }

    private static string RemoveWhiteSpaces(string text) =>
        new(text.Where(c => !char.IsWhiteSpace(c)).ToArray());

    private static DateTime ParseDateTime(string value, DateTimeKind kind = DateTimeKind.Unspecified) =>
        DateTime.SpecifyKind(DateTime.Parse(value, CultureInfo.InvariantCulture), kind);

    private static DateTimeOffset ParseDateTimeOffset(string value) =>
        DateTimeOffset.Parse(value, CultureInfo.InvariantCulture);

    private static T GetObject<T>(DbDataReader reader, int ordinal)
        where T : class, new() =>
        reader is OldSnowflakeDbDataReader oldReader
            ? oldReader.GetObject<T>(ordinal)
            : throw new NotSupportedException($"{reader.GetType()} has no GetObject<T>");

    private sealed class DateTimeWrapper
    {
        public DateTime Value { get; set; }
    }

    private sealed class DateTimeOffsetWrapper
    {
        public DateTimeOffset Value { get; set; }
    }

    private sealed class AllUnstructuredTypesClass
    {
        public string? StringValue { get; set; }
        public char CharValue { get; set; }
        public byte ByteValue { get; set; }
        public sbyte SByteValue { get; set; }
        public short ShortValue { get; set; }
        public ushort UShortValue { get; set; }
        public int IntValue { get; set; }
        public int UIntValue { get; set; }
        public long LongValue { get; set; }
        public long ULongValue { get; set; }
        public float FloatValue { get; set; }
        public double DoubleValue { get; set; }
        public decimal DecimalValue { get; set; }
        public bool BooleanValue { get; set; }
        public Guid GuidValue { get; set; }
        public DateTime DateTimeValue { get; set; }
        public DateTimeOffset DateTimeOffsetValue { get; set; }
        public TimeSpan TimeSpanValue { get; set; }
        public byte[]? BinaryValue { get; set; }
        public string? SemiStructuredValue { get; set; }
    }

    private sealed class AllNullableUnstructuredTypesClass
    {
        public string? StringValue { get; set; }
        public char? CharValue { get; set; }
        public byte? ByteValue { get; set; }
        public sbyte? SByteValue { get; set; }
        public short? ShortValue { get; set; }
        public ushort? UShortValue { get; set; }
        public int? IntValue { get; set; }
        public int? UIntValue { get; set; }
        public long? LongValue { get; set; }
        public long? ULongValue { get; set; }
        public float? FloatValue { get; set; }
        public double? DoubleValue { get; set; }
        public decimal? DecimalValue { get; set; }
        public bool? BooleanValue { get; set; }
        public Guid? GuidValue { get; set; }
        public DateTime? DateTimeValue { get; set; }
        public DateTimeOffset? DateTimeOffsetValue { get; set; }
        public TimeSpan? TimeSpanValue { get; set; }
        public byte[]? BinaryValue { get; set; }
        public string? SemiStructuredValue { get; set; }
    }
}
