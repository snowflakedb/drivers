package net.snowflake.client.internal.api.implementation.connection;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertFalse;
import static org.junit.jupiter.api.Assertions.assertNull;
import static org.junit.jupiter.api.Assertions.assertThrows;
import static org.junit.jupiter.api.Assertions.assertTrue;
import static org.mockito.ArgumentMatchers.any;
import static org.mockito.Mockito.verify;
import static org.mockito.Mockito.verifyNoInteractions;
import static org.mockito.Mockito.when;

import java.sql.SQLException;
import java.util.Map;
import java.util.Properties;
import java.util.function.Function;
import java.util.stream.Stream;
import net.snowflake.client.api.exception.ErrorCode;
import net.snowflake.client.internal.api.implementation.exception.CoreException;
import net.snowflake.client.internal.api.implementation.exception.SFSQLException;
import net.snowflake.client.internal.api.implementation.exception.SqlExceptionMapper;
import net.snowflake.client.internal.unicore.CoreDriverApi;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.ConfigSetting;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.ConnectionHandle;
import org.junit.jupiter.api.BeforeEach;
import org.junit.jupiter.api.Test;
import org.junit.jupiter.params.ParameterizedTest;
import org.junit.jupiter.params.provider.MethodSource;
import org.mockito.ArgumentCaptor;

public class AutoConnectionPrefixTests {
  private CoreDriverApi coreDriverApi;

  @BeforeEach
  public void setUp() throws Exception {
    coreDriverApi = SnowflakeConnectionImplTestFixtures.newMockCoreApiWithCloseStubs();
  }

  @Test
  public void shouldSelectTheConnectionNamedInTheAutoConnectionURL() throws Exception {
    // Given a JDBC caller selects the name readOnly
    String connectionName = "readOnly";

    // When a JDBC connection is requested with jdbc:snowflake:auto?connectionName=readOnly
    Map<String, ConfigSetting> options =
        connectAndCaptureOptions(
            "jdbc:snowflake:auto?connectionName=" + connectionName, ignored -> null);

    // Then JDBC asks sf_core to select the readOnly connection profile
    assertEquals(connectionName, options.get("connection_name").getStringValue());
  }

  @Test
  // spotless:off
  public void shouldPreferTheConnectionNamedInTheAutoConnectionURLOverTheEnvironmentDefault() throws Exception {
    // spotless:on
    // Given a JDBC caller selects the name readOnly
    String connectionName = "readOnly";

    // And SNOWFLAKE_DEFAULT_CONNECTION_NAME is set to environmentDefault
    Function<String, String> environment = ignored -> "environmentDefault";

    // When a JDBC connection is requested with jdbc:snowflake:auto?connectionName=readOnly
    Map<String, ConfigSetting> options =
        connectAndCaptureOptions(
            "jdbc:snowflake:auto?connectionName=" + connectionName, environment);

    // Then JDBC asks sf_core to select the readOnly connection profile
    assertEquals(connectionName, options.get("connection_name").getStringValue());
  }

  @Test
  public void shouldSelectTheConnectionNamedByTheEnvironmentForABareAutoConnectionURL()
      throws Exception {
    // Given the process default connection name is environmentDefault
    String connectionName = "environmentDefault";

    // And SNOWFLAKE_DEFAULT_CONNECTION_NAME is set to environmentDefault
    Function<String, String> environment =
        key -> "SNOWFLAKE_DEFAULT_CONNECTION_NAME".equals(key) ? connectionName : null;

    // When a JDBC connection is requested with jdbc:snowflake:auto
    Map<String, ConfigSetting> options =
        connectAndCaptureOptions("jdbc:snowflake:auto", environment);

    // Then JDBC asks sf_core to select the environmentDefault connection profile
    assertEquals(connectionName, options.get("connection_name").getStringValue());
  }

  @Test
  public void shouldDelegateDefaultConnectionSelectionForABareAutoConnectionURL() throws Exception {
    // Given no connection name is supplied by the JDBC caller
    Function<String, String> environment = ignored -> null;

    // When a JDBC connection is requested with jdbc:snowflake:auto
    try (SnowflakeConnectionImpl ignored =
        new SnowflakeConnectionImpl(
            "jdbc:snowflake:auto", new Properties(), coreDriverApi, environment)) {
      // Then JDBC asks sf_core to apply its default profile resolution
      @SuppressWarnings("unchecked")
      ArgumentCaptor<Map<String, ConfigSetting>> options = ArgumentCaptor.forClass(Map.class);
      verify(coreDriverApi)
          .connectionSetOptionsForDefaultProfile(any(ConnectionHandle.class), options.capture());

      assertFalse(options.getValue().containsKey("connection_name"));
    }
  }

  @ParameterizedTest
  @MethodSource("missingUrlArguments")
  public void shouldDelegateDefaultSelectionWhenTheAutoUrlComesFromProperties(String url)
      throws Exception {
    // Given a bare auto URL is supplied through JDBC Properties
    Properties properties = new Properties();
    properties.setProperty("url", "jdbc:snowflake:auto");

    // When the explicit URL argument is absent or empty
    try (SnowflakeConnectionImpl ignored =
        new SnowflakeConnectionImpl(url, properties, coreDriverApi, ignoredKey -> null)) {
      // Then JDBC asks sf_core to apply its default profile resolution
      @SuppressWarnings("unchecked")
      ArgumentCaptor<Map<String, ConfigSetting>> options = ArgumentCaptor.forClass(Map.class);
      verify(coreDriverApi)
          .connectionSetOptionsForDefaultProfile(any(ConnectionHandle.class), options.capture());

      assertFalse(options.getValue().containsKey("connection_name"));
    }
  }

  @Test
  public void shouldDelegateDefaultSelectionWhenTheAutoUrlContainsProfileOverrides()
      throws Exception {
    // Given an auto URL has a profile override but no profile selector
    String url = "jdbc:snowflake:auto?warehouse=TEST_WH";

    // When the connection options are forwarded
    try (SnowflakeConnectionImpl ignored =
        new SnowflakeConnectionImpl(url, new Properties(), coreDriverApi, ignoredKey -> null)) {
      // Then sf_core receives the override and resolves the default profile
      @SuppressWarnings("unchecked")
      ArgumentCaptor<Map<String, ConfigSetting>> options = ArgumentCaptor.forClass(Map.class);
      verify(coreDriverApi)
          .connectionSetOptionsForDefaultProfile(any(ConnectionHandle.class), options.capture());

      assertEquals("TEST_WH", options.getValue().get("warehouse").getStringValue());
      assertFalse(options.getValue().containsKey("connection_name"));
    }
  }

  private static Stream<String> missingUrlArguments() {
    return Stream.of(null, "", " ");
  }

  @Test
  public void shouldSurfaceCoreInitializationFailureAfterForwardingTheSelectedName()
      throws Exception {
    // Given sf_core rejects initialization after JDBC forwards the selected name
    when(coreDriverApi.connectionInit(any(), any(), any()))
        .thenThrow(new CoreException("Connection 'missing' not found in config files"));

    // When a JDBC connection is requested with jdbc:snowflake:auto?connectionName=missing
    SQLException exception =
        assertThrows(
            SQLException.class,
            () ->
                SqlExceptionMapper.call(
                    () ->
                        new SnowflakeConnectionImpl(
                            "jdbc:snowflake:auto?connectionName=missing",
                            new Properties(),
                            coreDriverApi,
                            ignored -> null)));

    // Then the JDBC boundary surfaces the core failure and preserves the selected name
    assertEquals("Connection 'missing' not found in config files", exception.getMessage());
    assertNull(exception.getSQLState());
    assertEquals(0, exception.getErrorCode());
    assertEquals("missing", captureOptions().get("connection_name").getStringValue());
  }

  @Test
  public void shouldPreferTheConnectionNamePropertyOverTheAutoURL() throws Exception {
    // Given a JDBC Properties selector and a different URL selector
    Properties properties = new Properties();
    properties.setProperty("connection_name", "fromProperties");

    // When JDBC resolves and forwards connection options
    Map<String, ConfigSetting> options =
        connectAndCaptureOptions(
            "jdbc:snowflake:auto?connectionName=fromUrl", properties, ignored -> null);

    // Then JDBC forwards the Properties selector
    assertEquals("fromProperties", options.get("connection_name").getStringValue());
  }

  @Test
  public void shouldNotAllocateNativeHandlesWhenTheAutoUrlCannotBeParsed() {
    SFSQLException thrown =
        assertThrows(
            SFSQLException.class,
            () ->
                new SnowflakeConnectionImpl(
                    "jdbc:snowflake:auto#fragment",
                    new Properties(),
                    coreDriverApi,
                    ignored -> null));

    assertTrue(thrown.getMessage().contains("must not contain fragments"));
    assertEquals(ErrorCode.INVALID_PARAMETER_VALUE, thrown.getErrorCode());
    verifyNoInteractions(coreDriverApi);
  }

  private Map<String, ConfigSetting> connectAndCaptureOptions(
      String url, Function<String, String> environment) throws Exception {
    return connectAndCaptureOptions(url, new Properties(), environment);
  }

  private Map<String, ConfigSetting> connectAndCaptureOptions(
      String url, Properties properties, Function<String, String> environment) throws Exception {
    try (SnowflakeConnectionImpl ignored =
        new SnowflakeConnectionImpl(url, properties, coreDriverApi, environment)) {
      return captureOptions();
    }
  }

  @SuppressWarnings("unchecked")
  private Map<String, ConfigSetting> captureOptions() {
    ArgumentCaptor<Map<String, ConfigSetting>> options = ArgumentCaptor.forClass(Map.class);
    verify(coreDriverApi).connectionSetOptions(any(ConnectionHandle.class), options.capture());
    return options.getValue();
  }
}
