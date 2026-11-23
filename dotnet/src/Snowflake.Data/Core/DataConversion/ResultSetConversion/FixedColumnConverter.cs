using System.Globalization;
using System.Numerics;
using Apache.Arrow;
using Snowflake.Data.Interfaces;

namespace Snowflake.Data.Core.DataConversion.ResultSetConversion;

/// <summary>
/// Reads a FIXED column with scale 0.
/// </summary>
/// <remarks>
/// The server picks the narrowest Arrow type for each batch from the values in it, so one column can
/// arrive as Int8, Int16, Int32, Int64 or Decimal128 in different batches. <see cref="ColumnConverter.GetValue"/>
/// returns <see cref="long"/> whatever the batch's type, and <see cref="decimal"/> only for a value
/// outside the <see cref="long"/> range. Reading a value as <see cref="string"/>, <see cref="double"/> or
/// <see cref="float"/> accepts every value NUMBER(38,0) holds, including those outside the
/// <see cref="decimal"/> range; reading it as an integer type outside that type's range, or as
/// <see cref="decimal"/> outside the <see cref="decimal"/> range, is a conversion error. A NULL value read as any
/// type is a conversion error.
/// </remarks>
internal sealed class FixedColumnConverter :
    ColumnConverter,
    IValueReader<long>,
    IValueReader<int>,
    IValueReader<short>,
    IValueReader<byte>,
    IValueReader<decimal>,
    IValueReader<double>,
    IValueReader<float>,
    IValueReader<string>
{
    private const int Decimal128Width = 16;

    private PhysicalType _physicalType;
    private Int8Array? _int8;
    private Int16Array? _int16;
    private Int32Array? _int32;
    private Int64Array? _int64;
    private Decimal128Array? _decimal128;

    internal FixedColumnConverter()
        : base(nameof(SFDataType.FIXED))
    {
    }

    private enum PhysicalType
    {
        Int8,
        Int16,
        Int32,
        Int64,
        Decimal128,
    }

    internal override Type FieldType => SnowflakeTypes.CtsType(SFDataType.FIXED, scale: 0);

    long IValueReader<long>.Read(int row)
    {
        ThrowIfNull(row, typeof(long));
        return ReadInt64(row, typeof(long));
    }

    int IValueReader<int>.Read(int row)
    {
        ThrowIfNull(row, typeof(int));
        var value = ReadInt64(row, typeof(int));
        return value is >= int.MinValue and <= int.MaxValue ? (int)value : throw ConversionFailed(typeof(int));
    }

    short IValueReader<short>.Read(int row)
    {
        ThrowIfNull(row, typeof(short));
        var value = ReadInt64(row, typeof(short));
        return value is >= short.MinValue and <= short.MaxValue ? (short)value : throw ConversionFailed(typeof(short));
    }

    byte IValueReader<byte>.Read(int row)
    {
        ThrowIfNull(row, typeof(byte));
        var value = ReadInt64(row, typeof(byte));
        return value is >= byte.MinValue and <= byte.MaxValue ? (byte)value : throw ConversionFailed(typeof(byte));
    }

    decimal IValueReader<decimal>.Read(int row)
    {
        ThrowIfNull(row, typeof(decimal));
        return _physicalType == PhysicalType.Decimal128 ? ReadDecimal(row, typeof(decimal)) : ReadInt64(row, typeof(decimal));
    }

    double IValueReader<double>.Read(int row)
    {
        ThrowIfNull(row, typeof(double));
        return _physicalType == PhysicalType.Decimal128 ? (double)ReadInt128(row) : ReadInt64(row, typeof(double));
    }

    float IValueReader<float>.Read(int row)
    {
        ThrowIfNull(row, typeof(float));
        return _physicalType == PhysicalType.Decimal128 ? (float)ReadInt128(row) : ReadInt64(row, typeof(float));
    }

    // TODO(SNOW-4483010) future milestone: format with the culture the client configures instead of always the invariant one.
    string IValueReader<string>.Read(int row)
    {
        ThrowIfNull(row, typeof(string));
        return _physicalType == PhysicalType.Decimal128
            ? ReadInt128(row).ToString(CultureInfo.InvariantCulture)
            : ReadInt64(row, typeof(string)).ToString(CultureInfo.InvariantCulture);
    }

    protected override void OnUnbind()
    {
        _int8 = null;
        _int16 = null;
        _int32 = null;
        _int64 = null;
        _decimal128 = null;
    }

    protected override void OnBind(IArrowArray array)
    {
        switch (array)
        {
            case Int8Array int8:
                _int8 = int8;
                _physicalType = PhysicalType.Int8;
                break;
            case Int16Array int16:
                _int16 = int16;
                _physicalType = PhysicalType.Int16;
                break;
            case Int32Array int32:
                _int32 = int32;
                _physicalType = PhysicalType.Int32;
                break;
            case Int64Array int64:
                _int64 = int64;
                _physicalType = PhysicalType.Int64;
                break;
            case Decimal128Array decimal128:
                _decimal128 = decimal128;
                _physicalType = PhysicalType.Decimal128;
                break;
            default:
                throw UnexpectedArrowType(array);
        }
    }

    // TODO(SNOW-4483010) future milestone: return a value outside the decimal range as a string when ALLOW_NUMBER_OVERFLOW_AS_STRING is set.
    protected override object GetNonNullValue(int row)
    {
        if (_physicalType != PhysicalType.Decimal128)
            return ReadInt64(row, typeof(long));

        var value = ReadDecimal(row, typeof(object));
        return value is >= long.MinValue and <= long.MaxValue ? (object)(long)value : value;
    }

    private long ReadInt64(int row, Type target)
    {
        switch (_physicalType)
        {
            case PhysicalType.Int8:
                return _int8!.Values[row];
            case PhysicalType.Int16:
                return _int16!.Values[row];
            case PhysicalType.Int32:
                return _int32!.Values[row];
            case PhysicalType.Int64:
                return _int64!.Values[row];
            default:
                var value = ReadDecimal(row, target);
                return value is >= long.MinValue and <= long.MaxValue ? (long)value : throw ConversionFailed(target);
        }
    }

    private decimal ReadDecimal(int row, Type target)
    {
        try
        {
            return _decimal128!.GetValue(row)!.Value;
        }
        catch (OverflowException ex)
        {
            // Apache.Arrow's Decimal128Array throws OverflowException for a value outside the decimal range.
            throw ConversionFailed(target, ex);
        }
    }

    private BigInteger ReadInt128(int row)
    {
        var bytes = _decimal128!.ValueBuffer.Span.Slice((_decimal128.Offset + row) * Decimal128Width, Decimal128Width).ToArray();
        return new BigInteger(bytes);
    }
}
