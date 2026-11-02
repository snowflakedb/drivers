package net.snowflake.client.internal.api.driver;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertFalse;
import static org.junit.jupiter.api.Assertions.assertNull;
import static org.junit.jupiter.api.Assertions.assertThrows;
import static org.junit.jupiter.api.Assertions.assertTrue;

import java.sql.DriverPropertyInfo;
import java.sql.SQLException;
import java.util.Properties;
import java.util.stream.Stream;
import net.snowflake.client.api.driver.SnowflakeDriver;
import net.snowflake.client.api.exception.ErrorCode;
import net.snowflake.client.api.exception.SnowflakeSQLException;
import org.junit.jupiter.api.Test;
import org.junit.jupiter.params.ParameterizedTest;
import org.junit.jupiter.params.provider.CsvSource;
import org.junit.jupiter.params.provider.MethodSource;

/** Basic tests for the Snowflake JDBC Driver */
public class SnowflakeDriverTest {
  @Test
  public void testAcceptsNullUrlThrowsSQLException() {
    SnowflakeDriver driver = new SnowflakeDriver();
    assertThrows(SQLException.class, () -> driver.acceptsURL(null));
  }

  @ParameterizedTest
  @MethodSource("malformedSnowflakeUrls")
  public void testAcceptsMalformedSnowflakeURL(String url) throws SQLException {
    SnowflakeDriver driver = new SnowflakeDriver();
    assertTrue(driver.acceptsURL(url), "Expected Snowflake subprotocol URL to be accepted: " + url);
  }

  private static Stream<String> malformedSnowflakeUrls() {
    return Stream.of(
        "jdbc:snowflake://",
        "jdbc:snowflake://:",
        "jdbc:snowflake://:8080",
        "jdbc:snowflake://localhost:xyz",
        "jdbc:snowflake://localhost:8080/a=b",
        "jdbc:snowflake://testaccount.com?proxyHost=%%",
        "jdbc:snowflake://abc-test.us-east-1.snowflakecomputing.com/?private_key_file=C:\\temp\\rsa_key.p8&user=test_user");
  }

  @ParameterizedTest
  @CsvSource({
    "5.2.10, 0, 5",
    "5.2.10, 1, 2",
    "5.2.10, 2, 10",
    "0.0.6-SNAPSHOT, 2, 6",
    "5.0.0.prpr6, 3, 0",
    "5.2, 5, 0",
    "5.2, -1, 0",
    "abc.def, 0, 0"
  })
  public void shouldParseVersionComponentsAndFallbackToZero(
      String version, int index, int expected) {
    assertEquals(expected, SnowflakeDriver.parseVersionComponent(version, index));
  }

  @Test
  public void shouldReturnZeroWhenParsingNullVersionString() {
    assertEquals(0, SnowflakeDriver.parseVersionComponent(null, 0));
  }

  @Test
  public void shouldReturnNullFromConnectForUnsupportedUrlPrefix() throws SQLException {
    SnowflakeDriver driver = new SnowflakeDriver();
    assertNull(driver.connect("jdbc:postgresql://localhost/db", new Properties()));
    assertNull(driver.connect(null, new Properties()));
  }

  @Test
  public void shouldThrowSnowflakeSQLExceptionOnInvalidSnowflakeUrlInConnect() {
    SnowflakeDriver driver = new SnowflakeDriver();
    SnowflakeSQLException ex =
        assertThrows(
            SnowflakeSQLException.class,
            () -> driver.connect("jdbc:snowflake://", new Properties()));
    assertEquals("Connection string is invalid. Unable to parse.", ex.getMessage());
    // Same text as ErrorCode.INVALID_CONNECTION_STRING, but the message-only constructor
    // does not copy that code's SQLState or vendor code.
    assertNull(ex.getSQLState());
    assertEquals(0, ex.getErrorCode());
  }

  @Test
  public void shouldTranslateAutoConnectionValidationFailureInConnect() {
    SnowflakeDriver driver = new SnowflakeDriver();
    SnowflakeSQLException ex =
        assertThrows(
            SnowflakeSQLException.class,
            () -> driver.connect("jdbc:snowflake:auto#fragment", new Properties()));
    assertEquals(ErrorCode.INVALID_PARAMETER_VALUE.getSqlState(), ex.getSQLState());
    assertEquals(ErrorCode.INVALID_PARAMETER_VALUE.getMessageCode(), ex.getErrorCode());
  }

  @Test
  public void shouldTranslateInvalidPropertyTypeInGetPropertyInfo() {
    SnowflakeDriver driver = new SnowflakeDriver();
    Properties info = new Properties();
    info.put("user", 42);

    SnowflakeSQLException ex =
        assertThrows(
            SnowflakeSQLException.class,
            () -> driver.getPropertyInfo("jdbc:snowflake://acct.snowflakecomputing.com", info));
    assertEquals(ErrorCode.INVALID_PARAMETER_TYPE.getSqlState(), ex.getSQLState());
    assertEquals(ErrorCode.INVALID_PARAMETER_TYPE.getMessageCode(), ex.getErrorCode());
  }

  @Test
  public void shouldReturnMissingPropertyInfoAndDriverMetadata() throws SQLException {
    SnowflakeDriver driver = new SnowflakeDriver();
    DriverPropertyInfo[] props = driver.getPropertyInfo(null, new Properties());

    assertEquals(1, props.length);
    assertEquals("serverURL", props[0].name);
    assertEquals(SnowflakeDriver.DRIVER_VERSION, SnowflakeDriver.getDriverVersion());
    assertEquals(SnowflakeDriver.MAJOR_VERSION, driver.getMajorVersion());
    assertEquals(SnowflakeDriver.MINOR_VERSION, driver.getMinorVersion());
    assertFalse(driver.jdbcCompliant());
    assertNull(driver.getParentLogger());
  }
}
