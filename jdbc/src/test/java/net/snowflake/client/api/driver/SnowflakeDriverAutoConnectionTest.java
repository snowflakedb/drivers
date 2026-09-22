package net.snowflake.client.api.driver;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertFalse;
import static org.junit.jupiter.api.Assertions.assertNull;
import static org.junit.jupiter.api.Assertions.assertSame;
import static org.junit.jupiter.api.Assertions.assertThrows;
import static org.mockito.ArgumentMatchers.any;
import static org.mockito.Mockito.mock;
import static org.mockito.Mockito.when;

import java.sql.Connection;
import java.sql.DriverPropertyInfo;
import java.sql.SQLException;
import java.util.Properties;
import java.util.concurrent.atomic.AtomicBoolean;
import net.snowflake.client.api.exception.ErrorCode;
import net.snowflake.client.internal.api.implementation.connection.SnowflakeConnectionImpl;
import net.snowflake.client.internal.api.implementation.exception.CoreException;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.DriverException;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.ErrorKind;
import org.junit.jupiter.api.Test;
import org.junit.jupiter.params.ParameterizedTest;
import org.junit.jupiter.params.provider.NullAndEmptySource;
import org.junit.jupiter.params.provider.ValueSource;

/**
 * New-driver-only public boundary tests for {@code jdbc:snowflake:auto}. Excluded from the
 * old-driver reference classpath because they use {@link ConnectionFactory} and core carriers that
 * do not exist on snowflake-jdbc.
 */
public class SnowflakeDriverAutoConnectionTest {

  @Test
  public void shouldReturnTheDecoratedConnectionForAValidAutoUrl() throws SQLException {
    SnowflakeConnectionImpl impl = mock(SnowflakeConnectionImpl.class);
    Connection decorated = mock(Connection.class);
    when(impl.decoratedSelf(any())).thenReturn(decorated);
    SnowflakeDriver driver = new SnowflakeDriver((url, info) -> impl);

    assertSame(
        decorated, driver.connect("jdbc:snowflake:auto?connectionName=readOnly", new Properties()));
  }

  @Test
  public void shouldRejectInvalidAutoUrlBeforeInvokingConnectionFactory() {
    AtomicBoolean factoryCalled = new AtomicBoolean(false);
    SnowflakeDriver driver =
        new SnowflakeDriver(
            (url, info) -> {
              factoryCalled.set(true);
              throw new AssertionError("factory must not run for an invalid auto URL");
            });

    SQLException exception =
        assertThrows(
            SQLException.class,
            () -> driver.connect("jdbc:snowflake:auto#fragment", new Properties()));

    assertFalse(factoryCalled.get());
    assertEquals(ErrorCode.INVALID_PARAMETER_VALUE.getSqlState(), exception.getSQLState());
    assertEquals(ErrorCode.INVALID_PARAMETER_VALUE.getMessageCode(), exception.getErrorCode());
  }

  @ParameterizedTest
  @NullAndEmptySource
  @ValueSource(strings = " ")
  public void shouldConnectWithAutoUrlFromPropertiesWhenUrlArgumentIsBlank(String url)
      throws SQLException {
    SnowflakeConnectionImpl impl = mock(SnowflakeConnectionImpl.class);
    Connection decorated = mock(Connection.class);
    when(impl.decoratedSelf(any())).thenReturn(decorated);
    SnowflakeDriver driver = new SnowflakeDriver((ignoredUrl, ignoredInfo) -> impl);
    Properties properties = new Properties();
    properties.setProperty("url", "jdbc:snowflake:auto");

    assertSame(decorated, driver.connect(url, properties));
  }

  @Test
  public void shouldMapMissingProfileWhenAutoUrlComesFromProperties() {
    CoreException missingProfile = missingProfileErrorFor("missing");
    SnowflakeDriver driver =
        new SnowflakeDriver(
            (url, info) -> {
              throw missingProfile;
            });
    Properties properties = new Properties();
    properties.setProperty("url", "jdbc:snowflake:auto?connectionName=missing");

    SQLException exception =
        assertThrows(SQLException.class, () -> driver.connect(null, properties));

    assertEquals(ErrorCode.INVALID_PARAMETER_VALUE.getSqlState(), exception.getSQLState());
    assertEquals(ErrorCode.INVALID_PARAMETER_VALUE.getMessageCode(), exception.getErrorCode());
    assertSame(missingProfile, exception.getCause());
  }

  @Test
  public void shouldReportMissingCredentialsForAValidAutoUrlDuringPropertyInfo()
      throws SQLException {
    DriverPropertyInfo[] properties =
        new SnowflakeDriver().getPropertyInfo("jdbc:snowflake:auto", new Properties());

    assertEquals("user", properties[0].name);
    assertEquals("password", properties[1].name);
    assertEquals(2, properties.length);
  }

  @ParameterizedTest
  @NullAndEmptySource
  @ValueSource(strings = " ")
  public void shouldReportMissingCredentialsWhenAutoUrlComesFromProperties(String url)
      throws SQLException {
    Properties info = new Properties();
    info.setProperty("url", "jdbc:snowflake:auto");

    DriverPropertyInfo[] properties = new SnowflakeDriver().getPropertyInfo(url, info);

    assertEquals("user", properties[0].name);
    assertEquals("password", properties[1].name);
    assertEquals(2, properties.length);
  }

  @Test
  public void shouldRejectMalformedAutoUrlDuringPropertyInfo() {
    SQLException exception =
        assertThrows(
            SQLException.class,
            () ->
                new SnowflakeDriver()
                    .getPropertyInfo("jdbc:snowflake:auto#fragment", new Properties()));

    assertEquals(ErrorCode.INVALID_PARAMETER_VALUE.getSqlState(), exception.getSQLState());
    assertEquals(ErrorCode.INVALID_PARAMETER_VALUE.getMessageCode(), exception.getErrorCode());
  }

  @Test
  public void shouldRejectMalformedAutoUrlFromPropertiesDuringPropertyInfo() {
    Properties properties = new Properties();
    properties.setProperty("url", "jdbc:snowflake:auto#fragment");

    SQLException exception =
        assertThrows(
            SQLException.class, () -> new SnowflakeDriver().getPropertyInfo(null, properties));

    assertEquals(ErrorCode.INVALID_PARAMETER_VALUE.getSqlState(), exception.getSQLState());
    assertEquals(ErrorCode.INVALID_PARAMETER_VALUE.getMessageCode(), exception.getErrorCode());
  }

  @Test
  public void shouldMapMissingAutoProfileToInvalidParameterAtPublicConnect() {
    CoreException missingProfile = missingProfileErrorFor("missing");
    SnowflakeDriver driver =
        new SnowflakeDriver(
            (url, info) -> {
              throw missingProfile;
            });

    SQLException exception =
        assertThrows(
            SQLException.class,
            () -> driver.connect("jdbc:snowflake:auto?connectionName=missing", new Properties()));

    assertEquals(ErrorCode.INVALID_PARAMETER_VALUE.getSqlState(), exception.getSQLState());
    assertEquals(ErrorCode.INVALID_PARAMETER_VALUE.getMessageCode(), exception.getErrorCode());
    assertSame(missingProfile, exception.getCause());
    assertEquals(
        "The Connection missing not found in connections.toml file.", exception.getMessage());
  }

  @Test
  public void shouldNotRemapMissingParameterWhenUrlIsNotAuto() {
    CoreException missingProfile = missingProfileErrorFor("missing");
    SnowflakeDriver driver =
        new SnowflakeDriver(
            (url, info) -> {
              throw missingProfile;
            });

    SQLException exception =
        assertThrows(
            SQLException.class,
            () ->
                driver.connect(
                    "jdbc:snowflake://account.snowflakecomputing.com", new Properties()));

    assertSame(missingProfile, exception.getCause());
    assertEquals(0, exception.getErrorCode());
    assertNull(exception.getSQLState());
  }

  @Test
  public void shouldNotRemapAMissingUserOnAnAutoUrl() {
    CoreException missingUser =
        new CoreException(
            DriverException.newBuilder()
                .setKind(ErrorKind.ERROR_KIND_MISSING_PARAMETER)
                .setMessage("Missing required parameter 'user'")
                .setParameter("user")
                .build(),
            null);
    SnowflakeDriver driver =
        new SnowflakeDriver(
            (url, info) -> {
              throw missingUser;
            });

    SQLException exception =
        assertThrows(
            SQLException.class,
            () -> driver.connect("jdbc:snowflake:auto?connectionName=readOnly", new Properties()));

    assertEquals(missingUser, exception.getCause());
    assertEquals(0, exception.getErrorCode());
    assertNull(exception.getSQLState());
  }

  private static CoreException missingProfileErrorFor(String name) {
    return new CoreException(
        DriverException.newBuilder()
            .setKind(ErrorKind.ERROR_KIND_MISSING_PARAMETER)
            .setMessage("Configuration error: Connection '" + name + "' not found in config files")
            .setRootCause("Connection '" + name + "' not found in config files")
            .setParameter("connection: " + name)
            .build(),
        null);
  }
}
