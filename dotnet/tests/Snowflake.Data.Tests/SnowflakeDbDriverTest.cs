namespace Snowflake.Data.Tests;

// FIPS_TLS_STATUS_WITHHELD: withheld from the shipped contract while the status shape is still under consideration.
// public sealed class SnowflakeDbDriverTest
// {
//     [Trait("Category", "Interop")]
//     [SnowflakeFact]
//     public void GetTlsStatus_ReportsNativeProviderAndBuildFlagsWithoutConnection()
//     {
//         var expectedMarker = Environment.GetEnvironmentVariable("SF_CORE_EXPECT_FIPS");
//         var expectedFips = expectedMarker == "1" ||
//                               string.Equals(expectedMarker, "true", StringComparison.OrdinalIgnoreCase);
//
//         var status = new SnowflakeDbDriver().GetTlsStatus();
//
//         Assert.Equal(expectedFips, status.TlsProviderIsFips);
//         Assert.Equal(expectedFips, status.FipsBuildEnabled);
//     }
// }
