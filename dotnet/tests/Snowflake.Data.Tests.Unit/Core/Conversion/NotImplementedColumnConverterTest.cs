using Snowflake.Data.Core.DataConversion.ResultSetConversion;

namespace Snowflake.Data.Tests.Core.Conversion;

[Trait("Category", "Unit")]
public sealed class NotImplementedColumnConverterTest
{
    private readonly NotImplementedColumnConverter _converter = new("DATE");

    [SnowflakeFact(RetriesCount = RetriesCount.NoRetries)]
    public void NullDetectionWorksForAnyArrowType()
    {
        _converter.Bind(UnitBatches.Strings("2024-01-15", null));

        Assert.False(_converter.IsNull(0));
        Assert.True(_converter.IsNull(1));
    }

    [SnowflakeFact(RetriesCount = RetriesCount.NoRetries)]
    public void NullDetectionWorksForOtherArrowTypesAndTreatsAnUnboundColumnAsNull()
    {
        Assert.True(_converter.IsNull(0));

        _converter.Bind(UnitBatches.Int64(1, null));
        Assert.False(_converter.IsNull(0));
        Assert.True(_converter.IsNull(1));

        _converter.Unbind();
        Assert.True(_converter.IsNull(0));
    }

    [SnowflakeFact(RetriesCount = RetriesCount.NoRetries)]
    public void FieldTypeValueAndEveryGetterAreNotImplemented()
    {
        _converter.Bind(UnitBatches.Strings("2024-01-15", null));

        Assert.Throws<NotImplementedException>(() => _converter.FieldType);
        Assert.Throws<NotImplementedException>(() => _converter.GetValue(0));
        Assert.Throws<NotImplementedException>(() => _converter.GetValue(1));
        Assert.Throws<NotImplementedException>(() => _converter.Get<string>(0));
        Assert.Throws<NotImplementedException>(() => _converter.Get<long>(0));
        Assert.Throws<NotImplementedException>(() => _converter.Get<DateTime>(0));
        Assert.Throws<NotImplementedException>(() => _converter.GetBytes(0, 0, null, 0, 0));
        Assert.Throws<NotImplementedException>(() => _converter.GetChars(0, 0, null, 0, 0));
    }
}
