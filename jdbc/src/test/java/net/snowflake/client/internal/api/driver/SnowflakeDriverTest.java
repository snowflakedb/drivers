package net.snowflake.client.internal.api.driver;

import static org.junit.jupiter.api.Assertions.assertThrows;
import static org.junit.jupiter.api.Assertions.assertTrue;

import java.sql.SQLException;
import java.util.stream.Stream;
import net.snowflake.client.api.driver.SnowflakeDriver;
import org.junit.jupiter.api.Test;
import org.junit.jupiter.params.ParameterizedTest;
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
}
