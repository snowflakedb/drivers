package net.snowflake.client.internal.unicore;

// FIPS_TLS_STATUS_WITHHELD: withheld from the shipped contract while the status shape is still
// under consideration.
// public class SnowflakeDriverTlsStatusTest {
//   @Test
//   public void shouldReportLinkedTlsProviderAndBuildFeatureWithoutConnection() throws SQLException
// {
//     String corePath = System.getenv("CORE_PATH");
//     Assumptions.assumeTrue(
//         corePath != null && new File(corePath).exists(),
//         "jdbc_bridge native library not built; skipping JNI TLS status probe");
//
//     boolean expectedFipsBuild =
// Boolean.parseBoolean(System.getenv("JDBC_EXPECT_FIPS_BUILD"));
//     TlsStatus status = new SnowflakeDriver().getTlsStatus();
//
//     assertEquals(expectedFipsBuild, status.isFipsBuildEnabled());
//     assertEquals(expectedFipsBuild, status.isTlsProviderFips());
//   }
// }
