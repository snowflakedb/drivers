package net.snowflake.client.api.driver;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertFalse;
import static org.junit.jupiter.api.Assertions.assertInstanceOf;
import static org.junit.jupiter.api.Assertions.assertNotNull;
import static org.junit.jupiter.api.Assertions.assertNull;
import static org.junit.jupiter.api.Assertions.assertThrows;
import static org.junit.jupiter.api.Assertions.assertTrue;

import java.sql.Driver;
import java.sql.DriverManager;
import java.sql.DriverPropertyInfo;
import java.sql.SQLException;
import java.util.Properties;
import java.util.stream.Stream;
import net.snowflake.client.api.exception.SnowflakeSQLException;
import org.junit.jupiter.api.Test;
import org.junit.jupiter.params.ParameterizedTest;
import org.junit.jupiter.params.provider.MethodSource;

/** Basic tests for the Snowflake JDBC Driver */
public class SnowflakeDriverTest {

  @Test
  public void testDriverRegistration() throws SQLException {
    // Test that the driver is properly registered
    Driver driver = DriverManager.getDriver("jdbc:snowflake://test.snowflakecomputing.com");
    assertNotNull(driver, "Driver should be registered");
    assertInstanceOf(SnowflakeDriver.class, driver, "Driver should be instance of SnowflakeDriver");
  }

  @ParameterizedTest
  @MethodSource("validUrls")
  public void testAcceptsValidURL(String url) throws SQLException {
    SnowflakeDriver driver = new SnowflakeDriver();
    assertTrue(driver.acceptsURL(url), "Expected valid URL to be accepted: " + url);
  }

  @ParameterizedTest
  @MethodSource("nonSnowflakeUrls")
  public void testRejectsNonSnowflakeURL(String url) throws SQLException {
    SnowflakeDriver driver = new SnowflakeDriver();
    assertFalse(driver.acceptsURL(url), "Expected non-Snowflake URL to be rejected: " + url);
  }

  @Test
  public void testDriverVersion() {
    SnowflakeDriver driver = new SnowflakeDriver();
    assertTrue(driver.getMajorVersion() > 0, "Major version should be gt 0");
    assertTrue(driver.getMinorVersion() >= 0, "Minor version should be gte 0");
    assertFalse(driver.jdbcCompliant(), "Driver should not claim JDBC compliance");
  }

  @Test
  public void testGetParentLogger() {
    SnowflakeDriver driver = new SnowflakeDriver();
    assertNull(driver.getParentLogger(), "Expected getParentLogger to be null");
  }

  @Test
  public void testGetPropertyInfo() throws SQLException {
    SnowflakeDriver driver = new SnowflakeDriver();
    DriverPropertyInfo[] props =
        driver.getPropertyInfo("jdbc:snowflake://test.snowflakecomputing.com", new Properties());
    assertNotNull(props, "Property info should not be null");
  }

  @Test
  public void testConnectReturnsNullForNonSnowflakePrefix() throws SQLException {
    SnowflakeDriver driver = new SnowflakeDriver();
    assertNull(driver.connect("jdbc:nonsnowflake://host:3306/database", new Properties()));
  }

  @Test
  public void testConnectRejectsMalformedSnowflakeUrl() {
    SnowflakeDriver driver = new SnowflakeDriver();
    SnowflakeSQLException ex =
        assertThrows(
            SnowflakeSQLException.class,
            () ->
                driver.connect(
                    "jdbc:snowflake://abc-test.com/?private_key_file=C:\\temp\\k.p8",
                    new Properties()));
    assertEquals("Connection string is invalid. Unable to parse.", ex.getMessage());
  }

  @Test
  public void testConnectRejectsInvalidPathInSnowflakeUrl() {
    SnowflakeDriver driver = new SnowflakeDriver();
    SnowflakeSQLException ex =
        assertThrows(
            SnowflakeSQLException.class,
            () -> driver.connect("jdbc:snowflake://localhost:8080/a=b", new Properties()));
    assertEquals("Connection string is invalid. Unable to parse.", ex.getMessage());
  }

  private static Stream<String> validUrls() {
    return Stream.of(
        "jdbc:snowflake://testaccount.snowflakecomputing.com",
        "jdbc:snowflake://testaccount.snowflakecomputing.com:443?db=TEST_DB&schema=PUBLIC",
        "jdbc:snowflake://http://testaccount.localhost?prop1=value1",
        "jdbc:snowflake://testaccount.com:8080?proxyHost=%3d%2f&proxyPort=777&ssl=off",
        "jdbc:snowflake://snowflake.reg-7387_2.local:8082",
        "jdbc:snowflake://globalaccount-12345.global.snowflakecomputing.com");
  }

  private static Stream<String> nonSnowflakeUrls() {
    return Stream.of(
        "jdbc:", "jdbc:snowflak://localhost:8080", "jdbc:nonsnowflake://localhost:3306/test");
  }
}
