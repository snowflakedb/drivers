using Snowflake.Data.Core.DataConversion.ResultSetConversion;
using Snowflake.Data.Proto;

namespace Snowflake.Data.Tests.Core.Conversion;

[Trait("Category", "Unit")]
public sealed class ColumnConverterFactoryTest
{
    [SnowflakeTheory(RetriesCount = RetriesCount.NoRetries)]
    [InlineData("text")]
    [InlineData("TEXT")]
    public void TextColumnsGetTheTextConverter(string type)
    {
        var converter = ColumnConverterFactory.Instance.Create(UnitBatches.Column("C1", type));

        Assert.IsType<TextColumnConverter>(converter);
    }

    [SnowflakeTheory(RetriesCount = RetriesCount.NoRetries)]
    [InlineData("fixed")]
    [InlineData("FIXED")]
    public void FixedColumnsWithScaleZeroGetTheFixedConverter(string type)
    {
        var converter = ColumnConverterFactory.Instance.Create(new ColumnMetadata { Name = "C1", Type = type, Scale = 0 });

        Assert.IsType<FixedColumnConverter>(converter);
    }

    [SnowflakeFact(RetriesCount = RetriesCount.NoRetries)]
    public void FixedColumnsWithAScaleGetTheStub()
    {
        var converter = ColumnConverterFactory.Instance.Create(UnitBatches.FixedColumn(scale: 2));

        Assert.IsType<NotImplementedColumnConverter>(converter);
    }

    [SnowflakeTheory(RetriesCount = RetriesCount.NoRetries)]
    [InlineData("real")]
    [InlineData("boolean")]
    [InlineData("date")]
    [InlineData("timestamp_ltz")]
    [InlineData("variant")]
    [InlineData("geography")]
    [InlineData("a_type_the_driver_has_never_seen")]
    public void EveryOtherTypeGetsTheStub(string type)
    {
        var converter = ColumnConverterFactory.Instance.Create(UnitBatches.Column("C1", type));

        Assert.IsType<NotImplementedColumnConverter>(converter);
    }

    [SnowflakeFact(RetriesCount = RetriesCount.NoRetries)]
    public void EachCallCreatesItsOwnConverter()
    {
        var column = UnitBatches.TextColumn();

        Assert.NotSame(ColumnConverterFactory.Instance.Create(column), ColumnConverterFactory.Instance.Create(column));
    }
}
