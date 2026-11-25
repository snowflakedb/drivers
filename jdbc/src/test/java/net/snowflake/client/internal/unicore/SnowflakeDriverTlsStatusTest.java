package net.snowflake.client.internal.unicore;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertNotNull;
import static org.junit.jupiter.api.Assertions.assertNull;

import java.io.File;
import java.io.IOException;
import java.net.JarURLConnection;
import java.net.URISyntaxException;
import java.net.URL;
import java.nio.file.Path;
import java.nio.file.Paths;
import java.sql.SQLException;
import net.snowflake.client.api.driver.FipsStatus;
import net.snowflake.client.api.driver.SnowflakeDriver;
import org.junit.jupiter.api.Assumptions;
import org.junit.jupiter.api.Test;

public class SnowflakeDriverTlsStatusTest {
  @Test
  public void shouldReportWhetherThisProcessIsTheFipsDriverWithoutConnection()
      throws SQLException, IOException, URISyntaxException {
    String corePath = System.getenv("CORE_PATH");
    String expectedJar = System.getenv("JDBC_EXPECT_NATIVE_JAR");
    if (expectedJar != null && !expectedJar.isEmpty()) {
      assertNull(corePath, "CORE_PATH would bypass the native bundled in the jar");
      assertBundledNativeComesFrom(Paths.get(expectedJar));
    } else {
      Assumptions.assumeTrue(
          corePath != null && new File(corePath).exists(),
          "jdbc_bridge native library not built; skipping JNI FIPS status probe");
    }

    boolean expectedFips = Boolean.parseBoolean(System.getenv("JDBC_EXPECT_FIPS_BUILD"));
    FipsStatus status = new SnowflakeDriver().getFipsStatus();

    assertEquals(expectedFips, status.isFips());
  }

  private static void assertBundledNativeComesFrom(Path expectedJar)
      throws IOException, URISyntaxException {
    URL bundled = NativeLibraryLoader.bundledNativeUrl();
    assertNotNull(bundled, "the classpath has no bundled native library");
    JarURLConnection connection = (JarURLConnection) bundled.openConnection();
    assertEquals(
        expectedJar.toRealPath(),
        Paths.get(connection.getJarFileURL().toURI()).toRealPath(),
        "the bundled native does not come from the jar named by -PnativeJar");
  }
}
