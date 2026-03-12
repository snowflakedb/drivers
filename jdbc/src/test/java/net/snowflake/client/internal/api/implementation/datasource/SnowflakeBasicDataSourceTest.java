package net.snowflake.client.internal.api.implementation.datasource;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertNull;
import static org.junit.jupiter.api.Assertions.assertSame;
import static org.junit.jupiter.api.Assertions.assertThrows;
import static org.mockito.ArgumentMatchers.anyString;
import static org.mockito.Mockito.mock;
import static org.mockito.Mockito.mockStatic;

import java.io.PrintWriter;
import java.sql.Connection;
import java.sql.DriverManager;
import java.sql.SQLFeatureNotSupportedException;
import java.util.Properties;
import org.junit.jupiter.api.BeforeEach;
import org.junit.jupiter.api.Test;
import org.mockito.ArgumentCaptor;
import org.mockito.MockedStatic;

public class SnowflakeBasicDataSourceTest {

  private SnowflakeBasicDataSource dataSource;

  @BeforeEach
  public void setUp() {
    dataSource = new SnowflakeBasicDataSource();
    dataSource.setUrl("jdbc:snowflake://testaccount.snowflakecomputing.com");
  }

  @Test
  public void testGetConnectionDelegatesToGetConnectionWithConfiguredUserAndPassword()
      throws Exception {
    dataSource.setUser("testuser");
    dataSource.setPassword("testpassword");

    ArgumentCaptor<Properties> propertiesCaptor = ArgumentCaptor.forClass(Properties.class);
    Connection mockConnection = mock(Connection.class);
    try (MockedStatic<DriverManager> driverManager = mockStatic(DriverManager.class)) {
      driverManager
          .when(() -> DriverManager.getConnection(anyString(), propertiesCaptor.capture()))
          .thenReturn(mockConnection);

      Connection result = dataSource.getConnection();

      assertSame(mockConnection, result);
      Properties capturedProperties = propertiesCaptor.getValue();
      assertEquals("testuser", capturedProperties.getProperty("user"));
      assertEquals("testpassword", capturedProperties.getProperty("password"));
    }
  }

  @Test
  public void testGetConnectionWithUsernameAndPasswordSetsPropertiesAndReturnsConnection()
      throws Exception {
    ArgumentCaptor<Properties> propertiesCaptor = ArgumentCaptor.forClass(Properties.class);
    Connection mockConnection = mock(Connection.class);
    try (MockedStatic<DriverManager> driverManager = mockStatic(DriverManager.class)) {
      driverManager
          .when(() -> DriverManager.getConnection(anyString(), propertiesCaptor.capture()))
          .thenReturn(mockConnection);

      Connection result = dataSource.getConnection("user1", "pass1");

      assertSame(mockConnection, result);
      Properties capturedProperties = propertiesCaptor.getValue();
      assertEquals("user1", capturedProperties.getProperty("user"));
      assertEquals("pass1", capturedProperties.getProperty("password"));
    }
  }

  @Test
  public void testGetConnectionWithNullUsernameDoesNotSetUserProperty() throws Exception {
    ArgumentCaptor<Properties> propertiesCaptor = ArgumentCaptor.forClass(Properties.class);
    Connection mockConnection = mock(Connection.class);
    try (MockedStatic<DriverManager> driverManager = mockStatic(DriverManager.class)) {
      driverManager
          .when(() -> DriverManager.getConnection(anyString(), propertiesCaptor.capture()))
          .thenReturn(mockConnection);

      Connection result = dataSource.getConnection(null, "pass1");

      assertSame(mockConnection, result);
      Properties capturedProperties = propertiesCaptor.getValue();
      assertNull(capturedProperties.getProperty("user"));
      assertEquals("pass1", capturedProperties.getProperty("password"));
    }
  }

  @Test
  public void testGetConnectionWithNullPasswordDoesNotSetPasswordProperty() throws Exception {
    ArgumentCaptor<Properties> propertiesCaptor = ArgumentCaptor.forClass(Properties.class);
    Connection mockConnection = mock(Connection.class);
    try (MockedStatic<DriverManager> driverManager = mockStatic(DriverManager.class)) {
      driverManager
          .when(() -> DriverManager.getConnection(anyString(), propertiesCaptor.capture()))
          .thenReturn(mockConnection);

      Connection result = dataSource.getConnection("user1", null);

      assertSame(mockConnection, result);
      Properties capturedProperties = propertiesCaptor.getValue();
      assertEquals("user1", capturedProperties.getProperty("user"));
      assertNull(capturedProperties.getProperty("password"));
    }
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
