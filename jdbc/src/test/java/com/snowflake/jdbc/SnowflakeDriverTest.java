package com.snowflake.jdbc;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertFalse;
import static org.junit.jupiter.api.Assertions.assertNotNull;
import static org.junit.jupiter.api.Assertions.assertThrows;
import static org.junit.jupiter.api.Assertions.assertTrue;

import java.sql.Driver;
import java.sql.DriverManager;
import java.sql.DriverPropertyInfo;
import java.sql.SQLException;
import java.sql.SQLFeatureNotSupportedException;
import org.junit.jupiter.api.Test;

/** Basic tests for the Snowflake JDBC Driver */
public class SnowflakeDriverTest {

  @Test
  public void testDriverRegistration() throws SQLException {
    // Test that the driver is properly registered
    Driver driver = DriverManager.getDriver("jdbc:snowflake://test.snowflakecomputing.com");
    assertNotNull(driver, "Driver should be registered");
    assertTrue(driver instanceof SnowflakeDriver, "Driver should be instance of SnowflakeDriver");
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
    assertEquals(0, driver.getMajorVersion(), "Major version should be 0");
    assertEquals(1, driver.getMinorVersion(), "Minor version should be 1");
    assertFalse(driver.jdbcCompliant(), "Driver should not claim JDBC compliance in stub");
  }

  @Test
  public void testGetParentLogger() throws SQLException {
    SnowflakeDriver driver = new SnowflakeDriver();
    assertThrows(
        SQLFeatureNotSupportedException.class,
        driver::getParentLogger,
        "Expected getParentLogger to throw");
  }

  @Test
  public void testGetPropertyInfo() throws SQLException {
    SnowflakeDriver driver = new SnowflakeDriver();
    DriverPropertyInfo[] props =
        driver.getPropertyInfo("jdbc:snowflake://test.snowflakecomputing.com", null);
    assertNotNull(props, "Property info should not be null");
    assertEquals(0, props.length, "Should return empty array in stub");
  }
}
