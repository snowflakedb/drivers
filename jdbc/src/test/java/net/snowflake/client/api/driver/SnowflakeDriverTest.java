package net.snowflake.client.api.driver;

import static org.junit.jupiter.api.Assertions.assertFalse;
import static org.junit.jupiter.api.Assertions.assertInstanceOf;
import static org.junit.jupiter.api.Assertions.assertNotNull;
import static org.junit.jupiter.api.Assertions.assertNull;
import static org.junit.jupiter.api.Assertions.assertTrue;

import java.sql.Driver;
import java.sql.DriverManager;
import java.sql.DriverPropertyInfo;
import java.sql.SQLException;
import java.util.Properties;
import org.junit.jupiter.api.Test;

/** Basic tests for the Snowflake JDBC Driver */
public class SnowflakeDriverTest {

  @Test
  public void testDriverRegistration() throws SQLException {
    // Test that the driver is properly registered
    Driver driver = DriverManager.getDriver("jdbc:snowflake://test.snowflakecomputing.com");
    assertNotNull(driver, "Driver should be registered");
    assertInstanceOf(SnowflakeDriver.class, driver, "Driver should be instance of SnowflakeDriver");
  }

  @Test
  public void testAcceptsURL() throws SQLException {
    SnowflakeDriver driver = new SnowflakeDriver();

    // Test valid URLs
    assertTrue(
        driver.acceptsURL("jdbc:snowflake://test.snowflakecomputing.com"),
        "Should accept snowflake URL");
    assertTrue(
        driver.acceptsURL("jdbc:snowflake://test.snowflakecomputing.com?db=test"),
        "Should accept snowflake URL with parameters");

    // Test invalid URLs
    assertFalse(driver.acceptsURL(null), "Should not accept null URL");
    assertFalse(
        driver.acceptsURL("jdbc:mysql://localhost:3306/test"),
        "Should not accept non-snowflake URL");
    assertFalse(driver.acceptsURL("not-a-url"), "Should not accept malformed URL");
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
}
