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
  public void testGetConnectionDelegatesToGetConnectionWithConfiguredUserAndPassword()
      throws Exception {
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
  public void testGetConnectionWithUsernameAndPasswordSetsPropertiesAndReturnsConnection()
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
  public void testGetConnectionWithNullUsernameDoesNotSetUserProperty() throws Exception {
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
  public void testGetConnectionWithNullPasswordDoesNotSetPasswordProperty() throws Exception {
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
  public void testGetUrlReturnsConfiguredUrl() {
    dataSource.setUrl("jdbc:snowflake://custom-url.snowflakecomputing.com");

    assertEquals("jdbc:snowflake://custom-url.snowflakecomputing.com", dataSource.getUrl());
  }

  @Test
  public void testGetLogWriterThrowsSQLFeatureNotSupportedException() {
    assertThrows(SQLFeatureNotSupportedException.class, () -> dataSource.getLogWriter());
  }

  @Test
  public void testSetLogWriterThrowsSQLFeatureNotSupportedException() {
    assertThrows(
        SQLFeatureNotSupportedException.class,
        () -> dataSource.setLogWriter(new PrintWriter(System.out)));
  }

  @Test
  public void testGetLoginTimeoutWhenNotSetReturnsZero() {
    assertEquals(0, dataSource.getLoginTimeout());
  }

  @Test
  public void testGetLoginTimeoutReturnsSetValue() {
    dataSource.setLoginTimeout(30);

    assertEquals(30, dataSource.getLoginTimeout());
  }

  @Test
  public void testGetParentLoggerThrowsSQLFeatureNotSupportedException() {
    assertThrows(SQLFeatureNotSupportedException.class, () -> dataSource.getParentLogger());
  }

  @Test
  public void testIsWrapperForReturnsFalse() {
    assertThrows(
        SQLFeatureNotSupportedException.class, () -> dataSource.isWrapperFor(Object.class));
  }

  @Test
  public void testUnwrapReturnsNull() {
    assertThrows(SQLFeatureNotSupportedException.class, () -> dataSource.unwrap(Object.class));
  }

  @Test
  public void testSettersStorePropertiesCorrectly() {
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
  public void testGetPropertiesReturnsCopy() {
    dataSource.setAccount("myaccount");

    Properties props = dataSource.getProperties();
    props.setProperty("injected", "value");

    Properties freshProps = dataSource.getProperties();
    assertEquals("myaccount", freshProps.getProperty("account"));
    assertNull(freshProps.getProperty("injected"));
  }
}
