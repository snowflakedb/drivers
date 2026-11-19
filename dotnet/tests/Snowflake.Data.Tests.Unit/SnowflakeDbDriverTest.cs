namespace Snowflake.Data.Tests;

public sealed class SnowflakeDbDriverTest
{
    [Trait("Category", "Interop")]
    [SnowflakeFact]
    public void GetFipsStatus_ReportsWhetherThisProcessIsTheFipsDriverWithoutConnection()
    {
        var expectedMarker = Environment.GetEnvironmentVariable("SF_CORE_EXPECT_FIPS");
        var expectedFips = expectedMarker == "1" ||
                              string.Equals(expectedMarker, "true", StringComparison.OrdinalIgnoreCase);

        var status = new SnowflakeDbDriver().GetFipsStatus();

        Assert.Equal(expectedFips, status.IsFips);
    }
}
