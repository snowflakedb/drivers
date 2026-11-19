using System.Globalization;
using System.Numerics;
using Apache.Arrow;
using Snowflake.Data.Core.DataConversion.ResultSetConversion;
using Snowflake.Data.Core.Errors;

namespace Snowflake.Data.Tests.Core.Conversion;

[Trait("Category", "Unit")]
public sealed class FixedColumnConverterTest
{
    private readonly FixedColumnConverter _converter = new();

    [SnowflakeTheory(RetriesCount = RetriesCount.NoRetries)]
    [InlineData("Int8")]
    [InlineData("Int16")]
    [InlineData("Int32")]
    [InlineData("Int64")]
    [InlineData("Decimal128")]
    public void ValueIsALongWhateverTheBatchWidth(string arrowType)
    {
        _converter.Bind(MinusFortyTwo(arrowType));

        var value = _converter.GetValue(0);

        Assert.Equal(-42L, Assert.IsType<long>(value));
        Assert.Equal(-42L, _converter.Get<long>(0));
        Assert.Equal(-42, _converter.Get<int>(0));
        Assert.Equal((short)-42, _converter.Get<short>(0));
        Assert.Equal(-42m, _converter.Get<decimal>(0));
        Assert.Equal(-42d, _converter.Get<double>(0));
        Assert.Equal(-42f, _converter.Get<float>(0));
        Assert.Equal("-42", _converter.Get<string>(0));
    }

    [SnowflakeFact(RetriesCount = RetriesCount.NoRetries)]
    public void FieldTypeIsLong()
    {
        Assert.Equal(typeof(long), _converter.FieldType);
    }

    [SnowflakeFact(RetriesCount = RetriesCount.NoRetries)]
    public void ValueBeyondTheLongRangeIsADecimal()
    {
        var beyondLong = (decimal)long.MaxValue + 1;
        _converter.Bind(UnitBatches.Decimals(beyondLong));

        Assert.Equal(beyondLong, Assert.IsType<decimal>(_converter.GetValue(0)));
        Assert.Equal(beyondLong, _converter.Get<decimal>(0));
        Assert.Equal("9223372036854775808", _converter.Get<string>(0));
        ConversionAssert.Fails(() => _converter.Get<long>(0), "FIXED", typeof(long));
    }

    [SnowflakeFact(RetriesCount = RetriesCount.NoRetries)]
    public void ValueBeyondTheDecimalRangeIsAConversionErrorExceptAsStringOrFloatingPoint()
    {
        var max = BigInteger.Pow(10, 38) - 1;
        _converter.Bind(UnitBatches.Int128(max, -max, 7));

        Assert.IsType<OverflowException>(ConversionAssert.Fails(() => _converter.GetValue(0), "FIXED", typeof(object)).InnerException);
        Assert.IsType<OverflowException>(ConversionAssert.Fails(() => _converter.Get<decimal>(1), "FIXED", typeof(decimal)).InnerException);
        ConversionAssert.Fails(() => _converter.Get<long>(0), "FIXED", typeof(long));
        Assert.Equal(1e38, _converter.Get<double>(0), 1e24);
        Assert.Equal(-1e38f, _converter.Get<float>(1), 1e24f);
        Assert.Equal("99999999999999999999999999999999999999", _converter.Get<string>(0));
        Assert.Equal("-99999999999999999999999999999999999999", _converter.Get<string>(1));
        Assert.Equal("7", _converter.Get<string>(2));
        Assert.Equal(7L, _converter.GetValue(2));
    }

    [SnowflakeFact(RetriesCount = RetriesCount.NoRetries)]
    public void NarrowingGettersFailOutsideTheirRange()
    {
        _converter.Bind(UnitBatches.Int64((long)int.MaxValue + 1, short.MaxValue + 1, -1, 256));

        ConversionAssert.Fails(() => _converter.Get<int>(0), "FIXED", typeof(int));
        ConversionAssert.Fails(() => _converter.Get<short>(1), "FIXED", typeof(short));
        ConversionAssert.Fails(() => _converter.Get<byte>(2), "FIXED", typeof(byte));
        ConversionAssert.Fails(() => _converter.Get<byte>(3), "FIXED", typeof(byte));
    }

    [SnowflakeFact(RetriesCount = RetriesCount.NoRetries)]
    public void NarrowingGettersReturnTheirRangeLimits()
    {
        _converter.Bind(UnitBatches.Int64(int.MaxValue, short.MinValue, byte.MaxValue));

        Assert.Equal(int.MaxValue, _converter.Get<int>(0));
        Assert.Equal(short.MinValue, _converter.Get<short>(1));
        Assert.Equal(byte.MaxValue, _converter.Get<byte>(2));
    }

    [SnowflakeFact(RetriesCount = RetriesCount.NoRetries)]
    public void StringUsesTheInvariantCultureWhateverTheCurrentCulture()
    {
        var original = CultureInfo.CurrentCulture;
        // sv-SE formats negative numbers with U+2212 MINUS SIGN instead of '-'.
        CultureInfo.CurrentCulture = CultureInfo.GetCultureInfo("sv-SE");
        try
        {
            _converter.Bind(UnitBatches.Int64(-5));

            Assert.Equal("-5", _converter.Get<string>(0));
        }
        finally
        {
            CultureInfo.CurrentCulture = original;
        }
    }

    [SnowflakeFact(RetriesCount = RetriesCount.NoRetries)]
    public void NullIsDbNullAndEveryTypedGetterReportsAConversionError()
    {
        _converter.Bind(UnitBatches.Int64([null]));

        Assert.True(_converter.IsNull(0));
        Assert.Same(DBNull.Value, _converter.GetValue(0));
        ConversionAssert.FailsForNull(() => _converter.Get<long>(0), "FIXED", typeof(long));
        ConversionAssert.FailsForNull(() => _converter.Get<int>(0), "FIXED", typeof(int));
        ConversionAssert.FailsForNull(() => _converter.Get<short>(0), "FIXED", typeof(short));
        ConversionAssert.FailsForNull(() => _converter.Get<byte>(0), "FIXED", typeof(byte));
        ConversionAssert.FailsForNull(() => _converter.Get<decimal>(0), "FIXED", typeof(decimal));
        ConversionAssert.FailsForNull(() => _converter.Get<double>(0), "FIXED", typeof(double));
        ConversionAssert.FailsForNull(() => _converter.Get<float>(0), "FIXED", typeof(float));
        ConversionAssert.FailsForNull(() => _converter.Get<string>(0), "FIXED", typeof(string));
    }

    [SnowflakeFact(RetriesCount = RetriesCount.NoRetries)]
    public void ConversionsANumberDoesNotSupportAreConversionErrors()
    {
        _converter.Bind(UnitBatches.Int64(1));

        ConversionAssert.Fails(() => _converter.Get<bool>(0), "FIXED", typeof(bool));
        ConversionAssert.Fails(() => _converter.Get<char>(0), "FIXED", typeof(char));
        ConversionAssert.Fails(() => _converter.Get<Guid>(0), "FIXED", typeof(Guid));
        ConversionAssert.Fails(() => _converter.Get<DateTime>(0), "FIXED", typeof(DateTime));
        ConversionAssert.Fails(() => _converter.GetBytes(0, 0, null, 0, 0), "FIXED", typeof(byte[]));
        ConversionAssert.Fails(() => _converter.GetChars(0, 0, null, 0, 0), "FIXED", typeof(char[]));
    }

    [SnowflakeFact(RetriesCount = RetriesCount.NoRetries)]
    public void RebindingFollowsTheNewBatchWidth()
    {
        _converter.Bind(new Int8Array.Builder().Append(7).Build());
        Assert.Equal(7L, _converter.GetValue(0));

        _converter.Bind(UnitBatches.Decimals((decimal)long.MaxValue + 1));
        Assert.IsType<decimal>(_converter.GetValue(0));

        _converter.Bind(UnitBatches.Int64(8));
        Assert.Equal(8L, _converter.GetValue(0));
    }

    [SnowflakeFact(RetriesCount = RetriesCount.NoRetries)]
    public void BindingAnArrowTypeSfCoreDoesNotSendForFixedIsAnInternalError()
    {
        var exception = Assert.Throws<SnowflakeDriverException>(() => _converter.Bind(UnitBatches.Strings("1")));

        Assert.Equal(SFError.INTERNAL_ERROR, exception.Error);
    }

    private static IArrowArray MinusFortyTwo(string arrowType) => arrowType switch
    {
        "Int8" => new Int8Array.Builder().Append(-42).Build(),
        "Int16" => new Int16Array.Builder().Append(-42).Build(),
        "Int32" => new Int32Array.Builder().Append(-42).Build(),
        "Int64" => UnitBatches.Int64(-42),
        "Decimal128" => UnitBatches.Decimals(-42m),
        _ => throw new ArgumentOutOfRangeException(nameof(arrowType), arrowType, null),
    };
}
