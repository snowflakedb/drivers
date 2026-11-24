package net.snowflake.jdbc.e2e.tls;

import static net.snowflake.jdbc.utils.TestParameters.loadDefaultConnectionProperties;
import static net.snowflake.jdbc.utils.TestParameters.withDefaultAuth;
import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertFalse;
import static org.junit.jupiter.api.Assertions.assertNull;
import static org.junit.jupiter.api.Assertions.assertThrows;
import static org.junit.jupiter.api.Assertions.assertTrue;
import static org.junit.jupiter.api.Assumptions.assumeTrue;

import java.io.StringWriter;
import java.nio.charset.StandardCharsets;
import java.nio.file.Files;
import java.nio.file.Path;
import java.security.KeyPairGenerator;
import java.security.PrivateKey;
import java.sql.Connection;
import java.sql.ResultSet;
import java.sql.SQLException;
import java.sql.Statement;
import java.util.Properties;
import java.util.UUID;
import net.snowflake.client.api.driver.SnowflakeDriver;
import net.snowflake.jdbc.utils.PrivateKeyHelper;
import net.snowflake.jdbc.utils.TestParameters;
import net.snowflake.jdbc.utils.WithConnect;
import org.bouncycastle.jce.provider.BouncyCastleProvider;
import org.bouncycastle.openssl.jcajce.JcaPEMWriter;
import org.bouncycastle.openssl.jcajce.JcaPKCS8Generator;
import org.bouncycastle.openssl.jcajce.JceOpenSSLPKCS8EncryptorBuilder;
import org.bouncycastle.operator.OutputEncryptor;
import org.junit.jupiter.api.BeforeEach;
import org.junit.jupiter.api.Test;
import org.junit.jupiter.api.io.TempDir;
import org.junit.jupiter.params.ParameterizedTest;
import org.junit.jupiter.params.provider.CsvSource;

/**
 * Runs only against a FIPS {@code jdbc_bridge}, selected by {@code JDBC_EXPECT_FIPS_BUILD=true}. A
 * standard build accepts disabled certificate verification and 3DES-encrypted keys, so the
 * rejection scenarios here would fail on it.
 */
class FipsBuildTest implements WithConnect {

  private static final String TRIPLE_DES_KEY_PASSWORD = "fips-3des-test-password";

  @TempDir Path tempDir;

  @BeforeEach
  void requireFipsBuild() throws SQLException {
    assumeTrue(
        Boolean.parseBoolean(System.getenv("JDBC_EXPECT_FIPS_BUILD")),
        "JDBC_EXPECT_FIPS_BUILD is not true; FIPS-only scenarios skipped");
    assertTrue(
        new SnowflakeDriver().getFipsStatus().isFips(),
        "JDBC_EXPECT_FIPS_BUILD=true but the loaded jdbc_bridge is not a FIPS build");
  }

  @Test
  void shouldRunShowParametersOverFipsTls() throws Exception {
    // Given A FIPS build connected with the default test credentials
    try (Connection conn = connect(withDefaultAuth(loadDefaultConnectionProperties()));
        Statement stmt = conn.createStatement();
        // When SHOW PARAMETERS is executed
        ResultSet rs = stmt.executeQuery("SHOW PARAMETERS")) {
      // Then At least one parameter row is returned
      assertTrue(rs.next(), "SHOW PARAMETERS returned no rows");
      String key = rs.getString("key");
      assertFalse(rs.wasNull(), "SHOW PARAMETERS key column should be non-null");
      assertFalse(key.isEmpty(), "SHOW PARAMETERS key column should be non-empty");
    }
  }

  @Test
  void shouldFetchTenThousandGeneratedRowsInOrder() throws Exception {
    // Given A FIPS build connected with the default test credentials
    try (Connection conn = connect(withDefaultAuth(loadDefaultConnectionProperties()));
        Statement stmt = conn.createStatement();
        // When A 10000-row generator query is fetched
        ResultSet rs =
            stmt.executeQuery(
                "SELECT seq4() AS n FROM TABLE(GENERATOR(ROWCOUNT => 10000)) ORDER BY n")) {
      // Then Every row arrives in order
      long expected = 0;
      while (rs.next()) {
        long n = rs.getLong(1);
        assertFalse(rs.wasNull(), "generator value should be non-null at row " + expected);
        assertEquals(expected, n, "generator value out of order");
        expected++;
      }
      assertEquals(10000, expected, "unexpected generator row count");
    }
  }

  @Test
  void shouldPutFileToUserStage() throws Exception {
    // Given A local file and a unique path on the user stage
    Path file = tempDir.resolve("fips_put.csv");
    Files.write(file, "1,2,3\n".getBytes(StandardCharsets.UTF_8));
    String stagePath = "@~/fips_put_" + UUID.randomUUID().toString().replace("-", "");
    String fileUri = "file://" + file.toAbsolutePath().toString().replace('\\', '/');

    // PUT and REMOVE take the file URI and stage path as literals and accept no bindings.
    // The URI is a @TempDir path and the stage path ends in a generated UUID.
    try (Connection conn = connect(withDefaultAuth(loadDefaultConnectionProperties()));
        Statement stmt = conn.createStatement()) {
      try {
        // When The file is uploaded with PUT
        try (ResultSet rs = stmt.executeQuery("PUT '" + fileUri + "' " + stagePath)) {
          // Then PUT reports the file as uploaded
          assertTrue(rs.next(), "PUT returned no rows");
          String status = rs.getString("status");
          assertFalse(rs.wasNull(), "PUT status should be non-null");
          assertEquals("UPLOADED", status, "unexpected PUT status");
          assertFalse(rs.next(), "PUT of one file should return one row");
        }
      } finally {
        stmt.execute("REMOVE " + stagePath);
      }
    }
  }

  @Test
  void shouldAuthenticateWithPrivateKeyObject() throws Exception {
    // Given A PrivateKey object for the test user
    Properties props = loadDefaultConnectionProperties();
    props.setProperty("user", TestParameters.get("SNOWFLAKE_TEST_USER"));
    props.put("privateKey", PrivateKeyHelper.loadUnencryptedPrivateKey(unencryptedTestKeyFile()));

    // When Connecting
    try (Connection conn = connect(props)) {
      // Then Login succeeds and a query runs
      assertSelectOneSucceeds(conn);
    }
  }

  @Test
  void shouldAuthenticateWithPrivateKeyFile() throws Exception {
    // Given An unencrypted private_key_file for the test user
    Properties props = loadDefaultConnectionProperties();
    props.setProperty("authenticator", "SNOWFLAKE_JWT");
    props.setProperty("user", TestParameters.get("SNOWFLAKE_TEST_USER"));
    props.setProperty("private_key_file", unencryptedTestKeyFile().toString());

    // When Connecting
    try (Connection conn = connect(props)) {
      // Then Login succeeds and a query runs
      assertSelectOneSucceeds(conn);
    }
  }

  @ParameterizedTest
  @CsvSource({"verify_certificates, false", "tls_skip_verify, true"})
  void shouldRejectDisabledCertificateVerification(String parameter, String value) {
    // Given Connection properties that disable certificate verification
    Properties props = withDefaultAuth(loadDefaultConnectionProperties());
    props.setProperty(parameter, value);

    // When Connecting
    SQLException e = assertThrows(SQLException.class, () -> connect(props).close());

    // Then The FIPS build refuses the configuration and names the parameter
    assertTrue(
        e.getMessage().contains("FIPS builds require TLS certificate verification"),
        "unexpected message: " + e.getMessage());
    assertTrue(e.getMessage().contains(parameter), "unexpected message: " + e.getMessage());
    assertNull(e.getSQLState(), "a client-side rejection carries no SQLSTATE");
    assertEquals(0, e.getErrorCode(), "a client-side rejection carries no vendor code");
  }

  @Test
  void shouldRejectTripleDesEncryptedPrivateKey() throws Exception {
    // Given A private key file wrapped with PBES2 and 3DES-CBC
    Properties props = loadDefaultConnectionProperties();
    props.setProperty("authenticator", "SNOWFLAKE_JWT");
    props.setProperty("user", TestParameters.get("SNOWFLAKE_TEST_USER"));
    props.setProperty("private_key_file", writeTripleDesEncryptedKey().toString());
    props.setProperty("private_key_pwd", TRIPLE_DES_KEY_PASSWORD);

    // When Connecting
    SQLException e = assertThrows(SQLException.class, () -> connect(props).close());

    // Then The FIPS build refuses to unwrap the key
    assertTrue(
        e.getMessage()
            .contains("This private key is encrypted with 3DES-CBC. This is a FIPS build"),
        "unexpected message: " + e.getMessage());
    assertNull(e.getSQLState(), "a client-side rejection carries no SQLSTATE");
    assertEquals(0, e.getErrorCode(), "a client-side rejection carries no vendor code");
  }

  private Path unencryptedTestKeyFile() throws Exception {
    PrivateKeyHelper helper = PrivateKeyHelper.fromParameters(tempDir.resolve("test_key.p8"));
    Path unencrypted = tempDir.resolve("test_key_unencrypted.p8");
    helper.writeUnencryptedKeyFile(unencrypted);
    return unencrypted;
  }

  private Path writeTripleDesEncryptedKey() throws Exception {
    KeyPairGenerator generator = KeyPairGenerator.getInstance("RSA");
    generator.initialize(2048);
    PrivateKey key = generator.generateKeyPair().getPrivate();
    OutputEncryptor encryptor =
        new JceOpenSSLPKCS8EncryptorBuilder(JcaPKCS8Generator.DES3_CBC)
            .setProvider(new BouncyCastleProvider())
            .setPassword(TRIPLE_DES_KEY_PASSWORD.toCharArray())
            .build();
    StringWriter pem = new StringWriter();
    try (JcaPEMWriter writer = new JcaPEMWriter(pem)) {
      writer.writeObject(new JcaPKCS8Generator(key, encryptor));
    }
    Path keyFile = tempDir.resolve("triple_des_key.p8");
    Files.write(keyFile, pem.toString().getBytes(StandardCharsets.UTF_8));
    return keyFile;
  }

  private static void assertSelectOneSucceeds(Connection conn) throws SQLException {
    try (Statement stmt = conn.createStatement();
        ResultSet rs = stmt.executeQuery("SELECT 1")) {
      assertTrue(rs.next(), "SELECT 1 returned no rows");
      assertEquals(1, rs.getInt(1), "unexpected SELECT 1 value");
      assertFalse(rs.next(), "SELECT 1 should return one row");
    }
  }
}
