package net.snowflake.client.internal.api.implementation.statement;

import static org.junit.jupiter.api.Assertions.assertArrayEquals;
import static org.junit.jupiter.api.Assertions.assertDoesNotThrow;
import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertFalse;
import static org.junit.jupiter.api.Assertions.assertNull;
import static org.junit.jupiter.api.Assertions.assertSame;
import static org.junit.jupiter.api.Assertions.assertThrows;
import static org.junit.jupiter.api.Assertions.assertTrue;

import java.sql.ResultSet;
import java.sql.SQLException;
import java.sql.Statement;
import net.snowflake.client.api.exception.ErrorCode;
import net.snowflake.client.internal.api.decorator.Telemetry;
import net.snowflake.client.internal.api.implementation.Decorators;
import org.junit.jupiter.api.Test;

class EmptyGeneratedKeysResultSetTest {

  private static ResultSet generatedKeys() {
    return Decorators.resultSet(new EmptyGeneratedKeysResultSet(), Telemetry.NOOP);
  }

  @Test
  void shouldReturnTypeDefaultsFromGetters() throws Exception {
    try (ResultSet keys = generatedKeys()) {
      assertFalse(keys.next());
      assertEquals(0, keys.getInt(1));
      assertEquals(0L, keys.getLong("id"));
      assertFalse(keys.wasNull());
      assertEquals("", keys.getString(1));
      assertEquals("", keys.getString("col1"));
      assertNull(keys.getNString(1));
      assertFalse(keys.wasNull());
      assertFalse(keys.getBoolean(1));
      assertArrayEquals(new byte[0], keys.getBytes(1));
      assertNull(keys.getObject(1));
      assertFalse(keys.wasNull());
      assertNull(keys.getObject(1, Long.class));
      assertFalse(keys.wasNull());
      assertNull(keys.getBigDecimal(1));
      assertFalse(keys.wasNull());
      assertNull(keys.getDate(1));
      assertFalse(keys.wasNull());
      assertNull(keys.getClob(1));
      assertFalse(keys.wasNull());
      assertNull(keys.getCharacterStream(1));
      assertFalse(keys.wasNull());
      assertEquals(0, keys.findColumn("col1"));
    }
  }

  @Test
  void shouldReportNoCursorAndAcceptNavigationAsNoOps() throws Exception {
    try (ResultSet keys = generatedKeys()) {
      assertFalse(keys.isBeforeFirst());
      assertFalse(keys.isAfterLast());
      assertFalse(keys.isFirst());
      assertFalse(keys.isLast());
      assertFalse(keys.first());
      assertFalse(keys.last());
      assertFalse(keys.absolute(1));
      assertFalse(keys.relative(1));
      assertFalse(keys.previous());
      assertEquals(0, keys.getRow());
      assertDoesNotThrow(keys::beforeFirst);
      assertDoesNotThrow(() -> keys.setFetchDirection(ResultSet.FETCH_REVERSE));
      assertDoesNotThrow(() -> keys.updateInt(1, 7));
      assertDoesNotThrow(keys::insertRow);
    }
  }

  @Test
  void shouldReturnNullOrZeroForResultSetProperties() throws Exception {
    try (ResultSet keys = generatedKeys()) {
      assertNull(keys.getMetaData());
      assertNull(keys.getStatement());
      assertNull(keys.getCursorName());
      assertNull(keys.getWarnings());
      assertEquals(0, keys.getType());
      assertEquals(0, keys.getConcurrency());
      assertEquals(0, keys.getFetchDirection());
      assertEquals(0, keys.getHoldability());
      assertNull(keys.unwrap(Statement.class));
      assertFalse(keys.isWrapperFor(Statement.class));
    }
  }

  @Test
  void shouldUnwrapToTheDecoratedResultSet() throws Exception {
    try (ResultSet keys = generatedKeys()) {
      assertSame(keys, keys.unwrap(ResultSet.class));
      assertTrue(keys.isWrapperFor(ResultSet.class));
    }
  }

  @Test
  void shouldRejectCallsAfterCloseWithResultSetClosedError() throws Exception {
    ResultSet keys = generatedKeys();
    keys.close();

    assertTrue(keys.isClosed());
    assertFalse(keys.next());
    SQLException ex = assertThrows(SQLException.class, () -> keys.getLong(1));
    assertEquals(ErrorCode.RESULTSET_ALREADY_CLOSED.getSqlState(), ex.getSQLState());
    assertEquals(ErrorCode.RESULTSET_ALREADY_CLOSED.getMessageCode(), ex.getErrorCode());
    assertThrows(SQLException.class, keys::getMetaData);
    assertThrows(SQLException.class, () -> keys.unwrap(Statement.class));
  }
}
