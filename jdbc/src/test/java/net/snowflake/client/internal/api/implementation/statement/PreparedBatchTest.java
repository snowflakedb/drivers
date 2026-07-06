package net.snowflake.client.internal.api.implementation.statement;

import static org.junit.jupiter.api.Assertions.assertDoesNotThrow;
import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertThrows;
import static org.junit.jupiter.api.Assertions.assertTrue;

import java.sql.SQLException;
import java.util.HashMap;
import java.util.Map;
import net.snowflake.client.api.exception.SnowflakeSQLException;
import net.snowflake.client.api.resultset.SnowflakeType;
import net.snowflake.client.internal.api.implementation.statement.PreparedStatementBindingSerializer.ParameterValue;
import org.junit.jupiter.api.Test;

/** Unit tests for {@link PreparedBatch} via its public API only. */
class PreparedBatchTest {

  private static SqlPlaceholderMetadata twoCol() {
    return SqlPlaceholderMetadata.analyze("INSERT INTO t VALUES (?, ?)");
  }

  private static SqlPlaceholderMetadata oneCol() {
    return SqlPlaceholderMetadata.analyze("INSERT INTO t VALUES (?)");
  }

  private static Map<Integer, ParameterValue> row(Object... pairs) {
    if (pairs.length % 3 != 0) {
      throw new IllegalArgumentException("pairs must be (idx, type, value) triples");
    }
    Map<Integer, ParameterValue> r = new HashMap<>();
    for (int i = 0; i < pairs.length; i += 3) {
      r.put((Integer) pairs[i], new ParameterValue((SnowflakeType) pairs[i + 1], pairs[i + 2]));
    }
    return r;
  }

  @Test
  void rejectsTypeMismatchWithErrorCodeAndSqlState() throws SQLException {
    PreparedBatch batch = new PreparedBatch();
    SqlPlaceholderMetadata meta = oneCol();
    batch.addRow(meta, row(1, SnowflakeType.FIXED, "1"));

    SnowflakeSQLException ex =
        assertThrows(
            SnowflakeSQLException.class,
            () -> batch.addRow(meta, row(1, SnowflakeType.TEXT, "boom")));
    assertEquals(200023, ex.getErrorCode());
    assertEquals("0A000", ex.getSQLState());
  }

  @Test
  void typeMismatchDoesNotAdvanceBatchSize() throws SQLException {
    PreparedBatch batch = new PreparedBatch();
    SqlPlaceholderMetadata meta = twoCol();
    batch.addRow(meta, row(1, SnowflakeType.FIXED, "1", 2, SnowflakeType.TEXT, "ok"));
    assertEquals(1, batch.size());

    assertThrows(
        SnowflakeSQLException.class,
        () -> batch.addRow(meta, row(1, SnowflakeType.FIXED, "2", 2, SnowflakeType.FIXED, "999")));

    // size() reads the first column's list length; if the failed addRow had partially
    // appended to column 1 before throwing on column 2, size would be 2.
    assertEquals(1, batch.size(), "failed addRow must not advance batch size");
  }

  @Test
  void allNullsThenTypedDoesNotThrow() {
    PreparedBatch batch = new PreparedBatch();
    SqlPlaceholderMetadata meta = oneCol();
    assertDoesNotThrow(
        () -> {
          batch.addRow(meta, row(1, SnowflakeType.ANY, null));
          batch.addRow(meta, row(1, SnowflakeType.ANY, null));
          batch.addRow(meta, row(1, SnowflakeType.FIXED, "7"));
        });
  }

  @Test
  void typedThenNullThenTypedDoesNotThrow() {
    PreparedBatch batch = new PreparedBatch();
    SqlPlaceholderMetadata meta = oneCol();
    assertDoesNotThrow(
        () -> {
          batch.addRow(meta, row(1, SnowflakeType.TEXT, "hi"));
          batch.addRow(meta, row(1, SnowflakeType.ANY, null));
          batch.addRow(meta, row(1, SnowflakeType.TEXT, "there"));
        });
  }

  @Test
  void rejectsMissingValue() {
    PreparedBatch batch = new PreparedBatch();
    SQLException ex =
        assertThrows(
            SQLException.class, () -> batch.addRow(twoCol(), row(1, SnowflakeType.FIXED, "1")));
    assertTrue(ex.getMessage().contains("Missing value for parameter index: 2"));
  }

  @Test
  void clearResetsState() throws SQLException {
    PreparedBatch batch = new PreparedBatch();
    SqlPlaceholderMetadata meta = oneCol();
    batch.addRow(meta, row(1, SnowflakeType.FIXED, "1"));
    batch.addRow(meta, row(1, SnowflakeType.FIXED, "2"));

    batch.clear();

    assertEquals(0, batch.size());
    assertTrue(batch.isEmpty());
  }
}
