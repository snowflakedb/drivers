package net.snowflake.client.api.datasource;

import static org.junit.jupiter.api.Assertions.assertFalse;
import static org.junit.jupiter.api.Assertions.assertNotNull;
import static org.junit.jupiter.api.Assertions.assertThrows;
import static org.junit.jupiter.api.Assertions.assertTrue;

import java.sql.Connection;
import java.sql.ResultSet;
import java.sql.SQLException;
import java.sql.Statement;
import java.util.Properties;
import net.snowflake.client.SnowflakeIntegrationTestBase;
import org.junit.jupiter.api.Test;

public class DataSourceConnectionTests extends SnowflakeIntegrationTestBase {

  @Test
  public void testConnectUsingDataSourceWithExplicitUrl() throws Exception {
    Properties props = loadConnectionProperties();
    SnowflakeDataSource ds = SnowflakeDataSourceFactory.createDataSource();
    ds.setUrl(buildJdbcUrl(props));
    ds.setUser(props.getProperty("user"));
    ds.setPassword(props.getProperty("password"));
    ds.setAccount(props.getProperty("account"));

    try (Connection connection = ds.getConnection()) {
      assertNotNull(connection, "Connection should not be null");
      assertFalse(connection.isClosed(), "Connection should be open");
      assertQueryReturnsCurrentRole(connection);
    }
  }

  @Test
  public void testConnectUsingDataSourceWithExplicitCredentials() throws Exception {
    Properties props = loadConnectionProperties();
    SnowflakeDataSource ds = SnowflakeDataSourceFactory.createDataSource();
    ds.setUrl(buildJdbcUrl(props));
    ds.setAccount(props.getProperty("account"));

    try (Connection connection =
        ds.getConnection(props.getProperty("user"), props.getProperty("password"))) {
      assertNotNull(connection, "Connection should not be null");
      assertFalse(connection.isClosed(), "Connection should be open");
      assertQueryReturnsCurrentRole(connection);
    }
  }

  @Test
  public void testFailToConnectWithInvalidPassword() throws Exception {
    Properties props = loadConnectionProperties();
    SnowflakeDataSource ds = SnowflakeDataSourceFactory.createDataSource();
    ds.setUrl(buildJdbcUrl(props));
    ds.setUser(props.getProperty("user"));
    ds.setPassword("INVALID_PASSWORD_THAT_WILL_NOT_MATCH");
    ds.setAccount(props.getProperty("account"));

    assertThrows(SQLException.class, ds::getConnection);
  }

  @Test
  public void testFailToConnectWithInvalidUser() throws Exception {
    Properties props = loadConnectionProperties();
    SnowflakeDataSource ds = SnowflakeDataSourceFactory.createDataSource();
    ds.setUrl(buildJdbcUrl(props));
    ds.setUser("NONEXISTENT_USER_XYZ_12345");
    ds.setPassword(props.getProperty("password"));
    ds.setAccount(props.getProperty("account"));

    assertThrows(SQLException.class, ds::getConnection);
  }

  @Test
  public void testFailToConnectWhenUserIsNotSet() throws Exception {
    Properties props = loadConnectionProperties();
    SnowflakeDataSource ds = SnowflakeDataSourceFactory.createDataSource();
    ds.setUrl(buildJdbcUrl(props));
    ds.setPassword(props.getProperty("password"));
    ds.setAccount(props.getProperty("account"));

    assertThrows(SQLException.class, ds::getConnection);
  }

  @Test
  public void testFailToConnectWhenPasswordIsNotSet() throws Exception {
    Properties props = loadConnectionProperties();
    SnowflakeDataSource ds = SnowflakeDataSourceFactory.createDataSource();
    ds.setUrl(buildJdbcUrl(props));
    ds.setUser(props.getProperty("user"));
    ds.setAccount(props.getProperty("account"));

    assertThrows(SQLException.class, ds::getConnection);
  }

  @Test
  public void testFailToConnectWithNullUserViaExplicitCredentials() throws Exception {
    Properties props = loadConnectionProperties();
    SnowflakeDataSource ds = SnowflakeDataSourceFactory.createDataSource();
    ds.setUrl(buildJdbcUrl(props));
    ds.setAccount(props.getProperty("account"));

    assertThrows(SQLException.class, () -> ds.getConnection(null, props.getProperty("password")));
  }

  @Test
  public void testFailToConnectWithNullPasswordViaExplicitCredentials() throws Exception {
    Properties props = loadConnectionProperties();
    SnowflakeDataSource ds = SnowflakeDataSourceFactory.createDataSource();
    ds.setUrl(buildJdbcUrl(props));
    ds.setAccount(props.getProperty("account"));

    assertThrows(SQLException.class, () -> ds.getConnection(props.getProperty("user"), null));
  }

  @Test
  public void testConnectionReportsClosedAfterClose() throws Exception {
    Properties props = loadConnectionProperties();
    SnowflakeDataSource ds = SnowflakeDataSourceFactory.createDataSource();
    ds.setUrl(buildJdbcUrl(props));
    ds.setUser(props.getProperty("user"));
    ds.setPassword(props.getProperty("password"));
    ds.setAccount(props.getProperty("account"));

    Connection connection = ds.getConnection();
    assertFalse(connection.isClosed(), "Connection should be open before close");
    connection.close();
    assertTrue(connection.isClosed(), "Connection should be closed after close()");
  }

  @Test
  public void testCreateStatementOnClosedConnectionThrowsSQLException() throws Exception {
    Properties props = loadConnectionProperties();
    SnowflakeDataSource ds = SnowflakeDataSourceFactory.createDataSource();
    ds.setUrl(buildJdbcUrl(props));
    ds.setUser(props.getProperty("user"));
    ds.setPassword(props.getProperty("password"));
    ds.setAccount(props.getProperty("account"));

    Connection connection = ds.getConnection();
    connection.close();

    assertThrows(SQLException.class, connection::createStatement);
  }

  private void assertQueryReturnsCurrentRole(Connection connection) throws Exception {
    try (Statement statement = connection.createStatement();
        ResultSet resultSet = statement.executeQuery("SELECT CURRENT_ROLE()")) {
      assertTrue(resultSet.next(), "Result set should have at least one row");
      assertNotNull(resultSet.getString(1), "CURRENT_ROLE() should not be null");
      assertFalse(resultSet.getString(1).isEmpty(), "CURRENT_ROLE() should not be empty");
    }
  }
}
