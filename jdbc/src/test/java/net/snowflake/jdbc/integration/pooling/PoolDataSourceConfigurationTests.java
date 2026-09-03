package net.snowflake.jdbc.integration.pooling;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertFalse;
import static org.junit.jupiter.api.Assertions.assertNull;
import static org.junit.jupiter.api.Assertions.assertSame;
import static org.junit.jupiter.api.Assertions.assertThrows;
import static org.junit.jupiter.api.Assertions.assertTrue;

import java.io.PrintWriter;
import java.security.KeyPairGenerator;
import java.security.PrivateKey;
import java.sql.SQLException;
import java.sql.SQLFeatureNotSupportedException;
import java.util.Base64;
import java.util.Properties;
import java.util.function.Consumer;
import java.util.stream.Stream;
import net.snowflake.client.api.pooling.SnowflakeConnectionPoolDataSource;
import net.snowflake.client.internal.api.decorator.Telemetry;
import net.snowflake.client.internal.api.implementation.pooling.DecoratedSnowflakePooledConnectionDataSource;
import net.snowflake.client.internal.api.implementation.pooling.SnowflakePooledConnectionDataSource;
import org.junit.jupiter.api.Test;
import org.junit.jupiter.api.function.Executable;
import org.junit.jupiter.params.ParameterizedTest;
import org.junit.jupiter.params.provider.Arguments;
import org.junit.jupiter.params.provider.MethodSource;

/**
 * Offline (@jdbc_int) coverage for the {@link SnowflakeConnectionPoolDataSource} configuration and
 * DataSource API surface. Mirrors {@code
 * tests/definitions/shared/pooling/pool_data_source_configuration.feature}. None of these tests
 * open a Snowflake session; they assert what each setter stores/exposes on the pooled data source.
 * Live pooling behavior is covered by {@code ConnectionPoolTests} / {@code
 * connection_pool.feature}.
 */
class PoolDataSourceConfigurationTests {

  @Test
  void shouldReturnTheUrlThatWasExplicitlySet() {
    // Given a new Snowflake connection pool data source
    SnowflakePooledConnectionDataSource ds = new SnowflakePooledConnectionDataSource();

    // When the URL is configured with setUrl
    ds.setUrl("jdbc:snowflake://explicit.snowflakecomputing.com");

    // Then getUrl returns the same URL
    assertEquals("jdbc:snowflake://explicit.snowflakecomputing.com", ds.getUrl());
  }

  @Test
  void shouldBuildTheJdbcUrlFromTheServerNameAndPortNumber() {
    // Given a new Snowflake connection pool data source with no explicit URL
    SnowflakePooledConnectionDataSource ds = new SnowflakePooledConnectionDataSource();

    // When the server name is configured with setServerName and the port with setPortNumber
    ds.setServerName("account.snowflakecomputing.com");
    ds.setPortNumber(443);

    // Then getUrl returns a jdbc:snowflake URL that contains the server name and port
    assertEquals("jdbc:snowflake://account.snowflakecomputing.com:443", ds.getUrl());
  }

  static Stream<Arguments> endpointProperties() {
    return Stream.of(
        Arguments.of(
            "account",
            "myaccount",
            (Consumer<SnowflakePooledConnectionDataSource>) ds -> ds.setAccount("myaccount")),
        Arguments.of(
            "database",
            "mydb",
            (Consumer<SnowflakePooledConnectionDataSource>) ds -> ds.setDatabaseName("mydb")),
        Arguments.of(
            "schema",
            "myschema",
            (Consumer<SnowflakePooledConnectionDataSource>) ds -> ds.setSchema("myschema")),
        Arguments.of(
            "warehouse",
            "mywh",
            (Consumer<SnowflakePooledConnectionDataSource>) ds -> ds.setWarehouse("mywh")),
        Arguments.of(
            "role",
            "myrole",
            (Consumer<SnowflakePooledConnectionDataSource>) ds -> ds.setRole("myrole")));
  }

  @ParameterizedTest
  @MethodSource("endpointProperties")
  void shouldStoreThePropertyEndpointProperty(
      String property,
      String expectedValue,
      Consumer<SnowflakePooledConnectionDataSource> configurer) {
    // Given a new Snowflake connection pool data source
    SnowflakePooledConnectionDataSource ds = new SnowflakePooledConnectionDataSource();

    // When <setter> is called with a value
    configurer.accept(ds);

    // Then the <property> is stored in the data source configuration
    assertEquals(expectedValue, ds.getProperties().getProperty(property));
  }

  static Stream<Arguments> authenticationProperties() {
    return Stream.of(
        Arguments.of(
            "authenticator",
            "PROGRAMMATIC_ACCESS_TOKEN",
            (Consumer<SnowflakePooledConnectionDataSource>)
                ds -> ds.setAuthenticator("PROGRAMMATIC_ACCESS_TOKEN")),
        Arguments.of(
            "token",
            "pat-token",
            (Consumer<SnowflakePooledConnectionDataSource>) ds -> ds.setToken("pat-token")),
        Arguments.of(
            "passcode",
            "123456",
            (Consumer<SnowflakePooledConnectionDataSource>) ds -> ds.setPasscode("123456")),
        Arguments.of(
            "passcodeInPassword",
            "true",
            (Consumer<SnowflakePooledConnectionDataSource>) ds -> ds.setPasscodeInPassword(true)),
        Arguments.of(
            "disable_saml_url_check",
            "true",
            (Consumer<SnowflakePooledConnectionDataSource>) ds -> ds.setDisableSamlURLCheck(true)),
        Arguments.of(
            "ssl", "true", (Consumer<SnowflakePooledConnectionDataSource>) ds -> ds.setSsl(true)));
  }

  @ParameterizedTest
  @MethodSource("authenticationProperties")
  void shouldStoreThePropertyAuthenticationProperty(
      String property,
      String expectedValue,
      Consumer<SnowflakePooledConnectionDataSource> configurer) {
    // Given a new Snowflake connection pool data source
    SnowflakePooledConnectionDataSource ds = new SnowflakePooledConnectionDataSource();

    // When <setter> is called with a value
    configurer.accept(ds);

    // Then the <property> is stored in the data source configuration
    assertEquals(expectedValue, ds.getProperties().getProperty(property));
  }

  static Stream<Arguments> privateKeyMaterial() {
    PrivateKey privateKey = generateRsaPrivateKey();
    String base64 = Base64.getEncoder().encodeToString(privateKey.getEncoded());
    return Stream.of(
        Arguments.of(
            "private_key",
            base64,
            (Consumer<SnowflakePooledConnectionDataSource>) ds -> ds.setPrivateKey(privateKey)),
        Arguments.of(
            "private_key_file",
            "/keys/rsa_key.p8",
            (Consumer<SnowflakePooledConnectionDataSource>)
                ds -> ds.setPrivateKeyFile("/keys/rsa_key.p8", "secret")),
        Arguments.of(
            "private_key",
            "BASE64KEY",
            (Consumer<SnowflakePooledConnectionDataSource>)
                ds -> ds.setPrivateKeyBase64("BASE64KEY", "secret")));
  }

  @ParameterizedTest
  @MethodSource("privateKeyMaterial")
  void shouldStorePrivateKeyMaterialConfiguredViaSetter(
      String property,
      String expectedValue,
      Consumer<SnowflakePooledConnectionDataSource> configurer) {
    // Given a new Snowflake connection pool data source
    SnowflakePooledConnectionDataSource ds = new SnowflakePooledConnectionDataSource();

    // When <setter> is called with the key material
    configurer.accept(ds);

    // Then the corresponding private key configuration is stored in the data source
    assertEquals(expectedValue, ds.getProperties().getProperty(property));
  }

  @Test
  void shouldStoreTheClientStoreTemporaryCredentialProperty() {
    // Given a new Snowflake connection pool data source
    SnowflakePooledConnectionDataSource ds = new SnowflakePooledConnectionDataSource();

    // When setClientStoreTemporaryCredential is called with true
    ds.setClientStoreTemporaryCredential(true);

    // Then the clientStoreTemporaryCredential property is stored in the data source configuration
    assertEquals("true", ds.getProperties().getProperty("clientStoreTemporaryCredential"));
  }

  static Stream<Arguments> proxyProperties() {
    return Stream.of(
        Arguments.of(
            "proxyHost",
            "proxy.example.com",
            (Consumer<SnowflakePooledConnectionDataSource>)
                ds -> ds.setProxyHost("proxy.example.com")),
        Arguments.of(
            "proxyPort",
            "8080",
            (Consumer<SnowflakePooledConnectionDataSource>) ds -> ds.setProxyPort(8080)),
        Arguments.of(
            "proxyUser",
            "proxy-user",
            (Consumer<SnowflakePooledConnectionDataSource>) ds -> ds.setProxyUser("proxy-user")),
        Arguments.of(
            "proxyPassword",
            "proxy-pass",
            (Consumer<SnowflakePooledConnectionDataSource>)
                ds -> ds.setProxyPassword("proxy-pass")),
        Arguments.of(
            "nonProxyHosts",
            "localhost",
            (Consumer<SnowflakePooledConnectionDataSource>)
                ds -> ds.setNonProxyHosts("localhost")));
  }

  @ParameterizedTest
  @MethodSource("proxyProperties")
  void shouldStoreThePropertyProxyProperty(
      String property,
      String expectedValue,
      Consumer<SnowflakePooledConnectionDataSource> configurer) {
    // Given a new Snowflake connection pool data source
    SnowflakePooledConnectionDataSource ds = new SnowflakePooledConnectionDataSource();

    // When <setter> is called with a value
    configurer.accept(ds);

    // Then the <property> is stored in the data source configuration
    assertEquals(expectedValue, ds.getProperties().getProperty(property));
  }

  static Stream<Arguments> clientBehaviorProperties() {
    return Stream.of(
        Arguments.of(
            "application",
            "MyApp",
            (Consumer<SnowflakePooledConnectionDataSource>) ds -> ds.setApplication("MyApp")),
        Arguments.of(
            "allowUnderscoresInHost",
            "true",
            (Consumer<SnowflakePooledConnectionDataSource>)
                ds -> ds.setAllowUnderscoresInHost(true)),
        Arguments.of(
            "tracing",
            "FINE",
            (Consumer<SnowflakePooledConnectionDataSource>) ds -> ds.setTracing("FINE")),
        Arguments.of(
            "enablePatternSearch",
            "false",
            (Consumer<SnowflakePooledConnectionDataSource>) ds -> ds.setEnablePatternSearch(false)),
        Arguments.of(
            "JDBC_ARROW_TREAT_DECIMAL_AS_INT",
            "false",
            (Consumer<SnowflakePooledConnectionDataSource>)
                ds -> ds.setArrowTreatDecimalAsInt(false)),
        Arguments.of(
            "JDBC_DEFAULT_FORMAT_DATE_WITH_TIMEZONE",
            "false",
            (Consumer<SnowflakePooledConnectionDataSource>)
                ds -> ds.setJDBCDefaultFormatDateWithTimezone(false)),
        Arguments.of(
            "JDBC_GET_DATE_USE_NULL_TIMEZONE",
            "false",
            (Consumer<SnowflakePooledConnectionDataSource>)
                ds -> ds.setGetDateUseNullTimezone(false)));
  }

  @ParameterizedTest
  @MethodSource("clientBehaviorProperties")
  void shouldStoreThePropertyClientBehaviorProperty(
      String property,
      String expectedValue,
      Consumer<SnowflakePooledConnectionDataSource> configurer) {
    // Given a new Snowflake connection pool data source
    SnowflakePooledConnectionDataSource ds = new SnowflakePooledConnectionDataSource();

    // When <setter> is called with a value
    configurer.accept(ds);

    // Then the <property> is stored in the data source configuration
    assertEquals(expectedValue, ds.getProperties().getProperty(property));
  }

  static Stream<Arguments> timeoutOrRetryProperties() {
    return Stream.of(
        Arguments.of(
            "queryTimeoutSeconds",
            "120",
            (Consumer<SnowflakePooledConnectionDataSource>) ds -> ds.setQueryTimeout(120)),
        Arguments.of(
            "maxHttpRetries",
            "5",
            (Consumer<SnowflakePooledConnectionDataSource>) ds -> ds.setMaxHttpRetries(5)),
        Arguments.of(
            "putGetMaxRetries",
            "3",
            (Consumer<SnowflakePooledConnectionDataSource>) ds -> ds.setPutGetMaxRetries(3)),
        Arguments.of(
            "browser_response_timeout",
            "60",
            (Consumer<SnowflakePooledConnectionDataSource>)
                ds -> ds.setBrowserResponseTimeout(60)));
  }

  @ParameterizedTest
  @MethodSource("timeoutOrRetryProperties")
  void shouldStoreThePropertyTimeoutOrRetryProperty(
      String property,
      String expectedValue,
      Consumer<SnowflakePooledConnectionDataSource> configurer) {
    // Given a new Snowflake connection pool data source
    SnowflakePooledConnectionDataSource ds = new SnowflakePooledConnectionDataSource();

    // When <setter> is called with a value
    configurer.accept(ds);

    // Then the <property> is stored in the data source configuration
    assertEquals(expectedValue, ds.getProperties().getProperty(property));
  }

  @Test
  void shouldRoundTripTheLoginTimeout() {
    // Given a new Snowflake connection pool data source
    SnowflakePooledConnectionDataSource ds = new SnowflakePooledConnectionDataSource();

    // When the login timeout is configured with setLoginTimeout
    ds.setLoginTimeout(42);

    // Then getLoginTimeout returns the configured value
    assertEquals(42, ds.getLoginTimeout());
  }

  @Test
  void shouldAutoPromoteTheAuthenticatorWhenTheBrowserResponseTimeoutIsSet() {
    // Given a new Snowflake connection pool data source
    SnowflakePooledConnectionDataSource ds = new SnowflakePooledConnectionDataSource();

    // When setBrowserResponseTimeout is called
    ds.setBrowserResponseTimeout(60);

    // Then the browser response timeout is stored and the authenticator is promoted to
    // EXTERNALBROWSER
    Properties props = ds.getProperties();
    assertEquals("60", props.getProperty("browser_response_timeout"));
    assertEquals("EXTERNALBROWSER", props.getProperty("authenticator"));
  }

  static Stream<Arguments> diagnosticsProperties() {
    return Stream.of(
        Arguments.of(
            "enableDiagnostics",
            "true",
            (Consumer<SnowflakePooledConnectionDataSource>) ds -> ds.setEnableDiagnostics(true)),
        Arguments.of(
            "diagnosticsAllowlistFile",
            "/tmp/allowlist.json",
            (Consumer<SnowflakePooledConnectionDataSource>)
                ds -> ds.setDiagnosticsAllowlistFile("/tmp/allowlist.json")));
  }

  @ParameterizedTest
  @MethodSource("diagnosticsProperties")
  void shouldStoreThePropertyDiagnosticsProperty(
      String property,
      String expectedValue,
      Consumer<SnowflakePooledConnectionDataSource> configurer) {
    // Given a new Snowflake connection pool data source
    SnowflakePooledConnectionDataSource ds = new SnowflakePooledConnectionDataSource();

    // When <setter> is called with a value
    configurer.accept(ds);

    // Then the <property> is stored in the data source configuration
    assertEquals(expectedValue, ds.getProperties().getProperty(property));
  }

  static Stream<Arguments> unsupportedOperations() {
    return Stream.of(
        Arguments.of("getLogWriter"),
        Arguments.of("setLogWriter"),
        Arguments.of("getParentLogger"));
  }

  @ParameterizedTest
  @MethodSource("unsupportedOperations")
  void shouldRejectTheUnsupportedOperationOperation(String operation) {
    // The impl throws runtime carriers; the checked SQLFeatureNotSupportedException the JDBC API
    // promises is reconstructed by the generated boundary decorator, so the contract assertion goes
    // through it rather than the raw impl.
    // Given a new Snowflake connection pool data source
    DecoratedSnowflakePooledConnectionDataSource ds = decorated();

    // When <operation> is invoked on the data source
    Executable call = unsupportedOperationCall(ds, operation);

    // Then a SQLFeatureNotSupportedException is thrown
    assertThrows(SQLFeatureNotSupportedException.class, call);
  }

  /**
   * SnowflakePooledConnectionDataSource is a {@code @JdbcBoundary}: its generated decorator is the
   * public contract, translating the impl's runtime carriers (SFSQLFeatureNotSupportedException,
   * SFSQLException) into the checked SQLException types JDBC promises. Contract-asserting tests go
   * through the decorator; tests asserting stored configuration stay on the raw impl.
   */
  private static DecoratedSnowflakePooledConnectionDataSource decorated() {
    return new DecoratedSnowflakePooledConnectionDataSource(
        new SnowflakePooledConnectionDataSource(), Telemetry.NOOP);
  }

  private static Executable unsupportedOperationCall(
      DecoratedSnowflakePooledConnectionDataSource ds, String operation) {
    switch (operation) {
      case "getLogWriter":
        return () -> ds.getLogWriter();
      case "setLogWriter":
        return () -> ds.setLogWriter(new PrintWriter(System.out));
      case "getParentLogger":
        return ds::getParentLogger;
      default:
        throw new IllegalArgumentException("Unknown operation: " + operation);
    }
  }

  @Test
  void shouldUnwrapTheDataSourceToASupportedInterface() throws Exception {
    // Given a new Snowflake connection pool data source
    SnowflakePooledConnectionDataSource ds = new SnowflakePooledConnectionDataSource();

    // When isWrapperFor and unwrap are called with a supported interface
    boolean wrapsForSupported = ds.isWrapperFor(SnowflakeConnectionPoolDataSource.class);
    Object unwrapped = ds.unwrap(SnowflakeConnectionPoolDataSource.class);

    // Then isWrapperFor returns true and unwrap returns the data source instance
    assertTrue(wrapsForSupported);
    assertSame(ds, unwrapped);
  }

  @Test
  void shouldRejectUnwrappingToAnUnsupportedInterface() throws SQLException {
    // unwrap surfaces its failure as a runtime carrier from the impl; the checked SQLException is
    // reconstructed by the boundary decorator, so the rejection contract is asserted through it.
    // Given a new Snowflake connection pool data source
    DecoratedSnowflakePooledConnectionDataSource ds = decorated();

    // When isWrapperFor and unwrap are called with an unsupported interface
    boolean wrapsForUnsupported = ds.isWrapperFor(String.class);

    // Then isWrapperFor returns false and unwrap throws a SQLException
    assertFalse(wrapsForUnsupported);
    assertThrows(SQLException.class, () -> ds.unwrap(String.class));
  }

  @Test
  void shouldExposeTheConfigurationAsADefensiveCopyOfProperties() {
    // Given a Snowflake connection pool data source with configuration applied
    SnowflakePooledConnectionDataSource ds = new SnowflakePooledConnectionDataSource();
    ds.setAccount("myaccount");

    // When getProperties is called and the returned map is mutated
    Properties firstCopy = ds.getProperties();
    firstCopy.setProperty("injected", "value");

    // Then the data source's own configuration remains unchanged
    Properties freshCopy = ds.getProperties();
    assertEquals("myaccount", freshCopy.getProperty("account"));
    assertNull(freshCopy.getProperty("injected"));
  }

  private static PrivateKey generateRsaPrivateKey() {
    try {
      KeyPairGenerator generator = KeyPairGenerator.getInstance("RSA");
      generator.initialize(2048);
      return generator.generateKeyPair().getPrivate();
    } catch (Exception e) {
      throw new RuntimeException("Failed to generate RSA private key for test", e);
    }
  }
}
