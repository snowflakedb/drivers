using System.Globalization;
using Snowflake.Data.Core;
using Snowflake.Data.Core.DataConversion.ResultSetConversion;
using Snowflake.Data.Core.Errors;

namespace Snowflake.Data.Tests.Core.Conversion;

[Trait("Category", "Unit")]
public sealed class TextColumnConverterTest
{
    private readonly TextColumnConverter _converter = new();

    [SnowflakeFact(RetriesCount = RetriesCount.NoRetries)]
    public void ValueAndStringAreTheText()
    {
        _converter.Bind(UnitBatches.Strings("żółw", string.Empty));

        Assert.Equal(typeof(string), _converter.FieldType);
        Assert.Equal("żółw", Assert.IsType<string>(_converter.GetValue(0)));
        Assert.Equal("żółw", _converter.Get<string>(0));
        Assert.Equal(string.Empty, _converter.Get<string>(1));
    }

    [SnowflakeFact(RetriesCount = RetriesCount.NoRetries)]
    public void NullIsDbNullAndAnyStringIsNull()
    {
        _converter.Bind(UnitBatches.Strings([null]));

        Assert.True(_converter.IsNull(0));
        Assert.Same(DBNull.Value, _converter.GetValue(0));
        Assert.Null(_converter.Get<string>(0));
    }

    [SnowflakeFact(RetriesCount = RetriesCount.NoRetries)]
    public void TypedGettersOtherThanStringReportNullAsAConversionError()
    {
        _converter.Bind(UnitBatches.Strings([null]));

        ConversionAssert.FailsForNull(() => _converter.Get<char>(0), "TEXT", typeof(char));
        ConversionAssert.FailsForNull(() => _converter.GetChars(0, 0, null, 0, 0), "TEXT", typeof(char[]));
        ConversionAssert.FailsForNull(() => _converter.Get<long>(0), "TEXT", typeof(long));
        ConversionAssert.FailsForNull(() => _converter.Get<decimal>(0), "TEXT", typeof(decimal));
        ConversionAssert.FailsForNull(() => _converter.Get<double>(0), "TEXT", typeof(double));
        ConversionAssert.FailsForNull(() => _converter.Get<Guid>(0), "TEXT", typeof(Guid));
    }

    [SnowflakeFact(RetriesCount = RetriesCount.NoRetries)]
    public void CharIsTheSingleCharacterAndAnyOtherLengthIsAConversionError()
    {
        _converter.Bind(UnitBatches.Strings("x", "xyz", string.Empty));

        Assert.Equal('x', _converter.Get<char>(0));
        ConversionAssert.Fails(() => _converter.Get<char>(1), "TEXT", typeof(char));
        ConversionAssert.Fails(() => _converter.Get<char>(2), "TEXT", typeof(char));
    }

    [SnowflakeFact(RetriesCount = RetriesCount.NoRetries)]
    public void CharsRejectsANegativeOffsetAndReturnsZeroForOffsetsBeyondTheText()
    {
        _converter.Bind(UnitBatches.Strings("abc"));
        var buffer = new char[2];

        Assert.Throws<ArgumentOutOfRangeException>(() => _converter.GetChars(0, -1, buffer, 0, 2));
        Assert.Equal(0, _converter.GetChars(0, (long)int.MaxValue + 1, buffer, 0, 2));
        Assert.Equal(0, _converter.GetChars(0, long.MaxValue, buffer, 0, 2));
    }

    [SnowflakeFact(RetriesCount = RetriesCount.NoRetries)]
    public void UnbindDropsTheBatch()
    {
        _converter.Bind(UnitBatches.Strings("abc"));
        _converter.Unbind();

        Assert.True(_converter.IsNull(0));
    }

    [SnowflakeFact(RetriesCount = RetriesCount.NoRetries)]
    public void CharsReportTheLengthForANullBufferAndCopyFromTheOffset()
    {
        _converter.Bind(UnitBatches.Strings("abcdef"));
        var buffer = new char[4];

        Assert.Equal(6, _converter.GetChars(0, 0, null, 0, 0));
        Assert.Equal(3, _converter.GetChars(0, 2, buffer, 1, 3));
        Assert.Equal(['\0', 'c', 'd', 'e'], buffer);
        Assert.Equal(2, _converter.GetChars(0, 4, buffer, 0, 4));
        Assert.Equal(['e', 'f', 'd', 'e'], buffer);
        Assert.Equal(0, _converter.GetChars(0, 6, buffer, 0, 4));
    }

    [SnowflakeFact(RetriesCount = RetriesCount.NoRetries)]
    public void IntegerGettersParseIntegralText()
    {
        _converter.Bind(UnitBatches.Strings("-42", "255"));

        Assert.Equal(-42L, _converter.Get<long>(0));
        Assert.Equal(-42, _converter.Get<int>(0));
        Assert.Equal((short)-42, _converter.Get<short>(0));
        Assert.Equal((byte)255, _converter.Get<byte>(1));
    }

    [SnowflakeFact(RetriesCount = RetriesCount.NoRetries)]
    public void IntegerGettersReportNonIntegralTextAndOverflowAsConversionErrors()
    {
        _converter.Bind(UnitBatches.Strings("1.5", "abc", "3000000000", "-1"));

        ConversionAssert.Fails(() => _converter.Get<long>(0), "TEXT", typeof(long));
        ConversionAssert.Fails(() => _converter.Get<int>(1), "TEXT", typeof(int));
        ConversionAssert.Fails(() => _converter.Get<int>(2), "TEXT", typeof(int));
        ConversionAssert.Fails(() => _converter.Get<byte>(3), "TEXT", typeof(byte));
    }

    [SnowflakeFact(RetriesCount = RetriesCount.NoRetries)]
    public void FractionalGettersParseWithTheInvariantCultureWhateverTheCurrentCulture()
    {
        var original = CultureInfo.CurrentCulture;
        CultureInfo.CurrentCulture = CultureInfo.GetCultureInfo("de-DE");
        try
        {
            _converter.Bind(UnitBatches.Strings("1.25", "1e3"));

            Assert.Equal(1.25m, _converter.Get<decimal>(0));
            Assert.Equal(1.25d, _converter.Get<double>(0));
            Assert.Equal(1.25f, _converter.Get<float>(0));
            Assert.Equal(1000m, _converter.Get<decimal>(1));
        }
        finally
        {
            CultureInfo.CurrentCulture = original;
        }
    }

    [SnowflakeFact(RetriesCount = RetriesCount.NoRetries)]
    public void GuidParsesAndReportsMalformedTextAsAConversionError()
    {
        var guid = Guid.Parse("4b7e2c0e-8f3a-4d55-9b1e-2a6c7d8e9f01");
        _converter.Bind(UnitBatches.Strings(guid.ToString(), "not-a-guid"));

        Assert.Equal(guid, _converter.Get<Guid>(0));
        ConversionAssert.Fails(() => _converter.Get<Guid>(1), "TEXT", typeof(Guid));
    }

    [SnowflakeFact(RetriesCount = RetriesCount.NoRetries)]
    public void ConversionsTextDoesNotSupportAreConversionErrors()
    {
        _converter.Bind(UnitBatches.Strings("true"));

        ConversionAssert.Fails(() => _converter.Get<bool>(0), "TEXT", typeof(bool));
        ConversionAssert.Fails(() => _converter.GetBytes(0, 0, null, 0, 0), "TEXT", typeof(byte[]));
        ConversionAssert.Fails(() => _converter.Get<DateTime>(0), "TEXT", typeof(DateTime));
    }

    [SnowflakeFact(RetriesCount = RetriesCount.NoRetries)]
    public void BindingAnArrowTypeSfCoreDoesNotSendForTextIsAnInternalError()
    {
        var exception = Assert.Throws<SnowflakeDriverException>(() => _converter.Bind(UnitBatches.Int64(1)));

        Assert.Equal(SFError.INTERNAL_ERROR, exception.Error);
    }
}
