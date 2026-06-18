package net.snowflake.client.api.datasource;

import static net.snowflake.jdbc.utils.TestParameters.buildJdbcUrl;
import static net.snowflake.jdbc.utils.TestParameters.loadDefaultConnectionProperties;
import static org.junit.jupiter.api.Assertions.assertThrows;

import java.sql.Connection;
import java.sql.SQLException;
import java.util.Properties;
import net.snowflake.jdbc.utils.SkipOldDriver;
import net.snowflake.jdbc.utils.TestParameters;
import net.snowflake.jdbc.utils.WithQueryUtils;
import org.junit.jupiter.api.Test;

class PatTests implements WithQueryUtils {

  private static final String USER = TestParameters.get("SNOWFLAKE_TEST_USER");
  private static final String PAT = TestParameters.get("SNOWFLAKE_TEST_PAT");

  /**
   * Build a JDBC URL using the explicit {@code host} property when available, falling back to the
   * account-derived URL. The default {@link TestParameters#buildJdbcUrl} always uses {@code
   * {account}.snowflakecomputing.com}, which may differ from {@code SNOWFLAKE_TEST_HOST} (e.g. a
   * regional endpoint). Since {@link SnowflakeDataSource} has no {@code setHost} method, the host
   * must be baked into the URL so the Rust core connects to the correct endpoint.
   */
  private static String buildJdbcUrlFromHost(Properties props) {
    String host = props.getProperty("host");
    if (host == null) {
      return buildJdbcUrl(props);
    }
    String url = "jdbc:snowflake://" + host;
    if (props.getProperty("port") != null) {
      url += ":" + props.getProperty("port");
    }
    return url;
  }

  private SnowflakeDataSource createDataSource() {
    Properties props = loadDefaultConnectionProperties();
    SnowflakeDataSource ds = SnowflakeDataSourceFactory.createDataSource();
    ds.setUrl(buildJdbcUrlFromHost(props));
    ds.setUser(USER);
    ds.setAccount(props.getProperty("account"));
    return ds;
  }

  @Test
  void shouldAuthenticateUsingPatAsPassword() throws Exception {
    SnowflakeDataSource ds = createDataSource();
    ds.setPassword(PAT);

    try (Connection conn = ds.getConnection()) {
      assertSimpleQuerySucceeds(conn);
    }
  }

  @Test
  @SkipOldDriver("BD#1")
  void shouldAuthenticateUsingPatAsToken() throws Exception {
    SnowflakeDataSource ds = createDataSource();
    ds.setAuthenticator("PROGRAMMATIC_ACCESS_TOKEN");
    ds.setToken(PAT);

    try (Connection conn = ds.getConnection()) {
      assertSimpleQuerySucceeds(conn);
    }
  }

  @Test
  @SkipOldDriver("BD#1")
  void shouldAuthenticateUsingPatAsTokenWithLowercaseAuthenticator() throws Exception {
    SnowflakeDataSource ds = createDataSource();
    ds.setAuthenticator("programmatic_access_token");
    ds.setToken(PAT);

    try (Connection conn = ds.getConnection()) {
      assertSimpleQuerySucceeds(conn);
    }
  }

  @Test
  void shouldFailPatAuthenticationWhenInvalidTokenProvided() {
    SnowflakeDataSource ds = createDataSource();
    ds.setAuthenticator("PROGRAMMATIC_ACCESS_TOKEN");
    ds.setToken("invalid_token_12345");

    assertThrows(SQLException.class, ds::getConnection);
  }
}
