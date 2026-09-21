package net.snowflake.client.internal.api.implementation.parameters;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertFalse;
import static org.junit.jupiter.api.Assertions.assertNull;
import static org.junit.jupiter.api.Assertions.assertSame;
import static org.junit.jupiter.api.Assertions.assertThrows;
import static org.junit.jupiter.api.Assertions.assertTrue;

import java.security.PrivateKey;
import java.util.Map;
import java.util.Properties;
import net.snowflake.client.api.exception.ErrorCode;
import net.snowflake.client.internal.api.implementation.connection.ConnectionString;
import net.snowflake.client.internal.api.implementation.exception.SFSQLException;
import org.junit.jupiter.api.Test;
import org.junit.jupiter.params.ParameterizedTest;
import org.junit.jupiter.params.provider.CsvSource;

class ConnectionOptionsResolverTest {

  @Test
  public void buildConnectionOptionsUsesParsedParamsAndDerivesAccount() {
    Properties input = new Properties();
    Properties resolved =
        ConnectionOptionsResolver.resolve(
            "jdbc:snowflake://globalaccount-12345.global.snowflakecomputing.com?warehouse=TEST_WH&schema=PUBLIC",
            input);

    assertEquals("globalaccount-12345.global.snowflakecomputing.com", resolved.get("host"));
    assertEquals(443, resolved.get("port"));
    assertEquals("https", resolved.get("protocol"));
    assertEquals("globalaccount", resolved.get("account"));
    assertEquals("TEST_WH", resolved.get("warehouse"));
    assertEquals("PUBLIC", resolved.get("schema"));
  }

  @Test
  public void resolveWritesEffectiveUrlBackToUrlProperty() {
    Properties resolved =
        ConnectionOptionsResolver.resolve(
            "jdbc:snowflake://testaccount.snowflakecomputing.com", new Properties());
    assertEquals(
        "jdbc:snowflake://testaccount.snowflakecomputing.com", resolved.getProperty("url"));
  }

  @Test
  public void resolveFallsBackToUrlPropertyWhenInputUrlIsNull() {
    Properties input = new Properties();
    input.setProperty("url", "jdbc:snowflake://fromprops.snowflakecomputing.com");
    Properties resolved = ConnectionOptionsResolver.resolve(null, input);
    assertEquals("jdbc:snowflake://fromprops.snowflakecomputing.com", resolved.getProperty("url"));
    assertEquals("fromprops.snowflakecomputing.com", resolved.getProperty("host"));
  }

  @Test
  public void resolveLeavesUrlUnsetWhenNeitherInputNorPropertySupplied() {
    Properties resolved = ConnectionOptionsResolver.resolve(null, new Properties());
    assertNull(resolved.getProperty("url"));
  }

  @Test
  public void resolveAutoUrlUsesPropertiesOverQueryParameters() {
    Properties input = new Properties();
    input.setProperty("warehouse", "FROM_PROPERTIES");
    input.setProperty("database", "DATABASE_FROM_PROPERTIES");

    Properties resolved =
        ConnectionOptionsResolver.resolve(
            "jdbc:snowflake:auto?connectionName=readOnly&warehouse=FROM_URL&db=DATABASE_FROM_URL",
            input,
            ignored -> "FROM_ENVIRONMENT");

    assertEquals("readOnly", resolved.getProperty("connection_name"));
    assertEquals("FROM_PROPERTIES", resolved.getProperty("warehouse"));
    assertEquals("DATABASE_FROM_PROPERTIES", resolved.getProperty("database"));
    assertFalse(resolved.containsKey("db"));
    assertFalse(resolved.containsKey("url"));
    assertFalse(resolved.containsKey("connectionName"));
  }

  @Test
  public void shouldLeaveDefaultProfileSelectionToCoreWhenEnvironmentIsBlank() {
    Properties input = new Properties();
    input.setProperty("connection_name", "  ");

    Properties resolved =
        ConnectionOptionsResolver.resolve("jdbc:snowflake:auto", input, ignored -> "  ");

    assertFalse(resolved.containsKey("connection_name"));
  }

  @Test
  public void resolveAutoUrlUsesPropertiesSelectorBeforeUrlAndEnvironment() {
    Properties input = new Properties();
    input.setProperty("CONNECTION_NAME", "FROM_PROPERTIES");

    Properties resolved =
        ConnectionOptionsResolver.resolve(
            "jdbc:snowflake:auto?connectionName=FROM_URL", input, ignored -> "FROM_ENVIRONMENT");

    assertEquals("FROM_PROPERTIES", resolved.getProperty("connection_name"));
  }

  @Test
  public void shouldResolveAutoUrlUsingInheritedPropertyDefaults() {
    Properties defaults = new Properties();
    defaults.setProperty("connectionName", "FROM_DEFAULTS");
    defaults.setProperty("warehouse", "DEFAULT_WH");
    Properties input = new Properties(defaults);

    Properties resolved =
        ConnectionOptionsResolver.resolve(
            "jdbc:snowflake:auto?warehouse=URL_WH", input, ignored -> "FROM_ENVIRONMENT");

    assertEquals("FROM_DEFAULTS", resolved.getProperty("connection_name"));
    assertEquals("DEFAULT_WH", resolved.getProperty("warehouse"));
  }

  @Test
  public void shouldPreferRegularUrlQueryOverInheritedPropertyDefaults() {
    Properties defaults = new Properties();
    defaults.setProperty("warehouse", "OLD");
    Properties input = new Properties(defaults);

    Properties resolved =
        ConnectionOptionsResolver.resolve(
            "jdbc:snowflake://acct.snowflakecomputing.com?warehouse=NEW", input);

    assertEquals("NEW", resolved.getProperty("warehouse"));
  }

  @Test
  public void shouldIgnoreInheritedUrlDefaultWhenRegularUrlArgumentIsBlank() {
    Properties defaults = new Properties();
    defaults.setProperty("url", "jdbc:snowflake://fromdefaults.snowflakecomputing.com");
    Properties input = new Properties(defaults);

    Properties resolved = ConnectionOptionsResolver.resolve(null, input);

    assertNull(resolved.getProperty("url"));
    assertNull(resolved.getProperty("host"));
  }

  @Test
  public void shouldPreferADirectPropertyAliasOverAnInheritedAlias() {
    Properties defaults = new Properties();
    defaults.setProperty("loginTimeout", "5");
    Properties input = new Properties(defaults);
    input.put("login_timeout", 10);

    Properties resolved = ConnectionOptionsResolver.resolve("jdbc:snowflake:auto", input);

    assertEquals(10, resolved.get("login_timeout"));
  }

  @Test
  public void shouldKeepADirectTypedPropertyOverAnInheritedStringDefault() {
    Properties defaults = new Properties();
    defaults.setProperty("port", "443");
    Properties input = new Properties(defaults);
    input.put("port", 9999);

    Properties resolved = ConnectionOptionsResolver.resolve("jdbc:snowflake:auto", input);

    assertEquals(9999, resolved.get("port"));
  }

  @Test
  public void resolveAutoUrlTreatsBlankPropertiesSelectorAsUnspecified() {
    Properties input = new Properties();
    input.setProperty("connection_name", "  ");

    Properties resolved =
        ConnectionOptionsResolver.resolve(
            "jdbc:snowflake:auto?connectionName=FROM_URL", input, ignored -> "FROM_ENVIRONMENT");

    assertEquals("FROM_URL", resolved.getProperty("connection_name"));
  }

  @Test
  public void resolveAutoUrlRejectsConflictingPropertyAliasesAndCaseVariants() {
    Properties input = new Properties();
    input.setProperty("connectionName", "FIRST");
    input.setProperty("CONNECTION_NAME", "SECOND");

    SFSQLException exception =
        assertThrows(
            SFSQLException.class,
            () -> ConnectionOptionsResolver.resolve("jdbc:snowflake:auto", input, ignored -> null));

    assertEquals(ErrorCode.INVALID_PARAMETER_VALUE, exception.getErrorCode());
  }

  @Test
  public void resolveDoesNotOverrideTypedValuesWhenKeyAlreadyExists() {
    Properties input = new Properties();
    input.put("port", 9999);
    input.put("ssl", Boolean.TRUE);

    Properties resolved =
        ConnectionOptionsResolver.resolve(
            "jdbc:snowflake://typed.snowflakecomputing.com:443?ssl=off", input);

    assertEquals(9999, resolved.get("port"));
    assertEquals(Boolean.TRUE, resolved.get("ssl"));
  }

  @Test
  public void resolveSkipsBlankQueryParameterKeys() {
    Properties resolved =
        ConnectionOptionsResolver.resolve(
            "jdbc:snowflake://testaccount.snowflakecomputing.com?=blankkey&warehouse=WH",
            new Properties());

    assertFalse(resolved.containsKey(""));
    assertEquals("WH", resolved.get("warehouse"));
  }

  @Test
  public void parseConnectionStringDecodesEscapedValuesAndForcesHttpWhenSslOff() {
    ConnectionString parsed =
        ConnectionString.parse(
            "jdbc:snowflake://testaccount.com:8080?proxyHost=%3d%2f&proxyPort=777&ssl=off",
            new Properties());
    assertTrue(parsed.isValid());
    assertEquals("http", parsed.getScheme());
    assertEquals("testaccount.com", parsed.getHost());
    assertEquals(8080, parsed.getPort());
    assertEquals("testaccount", parsed.getAccount());

    Map<String, Object> params = parsed.getParameters();
    assertEquals("=/", params.get("PROXYHOST"));
    assertEquals("777", params.get("PROXYPORT"));
    assertEquals("off", params.get("SSL"));
    assertEquals("testaccount", params.get("ACCOUNT"));
  }

  @ParameterizedTest
  @CsvSource({
    "jdbc:snowflake://testaccount.com, https, 443",
    "jdbc:snowflake://testaccount.com?ssl=off, http, 80",
    "jdbc:snowflake://http://testaccount.localhost, http, 80"
  })
  public void parseConnectionStringUsesExpectedDefaultPortForEffectiveScheme(
      String url, String expectedScheme, int expectedPort) {
    ConnectionString parsed = ConnectionString.parse(url, new Properties());

    assertTrue(parsed.isValid());
    assertEquals(expectedScheme, parsed.getScheme());
    assertEquals(expectedPort, parsed.getPort());
  }

  @Test
  public void resolveUsesHttpDefaultPortWhenSslOffAndNoPortProvided() {
    Properties resolved =
        ConnectionOptionsResolver.resolve(
            "jdbc:snowflake://testaccount.snowflakecomputing.com?ssl=off", new Properties());

    // ssl is present (from the URL query), so protocol is not auto-derived; sf_core resolves the
    // http scheme from ssl=off.
    assertFalse(resolved.containsKey("protocol"));
    assertEquals("off", resolved.get("ssl"));
    assertEquals(80, resolved.get("port"));
  }

  @Test
  public void shouldNotDeriveProtocolWhenSslSuppliedInProperties() {
    // A caller-supplied ssl (e.g. Metabase's ssl=true) must not also get protocol=https, which
    // sf_core rejects as ConflictingParameters.
    Properties input = new Properties();
    input.setProperty("ssl", "true");

    Properties resolved =
        ConnectionOptionsResolver.resolve("jdbc:snowflake://acct.snowflakecomputing.com", input);

    assertFalse(
        resolved.containsKey("protocol"), "protocol must not be auto-derived alongside ssl");
    assertEquals("true", resolved.get("ssl"));
    assertEquals("acct.snowflakecomputing.com", resolved.get("host"));
  }

  @Test
  public void shouldDeriveProtocolFromSchemeWhenSslAbsent() {
    Properties resolved =
        ConnectionOptionsResolver.resolve(
            "jdbc:snowflake://acct.snowflakecomputing.com", new Properties());

    assertEquals("https", resolved.get("protocol"));
    assertFalse(resolved.containsKey("ssl"));
  }

  @Test
  public void parseConnectStringKeepsSchemeWhenSslOnOverridesUrlSslOffForJdbcCompatibility() {
    Properties input = new Properties();
    input.setProperty("warehouse", "FROM_PROPERTIES");
    input.setProperty("ssl", "on");
    input.setProperty("account", "from_properties_account");

    ConnectionString parsed =
        ConnectionString.parse(
            "jdbc:snowflake://fromurl.snowflakecomputing.com?warehouse=FROM_URL&ssl=off&account=from_url_account",
            input);

    assertTrue(parsed.isValid());
    assertEquals("http", parsed.getScheme());
    assertEquals("from_properties_account", parsed.getAccount());
    assertEquals("FROM_PROPERTIES", parsed.getParameters().get("WAREHOUSE"));
    assertEquals("on", parsed.getParameters().get("SSL"));
    assertEquals("from_properties_account", parsed.getParameters().get("ACCOUNT"));
  }

  @ParameterizedTest
  @CsvSource({"false,http", "true,https"})
  public void parseConnectionStringInterpretsBooleanSslByValue(
      boolean sslValue, String expectedScheme) {
    Properties input = new Properties();
    input.put("ssl", sslValue);

    ConnectionString parsed =
        ConnectionString.parse("jdbc:snowflake://testaccount.snowflakecomputing.com", input);

    assertTrue(parsed.isValid());
    assertEquals(expectedScheme, parsed.getScheme());
    assertEquals(sslValue, parsed.getParameters().get("SSL"));
  }

  @Test
  public void parseConnectionStringRetainsValueSuffixAndKeepsLegacyInvalidPairBehavior() {
    ConnectionString parsed =
        ConnectionString.parse(
            "jdbc:snowflake://testaccount.snowflakecomputing.com?token=abc==&empty=&novalue&=blankkey",
            new Properties());

    assertTrue(parsed.isValid());
    assertEquals("abc==", parsed.getParameters().get("TOKEN"));
    assertFalse(parsed.getParameters().containsKey("EMPTY"));
    assertFalse(parsed.getParameters().containsKey("NOVALUE"));
    assertEquals("blankkey", parsed.getParameters().get(""));
  }

  @Test
  public void shouldPreservePrivateKeyIdentityAcrossAutoUrlOptionMerging() {
    PrivateKey privateKey =
        new PrivateKey() {
          @Override
          public String getAlgorithm() {
            return "RSA";
          }

          @Override
          public String getFormat() {
            return "PKCS#8";
          }

          @Override
          public byte[] getEncoded() {
            return new byte[] {1, 2, 3};
          }
        };
    Properties input = new Properties();
    input.put("privateKey", privateKey);
    input.setProperty("warehouse", "FROM_PROPERTIES");

    Properties resolved =
        ConnectionOptionsResolver.resolve(
            "jdbc:snowflake:auto?connectionName=test&warehouse=FROM_URL", input);

    assertSame(privateKey, resolved.get("private_key"));
    assertEquals("FROM_PROPERTIES", resolved.getProperty("warehouse"));
    assertEquals("test", resolved.getProperty("connection_name"));
  }

  @Test
  public void shouldKeepAutoUrlOptionsAndLeaveDefaultProfileSelectionToCore() {
    Properties resolved =
        ConnectionOptionsResolver.resolve(
            "jdbc:snowflake:auto?warehouse=TEST_WH&schema=PUBLIC",
            new Properties(),
            ignored -> null);

    assertFalse(resolved.containsKey("connection_name"));
    assertEquals("TEST_WH", resolved.getProperty("warehouse"));
    assertEquals("PUBLIC", resolved.getProperty("schema"));
  }

  @Test
  public void shouldNotForwardTheUrlPropertyWhenResolvingAnAutoUrl() {
    Properties input = new Properties();
    input.setProperty("url", "jdbc:snowflake:auto?connectionName=readOnly");

    Properties resolved = ConnectionOptionsResolver.resolve(null, input, ignored -> null);

    assertEquals("readOnly", resolved.getProperty("connection_name"));
    assertFalse(resolved.containsKey("url"));
  }

  @Test
  public void shouldAcceptDuplicatePropertiesAliasesThatAgreeOnTheValue() {
    Properties input = new Properties();
    input.setProperty("connectionName", "readOnly");
    input.setProperty("connection_name", "readOnly");

    Properties resolved = ConnectionOptionsResolver.resolve("jdbc:snowflake:auto", input);

    assertEquals("readOnly", resolved.getProperty("connection_name"));
  }

  @Test
  public void shouldRejectDuplicatePropertiesAliasesWithDifferentRuntimeTypes() {
    Properties input = new Properties();
    input.put("loginTimeout", 10);
    input.setProperty("login_timeout", "10");

    SFSQLException thrown =
        assertThrows(
            SFSQLException.class,
            () -> ConnectionOptionsResolver.resolve("jdbc:snowflake:auto", input, ignored -> null));

    assertEquals(ErrorCode.INVALID_PARAMETER_VALUE, thrown.getErrorCode());
    assertEquals(
        "Conflicting JDBC connection Properties aliases for parameter: login_timeout",
        thrown.getMessage());
  }
}
