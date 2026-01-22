package com.snowflake.jdbc;

import java.io.FileInputStream;
import java.io.InputStream;
import java.io.InputStreamReader;
import java.sql.Connection;
import java.sql.DriverManager;
import java.sql.Statement;
import java.util.Properties;
import org.json.JSONObject;
import org.json.JSONTokener;

public abstract class SnowflakeIntegrationTestBase {

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
    SnowflakeDriver.empty();
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
}
