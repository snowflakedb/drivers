using System.Data;
using System.Globalization;
using System.Security;
using Snowflake.Data.Core;
using Snowflake.Data.Core.DataConversion.ParameterConversion;
using Snowflake.Data.Core.Errors;

namespace Snowflake.Data.Tests.Core.Conversion;

[Trait("Category", "Unit")]
public sealed class ParameterValueConverterTest
{
    private static readonly DateTimeOffset Epoch = new(1970, 1, 1, 0, 0, 0, TimeSpan.Zero);

    [SnowflakeTheory(RetriesCount = RetriesCount.NoRetries)]
    [InlineData(DbType.SByte, (sbyte)-1, "-1")]
    [InlineData(DbType.Int16, (short)123, "123")]
    [InlineData(DbType.Int32, 42, "42")]
    [InlineData(DbType.Int64, long.MinValue, "-9223372036854775808")]
    [InlineData(DbType.Byte, (byte)255, "255")]
    [InlineData(DbType.UInt16, (ushort)65535, "65535")]
    [InlineData(DbType.UInt32, 4294967295u, "4294967295")]
    [InlineData(DbType.UInt64, ulong.MaxValue, "18446744073709551615")]
    [InlineData(DbType.VarNumeric, 123, "123")]
    public void NumericDbTypesBindAsFixed(DbType dbType, object value, string expected)
    {
        Assert.Equal((SFDataType.FIXED, expected), ParameterValueConverter.Convert(dbType, value));
    }

    [SnowflakeTheory(RetriesCount = RetriesCount.NoRetries)]
    [MemberData(nameof(Decimals))]
    public void DecimalBindsAsFixedWithAllItsDigits(decimal value, string expected)
    {
        Assert.Equal((SFDataType.FIXED, expected), ParameterValueConverter.Convert(DbType.Decimal, value));
    }

    public static TheoryData<decimal, string> Decimals() => new()
    {
        { 42.4m, "42.4" },
        { 10.10m, "10.10" },
        { 0.0000001m, "0.0000001" },
        { decimal.MaxValue, "79228162514264337593543950335" },
        { decimal.MinValue, "-79228162514264337593543950335" },
    };

    [SnowflakeTheory(RetriesCount = RetriesCount.NoRetries)]
    [InlineData(DbType.Double, 1.5d, "1.5")]
    [InlineData(DbType.Single, 1.5f, "1.5")]
    [InlineData(DbType.Double, double.NaN, "NaN")]
    public void FloatingPointDbTypesBindAsReal(DbType dbType, object value, string expected)
    {
        Assert.Equal((SFDataType.REAL, expected), ParameterValueConverter.Convert(dbType, value));
    }

    [SnowflakeTheory(RetriesCount = RetriesCount.NoRetries)]
    [InlineData(0.1d, "0.1")]
    [InlineData(0.1d + 0.2d, "0.30000000000000004")]
    [InlineData(1d / 3d, "0.3333333333333333")]
    [InlineData(double.MaxValue, "1.7976931348623157E+308")]
    [InlineData(double.MinValue, "-1.7976931348623157E+308")]
    [InlineData(double.PositiveInfinity, "Infinity")]
    [InlineData(0.1f, "0.1")]
    [InlineData(1f / 3f, "0.33333334")]
    [InlineData(float.MaxValue, "3.4028235E+38")]
    public void FloatingPointIsTheShortestTextThatParsesBackToTheSameValue(object value, string expected)
    {
        Assert.Equal(expected, ParameterValueConverter.ToSnowflakeText(SFDataType.REAL, value));
    }

    [SnowflakeTheory(RetriesCount = RetriesCount.NoRetries)]
    [InlineData(true, "True")]
    [InlineData(false, "False")]
    public void BooleanBindsAsBoolean(bool value, string expected)
    {
        Assert.Equal((SFDataType.BOOLEAN, expected), ParameterValueConverter.Convert(DbType.Boolean, value));
    }

    [SnowflakeTheory(RetriesCount = RetriesCount.NoRetries)]
    [InlineData(DbType.AnsiString)]
    [InlineData(DbType.AnsiStringFixedLength)]
    [InlineData(DbType.String)]
    [InlineData(DbType.StringFixedLength)]
    public void StringDbTypesBindAsText(DbType dbType)
    {
        Assert.Equal((SFDataType.TEXT, "zażółć"), ParameterValueConverter.Convert(dbType, "zażółć"));
        Assert.Equal((SFDataType.TEXT, string.Empty), ParameterValueConverter.Convert(dbType, string.Empty));
    }

    [SnowflakeTheory(RetriesCount = RetriesCount.NoRetries)]
    [MemberData(nameof(ValuesOfAnotherClrTypeForATextualTarget))]
    public void TextualTargetsSendAFormattableValueOfAnotherClrTypeAsItsInvariantText(DbType dbType, object value, SFDataType expectedType, string expectedText)
    {
        Assert.Equal((expectedType, expectedText), ParameterValueConverter.Convert(dbType, value));
    }

    public static TheoryData<DbType, object, SFDataType, string> ValuesOfAnotherClrTypeForATextualTarget() => new()
    {
        { DbType.Int32, "12", SFDataType.FIXED, "12" },
        { DbType.Decimal, "not a number", SFDataType.FIXED, "not a number" },
        { DbType.Int64, true, SFDataType.FIXED, "True" },
        { DbType.Int16, 3.75d, SFDataType.FIXED, "3.75" },
        { DbType.Boolean, 1, SFDataType.BOOLEAN, "1" },
        { DbType.Boolean, "yes", SFDataType.BOOLEAN, "yes" },
        { DbType.Double, 7, SFDataType.REAL, "7" },
        { DbType.Single, 1.5m, SFDataType.REAL, "1.5" },
        { DbType.String, 42, SFDataType.TEXT, "42" },
        { DbType.String, -1.5d, SFDataType.TEXT, "-1.5" },
        { DbType.String, true, SFDataType.TEXT, "True" },
        { DbType.String, new DateTime(1969, 7, 20, 20, 17, 40), SFDataType.TEXT, "07/20/1969 20:17:40" },
        { DbType.String, new DateTimeOffset(1969, 7, 20, 20, 17, 40, TimeSpan.FromMinutes(345)), SFDataType.TEXT, "07/20/1969 20:17:40 +05:45" },
        { DbType.String, new TimeSpan(1, 2, 3), SFDataType.TEXT, "01:02:03" },
        { DbType.String, 0.1d + 0.2d, SFDataType.TEXT, "0.30000000000000004" },
        { DbType.String, new[] { 'a', 'b' }, SFDataType.TEXT, "ab" },
        { DbType.Guid, "not a guid", SFDataType.TEXT, "not a guid" },
    };

    [SnowflakeFact(RetriesCount = RetriesCount.NoRetries)]
    public void GuidBindsAsTextInTheHyphenatedLowercaseForm()
    {
        var guid = new Guid("A1B2C3D4-E5F6-7890-ABCD-EF1234567890");

        Assert.Equal((SFDataType.TEXT, "a1b2c3d4-e5f6-7890-abcd-ef1234567890"), ParameterValueConverter.Convert(DbType.Guid, guid));
    }

    [SnowflakeFact(RetriesCount = RetriesCount.NoRetries)]
    public void NumbersUseTheInvariantCultureWhateverTheCurrentCulture()
    {
        var original = CultureInfo.CurrentCulture;
        // fi-FI uses ',' as the decimal separator and U+2212 MINUS SIGN for negative numbers.
        CultureInfo.CurrentCulture = CultureInfo.GetCultureInfo("fi-FI");
        try
        {
            Assert.Equal((SFDataType.REAL, "-1.2345"), ParameterValueConverter.Convert(DbType.Double, -1.2345));
            Assert.Equal((SFDataType.FIXED, "-10.5"), ParameterValueConverter.Convert(DbType.Decimal, -10.5m));
            Assert.Equal((SFDataType.DATE, "-86400000"), ParameterValueConverter.Convert(DbType.Date, new DateTime(1969, 12, 31)));
            Assert.Equal(
                (SFDataType.TIMESTAMP_NTZ, "-1000000000"),
                ParameterValueConverter.Convert(DbType.DateTime, new DateTime(1969, 12, 31, 23, 59, 59)));
            var beforeTheEpoch = new DateTimeOffset(1969, 12, 31, 23, 59, 59, TimeSpan.FromHours(5));
            Assert.Equal("-18001000000000", ParameterValueConverter.ToSnowflakeText(SFDataType.TIMESTAMP_LTZ, beforeTheEpoch));
            Assert.Equal("-18001000000000 1740", ParameterValueConverter.ToSnowflakeText(SFDataType.TIMESTAMP_TZ, beforeTheEpoch));
        }
        finally
        {
            CultureInfo.CurrentCulture = original;
        }
    }

    [SnowflakeTheory(RetriesCount = RetriesCount.NoRetries)]
    [InlineData(DateTimeKind.Unspecified)]
    [InlineData(DateTimeKind.Utc)]
    [InlineData(DateTimeKind.Local)]
    public void DateBindsAsEpochMillisecondsOfTheDateWhateverTheKind(DateTimeKind kind)
    {
        var value = new DateTime(2024, 7, 15, 10, 30, 0, kind);

        Assert.Equal((SFDataType.DATE, "1721001600000"), ParameterValueConverter.Convert(DbType.Date, value));
    }

    [SnowflakeTheory(RetriesCount = RetriesCount.NoRetries)]
    [InlineData(1970, 1, 1, "0")]
    [InlineData(1969, 12, 31, "-86400000")]
    [InlineData(1, 1, 1, "-62135596800000")]
    [InlineData(9999, 12, 31, "253402214400000")]
    public void DateBindsDatesAroundAndFarFromTheEpoch(int year, int month, int day, string expected)
    {
        Assert.Equal((SFDataType.DATE, expected), ParameterValueConverter.Convert(DbType.Date, new DateTime(year, month, day)));
    }

    [SnowflakeTheory(RetriesCount = RetriesCount.NoRetries)]
    [InlineData(0, 0, 0, 0, "0")]
    [InlineData(13, 45, 30, 500, "49530500000000")]
    [InlineData(23, 59, 59, 999, "86399999000000")]
    public void TimeBindsAsNanosecondsSinceMidnight(int hour, int minute, int second, int millisecond, string expected)
    {
        var value = new DateTime(2024, 1, 1, hour, minute, second, millisecond);

        Assert.Equal((SFDataType.TIME, expected), ParameterValueConverter.Convert(DbType.Time, value));
    }

    [SnowflakeTheory(RetriesCount = RetriesCount.NoRetries)]
    [InlineData(DateTimeKind.Unspecified)]
    [InlineData(DateTimeKind.Utc)]
    [InlineData(DateTimeKind.Local)]
    public void TimeAndTimestampNtzIgnoreTheKind(DateTimeKind kind)
    {
        var value = new DateTime(2024, 7, 15, 10, 30, 0, kind);

        Assert.Equal((SFDataType.TIME, "37800000000000"), ParameterValueConverter.Convert(DbType.Time, value));
        Assert.Equal((SFDataType.TIMESTAMP_NTZ, "1721039400000000000"), ParameterValueConverter.Convert(DbType.DateTime, value));
    }

    [SnowflakeTheory(RetriesCount = RetriesCount.NoRetries)]
    [MemberData(nameof(TimesOfDay))]
    public void TimeSpanFromZeroUpToOneDayBindsAsNanosecondsSinceMidnight(TimeSpan value, string expected)
    {
        Assert.Equal((SFDataType.TIME, expected), ParameterValueConverter.Convert(DbType.Time, value));
    }

    public static TheoryData<TimeSpan, string> TimesOfDay() => new()
    {
        { TimeSpan.Zero, "0" },
        { new TimeSpan(0, 13, 45, 30, 500), "49530500000000" },
        { TimeSpan.FromDays(1) - TimeSpan.FromTicks(1), "86399999999900" },
    };

    [SnowflakeFact(RetriesCount = RetriesCount.NoRetries)]
    public void TimeKeepsTheSubMillisecondTicks()
    {
        var value = new DateTime(2024, 1, 1).AddTicks(1);

        Assert.Equal((SFDataType.TIME, "100"), ParameterValueConverter.Convert(DbType.Time, value));
    }

    [SnowflakeTheory(RetriesCount = RetriesCount.NoRetries)]
    [InlineData(DbType.DateTime)]
    [InlineData(DbType.DateTime2)]
    public void DateTimeDbTypesBindAsTimestampNtzInEpochNanoseconds(DbType dbType)
    {
        var value = new DateTime(2024, 7, 15, 10, 30, 0);

        Assert.Equal((SFDataType.TIMESTAMP_NTZ, "1721039400000000000"), ParameterValueConverter.Convert(dbType, value));
    }

    [SnowflakeTheory(RetriesCount = RetriesCount.NoRetries)]
    [MemberData(nameof(TimestampNtzEdgeCases))]
    public void TimestampNtzBindsTheEpochAndValuesBeforeIt(DateTime value, string expected)
    {
        Assert.Equal((SFDataType.TIMESTAMP_NTZ, expected), ParameterValueConverter.Convert(DbType.DateTime, value));
    }

    public static TheoryData<DateTime, string> TimestampNtzEdgeCases() => new()
    {
        { new DateTime(1970, 1, 1), "0" },
        { new DateTime(1969, 12, 31, 23, 59, 59), "-1000000000" },
        { new DateTime(1970, 1, 1).AddTicks(1), "100" },
        { DateTime.MinValue, "-62135596800000000000" },
    };

    [SnowflakeTheory(RetriesCount = RetriesCount.NoRetries)]
    [InlineData(5, "1721021400000000000 1740")]
    [InlineData(0, "1721039400000000000 1440")]
    [InlineData(-4, "1721053800000000000 1200")]
    public void DateTimeOffsetBindsAsTimestampTzWithTheBiasedOffsetInMinutes(int offsetHours, string expected)
    {
        var value = new DateTimeOffset(2024, 7, 15, 10, 30, 0, TimeSpan.FromHours(offsetHours));

        Assert.Equal((SFDataType.TIMESTAMP_TZ, expected), ParameterValueConverter.Convert(DbType.DateTimeOffset, value));
    }

    [SnowflakeFact(RetriesCount = RetriesCount.NoRetries)]
    public void TimestampTzKeepsAnOffsetThatIsNotAWholeHour()
    {
        var value = new DateTimeOffset(2024, 7, 15, 10, 30, 0, new TimeSpan(5, 45, 0));

        Assert.Equal("1721018700000000000 1785", ParameterValueConverter.ToSnowflakeText(SFDataType.TIMESTAMP_TZ, value));
    }

    [SnowflakeTheory(RetriesCount = RetriesCount.NoRetries)]
    [InlineData(0, "1721051130500000000")]
    [InlineData(5, "1721033130500000000")]
    public void TimestampLtzBindsTheUtcInstantInEpochNanoseconds(int offsetHours, string expected)
    {
        var value = new DateTimeOffset(2024, 7, 15, 13, 45, 30, 500, TimeSpan.FromHours(offsetHours));

        Assert.Equal(expected, ParameterValueConverter.ToSnowflakeText(SFDataType.TIMESTAMP_LTZ, value));
    }

    [SnowflakeTheory(RetriesCount = RetriesCount.NoRetries)]
    [InlineData(new byte[] { 0xCA, 0xFE, 0x00, 0x0A }, "cafe000a")]
    [InlineData(new byte[0], "")]
    public void BinaryBindsAsLowercaseHex(byte[] value, string expected)
    {
        Assert.Equal((SFDataType.BINARY, expected), ParameterValueConverter.Convert(DbType.Binary, value));
    }

    [SnowflakeTheory(RetriesCount = RetriesCount.NoRetries)]
    [InlineData(DbType.Int32, SFDataType.FIXED)]
    [InlineData(DbType.String, SFDataType.TEXT)]
    [InlineData(DbType.Date, SFDataType.DATE)]
    [InlineData(DbType.DateTimeOffset, SFDataType.TIMESTAMP_TZ)]
    [InlineData(DbType.Binary, SFDataType.BINARY)]
    public void NullAndDbNullBindAsNullOfTheMappedType(DbType dbType, SFDataType expected)
    {
        Assert.Equal((expected, null), ParameterValueConverter.Convert(dbType, null));
        Assert.Equal((expected, null), ParameterValueConverter.Convert(dbType, DBNull.Value));
    }

    [SnowflakeTheory(RetriesCount = RetriesCount.NoRetries)]
    [InlineData(DbType.Currency)]
    [InlineData(DbType.Object)]
    [InlineData(DbType.Xml)]
    public void DbTypeWithoutASnowflakeCounterpartIsRejectedEvenForNull(DbType dbType)
    {
        var exception = Assert.Throws<SnowflakeDriverException>(() => ParameterValueConverter.Convert(dbType, null));

        Assert.Equal(SFError.UNSUPPORTED_DOTNET_TYPE, exception.Error);
        Assert.Equal(new object[] { dbType }, exception.Args);
    }

    [SnowflakeTheory(RetriesCount = RetriesCount.NoRetries)]
    [MemberData(nameof(ValuesOfTheWrongClrType))]
    public void ValueOfTheWrongClrTypeIsRejectedWithoutEchoingIt(SFDataType type, object value, string target)
    {
        var exception = Assert.Throws<SnowflakeDriverException>(() => ParameterValueConverter.ToSnowflakeText(type, value));

        Assert.Equal(SFError.INVALID_DATA_CONVERSION, exception.Error);
        Assert.Equal(new object[] { "[value hidden]", value.GetType().ToString(), target }, exception.Args);
    }

    public static TheoryData<SFDataType, object, string> ValuesOfTheWrongClrType() => new()
    {
        { SFDataType.DATE, Epoch, "Date" },
        { SFDataType.TIME, TimeSpan.FromDays(1), "Time" },
        { SFDataType.TIME, TimeSpan.FromTicks(-1), "Time" },
        { SFDataType.TIME, Epoch, "Time" },
        { SFDataType.TIMESTAMP_NTZ, Epoch, "DateTime" },
        { SFDataType.TIMESTAMP_LTZ, Epoch.UtcDateTime, "TIMESTAMP_LTZ" },
        { SFDataType.TIMESTAMP_TZ, Epoch.UtcDateTime, "DateTimeOffset" },
        { SFDataType.BINARY, "secret", "Binary" },
        { SFDataType.TEXT, new byte[] { 1, 2 }, "TEXT" },
        { SFDataType.TEXT, new SecureString(), "TEXT" },
        { SFDataType.FIXED, new object(), "FIXED" },
        { SFDataType.BOOLEAN, new List<bool> { true }, "BOOLEAN" },
        { SFDataType.REAL, new object(), "REAL" },
    };

    [SnowflakeTheory(RetriesCount = RetriesCount.NoRetries)]
    [InlineData(SFDataType.None)]
    [InlineData(SFDataType.VARIANT)]
    [InlineData(SFDataType.OBJECT)]
    [InlineData(SFDataType.ARRAY)]
    [InlineData(SFDataType.MAP)]
    [InlineData(SFDataType.VECTOR)]
    [InlineData(SFDataType.DECFLOAT)]
    public void SnowflakeTypeThatCannotBeBoundIsRejected(SFDataType type)
    {
        var exception = Assert.Throws<SnowflakeDriverException>(() => ParameterValueConverter.ToSnowflakeText(type, "data"));

        Assert.Equal(SFError.UNSUPPORTED_SNOWFLAKE_TYPE_FOR_PARAM, exception.Error);
        Assert.Equal(new object[] { type.ToString() }, exception.Args);
    }
}
