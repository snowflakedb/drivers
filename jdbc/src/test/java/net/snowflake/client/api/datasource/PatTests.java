package net.snowflake.client.api.datasource;

import static org.junit.jupiter.api.Assertions.assertThrows;

import java.sql.Connection;
import java.sql.SQLException;
import java.util.Properties;
import net.snowflake.jdbc.utils.PatTokenHelper;
import net.snowflake.jdbc.utils.SkipOldDriver;
import net.snowflake.jdbc.utils.SnowflakeIntegrationTestBase;
import org.junit.jupiter.api.AfterAll;
import org.junit.jupiter.api.BeforeAll;
import org.junit.jupiter.api.Test;

class PatTests extends SnowflakeIntegrationTestBase {

  private final PatTokenHelper patHelper = new PatTokenHelper();
  private Properties props;
  private String jdbcUrl;

  @BeforeAll
  void setUp() throws Exception {
    props = loadConnectionProperties();
    jdbcUrl = buildJdbcUrl(props);
    try (Connection conn = openConnection()) {
      patHelper.create(conn, props.getProperty("user"), props.getProperty("role"));
    }
  }

  @AfterAll
  void tearDown() throws Exception {
    try (Connection conn = openConnection()) {
      patHelper.cleanup(conn, props.getProperty("user"));
    }
  }

  private SnowflakeDataSource createDataSource() {
    SnowflakeDataSource ds = SnowflakeDataSourceFactory.createDataSource();
    ds.setUrl(jdbcUrl);
    ds.setUser(props.getProperty("user"));
    ds.setAccount(props.getProperty("account"));
    return ds;
  }

  @Test
  void shouldAuthenticateUsingPatAsPassword() throws Exception {
    SnowflakeDataSource ds = createDataSource();
    ds.setPassword(patHelper.getTokenSecret());

    try (Connection conn = ds.getConnection()) {
      assertSimpleQuerySucceeds(conn);
    }
  }

  @Test
  @SkipOldDriver("BD#1")
  void shouldAuthenticateUsingPatAsToken() throws Exception {
    SnowflakeDataSource ds = createDataSource();
    ds.setAuthenticator("PROGRAMMATIC_ACCESS_TOKEN");
    ds.setToken(patHelper.getTokenSecret());

    try (Connection conn = ds.getConnection()) {
      assertSimpleQuerySucceeds(conn);
    }
  }

  @Test
  @SkipOldDriver("BD#1")
  void shouldAuthenticateUsingPatAsTokenWithLowercaseAuthenticator() throws Exception {
    SnowflakeDataSource ds = createDataSource();
    ds.setAuthenticator("programmatic_access_token");
    ds.setToken(patHelper.getTokenSecret());

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
