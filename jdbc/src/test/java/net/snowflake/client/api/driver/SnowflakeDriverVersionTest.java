package net.snowflake.client.api.driver;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertFalse;
import static org.junit.jupiter.api.Assertions.assertNotNull;

import org.junit.jupiter.api.Test;

/**
 * Excluded from the old-driver reference test because these assertions reference fields and methods
 * (MAJOR_VERSION, JDBC_SPEC_MAJOR, parseVersionComponent) that do not exist on the legacy
 * snowflake-jdbc artifact.
 */
public class SnowflakeDriverVersionTest {

  @Test
  public void driverNameIsExpectedLiteral() {
    assertEquals("Snowflake JDBC Driver", SnowflakeDriver.DRIVER_NAME);
  }

  @Test
  public void driverVersionIsPopulatedFromGeneratedConstant() {
    assertNotNull(SnowflakeDriver.DRIVER_VERSION);
    assertFalse(SnowflakeDriver.DRIVER_VERSION.trim().isEmpty());
  }

  @Test
  public void majorAndMinorVersionAgreeWithDriverVersion() {
    assertEquals(
        SnowflakeDriver.MAJOR_VERSION,
        SnowflakeDriver.parseVersionComponent(SnowflakeDriver.DRIVER_VERSION, 0));
    assertEquals(
        SnowflakeDriver.MINOR_VERSION,
        SnowflakeDriver.parseVersionComponent(SnowflakeDriver.DRIVER_VERSION, 1));
  }

  @Test
  public void instanceMethodsAgreeWithConstants() {
    SnowflakeDriver driver = new SnowflakeDriver();
    assertEquals(SnowflakeDriver.MAJOR_VERSION, driver.getMajorVersion());
    assertEquals(SnowflakeDriver.MINOR_VERSION, driver.getMinorVersion());
    assertEquals(SnowflakeDriver.DRIVER_VERSION, SnowflakeDriver.getDriverVersion());
  }

  @Test
  public void jdbcSpecVersionIsFourDotTwo() {
    assertEquals("4.2", SnowflakeDriver.JDBC_SPEC_VERSION);
    assertEquals(4, SnowflakeDriver.JDBC_SPEC_MAJOR);
    assertEquals(2, SnowflakeDriver.JDBC_SPEC_MINOR);
  }

  @Test
  public void parseVersionComponentHandlesStandardSemver() {
    assertEquals(1, SnowflakeDriver.parseVersionComponent("1.2.3", 0));
    assertEquals(2, SnowflakeDriver.parseVersionComponent("1.2.3", 1));
    assertEquals(3, SnowflakeDriver.parseVersionComponent("1.2.3", 2));
  }

  @Test
  public void parseVersionComponentReturnsZeroForOutOfBoundsIndex() {
    assertEquals(1, SnowflakeDriver.parseVersionComponent("1", 0));
    assertEquals(0, SnowflakeDriver.parseVersionComponent("1", 1));
    assertEquals(0, SnowflakeDriver.parseVersionComponent("1.2", 5));
    assertEquals(0, SnowflakeDriver.parseVersionComponent("1.2.3", -1));
  }

  @Test
  public void parseVersionComponentReturnsZeroForInvalidInput() {
    assertEquals(0, SnowflakeDriver.parseVersionComponent("", 0));
    assertEquals(0, SnowflakeDriver.parseVersionComponent(null, 0));
    assertEquals(0, SnowflakeDriver.parseVersionComponent("1.x", 1));
    assertEquals(1, SnowflakeDriver.parseVersionComponent("1.x", 0));
  }

  @Test
  public void parseVersionComponentStripsTrailingNonDigits() {
    assertEquals(4, SnowflakeDriver.parseVersionComponent("4.0.0-SNAPSHOT", 0));
    assertEquals(0, SnowflakeDriver.parseVersionComponent("4.0.0-SNAPSHOT", 2));
  }
}
