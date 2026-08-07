package net.snowflake.client.internal.api.implementation.datasource;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertFalse;
import static org.junit.jupiter.api.Assertions.assertInstanceOf;
import static org.junit.jupiter.api.Assertions.assertNull;
import static org.junit.jupiter.api.Assertions.assertSame;
import static org.junit.jupiter.api.Assertions.assertThrows;
import static org.junit.jupiter.api.Assertions.assertTrue;

import java.io.ByteArrayInputStream;
import java.io.ByteArrayOutputStream;
import java.io.ObjectInputStream;
import java.io.ObjectOutputStream;
import java.io.PrintWriter;
import java.lang.reflect.Proxy;
import java.security.KeyPairGenerator;
import java.security.PrivateKey;
import java.sql.Connection;
import java.sql.SQLException;
import java.sql.SQLFeatureNotSupportedException;
import java.util.Base64;
import java.util.Properties;
import java.util.stream.Stream;
import javax.sql.DataSource;
import net.snowflake.client.internal.api.decorator.Telemetry;
import org.junit.jupiter.api.BeforeEach;
import org.junit.jupiter.api.Test;
import org.junit.jupiter.params.ParameterizedTest;
import org.junit.jupiter.params.provider.Arguments;
import org.junit.jupiter.params.provider.MethodSource;

public class SnowflakeBasicDataSourceTest {

  private static final class TestableSnowflakeBasicDataSource extends SnowflakeBasicDataSource {
    private Connection nextConnection;
    private Properties lastProperties;
    private String lastUrl;

    void setNextConnection(Connection nextConnection) {
      this.nextConnection = nextConnection;
    }

    Properties getLastProperties() {
      return lastProperties;
    }

    String getLastUrl() {
      return lastUrl;
    }

    @Override
    protected Connection openConnection(String url, Properties properties) {
      this.lastUrl = url;
      this.lastProperties = new Properties();
      this.lastProperties.putAll(properties);
      return nextConnection;
    }
  }

  private SnowflakeBasicDataSource dataSource;

  // SnowflakeBasicDataSource is a @JdbcBoundary: its generated decorator is the public contract,
  // translating the impl's runtime carriers (SFSQLFeatureNotSupportedException, SFSQLException) and
  // foreign runtime exceptions (the IllegalStateException raised for an unset URL) into the checked
  // SQLException JDBC promises. Tests asserting that contract go through the decorator; tests
  // asserting internal behavior (captured url/properties, getters) stay on the raw impl.
  private static DataSource decorated(SnowflakeBasicDataSource dataSource) {
    return new DecoratedSnowflakeBasicDataSource(dataSource, Telemetry.NOOP);
  }

  private Connection createDummyConnection() {
    return (Connection)
        Proxy.newProxyInstance(
            Connection.class.getClassLoader(),
            new Class<?>[] {Connection.class},
            (proxy, method, args) -> {
              if ("isClosed".equals(method.getName())) {
                return false;
              }
              if ("close".equals(method.getName())) {
                return null;
              }
              if ("unwrap".equals(method.getName())) {
                Class<?> iface = (Class<?>) args[0];
                if (iface.isInstance(proxy)) {
                  return proxy;
                }
                throw new SQLFeatureNotSupportedException();
              }
              if ("isWrapperFor".equals(method.getName())) {
                Class<?> iface = (Class<?>) args[0];
                return iface.isInstance(proxy);
              }

              throw new UnsupportedOperationException(
                  "Unexpected Connection method in test: " + method.getName());
            });
  }

  @BeforeEach
  public void setUp() {
    dataSource = new TestableSnowflakeBasicDataSource();
    dataSource.setUrl("jdbc:snowflake://testaccount.snowflakecomputing.com");
  }

  @Test
  public void shouldDelegateGetConnectionWithConfiguredUserAndPassword() throws Exception {
    dataSource.setUser("testuser");
    dataSource.setPassword("testpassword");

    Connection mockConnection = createDummyConnection();
    TestableSnowflakeBasicDataSource testableDataSource =
        (TestableSnowflakeBasicDataSource) dataSource;
    testableDataSource.setNextConnection(mockConnection);

    Connection result = dataSource.getConnection();

    assertSame(mockConnection, result);
    assertEquals(
        "jdbc:snowflake://testaccount.snowflakecomputing.com", testableDataSource.getLastUrl());
    Properties capturedProperties = testableDataSource.getLastProperties();
    assertEquals("testuser", capturedProperties.getProperty("user"));
    assertEquals("testpassword", capturedProperties.getProperty("password"));
  }

  @Test
  public void shouldGetConnectionWithUsernameAndPasswordSetPropertiesAndReturnConnection()
      throws Exception {
    Connection mockConnection = createDummyConnection();
    TestableSnowflakeBasicDataSource testableDataSource =
        (TestableSnowflakeBasicDataSource) dataSource;
    testableDataSource.setNextConnection(mockConnection);

    Connection result = dataSource.getConnection("user1", "pass1");

    assertSame(mockConnection, result);
    Properties capturedProperties = testableDataSource.getLastProperties();
    assertEquals("user1", capturedProperties.getProperty("user"));
    assertEquals("pass1", capturedProperties.getProperty("password"));
  }

  @Test
  public void shouldNotSetUserPropertyWhenUsernameIsNull() throws Exception {
    Connection mockConnection = createDummyConnection();
    TestableSnowflakeBasicDataSource testableDataSource =
        (TestableSnowflakeBasicDataSource) dataSource;
    testableDataSource.setNextConnection(mockConnection);

    Connection result = dataSource.getConnection(null, "pass1");

    assertSame(mockConnection, result);
    Properties capturedProperties = testableDataSource.getLastProperties();
    assertNull(capturedProperties.getProperty("user"));
    assertEquals("pass1", capturedProperties.getProperty("password"));
  }

  @Test
  public void shouldNotSetPasswordPropertyWhenPasswordIsNull() throws Exception {
    Connection mockConnection = createDummyConnection();
    TestableSnowflakeBasicDataSource testableDataSource =
        (TestableSnowflakeBasicDataSource) dataSource;
    testableDataSource.setNextConnection(mockConnection);

    Connection result = dataSource.getConnection("user1", null);

    assertSame(mockConnection, result);
    Properties capturedProperties = testableDataSource.getLastProperties();
    assertEquals("user1", capturedProperties.getProperty("user"));
    assertNull(capturedProperties.getProperty("password"));
  }

  @Test
  public void shouldThrowSQLExceptionWhenUrlIsBlank() {
    SnowflakeBasicDataSource blankUrlDataSource = new TestableSnowflakeBasicDataSource();
    blankUrlDataSource.setUrl("   ");

    SQLException ex =
        assertThrows(
            SQLException.class, () -> decorated(blankUrlDataSource).getConnection("user", "pass"));
    assertEquals("URL is not set.", ex.getMessage());
  }

  @Test
  public void shouldThrowSQLExceptionWhenUrlIsUnsetOnNoArgGetConnection() {
    SnowflakeBasicDataSource unsetUrlDataSource = new TestableSnowflakeBasicDataSource();
    unsetUrlDataSource.setUser("user");
    unsetUrlDataSource.setPassword("pass");

    SQLException ex =
        assertThrows(SQLException.class, decorated(unsetUrlDataSource)::getConnection);
    assertEquals("URL is not set.", ex.getMessage());
  }

  @Test
  public void shouldNotOpenConnectionWhenUrlIsUnset() throws Exception {
    SnowflakeBasicDataSource unsetUrlDataSource = new TestableSnowflakeBasicDataSource();
    TestableSnowflakeBasicDataSource testable =
        (TestableSnowflakeBasicDataSource) unsetUrlDataSource;
    testable.setNextConnection(createDummyConnection());

    assertThrows(
        SQLException.class, () -> decorated(unsetUrlDataSource).getConnection("user", "pass"));
    assertNull(testable.getLastUrl());
  }

  @Test
  public void shouldThrowSQLExceptionWhenUrlIsBlankOnNoArgGetConnection() {
    SnowflakeBasicDataSource blankUrlDataSource = new TestableSnowflakeBasicDataSource();
    blankUrlDataSource.setUrl("   ");
    blankUrlDataSource.setUser("user");
    blankUrlDataSource.setPassword("pass");

    SQLException ex =
        assertThrows(SQLException.class, decorated(blankUrlDataSource)::getConnection);
    assertEquals("URL is not set.", ex.getMessage());
  }

  @Test
  public void shouldThrowSQLExceptionWhenUrlIsEmpty() {
    SnowflakeBasicDataSource emptyUrlDataSource = new TestableSnowflakeBasicDataSource();
    emptyUrlDataSource.setUrl("");

    SQLException ex =
        assertThrows(
            SQLException.class, () -> decorated(emptyUrlDataSource).getConnection("user", "pass"));
    assertEquals("URL is not set.", ex.getMessage());
  }

  @Test
  public void shouldNotOpenConnectionWhenUrlIsBlank() throws Exception {
    SnowflakeBasicDataSource blankUrlDataSource = new TestableSnowflakeBasicDataSource();
    blankUrlDataSource.setUrl("   ");
    TestableSnowflakeBasicDataSource testable =
        (TestableSnowflakeBasicDataSource) blankUrlDataSource;
    testable.setNextConnection(createDummyConnection());

    assertThrows(
        SQLException.class, () -> decorated(blankUrlDataSource).getConnection("user", "pass"));
    assertNull(testable.getLastUrl());
  }

  @Test
  public void shouldThrowSQLExceptionWhenUrlIsExplicitlyNull() {
    SnowflakeBasicDataSource nullUrlDataSource = new TestableSnowflakeBasicDataSource();
    nullUrlDataSource.setUrl(null);

    SQLException ex =
        assertThrows(
            SQLException.class, () -> decorated(nullUrlDataSource).getConnection("user", "pass"));
    assertEquals("URL is not set.", ex.getMessage());
    assertInstanceOf(IllegalStateException.class, ex.getCause());
  }

  @Test
  public void shouldThrowSQLExceptionWhenUrlIsEmptyOnNoArgGetConnection() {
    SnowflakeBasicDataSource emptyUrlDataSource = new TestableSnowflakeBasicDataSource();
    emptyUrlDataSource.setUrl("");
    emptyUrlDataSource.setUser("user");
    emptyUrlDataSource.setPassword("pass");

    SQLException ex =
        assertThrows(SQLException.class, decorated(emptyUrlDataSource)::getConnection);
    assertEquals("URL is not set.", ex.getMessage());
  }

  @Test
  public void shouldNotOpenConnectionWhenUrlIsUnsetOnNoArgGetConnection() throws Exception {
    SnowflakeBasicDataSource unsetUrlDataSource = new TestableSnowflakeBasicDataSource();
    TestableSnowflakeBasicDataSource testable =
        (TestableSnowflakeBasicDataSource) unsetUrlDataSource;
    testable.setNextConnection(createDummyConnection());

    assertThrows(SQLException.class, decorated(unsetUrlDataSource)::getConnection);
    assertNull(testable.getLastUrl());
  }

  @Test
  public void shouldNotOpenConnectionWhenUrlIsEmpty() throws Exception {
    SnowflakeBasicDataSource emptyUrlDataSource = new TestableSnowflakeBasicDataSource();
    emptyUrlDataSource.setUrl("");
    TestableSnowflakeBasicDataSource testable =
        (TestableSnowflakeBasicDataSource) emptyUrlDataSource;
    testable.setNextConnection(createDummyConnection());

    assertThrows(
        SQLException.class, () -> decorated(emptyUrlDataSource).getConnection("user", "pass"));
    assertNull(testable.getLastUrl());
  }

  @Test
  public void shouldGetUrlReturnNullWhenNeverConfigured() {
    SnowflakeBasicDataSource unsetUrlDataSource = new TestableSnowflakeBasicDataSource();

    assertNull(unsetUrlDataSource.getUrl());
  }

  @Test
  public void shouldThrowSQLExceptionWhenUrlIsUnset() {
    SnowflakeBasicDataSource unsetUrlDataSource = new TestableSnowflakeBasicDataSource();

    SQLException ex =
        assertThrows(
            SQLException.class, () -> decorated(unsetUrlDataSource).getConnection("user", "pass"));
    assertEquals("URL is not set.", ex.getMessage());
  }

  @Test
  public void shouldGetUrlReturnConfiguredUrl() {
    dataSource.setUrl("jdbc:snowflake://custom-url.snowflakecomputing.com");

    assertEquals("jdbc:snowflake://custom-url.snowflakecomputing.com", dataSource.getUrl());
  }

  @Test
  public void shouldBuildUrlFromServerNameWhenUrlNotSet() {
    SnowflakeBasicDataSource ds = new TestableSnowflakeBasicDataSource();
    ds.setServerName("account.snowflakecomputing.com");

    assertEquals("jdbc:snowflake://account.snowflakecomputing.com", ds.getUrl());
  }

  @Test
  public void shouldBuildUrlFromServerNameAndPortWhenUrlNotSet() {
    SnowflakeBasicDataSource ds = new TestableSnowflakeBasicDataSource();
    ds.setServerName("account.snowflakecomputing.com");
    ds.setPortNumber(443);

    assertEquals("jdbc:snowflake://account.snowflakecomputing.com:443", ds.getUrl());
  }

  @Test
  public void shouldPreferExplicitUrlOverServerName() {
    SnowflakeBasicDataSource ds = new TestableSnowflakeBasicDataSource();
    ds.setServerName("ignored.snowflakecomputing.com");
    ds.setPortNumber(443);
    ds.setUrl("jdbc:snowflake://explicit.snowflakecomputing.com");

    assertEquals("jdbc:snowflake://explicit.snowflakecomputing.com", ds.getUrl());
  }

  @Test
  public void shouldReturnNullUrlWhenOnlyPortConfigured() {
    SnowflakeBasicDataSource ds = new TestableSnowflakeBasicDataSource();
    ds.setPortNumber(443);

    assertNull(ds.getUrl());
  }

  @Test
  public void shouldSetSslProperty() {
    SnowflakeBasicDataSource ds = new TestableSnowflakeBasicDataSource();
    ds.setSsl(false);

    assertEquals("false", ds.getProperties().getProperty("ssl"));
  }

  @Test
  public void shouldThrowSQLFeatureNotSupportedExceptionFromGetLogWriter() {
    assertThrows(SQLFeatureNotSupportedException.class, () -> decorated(dataSource).getLogWriter());
  }

  @Test
  public void shouldThrowSQLFeatureNotSupportedExceptionFromSetLogWriter() {
    assertThrows(
        SQLFeatureNotSupportedException.class,
        () -> decorated(dataSource).setLogWriter(new PrintWriter(System.out)));
  }

  @Test
  public void shouldGetLoginTimeoutReturnZeroWhenNotSet() {
    assertEquals(0, dataSource.getLoginTimeout());
  }

  @Test
  public void shouldGetLoginTimeoutReturnSetValue() {
    dataSource.setLoginTimeout(30);

    assertEquals(30, dataSource.getLoginTimeout());
  }

  @Test
  public void shouldThrowSQLFeatureNotSupportedExceptionFromGetParentLogger() {
    assertThrows(
        SQLFeatureNotSupportedException.class, () -> decorated(dataSource).getParentLogger());
  }

  @Test
  public void shouldSupportUnwrapToSnowflakeBasicDataSource() throws Exception {
    assertSame(dataSource, dataSource.unwrap(SnowflakeBasicDataSource.class));
    assertTrue(dataSource.isWrapperFor(SnowflakeBasicDataSource.class));
  }

  @Test
  public void shouldThrowSQLExceptionWhenUnwrappingToUnsupportedInterface() throws Exception {
    DataSource decoratedDataSource = decorated(dataSource);

    assertFalse(decoratedDataSource.isWrapperFor(String.class));
    assertThrows(SQLException.class, () -> decoratedDataSource.unwrap(String.class));
  }

  @Test
  public void shouldStorePropertiesFromSetters() {
    dataSource.setAccount("myaccount");
    dataSource.setDatabase("mydb");
    dataSource.setSchema("myschema");
    dataSource.setRole("myrole");
    dataSource.setWarehouse("mywh");

    Properties props = dataSource.getProperties();
    assertEquals("myaccount", props.getProperty("account"));
    assertEquals("mydb", props.getProperty("database"));
    assertEquals("myschema", props.getProperty("schema"));
    assertEquals("myrole", props.getProperty("role"));
    assertEquals("mywh", props.getProperty("warehouse"));
  }

  @Test
  public void shouldGetPropertiesReturnCopy() {
    dataSource.setAccount("myaccount");

    Properties props = dataSource.getProperties();
    props.setProperty("injected", "value");

    Properties freshProps = dataSource.getProperties();
    assertEquals("myaccount", freshProps.getProperty("account"));
    assertNull(freshProps.getProperty("injected"));
  }

  @Test
  public void shouldSetAuthenticatorStoreProperty() {
    dataSource.setAuthenticator("PROGRAMMATIC_ACCESS_TOKEN");

    Properties props = dataSource.getProperties();
    assertEquals("PROGRAMMATIC_ACCESS_TOKEN", props.getProperty("authenticator"));
  }

  @Test
  public void shouldSetTokenStorePropertyWithoutTouchingAuthenticator() {
    dataSource.setToken("my_pat_token_value");

    Properties props = dataSource.getProperties();
    assertEquals("my_pat_token_value", props.getProperty("token"));
    assertNull(props.getProperty("authenticator"));
  }

  @Test
  public void shouldSetOauthTokenStoreAuthenticatorAndToken() {
    dataSource.setOauthToken("my_oauth_access_token");

    Properties props = dataSource.getProperties();
    assertEquals("OAUTH", props.getProperty("authenticator"));
    assertEquals("my_oauth_access_token", props.getProperty("token"));
  }

  @Test
  public void shouldSetPatStoreAuthenticatorAndToken() {
    dataSource.setPat("my_pat_token_value");

    Properties props = dataSource.getProperties();
    assertEquals("PROGRAMMATIC_ACCESS_TOKEN", props.getProperty("authenticator"));
    assertEquals("my_pat_token_value", props.getProperty("token"));
  }

  @Test
  public void shouldSetPasscodeStorePropertyAndPromoteAuthenticator() {
    dataSource.setPasscode("123456");

    Properties props = dataSource.getProperties();
    assertEquals("123456", props.getProperty("passcode"));
    assertEquals("USERNAME_PASSWORD_MFA", props.getProperty("authenticator"));
  }

  @Test
  public void shouldSetPasscodeInPasswordTrueStorePropertyAndPromoteAuthenticator() {
    dataSource.setPasscodeInPassword(true);

    Properties props = dataSource.getProperties();
    assertEquals("true", props.getProperty("passcodeInPassword"));
    assertEquals("USERNAME_PASSWORD_MFA", props.getProperty("authenticator"));
  }

  @Test
  public void shouldSetPasscodeInPasswordFalseStorePropertyAndNotTouchAuthenticator() {
    dataSource.setPasscodeInPassword(false);

    Properties props = dataSource.getProperties();
    assertEquals("false", props.getProperty("passcodeInPassword"));
    assertNull(props.getProperty("authenticator"));
  }

  @Test
  public void shouldSetClientStoreTemporaryCredentialTrueStorePropertyAndPromoteAuthenticator() {
    dataSource.setClientStoreTemporaryCredential(true);

    Properties props = dataSource.getProperties();
    assertEquals("true", props.getProperty("clientStoreTemporaryCredential"));
    assertEquals("EXTERNALBROWSER", props.getProperty("authenticator"));
  }

  @Test
  public void shouldSetClientStoreTemporaryCredentialFalseStorePropertyAndPromoteAuthenticator() {
    dataSource.setClientStoreTemporaryCredential(false);

    Properties props = dataSource.getProperties();
    assertEquals("false", props.getProperty("clientStoreTemporaryCredential"));
    assertEquals("EXTERNALBROWSER", props.getProperty("authenticator"));
  }

  @Test
  public void shouldStoreOauthSettersAsSnakeCaseProperties() {
    dataSource.setOauthClientId("client-id-value");
    dataSource.setOauthClientSecret("client-secret-value");
    dataSource.setOauthAuthorizationUrl("https://idp.example.com/oauth/authorize");
    dataSource.setOauthTokenRequestUrl("https://idp.example.com/oauth/token");
    dataSource.setOauthRedirectUri("http://127.0.0.1:8080/callback");
    dataSource.setOauthScope("session:role:my_role");
    dataSource.setOauthEnableSingleUseRefreshTokens(true);

    Properties props = dataSource.getProperties();
    assertEquals("client-id-value", props.getProperty("oauth_client_id"));
    assertEquals("client-secret-value", props.getProperty("oauth_client_secret"));
    assertEquals(
        "https://idp.example.com/oauth/authorize", props.getProperty("oauth_authorization_url"));
    assertEquals(
        "https://idp.example.com/oauth/token", props.getProperty("oauth_token_request_url"));
    assertEquals("http://127.0.0.1:8080/callback", props.getProperty("oauth_redirect_uri"));
    assertEquals("session:role:my_role", props.getProperty("oauth_scope"));
    assertEquals("true", props.getProperty("oauth_enable_single_use_refresh_tokens"));
    assertEquals("OAUTH_AUTHORIZATION_CODE", props.getProperty("authenticator"));
  }

  static Stream<Arguments> legacyDataSourceSetterProperties() {
    return Stream.of(
        Arguments.of(
            "application", "MyApp", (DataSourceConfigurer) ds -> ds.setApplication("MyApp")),
        Arguments.of(
            "queryTimeoutSeconds", "120", (DataSourceConfigurer) ds -> ds.setQueryTimeout(120)),
        Arguments.of("maxHttpRetries", "5", (DataSourceConfigurer) ds -> ds.setMaxHttpRetries(5)),
        Arguments.of(
            "putGetMaxRetries", "3", (DataSourceConfigurer) ds -> ds.setPutGetMaxRetries(3)),
        Arguments.of(
            "allowUnderscoresInHost",
            "true",
            (DataSourceConfigurer) ds -> ds.setAllowUnderscoresInHost(true)),
        Arguments.of(
            "proxyHost",
            "proxy.example.com",
            (DataSourceConfigurer) ds -> ds.setProxyHost("proxy.example.com")),
        Arguments.of("proxyPort", "8080", (DataSourceConfigurer) ds -> ds.setProxyPort(8080)),
        Arguments.of(
            "proxyUser", "proxy-user", (DataSourceConfigurer) ds -> ds.setProxyUser("proxy-user")),
        Arguments.of(
            "proxyPassword",
            "proxy-pass",
            (DataSourceConfigurer) ds -> ds.setProxyPassword("proxy-pass")),
        Arguments.of(
            "nonProxyHosts",
            "localhost",
            (DataSourceConfigurer) ds -> ds.setNonProxyHosts("localhost")),
        Arguments.of(
            "enableDiagnostics",
            "true",
            (DataSourceConfigurer) ds -> ds.setEnableDiagnostics(true)),
        Arguments.of(
            "diagnosticsAllowlistFile",
            "/tmp/allowlist.json",
            (DataSourceConfigurer) ds -> ds.setDiagnosticsAllowlistFile("/tmp/allowlist.json")),
        Arguments.of(
            "browser_response_timeout",
            "60",
            (DataSourceConfigurer) ds -> ds.setBrowserResponseTimeout(60)),
        Arguments.of("tracing", "FINE", (DataSourceConfigurer) ds -> ds.setTracing("FINE")),
        Arguments.of(
            "enablePatternSearch",
            "false",
            (DataSourceConfigurer) ds -> ds.setEnablePatternSearch(false)),
        Arguments.of(
            "JDBC_TREAT_DECIMAL_AS_INT",
            "false",
            (DataSourceConfigurer) ds -> ds.setArrowTreatDecimalAsInt(false)),
        Arguments.of(
            "JDBC_DEFAULT_FORMAT_DATE_WITH_TIMEZONE",
            "false",
            (DataSourceConfigurer) ds -> ds.setJDBCDefaultFormatDateWithTimezone(false)),
        Arguments.of(
            "JDBC_GET_DATE_USE_NULL_TIMEZONE",
            "false",
            (DataSourceConfigurer) ds -> ds.setGetDateUseNullTimezone(false)));
  }

  @ParameterizedTest
  @MethodSource("legacyDataSourceSetterProperties")
  void shouldStoreLegacyDataSourceSetterProperties(
      String propertyKey, String expectedValue, DataSourceConfigurer configurer) {
    SnowflakeBasicDataSource ds = new TestableSnowflakeBasicDataSource();
    configurer.configure(ds);

    assertEquals(expectedValue, ds.getProperties().getProperty(propertyKey));
  }

  @Test
  void shouldSetDatabaseNameStoreDatabaseProperty() {
    dataSource.setDatabaseName("mydb");

    assertEquals("mydb", dataSource.getProperties().getProperty("database"));
  }

  @Test
  void shouldSetDisableSamlURLCheckStoreProperty() {
    dataSource.setDisableSamlURLCheck(true);

    assertEquals("true", dataSource.getProperties().getProperty("disable_saml_url_check"));
  }

  @Test
  void shouldSetOktaUsernameStoreProperty() {
    dataSource.setOktaUsername("okta.user@example.com");

    assertEquals("okta.user@example.com", dataSource.getProperties().getProperty("okta_username"));
  }

  @Test
  void shouldSetPrivateKeyStoreBase64AndPromoteAuthenticator() throws Exception {
    PrivateKey privateKey = generateRsaPrivateKey();
    String expectedBase64 = Base64.getEncoder().encodeToString(privateKey.getEncoded());

    dataSource.setPrivateKey(privateKey);

    Properties props = dataSource.getProperties();
    assertEquals(expectedBase64, props.getProperty("private_key"));
    assertEquals("SNOWFLAKE_JWT", props.getProperty("authenticator"));
  }

  @Test
  void shouldSetPrivateKeyFileStoreLocationAndPasswordAndPromoteAuthenticator() {
    dataSource.setPrivateKeyFile("/keys/rsa_key.p8", "secret");

    Properties props = dataSource.getProperties();
    assertEquals("/keys/rsa_key.p8", props.getProperty("private_key_file"));
    assertEquals("secret", props.getProperty("private_key_password"));
    assertEquals("SNOWFLAKE_JWT", props.getProperty("authenticator"));
  }

  @Test
  void shouldNotStorePrivateKeyFilePasswordWhenNullOrEmpty() {
    SnowflakeBasicDataSource nullPwd = new TestableSnowflakeBasicDataSource();
    nullPwd.setPrivateKeyFile("/keys/rsa_key.p8", null);
    assertNull(nullPwd.getProperties().getProperty("private_key_password"));

    SnowflakeBasicDataSource emptyPwd = new TestableSnowflakeBasicDataSource();
    emptyPwd.setPrivateKeyFile("/keys/rsa_key.p8", "");
    assertNull(emptyPwd.getProperties().getProperty("private_key_password"));
  }

  @Test
  void shouldClearPrivateKeyFilePasswordWhenSubsequentCallOmitsPassword() {
    SnowflakeBasicDataSource ds = new TestableSnowflakeBasicDataSource();
    ds.setPrivateKeyFile("/keys/a.p8", "secret");
    assertEquals("secret", ds.getProperties().getProperty("private_key_password"));

    ds.setPrivateKeyFile("/keys/b.p8", null);
    assertEquals("/keys/b.p8", ds.getProperties().getProperty("private_key_file"));
    assertNull(ds.getProperties().getProperty("private_key_password"));

    ds.setPrivateKeyBase64("KEY", "secret");
    ds.setPrivateKeyBase64("KEY2", "");
    assertNull(ds.getProperties().getProperty("private_key_password"));
  }

  @Test
  void shouldSetPrivateKeyBase64StoreValueAndPasswordAndPromoteAuthenticator() {
    dataSource.setPrivateKeyBase64("BASE64KEY", "secret");

    Properties props = dataSource.getProperties();
    assertEquals("BASE64KEY", props.getProperty("private_key"));
    assertEquals("secret", props.getProperty("private_key_password"));
    assertEquals("SNOWFLAKE_JWT", props.getProperty("authenticator"));
  }

  @Test
  void shouldNotStorePrivateKeyBase64PasswordWhenNullOrEmpty() {
    SnowflakeBasicDataSource nullPwd = new TestableSnowflakeBasicDataSource();
    nullPwd.setPrivateKeyBase64("BASE64KEY", null);
    assertNull(nullPwd.getProperties().getProperty("private_key_password"));

    SnowflakeBasicDataSource emptyPwd = new TestableSnowflakeBasicDataSource();
    emptyPwd.setPrivateKeyBase64("BASE64KEY", "");
    assertNull(emptyPwd.getProperties().getProperty("private_key_password"));
  }

  @Test
  void shouldSetBrowserResponseTimeoutAndPromoteAuthenticator() {
    dataSource.setBrowserResponseTimeout(60);

    Properties props = dataSource.getProperties();
    assertEquals("60", props.getProperty("browser_response_timeout"));
    assertEquals("EXTERNALBROWSER", props.getProperty("authenticator"));
  }

  @Test
  void shouldGetConnectionWithTokenAuthAndNoPassword() throws Exception {
    dataSource.setAuthenticator("PROGRAMMATIC_ACCESS_TOKEN");
    dataSource.setToken("pat-token");
    TestableSnowflakeBasicDataSource testable = (TestableSnowflakeBasicDataSource) dataSource;
    Connection mockConnection = createDummyConnection();
    testable.setNextConnection(mockConnection);

    // Reference 4.3.1 threw because password was missing; the universal driver allows token auth
    // without a password (BD#1) and without a username (BD#45).
    Connection result = dataSource.getConnection(null, null);

    assertSame(mockConnection, result);
    Properties props = testable.getLastProperties();
    assertEquals("pat-token", props.getProperty("token"));
    assertNull(props.getProperty("password"));
  }

  @Test
  void shouldRemainSerializableForJndiParity() throws Exception {
    SnowflakeBasicDataSource ds = new SnowflakeBasicDataSource();
    ds.setUrl("jdbc:snowflake://acct.snowflakecomputing.com");
    ds.setAccount("myaccount");
    ds.setLoginTimeout(42);

    ByteArrayOutputStream bytes = new ByteArrayOutputStream();
    try (ObjectOutputStream out = new ObjectOutputStream(bytes)) {
      out.writeObject(ds);
    }

    Object restored;
    try (ObjectInputStream in =
        new ObjectInputStream(new ByteArrayInputStream(bytes.toByteArray()))) {
      restored = in.readObject();
    }

    SnowflakeBasicDataSource restoredDs = (SnowflakeBasicDataSource) restored;
    assertEquals("jdbc:snowflake://acct.snowflakecomputing.com", restoredDs.getUrl());
    assertEquals("myaccount", restoredDs.getProperties().getProperty("account"));
    assertEquals(42, restoredDs.getLoginTimeout());
  }

  @Test
  void shouldNotPersistPerCallCredentialsOntoDataSource() throws Exception {
    TestableSnowflakeBasicDataSource testable = (TestableSnowflakeBasicDataSource) dataSource;
    testable.setNextConnection(createDummyConnection());

    dataSource.getConnection("call-user", "call-pass");

    // BD#44: per-call credentials are applied to the connect copy only, not persisted on the
    // DataSource (reference wrote them into the shared properties map).
    Properties props = dataSource.getProperties();
    assertNull(props.getProperty("user"));
    assertNull(props.getProperty("password"));
  }

  private static PrivateKey generateRsaPrivateKey() throws Exception {
    KeyPairGenerator generator = KeyPairGenerator.getInstance("RSA");
    generator.initialize(2048);
    return generator.generateKeyPair().getPrivate();
  }

  @FunctionalInterface
  private interface DataSourceConfigurer {
    void configure(SnowflakeBasicDataSource dataSource);
  }
}
