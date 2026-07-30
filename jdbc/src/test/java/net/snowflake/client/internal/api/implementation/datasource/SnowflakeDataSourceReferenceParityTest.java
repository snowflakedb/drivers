package net.snowflake.client.internal.api.implementation.datasource;

import static org.junit.jupiter.api.Assertions.assertAll;
import static org.junit.jupiter.api.Assertions.assertTrue;

import java.lang.reflect.Method;
import java.util.Arrays;
import java.util.Collections;
import java.util.HashSet;
import java.util.Set;
import java.util.stream.Collectors;
import net.snowflake.client.api.datasource.SnowflakeDataSource;
import org.junit.jupiter.api.Test;

/**
 * Guards {@link SnowflakeDataSource} against drift from the reference 4.3.1 driver setter surface
 * (see BehaviorDifferences.yaml BD#31).
 */
class SnowflakeDataSourceReferenceParityTest {

  /**
   * Setter names the universal {@link SnowflakeDataSource} commits to. The surface covers
   * sf_core-supported connection parameters plus JDBC client-side knobs the wrapper already honors
   * (see BehaviorDifferences.yaml BD#31).
   *
   * <p>Intentionally dropped setters include OCSP ({@code setOcspFailOpen}), HTTP header
   * customizers ({@code setHttpHeadersCustomizers}), easy-logging ({@code setClientConfigFile}),
   * and other legacy-only or no-op properties such as {@code setNetworkTimeout}, {@code
   * setEnablePutGet}, {@code setStringsQuotedForColumnDef}, {@code setUseProxy}/{@code
   * setProxyProtocol}/{@code setDisableSocksProxy}, {@code setDisableGcsDefaultCredentials}, {@code
   * setEnableClientRequestMfaToken}, and {@code setEnableClientStoreTemporaryCredential} (see BD#5;
   * replaced by {@code setClientStoreTemporaryCredential}).
   *
   * <p>{@code setArrowTreatDecimalAsInt} stores {@code JDBC_TREAT_DECIMAL_AS_INT} rather than
   * legacy {@code JDBC_ARROW_TREAT_DECIMAL_AS_INT}.
   */
  private static final Set<String> REFERENCE_SETTERS =
      Collections.unmodifiableSet(
          new HashSet<>(
              Arrays.asList(
                  "setUrl",
                  "setDatabase",
                  "setDatabaseName",
                  "setSchema",
                  "setWarehouse",
                  "setRole",
                  "setUser",
                  "setServerName",
                  "setPassword",
                  "setPortNumber",
                  "setAccount",
                  "setSsl",
                  "setAuthenticator",
                  "setToken",
                  "setOauthToken",
                  "setPrivateKey",
                  "setPrivateKeyFile",
                  "setPrivateKeyBase64",
                  "setPasscode",
                  "setPasscodeInPassword",
                  "setOktaUsername",
                  "setDisableSamlURLCheck",
                  "setClientStoreTemporaryCredential",
                  "setOauthClientId",
                  "setOauthClientSecret",
                  "setOauthAuthorizationUrl",
                  "setOauthTokenRequestUrl",
                  "setOauthRedirectUri",
                  "setOauthScope",
                  "setOauthEnableSingleUseRefreshTokens",
                  "setApplication",
                  "setAllowUnderscoresInHost",
                  "setQueryTimeout",
                  "setMaxHttpRetries",
                  "setPutGetMaxRetries",
                  "setProxyHost",
                  "setProxyPort",
                  "setProxyUser",
                  "setProxyPassword",
                  "setNonProxyHosts",
                  "setEnableDiagnostics",
                  "setDiagnosticsAllowlistFile",
                  "setBrowserResponseTimeout",
                  "setTracing",
                  "setEnablePatternSearch",
                  "setArrowTreatDecimalAsInt",
                  "setJDBCDefaultFormatDateWithTimezone",
                  "setGetDateUseNullTimezone")));

  @Test
  void shouldNotExposeSettersOutsideTheReferenceSurface() {
    Set<String> universalSetters =
        Arrays.stream(SnowflakeDataSource.class.getDeclaredMethods())
            .map(Method::getName)
            .filter(name -> name.startsWith("set"))
            .collect(Collectors.toSet());

    assertAll(
        universalSetters.stream()
            .map(
                universalSetter ->
                    () ->
                        assertTrue(
                            REFERENCE_SETTERS.contains(universalSetter),
                            () ->
                                "SnowflakeDataSource exposes unexpected setter: "
                                    + universalSetter)));
  }
}
