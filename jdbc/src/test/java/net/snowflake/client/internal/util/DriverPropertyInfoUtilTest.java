package net.snowflake.client.internal.util;

import static java.util.stream.Collectors.toList;
import static net.snowflake.client.internal.util.DriverPropertyInfoUtil.findMissingProperties;
import static net.snowflake.client.internal.util.DriverPropertyInfoUtil.getPropertyInfo;
import static net.snowflake.client.internal.util.DriverPropertyInfoUtil.requiresPasswordCredentials;
import static net.snowflake.client.internal.util.DriverPropertyInfoUtil.resolve;
import static net.snowflake.client.internal.util.DriverPropertyInfoUtil.usesPrivateKeyCredentials;
import static net.snowflake.client.internal.util.DriverPropertyInfoUtil.validate;
import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertFalse;
import static org.junit.jupiter.api.Assertions.assertThrows;
import static org.junit.jupiter.api.Assertions.assertTrue;
import static org.mockito.Mockito.mock;

import java.security.PrivateKey;
import java.sql.DriverPropertyInfo;
import java.util.Arrays;
import java.util.List;
import java.util.Properties;
import java.util.stream.Stream;
import net.snowflake.client.api.exception.ErrorCode;
import net.snowflake.client.internal.api.implementation.exception.SFSQLException;
import org.junit.jupiter.api.Test;
import org.junit.jupiter.params.ParameterizedTest;
import org.junit.jupiter.params.provider.Arguments;
import org.junit.jupiter.params.provider.MethodSource;
import org.junit.jupiter.params.provider.NullAndEmptySource;
import org.junit.jupiter.params.provider.ValueSource;

class DriverPropertyInfoUtilTest {

  @ParameterizedTest
  @NullAndEmptySource
  @ValueSource(
      strings = {
        "snowflake",
        "SNOWFLAKE",
        "username_password_mfa",
        "USERNAME_PASSWORD_MFA",
        "https://idp.snowflake.com"
      })
  void shouldRequirePasswordCredentials(String authenticator) {
    assertTrue(requiresPasswordCredentials(authenticator));
  }

  @Test
  void shouldNotRequirePasswordCredentialsForExternalBrowser() {
    assertFalse(requiresPasswordCredentials("externalbrowser"));
  }

  @ParameterizedTest
  @NullAndEmptySource
  void shouldUsePrivateKeyCredentialsWhenAuthenticatorIsUnspecified(String authenticator) {
    Properties properties = new Properties();
    properties.put("private_key_file", "/tmp/key.p8");

    assertTrue(usesPrivateKeyCredentials(properties, authenticator));
  }

  @Test
  void shouldNotUsePrivateKeyCredentialsWhenAuthenticatorIsSnowflake() {
    Properties properties = new Properties();
    properties.put("privateKey", mock(PrivateKey.class));

    assertFalse(usesPrivateKeyCredentials(properties, "snowflake"));
  }

  @Test
  void shouldFindMissingCredentialsAndEnabledProxyProperties() {
    Properties resolved = new Properties();
    resolved.setProperty("useProxy", "true");

    assertEquals(
        Arrays.asList("user", "password", "proxyHost", "proxyPort"),
        names(findMissingProperties(resolved)));
  }

  @Test
  void shouldTreatEmptyProxyKeysAsPresent() {
    Properties resolved = new Properties();
    resolved.setProperty("use_proxy", "true");
    resolved.setProperty("proxy_host", "");
    resolved.setProperty("proxy_port", "");

    assertEquals(Arrays.asList("user", "password"), names(findMissingProperties(resolved)));
  }

  @Test
  void shouldNotFindPasswordCredentialsWhenPrivateKeyIsPresent() {
    Properties resolved = new Properties();
    resolved.put("privateKey", mock(PrivateKey.class));

    assertEquals(0, findMissingProperties(resolved).length);
  }

  @ParameterizedTest
  @MethodSource("invalidTypedPropertyValues")
  void shouldRejectInvalidTypedProperties(
      String propertyName, Object propertyValue, String expectedType) {
    Properties resolved = new Properties();
    resolved.put(propertyName, propertyValue);

    SFSQLException exception = assertThrows(SFSQLException.class, () -> validate(resolved));

    assertEquals(
        "Invalid parameter value type: "
            + propertyValue.getClass().getName()
            + ", expected type: "
            + expectedType
            + ".",
        exception.getMessage());
    assertEquals(ErrorCode.INVALID_PARAMETER_TYPE, exception.getErrorCode());
  }

  private static Stream<Arguments> invalidTypedPropertyValues() {
    return Stream.of(
        Arguments.of("user", 42, String.class.getName()),
        Arguments.of("useProxy", 42, Boolean.class.getName()),
        Arguments.of("proxy_host", 42, String.class.getName()),
        Arguments.of("proxy_port", 42, String.class.getName()),
        Arguments.of("privateKey", 42, PrivateKey.class.getName()),
        Arguments.of(
            "private_key", 42, String.class.getName() + " or " + PrivateKey.class.getName()),
        Arguments.of("private_key_file", 42, String.class.getName()),
        Arguments.of("private_key_base64", 42, String.class.getName()));
  }

  @Test
  void shouldPreferFirstListedAliasWhenBothArePresent() {
    Properties resolved = new Properties();
    resolved.setProperty("use_proxy", "true");
    resolved.setProperty("useProxy", "false");

    assertEquals(Arrays.asList("user", "password"), names(findMissingProperties(resolved)));
  }

  @Test
  void shouldResolveStandardUrlParameters() {
    Properties resolved =
        resolve(
            "jdbc:snowflake://test.snowflakecomputing.com?authenticator=externalbrowser",
            new Properties());

    assertEquals("externalbrowser", resolved.get("AUTHENTICATOR"));
    assertEquals(0, findMissingProperties(resolved).length);
  }

  @Test
  void shouldRejectInvalidConnectionString() {
    SFSQLException exception =
        assertThrows(SFSQLException.class, () -> resolve("jdbc:snowflake://", new Properties()));

    assertEquals("Connection string is invalid. Unable to parse.", exception.getMessage());
    assertEquals(ErrorCode.INVALID_CONNECTION_STRING, exception.getErrorCode());
  }

  @ParameterizedTest
  @NullAndEmptySource
  void shouldReportServerUrlWhenUrlIsUnspecified(String url) {
    DriverPropertyInfo[] properties = getPropertyInfo(url, new Properties());

    assertEquals(1, properties.length);
    assertEquals("serverURL", properties[0].name);
  }

  private static List<String> names(DriverPropertyInfo[] properties) {
    return Arrays.stream(properties).map(property -> property.name).collect(toList());
  }
}
