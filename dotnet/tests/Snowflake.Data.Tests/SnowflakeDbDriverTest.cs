namespace Snowflake.Data.Tests;

public sealed class SnowflakeDbDriverTest
{
    [Trait("Category", "Interop")]
    [SnowflakeFact]
    public void GetTlsStatus_ReportsNativeProviderAndBuildFlagsWithoutConnection()
    {
        var expectedMarker = Environment.GetEnvironmentVariable("SF_CORE_EXPECT_FIPS");
        var expectedFips = expectedMarker == "1" ||
                              string.Equals(expectedMarker, "true", StringComparison.OrdinalIgnoreCase);

        var status = new SnowflakeDbDriver().GetTlsStatus();

        Assert.Equal(expectedFips, status.TlsProviderIsFips);
        Assert.Equal(expectedFips, status.FipsBuildEnabled);
    }
}
