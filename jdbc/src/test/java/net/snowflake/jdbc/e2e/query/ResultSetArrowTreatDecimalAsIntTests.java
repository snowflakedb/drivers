package net.snowflake.jdbc.e2e.query;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertFalse;
import static org.junit.jupiter.api.Assertions.assertInstanceOf;
import static org.junit.jupiter.api.Assertions.assertTrue;

import java.math.BigDecimal;
import java.sql.Connection;
import java.util.Properties;
import net.snowflake.jdbc.utils.SkipOldDriver;
import net.snowflake.jdbc.utils.SnowflakeIntegrationTestBase;
import org.junit.jupiter.api.Test;

/**
 * JDBC-specific coverage for {@code JDBC_ARROW_TREAT_DECIMAL_AS_INT}. The new driver converts every
 * result to Arrow before exposing it to JDBC, so the property applies regardless of the backend
 * result format. There is no cross-driver Gherkin scenario.
 */
public class ResultSetArrowTreatDecimalAsIntTests extends SnowflakeIntegrationTestBase {

  private static final String SCALE_ZERO_INT_SQL = "SELECT 1::INT AS n";

  @Test
  public void shouldReturnLongOnArrowWhenJdbcTreatDecimalAsIntIsFalse() throws Exception {
    // Given Snowflake client is logged in with Arrow results and JDBC_TREAT_DECIMAL_AS_INT = false
    try (Connection connection = openConnection()) {
      execute(connection, "ALTER SESSION SET JDBC_QUERY_RESULT_FORMAT = 'ARROW'");
      execute(connection, "ALTER SESSION SET JDBC_TREAT_DECIMAL_AS_INT = false");

      // When a scale-0 INT column is read with getObject
      withQueryResult(
          connection,
          SCALE_ZERO_INT_SQL,
          resultSet -> {
            assertTrue(resultSet.next(), "Expected one row");
            Object value = resultSet.getObject(1);
            assertFalse(resultSet.wasNull());
            // Then the value is a Long because JDBC_ARROW_TREAT_DECIMAL_AS_INT defaults to true
            assertInstanceOf(Long.class, value);
            assertEquals(1L, value);
          });
    }
  }

  @Test
  public void shouldReturnBigDecimalOnArrowWhenBothTreatDecimalFlagsAreFalse() throws Exception {
    // Given Snowflake client is logged in with Arrow results and both treat-decimal flags false
    Properties overrides = new Properties();
    overrides.setProperty("JDBC_ARROW_TREAT_DECIMAL_AS_INT", "false");
    try (Connection connection = openConnection(overrides)) {
      execute(connection, "ALTER SESSION SET JDBC_QUERY_RESULT_FORMAT = 'ARROW'");
      execute(connection, "ALTER SESSION SET JDBC_TREAT_DECIMAL_AS_INT = false");

      // When a scale-0 INT column is read with getObject
      withQueryResult(
          connection,
          SCALE_ZERO_INT_SQL,
          resultSet -> {
            assertTrue(resultSet.next(), "Expected one row");
            Object value = resultSet.getObject(1);
            assertFalse(resultSet.wasNull());
            // Then the value is a BigDecimal
            assertInstanceOf(BigDecimal.class, value);
            assertEquals(BigDecimal.ONE, value);
          });
    }
  }

  @Test
  @SkipOldDriver("BD#60")
  public void shouldReturnLongWhenBackendReturnsJsonAndJdbcTreatDecimalAsIntIsFalse()
      throws Exception {
    // Given the backend returns JSON and JDBC_TREAT_DECIMAL_AS_INT = false
    try (Connection connection = openConnection()) {
      execute(connection, "ALTER SESSION SET JDBC_QUERY_RESULT_FORMAT = 'JSON'");
      execute(connection, "ALTER SESSION SET JDBC_TREAT_DECIMAL_AS_INT = false");

      // When a scale-0 INT column is read with getObject
      withQueryResult(
          connection,
          SCALE_ZERO_INT_SQL,
          resultSet -> {
            assertTrue(resultSet.next(), "Expected one row");
            Object value = resultSet.getObject(1);
            assertFalse(resultSet.wasNull());
            // Then the Arrow conversion override still applies after JSON-to-Arrow conversion
            assertInstanceOf(Long.class, value);
            assertEquals(1L, value);
          });
    }
  }
}
