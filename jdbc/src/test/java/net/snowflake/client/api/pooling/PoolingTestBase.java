package net.snowflake.client.api.pooling;

import static net.snowflake.jdbc.utils.TestParameters.buildJdbcUrl;
import static net.snowflake.jdbc.utils.TestParameters.get;
import static net.snowflake.jdbc.utils.TestParameters.has;
import static net.snowflake.jdbc.utils.TestParameters.loadDefaultConnectionProperties;
import static net.snowflake.jdbc.utils.TestParameters.withDefaultAuth;
import static net.snowflake.jdbc.utils.TestParameters.withSnowflakeAuth;
import static org.junit.jupiter.api.TestInstance.Lifecycle.PER_CLASS;

import java.sql.SQLException;
import java.util.List;
import java.util.Properties;
import java.util.concurrent.CopyOnWriteArrayList;
import javax.sql.PooledConnection;
import net.snowflake.client.api.driver.SnowflakeDriver;
import net.snowflake.jdbc.utils.PoolingTestCompat;
import net.snowflake.jdbc.utils.SnowflakeIntegrationTestBase;
import org.junit.jupiter.api.AfterEach;
import org.junit.jupiter.api.BeforeAll;
import org.junit.jupiter.api.TestInstance;

@TestInstance(PER_CLASS)
public abstract class PoolingTestBase extends SnowflakeIntegrationTestBase {

  private Properties connectionProperties;

  private final List<PooledConnection> trackedPooledConnections = new CopyOnWriteArrayList<>();

  @FunctionalInterface
  interface SQLErrorThrowingRunnable {
    void run() throws SQLException;
  }

  /**
   * Registers a pooled connection for automatic close in {@link #closeTrackedPooledConnections()},
   * so a failing assertion cannot leak the underlying physical Snowflake session. Returns the same
   * instance for fluent use at the call site.
   */
  protected PooledConnection trackPooledConnection(PooledConnection pooledConnection) {
    trackedPooledConnections.add(pooledConnection);
    return pooledConnection;
  }

  @AfterEach
  void closeTrackedPooledConnections() {
    for (PooledConnection pooledConnection : trackedPooledConnections) {
      try {
        pooledConnection.close();
      } catch (SQLException ignored) {
        // Best-effort cleanup; a test-body close or abort may already have closed it.
      }
    }
    trackedPooledConnections.clear();
  }

  @BeforeAll
  protected void setUp() throws Exception {
    Class.forName(SnowflakeDriver.class.getName());
    // Authenticate the pooled DataSource with key pair (SNOWFLAKE_JWT), matching
    // SnowflakeIntegrationTestBase, so the pooling integration tests run in the JWT-only CI
    // environment instead of hard-requiring SNOWFLAKE_TEST_PASSWORD.
    connectionProperties = withDefaultAuth(loadDefaultConnectionProperties());
  }

  protected SnowflakeConnectionPoolDataSource createConfiguredPoolDataSource() {
    return configurePoolDataSource(connectionProperties);
  }

  /**
   * Builds a pooled DataSource for password-based auth. Only for the credential-overload test,
   * which is gated on {@code SNOWFLAKE_TEST_PASSWORD} being present.
   */
  protected SnowflakeConnectionPoolDataSource createPasswordConfiguredPoolDataSource() {
    return configurePoolDataSource(withSnowflakeAuth(loadDefaultConnectionProperties()));
  }

  /**
   * Configures a pooled DataSource from the given properties, propagating whichever auth method the
   * properties carry: key pair ({@code authenticator} + {@code private_key_base64}[/ {@code
   * private_key_pwd}]) and/or password.
   */
  private SnowflakeConnectionPoolDataSource configurePoolDataSource(Properties props) {
    SnowflakeConnectionPoolDataSource ds =
        SnowflakeConnectionPoolDataSourceFactory.createConnectionPoolDataSource();
    ds.setUrl(buildJdbcUrl(props));
    ds.setAccount(props.getProperty("account"));
    ds.setUser(props.getProperty("user"));
    ds.setDatabaseName(props.getProperty("db"));
    ds.setSchema(props.getProperty("schema"));
    ds.setWarehouse(props.getProperty("warehouse"));
    String authenticator = props.getProperty("authenticator");
    if (authenticator != null) {
      ds.setAuthenticator(authenticator);
    }
    String privateKeyBase64 = props.getProperty("private_key_base64");
    if (privateKeyBase64 != null) {
      ds.setPrivateKeyBase64(privateKeyBase64, props.getProperty("private_key_pwd"));
    }
    String password = props.getProperty("password");
    if (password != null) {
      ds.setPassword(password);
    }
    return ds;
  }

  protected String getUser() {
    return connectionProperties.getProperty("user");
  }

  protected String getPassword() {
    return has("SNOWFLAKE_TEST_PASSWORD") ? get("SNOWFLAKE_TEST_PASSWORD") : null;
  }

  protected void expectConnectionClosed(SQLErrorThrowingRunnable action) {
    PoolingTestCompat.assertThrowsConnectionClosed(action::run);
  }
}
