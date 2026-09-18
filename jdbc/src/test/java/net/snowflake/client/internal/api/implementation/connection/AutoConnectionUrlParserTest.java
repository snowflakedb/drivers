package net.snowflake.client.internal.api.implementation.connection;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertFalse;
import static org.junit.jupiter.api.Assertions.assertIterableEquals;
import static org.junit.jupiter.api.Assertions.assertThrows;
import static org.junit.jupiter.api.Assertions.assertTrue;

import java.util.Arrays;
import java.util.Map;
import net.snowflake.client.api.exception.ErrorCode;
import net.snowflake.client.internal.api.implementation.exception.SFSQLException;
import org.junit.jupiter.api.Test;
import org.junit.jupiter.params.ParameterizedTest;
import org.junit.jupiter.params.provider.ValueSource;

class AutoConnectionUrlParserTest {

  @Test
  void shouldExtractConnectionNameFromAutoConnectionUrl() {
    Map<String, String> parameters =
        AutoConnectionUrl.parseParameters(
            "jdbc:snowflake:auto?connectionName=readOnly&warehouse=analytics");

    assertEquals("readOnly", parameters.get("connectionName"));
    assertEquals("analytics", parameters.get("warehouse"));
  }

  @Test
  void shouldParseBareAutoConnectionUrl() {
    assertTrue(AutoConnectionUrl.parseParameters("jdbc:snowflake:auto").isEmpty());
  }

  @Test
  void shouldTreatEmptyConnectionNameAsUnspecified() {
    Map<String, String> parameters =
        AutoConnectionUrl.parseParameters("jdbc:snowflake:auto?connectionName=");

    assertEquals("", parameters.get("connectionName"));
    assertEquals(
        "environmentDefault",
        AutoConnectionUrl.selectConfiguredConnectionName(
            parameters,
            key -> "SNOWFLAKE_DEFAULT_CONNECTION_NAME".equals(key) ? "environmentDefault" : null));
  }

  @Test
  void shouldExtractConnectionNameAlongsideUtf8QueryParameters() {
    Map<String, String> parameters =
        AutoConnectionUrl.parseParameters(
            "jdbc:snowflake:auto?connectionName=read%20only&ключ=значение");

    assertEquals("read only", parameters.get("connectionName"));
    assertEquals("значение", parameters.get("ключ"));
  }

  @ParameterizedTest
  @ValueSource(strings = {"%ZZ", "%AZ", "%ZA"})
  void shouldRejectMalformedPercentEncoding(String encodedValue) {
    SFSQLException thrown =
        assertThrows(
            SFSQLException.class,
            () ->
                AutoConnectionUrl.parseParameters(
                    "jdbc:snowflake:auto?connectionName=" + encodedValue));

    assertEquals(ErrorCode.INVALID_PARAMETER_VALUE, thrown.getErrorCode());
    assertEquals(
        "Invalid URL encoding in JDBC auto connection URL: Malformed percent escape",
        thrown.getMessage());
  }

  @Test
  void shouldMatchConnectionNameCaseInsensitively() {
    Map<String, String> parameters =
        AutoConnectionUrl.parseParameters("jdbc:snowflake:auto?CONNECTIONNAME=readOnly");

    assertEquals(
        "readOnly", AutoConnectionUrl.selectConfiguredConnectionName(parameters, ignored -> null));
  }

  @Test
  void shouldSelectTheProfileNamedByTheSnakeCaseUrlSelector() {
    Map<String, String> parameters =
        AutoConnectionUrl.parseParameters("jdbc:snowflake:auto?connection_name=readOnly");

    assertEquals(
        "readOnly", AutoConnectionUrl.selectConfiguredConnectionName(parameters, ignored -> null));
  }

  @Test
  void shouldPreserveTextualQueryOrder() {
    Map<String, String> parameters =
        AutoConnectionUrl.parseParameters(
            "jdbc:snowflake:auto?warehouse=WH&connectionName=readOnly&database=DB");

    assertIterableEquals(
        Arrays.asList("warehouse", "connectionName", "database"), parameters.keySet());
  }

  @ParameterizedTest
  @ValueSource(
      strings = {
        "jdbc:snowflake:auto?db=FIRST&database=SECOND",
        "jdbc:snowflake:auto?connectionName=FIRST&CONNECTION_NAME=SECOND",
        "jdbc:snowflake:auto?database=FIRST&DATABASE=SECOND"
      })
  void shouldRejectAliasEquivalentOrCaseVariantDuplicates(String url) {
    assertInvalidParameterValue(
        assertThrows(SFSQLException.class, () -> AutoConnectionUrl.parseParameters(url)));
  }

  @Test
  void shouldNotIncludeDecodedParameterNamesInDuplicateErrors() {
    SFSQLException thrown =
        assertThrows(
            SFSQLException.class,
            () ->
                AutoConnectionUrl.parseParameters(
                    "jdbc:snowflake:auto?token%3Dtest_token=1&token%3Dtest_token=2"));

    assertEquals(
        "JDBC auto connection URL query component 2 duplicates an earlier parameter after alias normalization",
        thrown.getMessage());
    assertInvalidParameterValue(thrown);
  }

  @Test
  void shouldRejectInvalidUtf8Bytes() {
    assertInvalidParameterValue(
        assertThrows(
            SFSQLException.class,
            () -> AutoConnectionUrl.parseParameters("jdbc:snowflake:auto?connectionName=%C3%28")));
  }

  @Test
  void shouldRejectAFragmentOnABareAutoUrl() {
    assertInvalidParameterValue(
        assertThrows(
            SFSQLException.class,
            () -> AutoConnectionUrl.parseParameters("jdbc:snowflake:auto#fragment")));
  }

  @Test
  void shouldRejectAFragmentAfterTheAutoUrlQuery() {
    assertInvalidParameterValue(
        assertThrows(
            SFSQLException.class,
            () ->
                AutoConnectionUrl.parseParameters(
                    "jdbc:snowflake:auto?connectionName=readOnly#fragment")));
  }

  @Test
  void shouldNotEchoAMalformedQueryComponent() {
    SFSQLException thrown =
        assertThrows(
            SFSQLException.class,
            () ->
                AutoConnectionUrl.parseParameters(
                    "jdbc:snowflake:auto?connectionName=readOnly&password%3Dtest_password"));
    assertEquals(
        "JDBC auto connection URL query component 2 must contain '=': parameter name and value are not separated",
        thrown.getMessage());
    assertInvalidParameterValue(thrown);
  }

  @Test
  void shouldSkipEmptyNonSelectorValues() {
    Map<String, String> parameters =
        AutoConnectionUrl.parseParameters("jdbc:snowflake:auto?warehouse=&connectionName=readOnly");

    assertFalse(parameters.containsKey("warehouse"));
    assertEquals("readOnly", parameters.get("connectionName"));
  }

  @ParameterizedTest
  @ValueSource(
      strings = {
        "jdbc:snowflake:auto",
        "jdbc:snowflake:auto?connectionName=readOnly",
        "jdbc:snowflake:auto#fragment"
      })
  void shouldRecognizeAutoUrlPrefixes(String url) {
    assertTrue(AutoConnectionUrl.isAutoConnectionUrl(url));
  }

  @ParameterizedTest
  @ValueSource(
      strings = {
        "jdbc:snowflake:auto-invalid",
        "jdbc:snowflake://account.snowflakecomputing.com?connectionName=readOnly",
        "jdbc:snowflake://account.snowflakecomputing.com?x=jdbc:snowflake:auto?y",
        "nonsense:jdbc:snowflake:auto?a=b"
      })
  void shouldRejectAutoUrlSubstringLookalikes(String url) {
    assertFalse(AutoConnectionUrl.isAutoConnectionUrl(url));
  }

  @Test
  void shouldRejectNullAsAnAutoUrl() {
    assertFalse(AutoConnectionUrl.isAutoConnectionUrl(null));
  }

  @Test
  void shouldRejectParsingParametersOfANonAutoUrl() {
    SFSQLException thrown =
        assertThrows(
            SFSQLException.class,
            () ->
                AutoConnectionUrl.parseParameters(
                    "jdbc:snowflake://account.snowflakecomputing.com?password=test_password"));

    assertEquals(
        "Invalid JDBC auto connection URL; expected prefix " + AutoConnectionUrl.PREFIX,
        thrown.getMessage());
    assertInvalidParameterValue(thrown);
  }

  @Test
  void shouldNameAnEmptyQuery() {
    SFSQLException thrown =
        assertThrows(
            SFSQLException.class, () -> AutoConnectionUrl.parseParameters("jdbc:snowflake:auto?"));

    assertEquals("JDBC auto connection URL contains an empty query", thrown.getMessage());
    assertInvalidParameterValue(thrown);
  }

  @Test
  void shouldNameATrailingEmptyQueryComponent() {
    SFSQLException thrown =
        assertThrows(
            SFSQLException.class,
            () ->
                AutoConnectionUrl.parseParameters("jdbc:snowflake:auto?connectionName=readOnly&"));

    assertEquals("JDBC auto connection URL contains an empty query component", thrown.getMessage());
    assertInvalidParameterValue(thrown);
  }

  @Test
  void shouldRejectAnEmptyParameterName() {
    SFSQLException thrown =
        assertThrows(
            SFSQLException.class,
            () -> AutoConnectionUrl.parseParameters("jdbc:snowflake:auto?=value"));

    assertEquals("JDBC auto connection URL contains an empty parameter name", thrown.getMessage());
    assertInvalidParameterValue(thrown);
  }

  @ParameterizedTest
  @ValueSource(strings = {"+", "%20"})
  void shouldRejectAWhitespaceOnlyParameterName(String encodedName) {
    SFSQLException thrown =
        assertThrows(
            SFSQLException.class,
            () -> AutoConnectionUrl.parseParameters("jdbc:snowflake:auto?" + encodedName + "=x"));

    assertEquals(
        "JDBC auto connection URL contains a whitespace-only parameter name", thrown.getMessage());
    assertInvalidParameterValue(thrown);
  }

  @Test
  void shouldRejectAnIncompletePercentEscape() {
    assertInvalidParameterValue(
        assertThrows(
            SFSQLException.class,
            () -> AutoConnectionUrl.parseParameters("jdbc:snowflake:auto?connectionName=%C")));
  }

  @Test
  void shouldRejectAnUnpairedUtf16Surrogate() {
    assertInvalidParameterValue(
        assertThrows(
            SFSQLException.class,
            () ->
                AutoConnectionUrl.parseParameters(
                    "jdbc:snowflake:auto?connectionName=" + '\ud800')));
  }

  @Test
  void shouldDecodePlusAsSpace() {
    Map<String, String> parameters =
        AutoConnectionUrl.parseParameters("jdbc:snowflake:auto?connectionName=read+only");

    assertEquals("read only", parameters.get("connectionName"));
  }

  @Test
  void shouldTreatAWhitespaceSelectorAsUnspecified() {
    assertEquals(
        "environmentDefault",
        AutoConnectionUrl.selectConfiguredConnectionName(
            AutoConnectionUrl.parseParameters("jdbc:snowflake:auto?connectionName=+"),
            key -> "SNOWFLAKE_DEFAULT_CONNECTION_NAME".equals(key) ? "environmentDefault" : null));
  }

  @Test
  void shouldIgnoreEmptyNonSelectorWhenCheckingDuplicates() {
    Map<String, String> parameters =
        AutoConnectionUrl.parseParameters("jdbc:snowflake:auto?warehouse=ANALYTICS&warehouse=");

    assertEquals("ANALYTICS", parameters.get("warehouse"));
    Map<String, String> aliasAfterEmpty =
        AutoConnectionUrl.parseParameters("jdbc:snowflake:auto?db=&database=SALES");
    assertEquals("SALES", aliasAfterEmpty.get("database"));
    assertFalse(aliasAfterEmpty.containsKey("db"));
  }

  private static void assertInvalidParameterValue(SFSQLException thrown) {
    assertEquals(ErrorCode.INVALID_PARAMETER_VALUE, thrown.getErrorCode());
  }
}
