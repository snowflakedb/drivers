using Snowflake.Data.Compatibility;

namespace Snowflake.Data.Tests.Compatibility;

[Trait("Category", "Unit")]
public sealed class PtrHelperTest
{
    [SnowflakeFact(RetriesCount = RetriesCount.NoRetries)]
    public void TestZeroIsNullAddress()
    {
        Assert.Equal((object)IntPtr.Zero, (object)PtrHelper.Zero);
    }
}
