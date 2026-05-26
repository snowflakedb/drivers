package net.snowflake.jdbc.e2e.authentication;

import static org.junit.jupiter.api.Assertions.assertThrows;

import java.nio.file.Path;
import java.security.PrivateKey;
import java.sql.Connection;
import java.sql.DriverManager;
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

  @BeforeAll
  void setUp(@TempDir Path tempDir) throws Exception {
    this.tempDir = tempDir;
    privateKeyHelper = PrivateKeyHelper.fromParameters(tempDir.resolve("encrypted_key.p8"));
  }

  @Test
  void shouldAuthenticateUsingPrivateFileWithPassword() throws Exception {
    // Given Authentication is set to JWT and private file with password is provided
    Properties props = loadConnectionProperties();
    props.remove("password");
    props.setProperty("authenticator", "SNOWFLAKE_JWT");
    props.setProperty("private_key_file", privateKeyHelper.getEncryptedKeyFile().toString());
    props.setProperty("private_key_pwd", privateKeyHelper.getPassword());

    // When Trying to Connect
    String url = buildJdbcUrl(props);
    try (Connection conn = DriverManager.getConnection(url, props)) {
      // Then Login is successful and simple query can be executed
      assertSimpleQuerySucceeds(conn);
    }
  }

  @Test
  void shouldAuthenticateUsingUnencryptedPrivateKeyFile() throws Exception {
    // Given Authentication is set to JWT and an unencrypted private key file is provided (no
    // password)
    Properties props = loadConnectionProperties();
    props.remove("password");
    props.setProperty("authenticator", "SNOWFLAKE_JWT");
    Path unencryptedKeyFile = tempDir.resolve("unencrypted_key.p8");
    privateKeyHelper.writeUnencryptedKeyFile(unencryptedKeyFile);
    props.setProperty("private_key_file", unencryptedKeyFile.toString());

    // When Trying to Connect
    String url = buildJdbcUrl(props);
    try (Connection conn = DriverManager.getConnection(url, props)) {
      // Then Login is successful and simple query can be executed
      assertSimpleQuerySucceeds(conn);
    }
  }

  @Test
  void shouldAuthenticateUsingPrivateKeyObject() throws Exception {
    // Given a PrivateKey object is provided directly
    Properties props = loadConnectionProperties();
    props.remove("password");
    Path unencryptedKeyFile = tempDir.resolve("unencrypted_key_obj.p8");
    privateKeyHelper.writeUnencryptedKeyFile(unencryptedKeyFile);
    PrivateKey key = PrivateKeyHelper.loadUnencryptedPrivateKey(unencryptedKeyFile);
    props.put("privateKey", key);

    // When Trying to Connect
    String url = buildJdbcUrl(props);
    try (Connection conn = DriverManager.getConnection(url, props)) {
      // Then Login is successful and simple query can be executed
      assertSimpleQuerySucceeds(conn);
    }
  }

  @Test
  void shouldFailJwtAuthenticationWhenInvalidPrivateKeyProvided() throws Exception {
    // Given Authentication is set to JWT and invalid private key file is provided
    Properties props = loadConnectionProperties();
    props.remove("password");
    props.setProperty("authenticator", "SNOWFLAKE_JWT");
    props.setProperty("private_key_file", "/nonexistent/invalid_key.p8");

    // When Trying to Connect
    String url = buildJdbcUrl(props);

    // Then There is error returned
    assertThrows(SQLException.class, () -> DriverManager.getConnection(url, props));
  }

  @Test
  void shouldFailJwtAuthenticationWhenNoPrivateFileProvided() throws Exception {
    // Given Authentication is set to JWT
    Properties props = loadConnectionProperties();
    props.remove("password");
    props.setProperty("authenticator", "SNOWFLAKE_JWT");

    // When Trying to Connect with no private file provided
    String url = buildJdbcUrl(props);

    // Then There is error returned
    assertThrows(SQLException.class, () -> DriverManager.getConnection(url, props));
  }

  @Test
  void shouldAuthenticateUsingPrivateKeyAsBase64String() throws Exception {
    // Given Authentication is set to JWT and private key is provided as base64-encoded string
    Properties props = loadConnectionProperties();
    props.remove("password");
    props.setProperty("authenticator", "SNOWFLAKE_JWT");
    props.setProperty("private_key_base64", privateKeyHelper.getBase64EncodedKey());
    props.setProperty("private_key_pwd", privateKeyHelper.getPassword());

    // When Trying to Connect
    String url = buildJdbcUrl(props);
    try (Connection conn = DriverManager.getConnection(url, props)) {
      // Then Login is successful and simple query can be executed
      assertSimpleQuerySucceeds(conn);
    }
  }

  @Test
  void shouldAutomaticallyUpdateAuthenticatorToJwtIfKeyPairParamsPresent() throws Exception {
    // Given private key or private key file is provided and authenticator is not explicitly set
    Properties props = loadConnectionProperties();
    props.remove("password");
    props.setProperty("private_key_file", privateKeyHelper.getEncryptedKeyFile().toString());
    props.setProperty("private_key_pwd", privateKeyHelper.getPassword());

    // When Trying to Connect
    String url = buildJdbcUrl(props);
    try (Connection conn = DriverManager.getConnection(url, props)) {
      // Then Connector changes authenticator to JWT and login is successful and simple query can be
      // executed
      assertSimpleQuerySucceeds(conn);
    }
  }
}
