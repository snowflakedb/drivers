package net.snowflake.jdbc.utils;

import java.sql.Connection;
import java.sql.DriverManager;
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

  protected Connection getDefaultConnection() throws Exception {
    if (defaultConnection == null) {
      throw new IllegalStateException("Default test connection is not initialized");
    }
    if (defaultConnection.isClosed()) {
      throw new IllegalStateException("Default test connection is closed");
    }
    return defaultConnection;
  }

  protected Connection openConnection() throws Exception {
    Properties props = TestParameters.loadConnectionProperties();

    // Read QUERY_RESULT_FORMAT from environment
    String resultFormat = System.getenv("QUERY_RESULT_FORMAT");
    if (resultFormat != null && !resultFormat.isEmpty()) {
      props.setProperty("PYTHON_CONNECTOR_QUERY_RESULT_FORMAT", resultFormat);
    }

    String url = TestParameters.buildJdbcUrl(props);
    prepareDriver();
    return DriverManager.getConnection(url, props);
  }

  protected void ensureDatabaseAndSchema(Connection conn) throws Exception {
    Properties props = TestParameters.loadConnectionProperties();

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

  private static synchronized void prepareDriver() throws Exception {
    Class.forName(SnowflakeDriver.class.getName());
  }
}
