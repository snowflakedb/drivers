using Snowflake.Data.Core;
using Snowflake.Data.Core.DataConversion.ResultSetConversion;
using Snowflake.Data.Proto;

namespace Snowflake.Data.Tests.Unit.Core.Conversion;

[Trait("Category", "Unit")]
public sealed class SnowflakeTypesTest
{
    [SnowflakeTheory(RetriesCount = RetriesCount.NoRetries)]
    [InlineData("fixed", SFDataType.FIXED)]
    [InlineData("FIXED", SFDataType.FIXED)]
    [InlineData("Timestamp_Ltz", SFDataType.TIMESTAMP_LTZ)]
    public void TypeNamesAreMatchedIgnoringCase(string name, SFDataType expected)
    {
        Assert.True(SnowflakeTypes.TryParse(name, out var type));
        Assert.Equal(expected, type);
    }

    [SnowflakeTheory(RetriesCount = RetriesCount.NoRetries)]
    [InlineData("")]
    [InlineData("none")]
    [InlineData("geography")]
    [InlineData("3")]
    [InlineData("-1")]
    [InlineData("fixed,real")]
    [InlineData("fixed ")]
    [InlineData(" fixed")]
    public void ANameThatIsNotATypeIsNotParsed(string name)
    {
        Assert.False(SnowflakeTypes.TryParse(name, out _));
    }

    [SnowflakeFact(RetriesCount = RetriesCount.NoRetries)]
    public void EveryTypeExceptNoneHasAClrType()
    {
        foreach (var type in EnumHelper.GetValues<SFDataType>().Where(type => type != SFDataType.None))
            Assert.NotNull(SnowflakeTypes.CtsType(type, scale: 0));
    }

    [SnowflakeFact(RetriesCount = RetriesCount.NoRetries)]
    public void NoneHasNoClrType()
    {
        Assert.Throws<ArgumentOutOfRangeException>(() => SnowflakeTypes.CtsType(SFDataType.None, scale: 0));
    }

    [SnowflakeTheory(RetriesCount = RetriesCount.NoRetries)]
    [InlineData(0L, typeof(long))]
    [InlineData(1L, typeof(decimal))]
    [InlineData(2L, typeof(decimal))]
    public void FixedIsLongOnlyWithoutAScale(long scale, Type expected)
    {
        Assert.Equal(expected, SnowflakeTypes.CtsType(SFDataType.FIXED, scale));
    }

    [SnowflakeTheory(RetriesCount = RetriesCount.NoRetries)]
    [InlineData("text", 0L)]
    [InlineData("fixed", 0L)]
    public void ARealConverterReportsTheClrTypeOfTheSharedMapping(string name, long scale)
    {
        var column = new ColumnMetadata { Name = "C1", Type = name, Scale = scale };
        var converter = ColumnConverterFactory.Instance.Create(column);

        Assert.True(SnowflakeTypes.TryParse(name, out var type));
        Assert.Equal(SnowflakeTypes.CtsType(type, scale), converter.FieldType);
    }
}
