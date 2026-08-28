package net.snowflake.jdbc.e2e.types;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertFalse;
import static org.junit.jupiter.api.Assertions.assertTrue;

import java.sql.Connection;
import java.util.TimeZone;
import net.snowflake.jdbc.utils.SnowflakeIntegrationTestBase;
import org.junit.jupiter.api.Test;

// Separate from TimeTests, which the tests-format-validator orphan-checks against the `time`
// feature; this pins a non-UTC JVM default timezone instead.
public class TimeGetTimestampTimeZoneRegressionTest extends SnowflakeIntegrationTestBase {

  @Test
  public void shouldReturnEpochWallClockTimestampFromTimeColumnRegardlessOfJvmTimeZone()
      throws Exception {
    Connection connection = getDefaultConnection();

    TimeZone originalTimeZone = TimeZone.getDefault();
    TimeZone.setDefault(TimeZone.getTimeZone("Europe/Warsaw"));
    try {
      // When getTimestamp() is called on TIME columns of varying scale
      String sql =
          "SELECT '10:30:50.123456789'::TIME(3), '10:30:50.123456789'::TIME(5),"
              + " '10:30:50.123456789'::TIME(9)";
      withQueryResult(
          connection,
          sql,
          resultSet -> {
            // Then toString() renders 10:30:50 (UTC-anchored), not 11:30:50 shifted by the offset
            assertTrue(resultSet.next());
            assertEquals("1970-01-01 10:30:50.123", resultSet.getTimestamp(1).toString());
            assertFalse(resultSet.wasNull());
            assertEquals("1970-01-01 10:30:50.12345", resultSet.getTimestamp(2).toString());
            assertFalse(resultSet.wasNull());
            assertEquals("1970-01-01 10:30:50.123456789", resultSet.getTimestamp(3).toString());
            assertFalse(resultSet.wasNull());
            assertFalse(resultSet.next());
          });
    } finally {
      TimeZone.setDefault(originalTimeZone);
    }
  }
}
