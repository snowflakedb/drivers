package net.snowflake.client.api.pooling;

import static org.junit.jupiter.api.TestInstance.Lifecycle.PER_CLASS;

import java.sql.SQLException;
import java.util.Properties;
import net.snowflake.client.SnowflakeIntegrationTestBase;
import net.snowflake.client.api.driver.SnowflakeDriver;
import org.junit.jupiter.api.BeforeAll;
import org.junit.jupiter.api.TestInstance;

@TestInstance(PER_CLASS)
public abstract class PoolingTestBase extends SnowflakeIntegrationTestBase {

  private Properties connectionProperties;

  @FunctionalInterface
  interface SQLErrorThrowingRunnable {
    void run() throws SQLException;
  }

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
}
