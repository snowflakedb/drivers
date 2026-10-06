package net.snowflake.client.internal.unicore;

import static org.junit.jupiter.api.Assertions.assertEquals;

import java.io.File;
import java.sql.SQLException;
import net.snowflake.client.api.driver.SnowflakeDriver;
import net.snowflake.client.api.driver.TlsStatus;
import org.junit.jupiter.api.Assumptions;
import org.junit.jupiter.api.Test;

/** Verifies the public no-connection API against the linked native bridge. */
public class SnowflakeDriverTlsStatusTest {
  @Test
  public void shouldReportLinkedTlsProviderAndBuildFeatureWithoutConnection() throws SQLException {
    String corePath = System.getenv("CORE_PATH");
    Assumptions.assumeTrue(
        corePath != null && new File(corePath).exists(),
        "jdbc_bridge native library not built; skipping JNI TLS status probe");

    boolean expectedFipsBuild = Boolean.parseBoolean(System.getenv("JDBC_EXPECT_FIPS_TLS_BUILD"));
    TlsStatus status = new SnowflakeDriver().getTlsStatus();

    assertEquals(expectedFipsBuild, status.isFipsTlsBuildEnabled());
    assertEquals(expectedFipsBuild, status.isTlsProviderFips());
  }
}
