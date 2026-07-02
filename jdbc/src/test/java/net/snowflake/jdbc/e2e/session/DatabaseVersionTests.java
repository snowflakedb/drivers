package net.snowflake.jdbc.e2e.session;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertNotNull;
import static org.junit.jupiter.api.Assertions.assertTrue;

import java.sql.Connection;
import java.sql.DatabaseMetaData;
import java.sql.ResultSet;
import java.sql.Statement;
import net.snowflake.jdbc.utils.SnowflakeIntegrationTestBase;
import org.junit.jupiter.api.Test;

public class DatabaseVersionTests extends SnowflakeIntegrationTestBase {

  @Test
  public void getDatabaseProductVersionMatchesSemverRegex() throws Exception {
    // Given Snowflake client is logged in
    Connection conn = getDefaultConnection();

    // When DatabaseMetaData.getDatabaseProductVersion() is called
    String version = conn.getMetaData().getDatabaseProductVersion();

    // Then the result is a stripped semver-style string with no build suffix
    assertNotNull(version, "getDatabaseProductVersion should not return null");
    assertTrue(
        version.matches("^\\d+\\.\\d+\\.\\d+$"), "Expected stripped SEMVER, got: " + version);
  }

  @Test
  public void getDatabaseProductVersionMatchesRawCurrentVersionQuery() throws Exception {
    // Given Snowflake client is logged in
    Connection conn = getDefaultConnection();

    // When the metadata version and the raw SELECT CURRENT_VERSION() result are both fetched
    String fromMetadata = conn.getMetaData().getDatabaseProductVersion();
    String fromQuery;
    try (Statement stmt = conn.createStatement();
        ResultSet rs = stmt.executeQuery("SELECT CURRENT_VERSION()")) {
      assertTrue(rs.next(), "Expected one row from SELECT CURRENT_VERSION()");
      fromQuery = rs.getString(1).split(" ")[0];
    }

    // Then both should return the same stripped version string
    assertEquals(fromQuery, fromMetadata);
  }

  @Test
  public void getDatabaseMajorAndMinorMatchVersionString() throws Exception {
    // Given Snowflake client is logged in
    Connection conn = getDefaultConnection();
    DatabaseMetaData metadata = conn.getMetaData();

    // When the version string and the major/minor integers are read from DatabaseMetaData
    String version = metadata.getDatabaseProductVersion();
    int major = metadata.getDatabaseMajorVersion();
    int minor = metadata.getDatabaseMinorVersion();

    // Then the integers match the dot-separated components of the version string
    String[] parts = version.split("\\.");
    assertEquals(Integer.parseInt(parts[0]), major);
    assertEquals(Integer.parseInt(parts[1]), minor);
  }

  @Test
  public void getDatabaseProductVersionIsStableAcrossCalls() throws Exception {
    // Given Snowflake client is logged in
    Connection conn = getDefaultConnection();
    DatabaseMetaData metadata = conn.getMetaData();

    // When DatabaseMetaData.getDatabaseProductVersion() is called twice
    // (value-equality only; this asserts the cache returns consistently, not that the
    //  cache prevents a second query - CURRENT_VERSION() is server-invariant within a session,
    //  so an uncached implementation would also pass this)
    String first = metadata.getDatabaseProductVersion();
    String second = metadata.getDatabaseProductVersion();

    // Then both calls return the same string
    assertEquals(first, second);
  }
}
