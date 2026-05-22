package net.snowflake.client.api.datasource;

import static org.junit.jupiter.api.Assertions.assertThrows;

import java.nio.file.Path;
import java.security.PrivateKey;
import java.sql.Connection;
import java.sql.SQLException;
import java.util.Properties;
import net.snowflake.jdbc.utils.PrivateKeyHelper;
import net.snowflake.jdbc.utils.SnowflakeIntegrationTestBase;
import org.junit.jupiter.api.BeforeAll;
import org.junit.jupiter.api.Test;
import org.junit.jupiter.api.io.TempDir;

class PrivateKeyTests extends SnowflakeIntegrationTestBase {

  private Path tempDir;
  private PrivateKeyHelper privateKeyHelper;
  private Properties props;
  private String jdbcUrl;

  @BeforeAll
  void setUp(@TempDir Path tempDir) throws Exception {
    this.tempDir = tempDir;
    privateKeyHelper = PrivateKeyHelper.fromParameters(tempDir.resolve("encrypted_key.p8"));
    props = loadConnectionProperties();
    jdbcUrl = buildJdbcUrl(props);
  }

  private SnowflakeDataSource createDataSource() {
    SnowflakeDataSource ds = SnowflakeDataSourceFactory.createDataSource();
    ds.setUrl(jdbcUrl);
    ds.setUser(props.getProperty("user"));
    ds.setAccount(props.getProperty("account"));
    return ds;
  }

  @Test
  void shouldAuthenticateUsingPrivateFileWithPassword() throws Exception {
    // Given Authentication is set to JWT and private file with password is provided
    SnowflakeDataSource ds = createDataSource();
    ds.setAuthenticator("SNOWFLAKE_JWT");
    ds.setPrivateKeyFile(
        privateKeyHelper.getEncryptedKeyFile().toString(), privateKeyHelper.getPassword());

    // When Trying to Connect
    try (Connection conn = ds.getConnection()) {
      // Then Login is successful and simple query can be executed
      assertSimpleQuerySucceeds(conn);
    }
  }

  @Test
  void shouldAuthenticateUsingUnencryptedPrivateKeyFile() throws Exception {
    // Given Authentication is set to JWT and an unencrypted private key file is provided (no
    // password)
    SnowflakeDataSource ds = createDataSource();
    ds.setAuthenticator("SNOWFLAKE_JWT");
    Path unencryptedKeyFile = tempDir.resolve("unencrypted_key.p8");
    privateKeyHelper.writeUnencryptedKeyFile(unencryptedKeyFile);
    ds.setPrivateKeyFile(unencryptedKeyFile.toString(), null);

    // When Trying to Connect
    try (Connection conn = ds.getConnection()) {
      // Then Login is successful and simple query can be executed
      assertSimpleQuerySucceeds(conn);
    }
  }

  @Test
  void shouldAuthenticateUsingPrivateKeyObject() throws Exception {
    // Given Authentication is set to JWT and a PrivateKey object is provided
    Path unencryptedKeyFile = tempDir.resolve("unencrypted_key_obj.p8");
    privateKeyHelper.writeUnencryptedKeyFile(unencryptedKeyFile);
    PrivateKey key = PrivateKeyHelper.loadUnencryptedPrivateKey(unencryptedKeyFile);

    SnowflakeDataSource ds = createDataSource();
    ds.setPrivateKey(key);

    // When Trying to Connect
    try (Connection conn = ds.getConnection()) {
      // Then Login is successful and simple query can be executed
      assertSimpleQuerySucceeds(conn);
    }
  }

  @Test
  void shouldFailJwtAuthenticationWhenInvalidPrivateKeyProvided() throws Exception {
    // Given Authentication is set to JWT and invalid private key file is provided
    SnowflakeDataSource ds = createDataSource();
    ds.setAuthenticator("SNOWFLAKE_JWT");
    ds.setPrivateKeyFile("/nonexistent/invalid_key.p8", null);

    // When Trying to Connect
    // Then There is error returned
    assertThrows(SQLException.class, ds::getConnection);
  }

  @Test
  void shouldFailJwtAuthenticationWhenNoPrivateFileProvided() throws Exception {
    // Given Authentication is set to JWT
    SnowflakeDataSource ds = createDataSource();
    ds.setAuthenticator("SNOWFLAKE_JWT");

    // When Trying to Connect with no private file provided
    // Then There is error returned
    assertThrows(SQLException.class, ds::getConnection);
  }

  @Test
  void shouldAuthenticateUsingPrivateKeyAsBase64String() throws Exception {
    // Given Authentication is set to JWT and private key is provided as base64-encoded string
    SnowflakeDataSource ds = createDataSource();
    ds.setAuthenticator("SNOWFLAKE_JWT");
    ds.setPrivateKeyBase64(privateKeyHelper.getBase64EncodedKey(), privateKeyHelper.getPassword());

    // When Trying to Connect
    try (Connection conn = ds.getConnection()) {
      // Then Login is successful and simple query can be executed
      assertSimpleQuerySucceeds(conn);
    }
  }

  @Test
  void shouldAutomaticallyUpdateAuthenticatorToJwtIfKeyPairParamsPresent() throws Exception {
    // Given private key or private key file is provided and authenticator is not explicitly set
    SnowflakeDataSource ds = createDataSource();
    ds.setPrivateKeyFile(
        privateKeyHelper.getEncryptedKeyFile().toString(), privateKeyHelper.getPassword());

    // When Trying to Connect
    try (Connection conn = ds.getConnection()) {
      // Then Connector changes authenticator to JWT and login is successful and simple query can be
      // executed
      assertSimpleQuerySucceeds(conn);
    }
  }
}
