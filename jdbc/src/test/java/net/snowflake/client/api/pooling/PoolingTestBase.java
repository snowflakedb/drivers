package net.snowflake.client.api.pooling;

import static java.nio.file.Files.newInputStream;
import static java.nio.file.Paths.get;
import static org.junit.jupiter.api.TestInstance.Lifecycle.PER_CLASS;

import java.io.InputStream;
import java.io.InputStreamReader;
import java.util.Properties;
import net.snowflake.client.api.driver.SnowflakeDriver;
import org.json.JSONObject;
import org.json.JSONTokener;
import org.junit.jupiter.api.BeforeAll;
import org.junit.jupiter.api.TestInstance;

@TestInstance(PER_CLASS)
public abstract class PoolingTestBase {

  private Properties connectionProperties;

  @BeforeAll
  protected void setUp() throws Exception {
    Class.forName(SnowflakeDriver.class.getName());
    connectionProperties = loadConnectionProperties();
  }

  protected SnowflakeConnectionPoolDataSource createConfiguredPoolDataSource() {
    SnowflakeConnectionPoolDataSource ds =
        SnowflakeConnectionPoolDataSourceFactory.createConnectionPoolDataSource();
    ds.setUrl(getUrl());
    ds.setAccount(connectionProperties.getProperty("account"));
    ds.setUser(connectionProperties.getProperty("user"));
    ds.setPassword(connectionProperties.getProperty("password"));
    ds.setDatabaseName(connectionProperties.getProperty("db"));
    ds.setSchema(connectionProperties.getProperty("schema"));
    ds.setWarehouse(connectionProperties.getProperty("warehouse"));
    return ds;
  }

  protected String getUrl() {
    String account = connectionProperties.getProperty("account");
    String port = connectionProperties.getProperty("port");
    return port != null
        ? "jdbc:snowflake://" + account + ".snowflakecomputing.com:" + port
        : "jdbc:snowflake://" + account + ".snowflakecomputing.com";
  }

  protected String getUser() {
    return connectionProperties.getProperty("user");
  }

  protected String getPassword() {
    return connectionProperties.getProperty("password");
  }

  private Properties loadConnectionProperties() throws Exception {
    String paramPath = System.getenv("PARAMETER_PATH");
    if (paramPath == null) {
      paramPath = "/parameters.json";
    }
    JSONObject params;
    try (InputStream input = newInputStream(get(paramPath))) {
      params = new JSONObject(new JSONTokener(new InputStreamReader(input)));
    }
    params = params.getJSONObject("testconnection");

    Properties props = new Properties();
    props.setProperty("user", params.getString("SNOWFLAKE_TEST_USER"));
    props.setProperty("password", params.getString("SNOWFLAKE_TEST_PASSWORD"));
    props.setProperty("db", params.getString("SNOWFLAKE_TEST_DATABASE"));
    props.setProperty("schema", params.getString("SNOWFLAKE_TEST_SCHEMA"));
    props.setProperty(
        "warehouse",
        params.has("SNOWFLAKE_TEST_WAREHOUSE_JDBC")
            ? params.getString("SNOWFLAKE_TEST_WAREHOUSE_JDBC")
            : params.getString("SNOWFLAKE_TEST_WAREHOUSE"));
    props.setProperty("account", params.getString("SNOWFLAKE_TEST_ACCOUNT"));

    if (params.has("SNOWFLAKE_TEST_PORT")) {
      props.setProperty("port", String.valueOf(params.getInt("SNOWFLAKE_TEST_PORT")));
    }
    return props;
  }
}
