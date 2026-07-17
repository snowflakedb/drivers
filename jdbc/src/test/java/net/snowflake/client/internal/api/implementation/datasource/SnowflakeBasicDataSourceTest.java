package net.snowflake.client.internal.api.implementation.datasource;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertFalse;
import static org.junit.jupiter.api.Assertions.assertInstanceOf;
import static org.junit.jupiter.api.Assertions.assertNull;
import static org.junit.jupiter.api.Assertions.assertSame;
import static org.junit.jupiter.api.Assertions.assertThrows;
import static org.junit.jupiter.api.Assertions.assertTrue;

import java.io.PrintWriter;
import java.lang.reflect.Proxy;
import java.sql.Connection;
import java.sql.SQLException;
import java.sql.SQLFeatureNotSupportedException;
import java.util.Properties;
import org.junit.jupiter.api.BeforeEach;
import org.junit.jupiter.api.Test;

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
    protected Connection openConnection(String url, Properties properties) throws SQLException {
      this.lastUrl = url;
      this.lastProperties = new Properties();
      this.lastProperties.putAll(properties);
      return nextConnection;
    }
  }

  private SnowflakeBasicDataSource dataSource;

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
        assertThrows(SQLException.class, () -> blankUrlDataSource.getConnection("user", "pass"));
    assertEquals("URL is not set.", ex.getMessage());
  }

  @Test
  public void shouldThrowSQLExceptionWhenUrlIsUnsetOnNoArgGetConnection() {
    SnowflakeBasicDataSource unsetUrlDataSource = new TestableSnowflakeBasicDataSource();
    unsetUrlDataSource.setUser("user");
    unsetUrlDataSource.setPassword("pass");

    SQLException ex = assertThrows(SQLException.class, unsetUrlDataSource::getConnection);
    assertEquals("URL is not set.", ex.getMessage());
  }

  @Test
  public void shouldNotOpenConnectionWhenUrlIsUnset() throws Exception {
    SnowflakeBasicDataSource unsetUrlDataSource = new TestableSnowflakeBasicDataSource();
    TestableSnowflakeBasicDataSource testable =
        (TestableSnowflakeBasicDataSource) unsetUrlDataSource;
    testable.setNextConnection(createDummyConnection());

    assertThrows(SQLException.class, () -> unsetUrlDataSource.getConnection("user", "pass"));
    assertNull(testable.getLastUrl());
  }

  @Test
  public void shouldThrowSQLExceptionWhenUrlIsBlankOnNoArgGetConnection() {
    SnowflakeBasicDataSource blankUrlDataSource = new TestableSnowflakeBasicDataSource();
    blankUrlDataSource.setUrl("   ");
    blankUrlDataSource.setUser("user");
    blankUrlDataSource.setPassword("pass");

    SQLException ex = assertThrows(SQLException.class, blankUrlDataSource::getConnection);
    assertEquals("URL is not set.", ex.getMessage());
  }

  @Test
  public void shouldThrowSQLExceptionWhenUrlIsEmpty() {
    SnowflakeBasicDataSource emptyUrlDataSource = new TestableSnowflakeBasicDataSource();
    emptyUrlDataSource.setUrl("");

    SQLException ex =
        assertThrows(SQLException.class, () -> emptyUrlDataSource.getConnection("user", "pass"));
    assertEquals("URL is not set.", ex.getMessage());
  }

  @Test
  public void shouldNotOpenConnectionWhenUrlIsBlank() throws Exception {
    SnowflakeBasicDataSource blankUrlDataSource = new TestableSnowflakeBasicDataSource();
    blankUrlDataSource.setUrl("   ");
    TestableSnowflakeBasicDataSource testable =
        (TestableSnowflakeBasicDataSource) blankUrlDataSource;
    testable.setNextConnection(createDummyConnection());

    assertThrows(SQLException.class, () -> blankUrlDataSource.getConnection("user", "pass"));
    assertNull(testable.getLastUrl());
  }

  @Test
  public void shouldThrowSQLExceptionWhenUrlIsExplicitlyNull() {
    SnowflakeBasicDataSource nullUrlDataSource = new TestableSnowflakeBasicDataSource();
    nullUrlDataSource.setUrl(null);

    SQLException ex =
        assertThrows(SQLException.class, () -> nullUrlDataSource.getConnection("user", "pass"));
    assertEquals("URL is not set.", ex.getMessage());
    assertInstanceOf(IllegalStateException.class, ex.getCause());
  }

  @Test
  public void shouldThrowSQLExceptionWhenUrlIsEmptyOnNoArgGetConnection() {
    SnowflakeBasicDataSource emptyUrlDataSource = new TestableSnowflakeBasicDataSource();
    emptyUrlDataSource.setUrl("");
    emptyUrlDataSource.setUser("user");
    emptyUrlDataSource.setPassword("pass");

    SQLException ex = assertThrows(SQLException.class, emptyUrlDataSource::getConnection);
    assertEquals("URL is not set.", ex.getMessage());
  }

  @Test
  public void shouldNotOpenConnectionWhenUrlIsUnsetOnNoArgGetConnection() throws Exception {
    SnowflakeBasicDataSource unsetUrlDataSource = new TestableSnowflakeBasicDataSource();
    TestableSnowflakeBasicDataSource testable =
        (TestableSnowflakeBasicDataSource) unsetUrlDataSource;
    testable.setNextConnection(createDummyConnection());

    assertThrows(SQLException.class, unsetUrlDataSource::getConnection);
    assertNull(testable.getLastUrl());
  }

  @Test
  public void shouldNotOpenConnectionWhenUrlIsEmpty() throws Exception {
    SnowflakeBasicDataSource emptyUrlDataSource = new TestableSnowflakeBasicDataSource();
    emptyUrlDataSource.setUrl("");
    TestableSnowflakeBasicDataSource testable =
        (TestableSnowflakeBasicDataSource) emptyUrlDataSource;
    testable.setNextConnection(createDummyConnection());

    assertThrows(SQLException.class, () -> emptyUrlDataSource.getConnection("user", "pass"));
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
        assertThrows(SQLException.class, () -> unsetUrlDataSource.getConnection("user", "pass"));
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
    assertThrows(SQLFeatureNotSupportedException.class, () -> dataSource.getLogWriter());
  }

  @Test
  public void shouldThrowSQLFeatureNotSupportedExceptionFromSetLogWriter() {
    assertThrows(
        SQLFeatureNotSupportedException.class,
        () -> dataSource.setLogWriter(new PrintWriter(System.out)));
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
    assertThrows(SQLFeatureNotSupportedException.class, () -> dataSource.getParentLogger());
  }

  @Test
  public void shouldSupportUnwrapToSnowflakeBasicDataSource() throws Exception {
    assertSame(dataSource, dataSource.unwrap(SnowflakeBasicDataSource.class));
    assertTrue(dataSource.isWrapperFor(SnowflakeBasicDataSource.class));
  }

  @Test
  public void shouldThrowSQLExceptionWhenUnwrappingToUnsupportedInterface() throws Exception {
    assertFalse(dataSource.isWrapperFor(String.class));
    assertThrows(SQLException.class, () -> dataSource.unwrap(String.class));
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
  public void shouldSetTokenStoreProperty() {
    dataSource.setToken("my_pat_token_value");

    Properties props = dataSource.getProperties();
    assertEquals("my_pat_token_value", props.getProperty("token"));
  }

  @Test
  public void shouldSetPasscodeStorePropertyAndNotTouchAuthenticator() {
    dataSource.setPasscode("123456");

    Properties props = dataSource.getProperties();
    assertEquals("123456", props.getProperty("passcode"));
    assertNull(props.getProperty("authenticator"));
  }

  @Test
  public void shouldSetPasscodeInPasswordTrueStorePropertyAndNotTouchAuthenticator() {
    dataSource.setPasscodeInPassword(true);

    Properties props = dataSource.getProperties();
    assertEquals("true", props.getProperty("passcodeInPassword"));
    assertNull(props.getProperty("authenticator"));
  }

  @Test
  public void shouldSetPasscodeInPasswordFalseStorePropertyAndNotTouchAuthenticator() {
    dataSource.setPasscodeInPassword(false);

    Properties props = dataSource.getProperties();
    assertEquals("false", props.getProperty("passcodeInPassword"));
    assertNull(props.getProperty("authenticator"));
  }

  @Test
  public void shouldSetClientStoreTemporaryCredentialTrueStoreProperty() {
    dataSource.setClientStoreTemporaryCredential(true);

    Properties props = dataSource.getProperties();
    assertEquals("true", props.getProperty("clientStoreTemporaryCredential"));
    assertNull(props.getProperty("authenticator"));
  }

  @Test
  public void shouldSetClientStoreTemporaryCredentialFalseStoreProperty() {
    dataSource.setClientStoreTemporaryCredential(false);

    Properties props = dataSource.getProperties();
    assertEquals("false", props.getProperty("clientStoreTemporaryCredential"));
    assertNull(props.getProperty("authenticator"));
  }

  @Test
  public void testOauthSettersStoreSnakeCaseProperties() {
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
  }
}
