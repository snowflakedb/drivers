package net.snowflake.client.api.driver;

import static java.util.stream.Collectors.toList;
import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertFalse;
import static org.junit.jupiter.api.Assertions.assertInstanceOf;
import static org.junit.jupiter.api.Assertions.assertNotNull;
import static org.junit.jupiter.api.Assertions.assertNull;
import static org.junit.jupiter.api.Assertions.assertThrows;
import static org.junit.jupiter.api.Assertions.assertTrue;
import static org.mockito.Mockito.mock;

import java.security.PrivateKey;
import java.sql.Driver;
import java.sql.DriverManager;
import java.sql.DriverPropertyInfo;
import java.sql.SQLException;
import java.util.Arrays;
import java.util.Properties;
import java.util.stream.Stream;
import net.snowflake.client.api.exception.SnowflakeSQLException;
import net.snowflake.jdbc.utils.DriverCompatibility;
import net.snowflake.jdbc.utils.SkipOldDriver;
import org.junit.jupiter.api.Test;
import org.junit.jupiter.params.ParameterizedTest;
import org.junit.jupiter.params.provider.Arguments;
import org.junit.jupiter.params.provider.MethodSource;
import org.junit.jupiter.params.provider.ValueSource;

/** Basic tests for the Snowflake JDBC Driver. */
public class SnowflakeDriverTest {

  @Test
  public void testDriverRegistration() throws SQLException {
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
    // Legacy snowflake-jdbc has major > 0; this module is 0.x.
    if (DriverCompatibility.isOldDriver()) {
      assertTrue(driver.getMajorVersion() > 0, "Legacy driver major version should be gt 0");
    } else {
      assertEquals(0, driver.getMajorVersion(), "New driver major version should be 0 (0.0.1)");
    }
    assertTrue(driver.getMinorVersion() >= 0, "Minor version should be gte 0");
    assertFalse(driver.jdbcCompliant(), "Driver should not claim JDBC compliance");
  }

  @Test
  public void testGetParentLogger() {
    SnowflakeDriver driver = new SnowflakeDriver();
    assertNull(driver.getParentLogger(), "Expected getParentLogger to be null");
  }

  @Test
  public void shouldReportServerUrlWhenPropertyInfoHasNoUrl() throws SQLException {
    DriverPropertyInfo[] properties = new SnowflakeDriver().getPropertyInfo(null, new Properties());

    assertEquals(1, properties.length);
    assertEquals("serverURL", properties[0].name);
    assertEquals(
        "server URL in form of <protocol>://<host or domain>:<port number>/<path of resource>",
        properties[0].description);
  }

  @Test
  public void shouldReportMissingCredentialsAndEnabledProxyProperties() throws SQLException {
    DriverPropertyInfo[] properties =
        new SnowflakeDriver()
            .getPropertyInfo(
                "jdbc:snowflake://test.snowflakecomputing.com?useProxy=true", new Properties());

    assertEquals(
        Arrays.asList("user", "password", "proxyHost", "proxyPort"),
        Arrays.stream(properties).map(property -> property.name).collect(toList()));
  }

  @Test
  public void shouldNotRequirePasswordCredentialsForExternalBrowser() throws SQLException {
    DriverPropertyInfo[] properties =
        new SnowflakeDriver()
            .getPropertyInfo(
                "jdbc:snowflake://test.snowflakecomputing.com?authenticator=externalbrowser",
                new Properties());

    assertEquals(0, properties.length);
  }

  @Test
  public void shouldReportMissingCredentialsForHttpsAuthenticator() throws SQLException {
    DriverPropertyInfo[] properties =
        new SnowflakeDriver()
            .getPropertyInfo(
                "jdbc:snowflake://test.snowflakecomputing.com"
                    + "?authenticator=https%3A%2F%2Fidp.snowflake.com",
                new Properties());

    assertEquals(
        Arrays.asList("user", "password"),
        Arrays.stream(properties).map(property -> property.name).collect(toList()));
  }

  @ParameterizedTest
  @ValueSource(strings = {"username_password_mfa", "USERNAME_PASSWORD_MFA"})
  @SkipOldDriver("BD#69")
  public void shouldReportMissingCredentialsForUsernamePasswordMfa(String authenticator)
      throws SQLException {
    Properties info = new Properties();
    info.setProperty("authenticator", authenticator);

    DriverPropertyInfo[] properties =
        new SnowflakeDriver().getPropertyInfo("jdbc:snowflake://test.snowflakecomputing.com", info);

    assertEquals(
        Arrays.asList("user", "password"),
        Arrays.stream(properties).map(property -> property.name).collect(toList()));
  }

  @Test
  @SkipOldDriver("BD#68")
  public void shouldReportMissingCredentialsForEmptyAuthenticator() throws SQLException {
    Properties info = new Properties();
    info.setProperty("authenticator", "");

    DriverPropertyInfo[] properties =
        new SnowflakeDriver().getPropertyInfo("jdbc:snowflake://test.snowflakecomputing.com", info);

    assertEquals(
        Arrays.asList("user", "password"),
        Arrays.stream(properties).map(property -> property.name).collect(toList()));
  }

  @ParameterizedTest
  @MethodSource("privateKeyCredentialProperties")
  public void shouldNotRequirePasswordCredentialsWhenPrivateKeyCredentialsArePresent(
      String propertyName, Object propertyValue, String authenticator) throws SQLException {
    Properties info = new Properties();
    info.put(propertyName, propertyValue);
    if (authenticator != null) {
      info.setProperty("authenticator", authenticator);
    }

    DriverPropertyInfo[] properties =
        new SnowflakeDriver().getPropertyInfo("jdbc:snowflake://test.snowflakecomputing.com", info);

    assertEquals(0, properties.length);
  }

  private static Stream<Arguments> privateKeyCredentialProperties() {
    return Stream.of(
        Arguments.of("privateKey", mock(PrivateKey.class), null),
        Arguments.of("private_key_file", "/tmp/key.p8", null),
        Arguments.of("private_key_base64", "AQ==", null),
        Arguments.of("privateKey", mock(PrivateKey.class), ""),
        Arguments.of("private_key_file", "/tmp/key.p8", ""),
        Arguments.of("private_key_base64", "AQ==", ""));
  }

  @ParameterizedTest
  @MethodSource("invalidTypedPropertyValues")
  public void shouldRejectInvalidTypedPropertiesDuringIntrospection(
      String propertyName, Object propertyValue, String expectedType) {
    assertRejectsInvalidTypedProperty(propertyName, propertyValue, expectedType);
  }

  @ParameterizedTest
  @MethodSource("invalidTypedSnakeCaseProxyPropertyValues")
  @SkipOldDriver("BD#67")
  public void shouldRejectInvalidTypedSnakeCaseProxyAliasesDuringIntrospection(
      String propertyName, Object propertyValue, String expectedType) {
    assertRejectsInvalidTypedProperty(propertyName, propertyValue, expectedType);
  }

  private static void assertRejectsInvalidTypedProperty(
      String propertyName, Object propertyValue, String expectedType) {
    Properties info = new Properties();
    info.put(propertyName, propertyValue);

    SQLException exception =
        assertThrows(
            SQLException.class,
            () ->
                new SnowflakeDriver()
                    .getPropertyInfo("jdbc:snowflake://test.snowflakecomputing.com", info));

    assertEquals(
        "Invalid parameter value type: "
            + propertyValue.getClass().getName()
            + ", expected type: "
            + expectedType
            + ".",
        exception.getMessage());
    assertEquals("22023", exception.getSQLState());
    assertEquals(200033, exception.getErrorCode());
  }

  private static Stream<Arguments> invalidTypedPropertyValues() {
    return Stream.of(
        Arguments.of("user", 42, String.class.getName()),
        Arguments.of("password", 42, String.class.getName()),
        Arguments.of("authenticator", 42, String.class.getName()),
        Arguments.of("useProxy", 42, Boolean.class.getName()),
        Arguments.of("proxyHost", 42, String.class.getName()),
        Arguments.of("proxyPort", 42, String.class.getName()),
        Arguments.of("privateKey", 42, PrivateKey.class.getName()),
        Arguments.of("private_key_file", 42, String.class.getName()),
        Arguments.of("private_key_base64", 42, String.class.getName()));
  }

  private static Stream<Arguments> invalidTypedSnakeCaseProxyPropertyValues() {
    return Stream.of(
        Arguments.of("proxy_host", 42, String.class.getName()),
        Arguments.of("proxy_port", 42, String.class.getName()));
  }

  @Test
  public void shouldTreatEmptyProxyPropertiesAsPresent() throws SQLException {
    Properties info = new Properties();
    info.setProperty("proxyHost", "");
    info.setProperty("proxyPort", "");

    DriverPropertyInfo[] properties =
        new SnowflakeDriver()
            .getPropertyInfo("jdbc:snowflake://test.snowflakecomputing.com?useProxy=true", info);

    assertEquals(
        Arrays.asList("user", "password"),
        Arrays.stream(properties).map(property -> property.name).collect(toList()));
  }

  @ParameterizedTest
  @ValueSource(strings = {"jdbc:snowflake://", "jdbc:snowflake://?user=test"})
  public void shouldRejectMalformedStandardUrlsDuringPropertyInfo(String url) {
    SnowflakeSQLException exception =
        assertThrows(
            SnowflakeSQLException.class,
            () -> new SnowflakeDriver().getPropertyInfo(url, new Properties()));
    assertEquals("08000", exception.getSQLState());
    assertEquals(200059, exception.getErrorCode());
  }

  @Test
  @SkipOldDriver("BD#66")
  public void shouldRedactMalformedUrlFromPropertyInfoError() {
    String url = "jdbc:snowflake://?password=secret";

    SnowflakeSQLException exception =
        assertThrows(
            SnowflakeSQLException.class,
            () -> new SnowflakeDriver().getPropertyInfo(url, new Properties()));

    assertEquals("Connection string is invalid. Unable to parse.", exception.getMessage());
    assertEquals("08000", exception.getSQLState());
    assertEquals(200059, exception.getErrorCode());
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
