package net.snowflake.client.internal.api.implementation.datasource;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertNull;
import static org.junit.jupiter.api.Assertions.assertSame;
import static org.junit.jupiter.api.Assertions.assertThrows;

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
  public void shouldGetUrlReturnConfiguredUrl() {
    dataSource.setUrl("jdbc:snowflake://custom-url.snowflakecomputing.com");

    assertEquals("jdbc:snowflake://custom-url.snowflakecomputing.com", dataSource.getUrl());
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
  public void shouldThrowSQLFeatureNotSupportedExceptionFromIsWrapperFor() {
    assertThrows(
        SQLFeatureNotSupportedException.class, () -> dataSource.isWrapperFor(Object.class));
  }

  @Test
  public void shouldThrowSQLFeatureNotSupportedExceptionFromUnwrap() {
    assertThrows(SQLFeatureNotSupportedException.class, () -> dataSource.unwrap(Object.class));
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
}
