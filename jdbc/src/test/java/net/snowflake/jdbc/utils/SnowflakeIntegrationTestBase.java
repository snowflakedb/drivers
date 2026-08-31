package net.snowflake.jdbc.utils;

import static net.snowflake.jdbc.utils.TestParameters.loadDefaultConnectionProperties;
import static net.snowflake.jdbc.utils.TestParameters.withDefaultAuth;

import java.sql.Connection;
import java.sql.DriverManager;
import java.sql.SQLException;
import java.sql.Statement;
import java.util.Properties;
import net.snowflake.client.api.driver.SnowflakeDriver;
import org.junit.jupiter.api.AfterAll;
import org.junit.jupiter.api.BeforeAll;
import org.junit.jupiter.api.TestInstance;

@TestInstance(TestInstance.Lifecycle.PER_CLASS)
public abstract class SnowflakeIntegrationTestBase implements WithQueryUtils {

  private Connection defaultConnection;

  @BeforeAll
  protected void setUpDefaultConnection() throws Exception {
    defaultConnection = openConnection();
    ensureDatabaseAndSchema(defaultConnection);
  }

  @AfterAll
  protected void tearDownDefaultConnection() throws Exception {
    if (defaultConnection != null && !defaultConnection.isClosed()) {
      defaultConnection.close();
    }
    defaultConnection = null;
  }

  protected Connection getDefaultConnection() throws SQLException {
    if (defaultConnection == null) {
      throw new IllegalStateException("Default test connection is not initialized");
    }
    if (defaultConnection.isClosed()) {
      throw new IllegalStateException("Default test connection is closed");
    }
    return defaultConnection;
  }

  protected Connection openConnection() throws SQLException {
    return openConnection(null);
  }

  protected Connection openConnection(String propertyKey, String propertyVal) throws SQLException {
    Properties properties = new Properties();
    properties.put(propertyKey, propertyVal);
    return openConnection(properties);
  }

  protected Connection openConnection(Properties overrides) throws SQLException {
    Properties props = withDefaultAuth(loadDefaultConnectionProperties());

    // Read QUERY_RESULT_FORMAT from environment
    String resultFormat = System.getenv("QUERY_RESULT_FORMAT");
    if (resultFormat != null && !resultFormat.isEmpty()) {
      props.setProperty("JDBC_QUERY_RESULT_FORMAT", resultFormat);
    }

    if (overrides != null) {
      props.putAll(overrides);
    }

    String url = TestParameters.buildJdbcUrl(props);
    prepareDriver();
    return DriverManager.getConnection(url, props);
  }

  protected void ensureDatabaseAndSchema(Connection conn) throws SQLException {
    Properties props = loadDefaultConnectionProperties();

    String database = props.getProperty("db");
    String schema = props.getProperty("schema");
    try (Statement stmt = conn.createStatement()) {
      if (database != null && !database.isEmpty()) {
        stmt.execute("use database " + database);
      }
      if (schema != null && !schema.isEmpty()) {
        stmt.execute("use schema " + schema);
      }
    }
  }

  private static synchronized void prepareDriver() throws SQLException {
    try {
      Class.forName(SnowflakeDriver.class.getName());
    } catch (ClassNotFoundException e) {
      throw new SQLException(e);
    }
  }
}
