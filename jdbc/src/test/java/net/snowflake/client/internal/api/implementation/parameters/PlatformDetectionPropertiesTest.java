package net.snowflake.client.internal.api.implementation.parameters;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertNull;

import java.util.Properties;
import org.junit.jupiter.api.AfterEach;
import org.junit.jupiter.api.BeforeEach;
import org.junit.jupiter.api.Test;

class PlatformDetectionPropertiesTest {

  @BeforeEach
  @AfterEach
  void clearPlatformDetectionSystemProperties() {
    System.clearProperty(PlatformDetectionProperties.TIMEOUT_SYSTEM_PROPERTY);
    System.clearProperty(PlatformDetectionProperties.DISABLE_SYSTEM_PROPERTY);
  }

  @Test
  void shouldLeaveConnectionPropertiesInPlace() {
    Properties properties = new Properties();
    properties.setProperty("platformDetectionTimeoutMs", "11");
    properties.setProperty("disablePlatformDetection", "on");

    PlatformDetectionProperties.apply(
        properties,
        key -> {
          if (PlatformDetectionProperties.TIMEOUT_SYSTEM_PROPERTY.equals(key)) {
            return "22";
          }
          if (PlatformDetectionProperties.DISABLE_SYSTEM_PROPERTY.equals(key)) {
            return "true";
          }
          return null;
        });

    assertEquals("11", properties.getProperty("platformDetectionTimeoutMs"));
    assertEquals("on", properties.getProperty("disablePlatformDetection"));
    assertEquals(2, properties.size());
  }

  @Test
  void shouldKeepConnectionDisableFalseOverSystemPropertyTrue() {
    Properties properties = new Properties();
    properties.setProperty("disablePlatformDetection", "false");

    PlatformDetectionProperties.apply(
        properties,
        key -> PlatformDetectionProperties.DISABLE_SYSTEM_PROPERTY.equals(key) ? "true" : null);

    assertEquals("false", properties.getProperty("disablePlatformDetection"));
    assertEquals(1, properties.size());
  }

  @Test
  void shouldCopyTimeoutFromTheSystemProperty() {
    Properties properties = new Properties();

    PlatformDetectionProperties.apply(
        properties,
        key -> PlatformDetectionProperties.TIMEOUT_SYSTEM_PROPERTY.equals(key) ? " 22 " : null);

    assertEquals("22", properties.getProperty(PlatformDetectionProperties.TIMEOUT_PROPERTY));
    assertNull(properties.getProperty(PlatformDetectionProperties.DISABLE_PROPERTY));
  }

  @Test
  void shouldCopyBoundaryTimeoutsFromTheSystemPropertyUnchanged() {
    for (String value : new String[] {"0", "-1", "-200", "5000", "5001", "300000", "2147483647"}) {
      Properties properties = new Properties();
      PlatformDetectionProperties.apply(
          properties,
          key -> PlatformDetectionProperties.TIMEOUT_SYSTEM_PROPERTY.equals(key) ? value : null);
      assertEquals(
          value, properties.getProperty(PlatformDetectionProperties.TIMEOUT_PROPERTY), value);
    }
  }

  @Test
  void shouldCopyDisableFromTheSystemProperty() {
    for (String value : new String[] {"true", "TRUE", "  true  ", "false"}) {
      Properties properties = new Properties();
      PlatformDetectionProperties.apply(
          properties,
          key -> PlatformDetectionProperties.DISABLE_SYSTEM_PROPERTY.equals(key) ? value : null);
      assertEquals(
          value.trim(),
          properties.getProperty(PlatformDetectionProperties.DISABLE_PROPERTY),
          value);
    }
  }

  @Test
  void shouldLeaveTimeoutUnsetWhenTheSystemPropertyIsBlankOrUnparseable() {
    for (String value : new String[] {null, "", "   ", "abc", "1.5"}) {
      Properties properties = new Properties();
      PlatformDetectionProperties.apply(
          properties,
          key -> PlatformDetectionProperties.TIMEOUT_SYSTEM_PROPERTY.equals(key) ? value : null);
      assertNull(properties.getProperty(PlatformDetectionProperties.TIMEOUT_PROPERTY), value);
    }
  }

  @Test
  void shouldLeaveDisableUnsetWhenTheSystemPropertyIsBlank() {
    for (String value : new String[] {null, "", "   "}) {
      Properties properties = new Properties();
      PlatformDetectionProperties.apply(
          properties,
          key -> PlatformDetectionProperties.DISABLE_SYSTEM_PROPERTY.equals(key) ? value : null);
      assertNull(properties.getProperty(PlatformDetectionProperties.DISABLE_PROPERTY), value);
    }
  }

  @Test
  void shouldReadDisableFromTheRealSystemProperty() {
    System.setProperty(PlatformDetectionProperties.DISABLE_SYSTEM_PROPERTY, "true");

    Properties unset = new Properties();
    PlatformDetectionProperties.apply(unset);
    assertEquals("true", unset.getProperty(PlatformDetectionProperties.DISABLE_PROPERTY));

    Properties connectionFalse = new Properties();
    connectionFalse.setProperty(PlatformDetectionProperties.DISABLE_PROPERTY, "false");
    PlatformDetectionProperties.apply(connectionFalse);
    assertEquals(
        "false", connectionFalse.getProperty(PlatformDetectionProperties.DISABLE_PROPERTY));
  }

  @Test
  void shouldReadTimeoutFromTheRealSystemProperty() {
    System.setProperty(PlatformDetectionProperties.TIMEOUT_SYSTEM_PROPERTY, "0");

    Properties unset = new Properties();
    PlatformDetectionProperties.apply(unset);
    assertEquals("0", unset.getProperty(PlatformDetectionProperties.TIMEOUT_PROPERTY));

    Properties connectionTimeout = new Properties();
    connectionTimeout.setProperty(PlatformDetectionProperties.TIMEOUT_PROPERTY, "77");
    PlatformDetectionProperties.apply(connectionTimeout);
    assertEquals("77", connectionTimeout.getProperty(PlatformDetectionProperties.TIMEOUT_PROPERTY));
  }
}
