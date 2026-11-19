using System.Globalization;
using Apache.Arrow;
using Snowflake.Data.Interfaces;

namespace Snowflake.Data.Core.DataConversion.ResultSetConversion;

/// <summary>
/// Reads a TEXT column.
/// </summary>
/// <remarks>
/// Reading the text as a number parses it with the invariant culture; text that does not parse, or a number
/// outside the target type's range, is a conversion error. Read as <see cref="string"/> a NULL value is null;
/// read as any other type it is a conversion error.
/// </remarks>
// TODO(SNOW-4483010) future milestone: parse numbers with the culture the client configures instead of always the invariant one.
// TODO(SNOW-4483010) future milestone: read DATE, TIME and TIMESTAMP text as DateTime once the temporal types are read.
internal sealed class TextColumnConverter :
    ColumnConverter,
    IValueReader<string>,
    IValueReader<char>,
    IValueReader<long>,
    IValueReader<int>,
    IValueReader<short>,
    IValueReader<byte>,
    IValueReader<decimal>,
    IValueReader<double>,
    IValueReader<float>,
    IValueReader<Guid>
{
    private StringArray? _strings;

    internal TextColumnConverter()
        : base(SnowflakeTypeNames.Text)
    {
    }

    internal override Type FieldType => typeof(string);

    /// <summary>The text at <paramref name="row"/>, or null when the value is NULL.</summary>
    string? IValueReader<string>.Read(int row) => IsNull(row) ? null : Text(row);

    char IValueReader<char>.Read(int row)
    {
        var value = ReadNonNull(row, typeof(char));
        return value.Length == 1 ? value[0] : throw ConversionFailed(typeof(char));
    }

    long IValueReader<long>.Read(int row) =>
        long.TryParse(ReadNonNull(row, typeof(long)), NumberStyles.Integer, CultureInfo.InvariantCulture, out var value)
            ? value
            : throw ConversionFailed(typeof(long));

    int IValueReader<int>.Read(int row) =>
        int.TryParse(ReadNonNull(row, typeof(int)), NumberStyles.Integer, CultureInfo.InvariantCulture, out var value)
            ? value
            : throw ConversionFailed(typeof(int));

    short IValueReader<short>.Read(int row) =>
        short.TryParse(ReadNonNull(row, typeof(short)), NumberStyles.Integer, CultureInfo.InvariantCulture, out var value)
            ? value
            : throw ConversionFailed(typeof(short));

    byte IValueReader<byte>.Read(int row) =>
        byte.TryParse(ReadNonNull(row, typeof(byte)), NumberStyles.Integer, CultureInfo.InvariantCulture, out var value)
            ? value
            : throw ConversionFailed(typeof(byte));

    decimal IValueReader<decimal>.Read(int row) =>
        decimal.TryParse(ReadNonNull(row, typeof(decimal)), NumberStyles.Number | NumberStyles.AllowExponent, CultureInfo.InvariantCulture, out var value)
            ? value
            : throw ConversionFailed(typeof(decimal));

    double IValueReader<double>.Read(int row) =>
        double.TryParse(ReadNonNull(row, typeof(double)), NumberStyles.Float, CultureInfo.InvariantCulture, out var value)
            ? value
            : throw ConversionFailed(typeof(double));

    float IValueReader<float>.Read(int row) =>
        float.TryParse(ReadNonNull(row, typeof(float)), NumberStyles.Float, CultureInfo.InvariantCulture, out var value)
            ? value
            : throw ConversionFailed(typeof(float));

    Guid IValueReader<Guid>.Read(int row) =>
        Guid.TryParse(ReadNonNull(row, typeof(Guid)), out var value) ? value : throw ConversionFailed(typeof(Guid));

    internal override long GetChars(int row, long dataOffset, char[]? buffer, int bufferOffset, int length)
    {
        var value = ReadNonNull(row, typeof(char[]));
        if (buffer is null)
            return value.Length;

        if (dataOffset < 0)
            throw new ArgumentOutOfRangeException(nameof(dataOffset));

        if (dataOffset >= value.Length)
            return 0;

        var count = (int)Math.Min((long)length, value.Length - dataOffset);
        value.CopyTo((int)dataOffset, buffer, bufferOffset, count);
        return count;
    }

    protected override void OnUnbind() => _strings = null;

    protected override void OnBind(IArrowArray array) =>
        _strings = array as StringArray ?? throw UnexpectedArrowType(array);

    protected override object GetNonNullValue(int row) => Text(row);

    private string Text(int row) => _strings!.GetString(row);

    private string ReadNonNull(int row, Type target)
    {
        ThrowIfNull(row, target);
        return Text(row);
    }
}
