using System.Data;
using System.Globalization;
using System.Text;
using Snowflake.Data.Core.Errors;

namespace Snowflake.Data.Core.DataConversion.ParameterConversion;

/// <summary>
/// Turns the value of a bound parameter into the Snowflake type and text the query request binds it as.
/// </summary>
/// <remarks>
/// <para>
/// FIXED, BOOLEAN, REAL and TEXT send the value's invariant-culture text and leave its validation to the server. A
/// <see cref="double"/> or <see cref="float"/> is sent as the shortest text that parses back to the same value, the
/// same on every target framework, and a <c>char[]</c> as the string of its characters.
/// </para>
/// <para>
/// Temporal values are sent as integers counted from the Unix epoch: DATE in milliseconds, TIME in nanoseconds since
/// midnight, and the timestamps in nanoseconds. TIMESTAMP_TZ is followed by a space and the offset in minutes plus
/// 1440. The <see cref="DateTimeKind"/> of a value is ignored. Errors carry the value as hidden.
/// </para>
/// </remarks>
internal static class ParameterValueConverter
{
    private const string HiddenValue = "[value hidden]";

    private static readonly TimeSpan TimestampTzOffsetBias = TimeSpan.FromMinutes(1440);
    private static readonly TimeSpan OneDay = TimeSpan.FromDays(1);
    private static readonly DateTime UnixEpoch = DateTime.UnixEpoch;
    private static readonly string[] ShorterDoubleFormats = ["G15", "G16"];
    private static readonly string[] ShorterSingleFormats = ["G7", "G8"];

    /// <summary>
    /// The Snowflake type <paramref name="dbType"/> binds as, and <paramref name="value"/> as text in that type.
    /// </summary>
    /// <exception cref="SnowflakeDriverException">
    /// <see cref="SFError.UNSUPPORTED_DOTNET_TYPE"/> when <paramref name="dbType"/> has no Snowflake counterpart, or
    /// a failure of <see cref="ToSnowflakeText"/>.
    /// </exception>
    internal static (SFDataType Type, string? Text) Convert(DbType dbType, object? value)
    {
        var type = ToSnowflakeType(dbType);
        return (type, ToSnowflakeText(type, value));
    }

    /// <summary>
    /// <paramref name="value"/> as text in <paramref name="type"/>, or null for a null or <see cref="DBNull"/> value.
    /// </summary>
    /// <exception cref="SnowflakeDriverException">
    /// <see cref="SFError.INVALID_DATA_CONVERSION"/> when a FIXED, BOOLEAN, REAL or TEXT value is neither
    /// <see cref="IFormattable"/>, <see cref="IConvertible"/> nor a <c>char[]</c>; a DATE or TIMESTAMP_NTZ value is not
    /// a <see cref="DateTime"/>; a TIME value is neither a <see cref="DateTime"/> nor a <see cref="TimeSpan"/> from zero
    /// up to one day; a TIMESTAMP_LTZ or TIMESTAMP_TZ value is not a <see cref="DateTimeOffset"/>; or a BINARY value is
    /// not a byte array. <see cref="SFError.UNSUPPORTED_SNOWFLAKE_TYPE_FOR_PARAM"/> for a type that cannot be bound.
    /// </exception>
    internal static string? ToSnowflakeText(SFDataType type, object? value)
    {
        if (value is null or DBNull)
            return null;

        return type switch
        {
            SFDataType.FIXED or SFDataType.BOOLEAN or SFDataType.REAL or SFDataType.TEXT => ToInvariantText(value, type),
            SFDataType.DATE => DateToText(Expect<DateTime>(value, DbType.Date)),
            SFDataType.TIME => value is TimeSpan timeOfDay ? TimeOfDayToText(timeOfDay) : TimeToText(Expect<DateTime>(value, DbType.Time)),
            SFDataType.TIMESTAMP_NTZ => TimestampNtzToText(Expect<DateTime>(value, DbType.DateTime)),
            SFDataType.TIMESTAMP_LTZ => TimestampLtzToText(Expect<DateTimeOffset>(value, nameof(SFDataType.TIMESTAMP_LTZ))),
            SFDataType.TIMESTAMP_TZ => TimestampTzToText(Expect<DateTimeOffset>(value, DbType.DateTimeOffset)),
            SFDataType.BINARY => BinaryToText(Expect<byte[]>(value, DbType.Binary)),
            _ => throw new SnowflakeDriverException(SFError.UNSUPPORTED_SNOWFLAKE_TYPE_FOR_PARAM, null, type.ToString()),
        };
    }

    private static SFDataType ToSnowflakeType(DbType dbType) => dbType switch
    {
        DbType.Decimal or DbType.SByte or DbType.Int16 or DbType.Int32 or DbType.Int64 or DbType.Byte
            or DbType.UInt16 or DbType.UInt32 or DbType.UInt64 or DbType.VarNumeric => SFDataType.FIXED,
        DbType.Boolean => SFDataType.BOOLEAN,
        DbType.Double or DbType.Single => SFDataType.REAL,
        DbType.Guid or DbType.AnsiString or DbType.AnsiStringFixedLength or DbType.String
            or DbType.StringFixedLength => SFDataType.TEXT,
        DbType.Date => SFDataType.DATE,
        DbType.Time => SFDataType.TIME,
        DbType.DateTime or DbType.DateTime2 => SFDataType.TIMESTAMP_NTZ,
        DbType.DateTimeOffset => SFDataType.TIMESTAMP_TZ,
        DbType.Binary => SFDataType.BINARY,
        _ => throw new SnowflakeDriverException(SFError.UNSUPPORTED_DOTNET_TYPE, null, dbType),
    };

    private static T Expect<T>(object value, DbType target) => Expect<T>(value, target.ToString());

    private static T Expect<T>(object value, string target) =>
        value is T typed ? typed : throw InvalidConversion(value, target);

    private static SnowflakeDriverException InvalidConversion(object value, string target) =>
        new(SFError.INVALID_DATA_CONVERSION, null, HiddenValue, value.GetType().ToString(), target);

    private static string ToInvariantText(object value, SFDataType target) => value switch
    {
        double number => RoundTripText(number),
        float number => RoundTripText(number),
        char[] characters => new string(characters),
        IFormattable or IConvertible => string.Format(CultureInfo.InvariantCulture, "{0}", value),
        _ => throw InvalidConversion(value, target.ToString()),
    };

    private static string RoundTripText(double value)
    {
        foreach (var format in ShorterDoubleFormats)
        {
            var text = value.ToString(format, CultureInfo.InvariantCulture);
            if (double.TryParse(text, NumberStyles.Float, CultureInfo.InvariantCulture, out var parsed) && parsed.Equals(value))
                return text;
        }

        return value.ToString("G17", CultureInfo.InvariantCulture);
    }

    private static string RoundTripText(float value)
    {
        foreach (var format in ShorterSingleFormats)
        {
            var text = value.ToString(format, CultureInfo.InvariantCulture);
            if (float.TryParse(text, NumberStyles.Float, CultureInfo.InvariantCulture, out var parsed) && parsed.Equals(value))
                return text;
        }

        return value.ToString("G9", CultureInfo.InvariantCulture);
    }

    private static string DateToText(DateTime value)
    {
        var millis = (long)value.Date.Subtract(UnixEpoch).TotalMilliseconds;
        return millis.ToString(CultureInfo.InvariantCulture);
    }

    private static string TimeToText(DateTime value) => TicksToNanosText(value.TimeOfDay.Ticks);

    private static string TimeOfDayToText(TimeSpan value) =>
        value >= TimeSpan.Zero && value < OneDay
            ? TicksToNanosText(value.Ticks)
            : throw InvalidConversion(value, DbType.Time.ToString());

    private static string TimestampNtzToText(DateTime value) => TicksToNanosText(value.Subtract(UnixEpoch).Ticks);

    private static string TimestampLtzToText(DateTimeOffset value) => TicksToNanosText(value.Subtract(UnixEpoch).Ticks);

    private static string TimestampTzToText(DateTimeOffset value)
    {
        var offset = (int)(value.Offset + TimestampTzOffsetBias).TotalMinutes;
        return $"{TimestampLtzToText(value)} {offset.ToString(CultureInfo.InvariantCulture)}";
    }

    private static string TicksToNanosText(long ticks) =>
        ticks == 0 ? "0" : ticks.ToString(CultureInfo.InvariantCulture) + "00";

    private static string BinaryToText(byte[] value)
    {
        var hex = new StringBuilder(value.Length * 2);
        foreach (var b in value)
            hex.Append(b.ToString("x2", CultureInfo.InvariantCulture));

        return hex.ToString();
    }
}
