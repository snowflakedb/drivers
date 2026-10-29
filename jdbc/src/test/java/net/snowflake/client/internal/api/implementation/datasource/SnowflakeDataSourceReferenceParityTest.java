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
 * Guards {@link SnowflakeDataSource} against drift from the pinned snowflake-jdbc setter surface
 * (see jdbc/behavior_differences/config/datasource-trimmed-setters.yaml, BD#31).
 */
class SnowflakeDataSourceReferenceParityTest {

  /**
   * Setter names the universal {@link SnowflakeDataSource} commits to. The surface covers
   * sf_core-supported connection parameters plus JDBC client-side knobs the wrapper already honors
   * (see jdbc/behavior_differences/config/datasource-trimmed-setters.yaml, BD#31).
   *
   * <p>Intentionally absent setters: OCSP ({@code setOcspFailOpen}, BD#46), HTTP header customizers
   * ({@code setHttpHeadersCustomizers}, BD#43), {@code setClientConfigFile}, {@code
   * setNetworkTimeout}, {@code setDisableGcsDefaultCredentials}, and {@code
   * setEnableClientRequestMfaToken} (BD#5). {@code setEnablePutGet} has no setter; the connection
   * property {@code enablePutGet} is still applied. {@code setStringsQuotedForColumnDef} has no
   * setter; {@code STRINGS_QUOTED_FOR_COLUMN_DEF} on the properties map is still read. {@code
   * setEnableClientStoreTemporaryCredential} is {@code setClientStoreTemporaryCredential}.
   *
   * <p>{@code setArrowTreatDecimalAsInt} stores {@code JDBC_ARROW_TREAT_DECIMAL_AS_INT}, matching
   * snowflake-jdbc.
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
                  "setPat",
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
                  "setDisableSocksProxy",
                  "setUseProxy",
                  "setProxyHost",
                  "setProxyPort",
                  "setProxyUser",
                  "setProxyPassword",
                  "setNonProxyHosts",
                  "setProxyProtocol",
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
