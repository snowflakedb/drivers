package net.snowflake.jdbc.e2e.authentication;

import static net.snowflake.jdbc.utils.TestParameters.loadDefaultConnectionProperties;
import static org.junit.jupiter.api.Assertions.assertThrows;

import java.nio.file.Path;
import java.security.PrivateKey;
import java.sql.Connection;
import java.sql.SQLException;
import java.util.Properties;
import net.snowflake.jdbc.utils.PrivateKeyHelper;
import net.snowflake.jdbc.utils.TestParameters;
import net.snowflake.jdbc.utils.WithConnect;
import net.snowflake.jdbc.utils.WithQueryUtils;
import org.junit.jupiter.api.BeforeAll;
import org.junit.jupiter.api.Test;
import org.junit.jupiter.api.TestInstance;
import org.junit.jupiter.api.TestInstance.Lifecycle;
import org.junit.jupiter.api.function.Executable;
import org.junit.jupiter.api.io.TempDir;

@TestInstance(Lifecycle.PER_CLASS)
class PrivateKeyTests implements WithQueryUtils, WithConnect {

  private static final String USER = TestParameters.get("SNOWFLAKE_TEST_USER");

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
    Properties props = loadDefaultConnectionProperties();
    props.setProperty("authenticator", "SNOWFLAKE_JWT");
    props.setProperty("user", USER);
    props.setProperty("private_key_file", privateKeyHelper.getEncryptedKeyFile().toString());
    props.setProperty("private_key_pwd", privateKeyHelper.getPassword());

    // When Trying to Connect
    try (Connection conn = connect(props)) {
      // Then Login is successful and simple query can be executed
      assertSimpleQuerySucceeds(conn);
    }
  }

  @Test
  void shouldAuthenticateUsingUnencryptedPrivateKeyFile() throws Exception {
    // Given Authentication is set to JWT and an unencrypted private key file is provided (no
    // password)
    Properties props = loadDefaultConnectionProperties();
    props.setProperty("authenticator", "SNOWFLAKE_JWT");
    props.setProperty("user", USER);
    Path unencryptedKeyFile = tempDir.resolve("unencrypted_key.p8");
    privateKeyHelper.writeUnencryptedKeyFile(unencryptedKeyFile);
    props.setProperty("private_key_file", unencryptedKeyFile.toString());

    // When Trying to Connect
    try (Connection conn = connect(props)) {
      // Then Login is successful and simple query can be executed
      assertSimpleQuerySucceeds(conn);
    }
  }

  @Test
  void shouldAuthenticateUsingPrivateKeyObject() throws Exception {
    // Given a PrivateKey object is provided directly
    Properties props = loadDefaultConnectionProperties();
    props.setProperty("user", USER);
    Path unencryptedKeyFile = tempDir.resolve("unencrypted_key_obj.p8");
    privateKeyHelper.writeUnencryptedKeyFile(unencryptedKeyFile);
    PrivateKey key = PrivateKeyHelper.loadUnencryptedPrivateKey(unencryptedKeyFile);
    props.put("privateKey", key);

    // When Trying to Connect
    try (Connection conn = connect(props)) {
      // Then Login is successful and simple query can be executed
      assertSimpleQuerySucceeds(conn);
    }
  }

  @Test
  void shouldFailJwtAuthenticationWhenInvalidPrivateKeyProvided() {
    // Given Authentication is set to JWT and invalid private key file is provided
    Properties props = loadDefaultConnectionProperties();
    props.setProperty("authenticator", "SNOWFLAKE_JWT");
    props.setProperty("user", USER);
    props.setProperty("private_key_file", "/nonexistent/invalid_key.p8");

    // When Trying to Connect
    Executable connect = () -> connect(props);

    // Then There is error returned
    assertThrows(SQLException.class, connect);
  }

  @Test
  void shouldFailJwtAuthenticationWhenNoPrivateFileProvided() throws Exception {
    // Given Authentication is set to JWT
    Properties props = loadDefaultConnectionProperties();
    props.setProperty("authenticator", "SNOWFLAKE_JWT");
    props.setProperty("user", USER);

    // When Trying to Connect with no private file provided
    Executable connect = () -> connect(props);

    // Then There is error returned
    assertThrows(SQLException.class, connect);
  }

  @Test
  void shouldAuthenticateUsingPrivateKeyAsBase64String() throws Exception {
    // Given Authentication is set to JWT and private key is provided as base64-encoded string
    Properties props = loadDefaultConnectionProperties();
    props.setProperty("authenticator", "SNOWFLAKE_JWT");
    props.setProperty("user", USER);
    props.setProperty("private_key_base64", privateKeyHelper.getBase64EncodedKey());
    props.setProperty("private_key_pwd", privateKeyHelper.getPassword());

    // When Trying to Connect
    try (Connection conn = connect(props)) {
      // Then Login is successful and simple query can be executed
      assertSimpleQuerySucceeds(conn);
    }
  }

  @Test
  void shouldAutomaticallyUpdateAuthenticatorToJwtIfKeyPairParamsPresent() throws Exception {
    // Given private key or private key file is provided and authenticator is not explicitly set
    Properties props = loadDefaultConnectionProperties();
    props.setProperty("user", USER);
    props.setProperty("private_key_file", privateKeyHelper.getEncryptedKeyFile().toString());
    props.setProperty("private_key_pwd", privateKeyHelper.getPassword());

    // When Trying to Connect
    try (Connection conn = connect(props)) {
      // Then Connector changes authenticator to JWT and login is successful and simple query can be
      // executed
      assertSimpleQuerySucceeds(conn);
    }
  }
}
