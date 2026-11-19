using Snowflake.Data.Core;
using Snowflake.Data.Core.Errors;
using Snowflake.Data.Tests.Discovery;

namespace Snowflake.Data.Tests.Core;

[Trait("Category", "Unit")]
public sealed class QueryIdTest
{
    [SnowflakeTheory(RetriesCount = RetriesCount.NoRetries)]
    [InlineData("01bf4a3e-0001-2c4f-0000-00a1b2c3d4e5")]
    [InlineData("01BF4A3E-0001-2C4F-0000-00A1B2C3D4E5")]
    [InlineData("00000000-0000-0000-0000-000000000000")]
    [InlineData(" 01bf4a3e-0001-2c4f-0000-00a1b2c3d4e5 ")]
    public void ParseKeepsAcceptedValueUnchanged(string value)
    {
        var queryId = QueryId.Parse(value);

        Assert.Equal(value, queryId.Value);
        Assert.Equal(value, queryId.ToString());
    }

    [SnowflakeTheory(RetriesCount = RetriesCount.NoRetries)]
    [InlineData(null)]
    [InlineData("")]
    [InlineData("   ")]
    [InlineData("not-a-uuid")]
    [InlineData("01bf4a3e00012c4f000000a1b2c3d4e5")]
    [InlineData("{01bf4a3e-0001-2c4f-0000-00a1b2c3d4e5}")]
    [InlineData("(01bf4a3e-0001-2c4f-0000-00a1b2c3d4e5)")]
    [InlineData("01bf4a3e-0001-2c4f-0000-00a1b2c3d4e")]
    [InlineData("01bf4a3e-0001-2c4f-0000-00a1b2c3d4e5a")]
    [InlineData("01bf4a3g-0001-2c4f-0000-00a1b2c3d4e5")]
    public void ParseRejectsValueNotInHyphenatedUuidForm(string? value)
    {
        var exception = Assert.Throws<SnowflakeDriverException>(() => QueryId.Parse(value));

        Assert.Equal(SFError.INVALID_QUERY_ID_FORMAT, exception.Error);
        Assert.Null(exception.QueryId);
    }

    [SnowflakeFact(RetriesCount = RetriesCount.NoRetries)]
    public void IdsWithTheSameValueAreEqual()
    {
        const string value = "01bf4a3e-0001-2c4f-0000-00a1b2c3d4e5";

        Assert.Equal(QueryId.Parse(value), QueryId.Parse(value));
        Assert.NotEqual(QueryId.Parse(value), QueryId.Parse(value.ToUpperInvariant()));
    }
}
