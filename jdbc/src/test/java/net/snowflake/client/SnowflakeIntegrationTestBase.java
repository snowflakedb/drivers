package net.snowflake.client;

import java.io.FileInputStream;
import java.io.InputStream;
import java.io.InputStreamReader;
import java.sql.Connection;
import java.sql.DriverManager;
import java.sql.PreparedStatement;
import java.sql.ResultSet;
import java.sql.Statement;
import java.util.Properties;
import java.util.UUID;
import net.snowflake.client.api.driver.SnowflakeDriver;
import org.json.JSONObject;
import org.json.JSONTokener;
import org.junit.jupiter.api.AfterAll;
import org.junit.jupiter.api.BeforeAll;
import org.junit.jupiter.api.TestInstance;

@TestInstance(TestInstance.Lifecycle.PER_CLASS)
public abstract class SnowflakeIntegrationTestBase {
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

  protected Properties loadConnectionProperties() throws Exception {
    // Load parameters.json from test resources
    String paramPath = System.getenv("PARAMETER_PATH");
    if (paramPath == null) {
      paramPath = "/parameters.json";
    }
    JSONObject params;
    try (InputStream input = new FileInputStream(paramPath)) {
      params = new JSONObject(new JSONTokener(new InputStreamReader(input)));
    }
    params = params.getJSONObject("testconnection");

    Properties props = new Properties();
    props.setProperty("user", params.getString("SNOWFLAKE_TEST_USER"));
    props.setProperty("password", params.getString("SNOWFLAKE_TEST_PASSWORD"));
    props.setProperty("db", params.getString("SNOWFLAKE_TEST_DATABASE"));
    props.setProperty("schema", params.getString("SNOWFLAKE_TEST_SCHEMA"));
    props.setProperty("warehouse", params.getString("SNOWFLAKE_TEST_WAREHOUSE"));
    props.setProperty("account", params.getString("SNOWFLAKE_TEST_ACCOUNT"));

    addOptionalConnectionProperties(params, props);
    return props;
  }

  protected Connection openConnection() throws Exception {
    Properties props = loadConnectionProperties();
    String url = buildJdbcUrl(props);
    prepareDriver();
    return DriverManager.getConnection(url, props);
  }

  protected String buildJdbcUrl(Properties props) {
    String defaultUrl =
        "jdbc:snowflake://" + props.getProperty("account") + ".snowflakecomputing.com";
    if (props.getProperty("port") != null) {
      defaultUrl += ":" + props.getProperty("port");
    }
    return props.getProperty("url", defaultUrl);
  }

  protected void ensureDatabaseAndSchema(Connection conn) throws Exception {
    Properties props = loadConnectionProperties();
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

  protected void execute(Connection connection, String sql) throws Exception {
    try (Statement statement = connection.createStatement()) {
      statement.execute(sql);
    }
  }

  protected String createTempTable(Connection connection, String tablePrefix, String columns)
      throws Exception {
    String tableName = tablePrefix + UUID.randomUUID().toString().replace("-", "");
    execute(connection, "CREATE TEMPORARY TABLE " + tableName + " (" + columns + ")");
    return tableName;
  }

  @FunctionalInterface
  protected interface ResultSetConsumer {
    void accept(ResultSet resultSet) throws Exception;
  }

  protected static void withQueryResult(
      Connection connection, String sql, ResultSetConsumer consumer) throws Exception {
    try (Statement statement = connection.createStatement();
        ResultSet resultSet = statement.executeQuery(sql)) {
      consumer.accept(resultSet);
    }
  }

  @FunctionalInterface
  protected interface PreparedStatementSetup {
    void accept(PreparedStatement preparedStatement) throws Exception;
  }

  protected static void withPreparedQueryResult(
      Connection connection, String sql, PreparedStatementSetup setup, ResultSetConsumer consumer)
      throws Exception {
    try (PreparedStatement preparedStatement = connection.prepareStatement(sql)) {
      setup.accept(preparedStatement);
      try (ResultSet resultSet = preparedStatement.executeQuery()) {
        consumer.accept(resultSet);
      }
    }
  }

  private void addOptionalConnectionProperties(JSONObject params, Properties props) {
    if (params.has("SNOWFLAKE_TEST_PORT")) {
      props.setProperty("port", String.valueOf(params.getInt("SNOWFLAKE_TEST_PORT")));
    }

    if (params.has("SNOWFLAKE_TEST_ROLE")) {
      props.setProperty("role", params.getString("SNOWFLAKE_TEST_ROLE"));
    }

    if (params.has("SNOWFLAKE_TEST_SERVER_URL")) {
      props.setProperty("server_url", params.getString("SNOWFLAKE_TEST_SERVER_URL"));
    }

    if (params.has("SNOWFLAKE_TEST_HOST")) {
      props.setProperty("host", params.getString("SNOWFLAKE_TEST_HOST"));
    }

    if (params.has("SNOWFLAKE_TEST_PROTOCOL")) {
      props.setProperty("protocol", params.getString("SNOWFLAKE_TEST_PROTOCOL"));
    }
  }

  private static synchronized void prepareDriver() throws Exception {
    Class.forName(SnowflakeDriver.class.getName());
  }
}
