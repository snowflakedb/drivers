package net.snowflake.client.internal.api.implementation.statement;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertThrows;
import static org.mockito.ArgumentMatchers.any;
import static org.mockito.Mockito.mock;
import static org.mockito.Mockito.times;
import static org.mockito.Mockito.verify;
import static org.mockito.Mockito.when;

import java.io.ByteArrayInputStream;
import java.io.StringReader;
import java.math.BigDecimal;
import java.net.URL;
import java.sql.CallableStatement;
import java.sql.Date;
import java.sql.ParameterMetaData;
import java.sql.SQLFeatureNotSupportedException;
import java.sql.Time;
import java.sql.Timestamp;
import java.sql.Types;
import java.util.Calendar;
import java.util.HashMap;
import net.snowflake.client.internal.api.decorator.Telemetry;
import net.snowflake.client.internal.api.implementation.connection.InternalSnowflakeConnection;
import net.snowflake.client.internal.unicore.CoreDriverApi;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.ColumnMetadata;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.ConnectionHandle;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.PrepareResult;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.StatementHandle;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.StatementNewResponse;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.StatementPrepareResponse;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.StatementReleaseResponse;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.StatementSetSqlQueryResponse;
import org.junit.jupiter.api.BeforeEach;
import org.junit.jupiter.api.Nested;
import org.junit.jupiter.api.Test;

class SnowflakeCallableStatementImplTest {

  private static final ConnectionHandle CONN_HANDLE =
      ConnectionHandle.newBuilder().setId(1).setMagic(100).build();
  private static final StatementHandle STMT_HANDLE =
      StatementHandle.newBuilder().setId(10).setMagic(1000).build();

  private CoreDriverApi mockCoreApi;
  private InternalSnowflakeConnection mockConnection;

  @BeforeEach
  void setUp() throws Exception {
    mockCoreApi = mock(CoreDriverApi.class);
    mockConnection = mock(InternalSnowflakeConnection.class);
    when(mockConnection.getHandle()).thenReturn(CONN_HANDLE);
    when(mockCoreApi.statementNew(any()))
        .thenReturn(StatementNewResponse.newBuilder().setStmtHandle(STMT_HANDLE).build());
    when(mockCoreApi.statementRelease(any()))
        .thenReturn(StatementReleaseResponse.getDefaultInstance());
  }

  // Route through the decorator: it wraps the raw impl and translates the runtime carriers it
  // throws (e.g. SFSQLFeatureNotSupportedException) into the checked java.sql exception types the
  // JDBC API promises. These tests assert that public contract, so they must go through the
  // boundary, not the raw impl.
  private CallableStatement createCallableStatement(String sql) throws Exception {
    return new DecoratedSnowflakeCallableStatementImpl(
        new SnowflakeCallableStatementImpl(mockConnection, sql, mockCoreApi), Telemetry.NOOP);
  }

  // ── parseSqlEscapeSyntax ──────────────────────────────────────────────────

  @Test
  void shouldStripOuterCurlyBracketsFromSqlEscapeSyntax() throws Exception {
    assertEquals(
        "CALL square_it(5)",
        SnowflakeCallableStatementImpl.parseSqlEscapeSyntax("{CALL square_it(5)}"));
  }

  @Test
  void shouldLeaveSqlWithoutBracketsUnchanged() throws Exception {
    assertEquals(
        "CALL no_bracket_function(44)",
        SnowflakeCallableStatementImpl.parseSqlEscapeSyntax("CALL no_bracket_function(44)"));
  }

  @Test
  void shouldLeaveInnerBracketsUnchanged() throws Exception {
    assertEquals(
        "CALL {bracket_function(a=?)}",
        SnowflakeCallableStatementImpl.parseSqlEscapeSyntax("CALL {bracket_function(a=?)}"));
  }

  @Test
  void shouldTrimLeadingAndTrailingWhitespaceBeforeMatching() throws Exception {
    assertEquals(
        "CALL square_it(5)",
        SnowflakeCallableStatementImpl.parseSqlEscapeSyntax("  {CALL square_it(5)}  "));
  }

  // ── registerOutParameter ─────────────────────────────────────────────────

  @Test
  void shouldThrowFeatureNotSupportedWhenRegisteringOutParameterByIndex() throws Exception {
    try (CallableStatement cs = createCallableStatement("CALL proc(?)")) {
      assertThrows(
          SQLFeatureNotSupportedException.class, () -> cs.registerOutParameter(1, Types.INTEGER));
    }
  }

  @Test
  void shouldThrowFeatureNotSupportedWhenRegisteringOutParameterByIndexWithScale()
      throws Exception {
    try (CallableStatement cs = createCallableStatement("CALL proc(?)")) {
      assertThrows(
          SQLFeatureNotSupportedException.class,
          () -> cs.registerOutParameter(1, Types.INTEGER, 0));
    }
  }

  @Test
  void shouldThrowFeatureNotSupportedWhenRegisteringOutParameterByIndexWithTypeName()
      throws Exception {
    try (CallableStatement cs = createCallableStatement("CALL proc(?)")) {
      assertThrows(
          SQLFeatureNotSupportedException.class,
          () -> cs.registerOutParameter(1, Types.INTEGER, "int"));
    }
  }

  @Test
  void shouldThrowFeatureNotSupportedWhenRegisteringOutParameterByName() throws Exception {
    try (CallableStatement cs = createCallableStatement("CALL proc(?)")) {
      assertThrows(
          SQLFeatureNotSupportedException.class, () -> cs.registerOutParameter("p", Types.INTEGER));
    }
  }

  @Test
  void shouldThrowFeatureNotSupportedWhenRegisteringOutParameterByNameWithScale() throws Exception {
    try (CallableStatement cs = createCallableStatement("CALL proc(?)")) {
      assertThrows(
          SQLFeatureNotSupportedException.class,
          () -> cs.registerOutParameter("p", Types.INTEGER, 0));
    }
  }

  @Test
  void shouldThrowFeatureNotSupportedWhenRegisteringOutParameterByNameWithTypeName()
      throws Exception {
    try (CallableStatement cs = createCallableStatement("CALL proc(?)")) {
      assertThrows(
          SQLFeatureNotSupportedException.class,
          () -> cs.registerOutParameter("p", Types.INTEGER, "int"));
    }
  }

  // ── wasNull ───────────────────────────────────────────────────────────────

  @Test
  void shouldThrowFeatureNotSupportedWhenCallingWasNull() throws Exception {
    try (CallableStatement cs = createCallableStatement("CALL proc(?)")) {
      assertThrows(SQLFeatureNotSupportedException.class, cs::wasNull);
    }
  }

  // ── OUT-parameter getters ─────────────────────────────────────────────────

  @Test
  void shouldThrowFeatureNotSupportedWhenGettingStringByIndex() throws Exception {
    try (CallableStatement cs = createCallableStatement("CALL proc(?)")) {
      assertThrows(SQLFeatureNotSupportedException.class, () -> cs.getString(1));
    }
  }

  @Test
  void shouldThrowFeatureNotSupportedWhenGettingStringByName() throws Exception {
    try (CallableStatement cs = createCallableStatement("CALL proc(?)")) {
      assertThrows(SQLFeatureNotSupportedException.class, () -> cs.getString("p"));
    }
  }

  @Test
  void shouldThrowFeatureNotSupportedWhenGettingBooleanByIndex() throws Exception {
    try (CallableStatement cs = createCallableStatement("CALL proc(?)")) {
      assertThrows(SQLFeatureNotSupportedException.class, () -> cs.getBoolean(1));
    }
  }

  @Test
  void shouldThrowFeatureNotSupportedWhenGettingBooleanByName() throws Exception {
    try (CallableStatement cs = createCallableStatement("CALL proc(?)")) {
      assertThrows(SQLFeatureNotSupportedException.class, () -> cs.getBoolean("p"));
    }
  }

  @Test
  void shouldThrowFeatureNotSupportedWhenGettingByteByIndex() throws Exception {
    try (CallableStatement cs = createCallableStatement("CALL proc(?)")) {
      assertThrows(SQLFeatureNotSupportedException.class, () -> cs.getByte(1));
    }
  }

  @Test
  void shouldThrowFeatureNotSupportedWhenGettingByteByName() throws Exception {
    try (CallableStatement cs = createCallableStatement("CALL proc(?)")) {
      assertThrows(SQLFeatureNotSupportedException.class, () -> cs.getByte("p"));
    }
  }

  @Test
  void shouldThrowFeatureNotSupportedWhenGettingShortByIndex() throws Exception {
    try (CallableStatement cs = createCallableStatement("CALL proc(?)")) {
      assertThrows(SQLFeatureNotSupportedException.class, () -> cs.getShort(1));
    }
  }

  @Test
  void shouldThrowFeatureNotSupportedWhenGettingShortByName() throws Exception {
    try (CallableStatement cs = createCallableStatement("CALL proc(?)")) {
      assertThrows(SQLFeatureNotSupportedException.class, () -> cs.getShort("p"));
    }
  }

  @Test
  void shouldThrowFeatureNotSupportedWhenGettingIntByIndex() throws Exception {
    try (CallableStatement cs = createCallableStatement("CALL proc(?)")) {
      assertThrows(SQLFeatureNotSupportedException.class, () -> cs.getInt(1));
    }
  }

  @Test
  void shouldThrowFeatureNotSupportedWhenGettingIntByName() throws Exception {
    try (CallableStatement cs = createCallableStatement("CALL proc(?)")) {
      assertThrows(SQLFeatureNotSupportedException.class, () -> cs.getInt("p"));
    }
  }

  @Test
  void shouldThrowFeatureNotSupportedWhenGettingLongByIndex() throws Exception {
    try (CallableStatement cs = createCallableStatement("CALL proc(?)")) {
      assertThrows(SQLFeatureNotSupportedException.class, () -> cs.getLong(1));
    }
  }

  @Test
  void shouldThrowFeatureNotSupportedWhenGettingLongByName() throws Exception {
    try (CallableStatement cs = createCallableStatement("CALL proc(?)")) {
      assertThrows(SQLFeatureNotSupportedException.class, () -> cs.getLong("p"));
    }
  }

  @Test
  void shouldThrowFeatureNotSupportedWhenGettingFloatByIndex() throws Exception {
    try (CallableStatement cs = createCallableStatement("CALL proc(?)")) {
      assertThrows(SQLFeatureNotSupportedException.class, () -> cs.getFloat(1));
    }
  }

  @Test
  void shouldThrowFeatureNotSupportedWhenGettingFloatByName() throws Exception {
    try (CallableStatement cs = createCallableStatement("CALL proc(?)")) {
      assertThrows(SQLFeatureNotSupportedException.class, () -> cs.getFloat("p"));
    }
  }

  @Test
  void shouldThrowFeatureNotSupportedWhenGettingDoubleByIndex() throws Exception {
    try (CallableStatement cs = createCallableStatement("CALL proc(?)")) {
      assertThrows(SQLFeatureNotSupportedException.class, () -> cs.getDouble(1));
    }
  }

  @Test
  void shouldThrowFeatureNotSupportedWhenGettingDoubleByName() throws Exception {
    try (CallableStatement cs = createCallableStatement("CALL proc(?)")) {
      assertThrows(SQLFeatureNotSupportedException.class, () -> cs.getDouble("p"));
    }
  }

  @Test
  void shouldThrowFeatureNotSupportedWhenGettingBigDecimalByIndex() throws Exception {
    try (CallableStatement cs = createCallableStatement("CALL proc(?)")) {
      assertThrows(SQLFeatureNotSupportedException.class, () -> cs.getBigDecimal(1));
    }
  }

  @Test
  void shouldThrowFeatureNotSupportedWhenGettingBigDecimalByName() throws Exception {
    try (CallableStatement cs = createCallableStatement("CALL proc(?)")) {
      assertThrows(SQLFeatureNotSupportedException.class, () -> cs.getBigDecimal("p"));
    }
  }

  @Test
  void shouldThrowFeatureNotSupportedWhenGettingBytesByIndex() throws Exception {
    try (CallableStatement cs = createCallableStatement("CALL proc(?)")) {
      assertThrows(SQLFeatureNotSupportedException.class, () -> cs.getBytes(1));
    }
  }

  @Test
  void shouldThrowFeatureNotSupportedWhenGettingBytesByName() throws Exception {
    try (CallableStatement cs = createCallableStatement("CALL proc(?)")) {
      assertThrows(SQLFeatureNotSupportedException.class, () -> cs.getBytes("p"));
    }
  }

  @Test
  void shouldThrowFeatureNotSupportedWhenGettingDateByIndex() throws Exception {
    try (CallableStatement cs = createCallableStatement("CALL proc(?)")) {
      assertThrows(SQLFeatureNotSupportedException.class, () -> cs.getDate(1));
    }
  }

  @Test
  void shouldThrowFeatureNotSupportedWhenGettingDateByName() throws Exception {
    try (CallableStatement cs = createCallableStatement("CALL proc(?)")) {
      assertThrows(SQLFeatureNotSupportedException.class, () -> cs.getDate("p"));
    }
  }

  @Test
  void shouldThrowFeatureNotSupportedWhenGettingDateByIndexWithCalendar() throws Exception {
    try (CallableStatement cs = createCallableStatement("CALL proc(?)")) {
      assertThrows(
          SQLFeatureNotSupportedException.class, () -> cs.getDate(1, Calendar.getInstance()));
    }
  }

  @Test
  void shouldThrowFeatureNotSupportedWhenGettingDateByNameWithCalendar() throws Exception {
    try (CallableStatement cs = createCallableStatement("CALL proc(?)")) {
      assertThrows(
          SQLFeatureNotSupportedException.class, () -> cs.getDate("p", Calendar.getInstance()));
    }
  }

  @Test
  void shouldThrowFeatureNotSupportedWhenGettingTimeByIndex() throws Exception {
    try (CallableStatement cs = createCallableStatement("CALL proc(?)")) {
      assertThrows(SQLFeatureNotSupportedException.class, () -> cs.getTime(1));
    }
  }

  @Test
  void shouldThrowFeatureNotSupportedWhenGettingTimeByName() throws Exception {
    try (CallableStatement cs = createCallableStatement("CALL proc(?)")) {
      assertThrows(SQLFeatureNotSupportedException.class, () -> cs.getTime("p"));
    }
  }

  @Test
  void shouldThrowFeatureNotSupportedWhenGettingTimeByIndexWithCalendar() throws Exception {
    try (CallableStatement cs = createCallableStatement("CALL proc(?)")) {
      assertThrows(
          SQLFeatureNotSupportedException.class, () -> cs.getTime(1, Calendar.getInstance()));
    }
  }

  @Test
  void shouldThrowFeatureNotSupportedWhenGettingTimeByNameWithCalendar() throws Exception {
    try (CallableStatement cs = createCallableStatement("CALL proc(?)")) {
      assertThrows(
          SQLFeatureNotSupportedException.class, () -> cs.getTime("p", Calendar.getInstance()));
    }
  }

  @Test
  void shouldThrowFeatureNotSupportedWhenGettingTimestampByIndex() throws Exception {
    try (CallableStatement cs = createCallableStatement("CALL proc(?)")) {
      assertThrows(SQLFeatureNotSupportedException.class, () -> cs.getTimestamp(1));
    }
  }

  @Test
  void shouldThrowFeatureNotSupportedWhenGettingTimestampByName() throws Exception {
    try (CallableStatement cs = createCallableStatement("CALL proc(?)")) {
      assertThrows(SQLFeatureNotSupportedException.class, () -> cs.getTimestamp("p"));
    }
  }

  @Test
  void shouldThrowFeatureNotSupportedWhenGettingTimestampByIndexWithCalendar() throws Exception {
    try (CallableStatement cs = createCallableStatement("CALL proc(?)")) {
      assertThrows(
          SQLFeatureNotSupportedException.class, () -> cs.getTimestamp(1, Calendar.getInstance()));
    }
  }

  @Test
  void shouldThrowFeatureNotSupportedWhenGettingTimestampByNameWithCalendar() throws Exception {
    try (CallableStatement cs = createCallableStatement("CALL proc(?)")) {
      assertThrows(
          SQLFeatureNotSupportedException.class,
          () -> cs.getTimestamp("p", Calendar.getInstance()));
    }
  }

  @Test
  void shouldThrowFeatureNotSupportedWhenGettingObjectByIndex() throws Exception {
    try (CallableStatement cs = createCallableStatement("CALL proc(?)")) {
      assertThrows(SQLFeatureNotSupportedException.class, () -> cs.getObject(1));
    }
  }

  @Test
  void shouldThrowFeatureNotSupportedWhenGettingObjectByIndexWithMap() throws Exception {
    try (CallableStatement cs = createCallableStatement("CALL proc(?)")) {
      assertThrows(SQLFeatureNotSupportedException.class, () -> cs.getObject(1, new HashMap<>()));
    }
  }

  @Test
  void shouldThrowFeatureNotSupportedWhenGettingObjectByIndexWithClass() throws Exception {
    try (CallableStatement cs = createCallableStatement("CALL proc(?)")) {
      assertThrows(SQLFeatureNotSupportedException.class, () -> cs.getObject(1, String.class));
    }
  }

  @Test
  void shouldThrowFeatureNotSupportedWhenGettingObjectByName() throws Exception {
    try (CallableStatement cs = createCallableStatement("CALL proc(?)")) {
      assertThrows(SQLFeatureNotSupportedException.class, () -> cs.getObject("p"));
    }
  }

  @Test
  void shouldThrowFeatureNotSupportedWhenGettingObjectByNameWithMap() throws Exception {
    try (CallableStatement cs = createCallableStatement("CALL proc(?)")) {
      assertThrows(SQLFeatureNotSupportedException.class, () -> cs.getObject("p", new HashMap<>()));
    }
  }

  @Test
  void shouldThrowFeatureNotSupportedWhenGettingObjectByNameWithClass() throws Exception {
    try (CallableStatement cs = createCallableStatement("CALL proc(?)")) {
      assertThrows(SQLFeatureNotSupportedException.class, () -> cs.getObject("p", String.class));
    }
  }

  @Test
  void shouldThrowFeatureNotSupportedWhenGettingRefByIndex() throws Exception {
    try (CallableStatement cs = createCallableStatement("CALL proc(?)")) {
      assertThrows(SQLFeatureNotSupportedException.class, () -> cs.getRef(1));
    }
  }

  @Test
  void shouldThrowFeatureNotSupportedWhenGettingRefByName() throws Exception {
    try (CallableStatement cs = createCallableStatement("CALL proc(?)")) {
      assertThrows(SQLFeatureNotSupportedException.class, () -> cs.getRef("p"));
    }
  }

  @Test
  void shouldThrowFeatureNotSupportedWhenGettingBlobByIndex() throws Exception {
    try (CallableStatement cs = createCallableStatement("CALL proc(?)")) {
      assertThrows(SQLFeatureNotSupportedException.class, () -> cs.getBlob(1));
    }
  }

  @Test
  void shouldThrowFeatureNotSupportedWhenGettingBlobByName() throws Exception {
    try (CallableStatement cs = createCallableStatement("CALL proc(?)")) {
      assertThrows(SQLFeatureNotSupportedException.class, () -> cs.getBlob("p"));
    }
  }

  @Test
  void shouldThrowFeatureNotSupportedWhenGettingClobByIndex() throws Exception {
    try (CallableStatement cs = createCallableStatement("CALL proc(?)")) {
      assertThrows(SQLFeatureNotSupportedException.class, () -> cs.getClob(1));
    }
  }

  @Test
  void shouldThrowFeatureNotSupportedWhenGettingClobByName() throws Exception {
    try (CallableStatement cs = createCallableStatement("CALL proc(?)")) {
      assertThrows(SQLFeatureNotSupportedException.class, () -> cs.getClob("p"));
    }
  }

  @Test
  void shouldThrowFeatureNotSupportedWhenGettingArrayByIndex() throws Exception {
    try (CallableStatement cs = createCallableStatement("CALL proc(?)")) {
      assertThrows(SQLFeatureNotSupportedException.class, () -> cs.getArray(1));
    }
  }

  @Test
  void shouldThrowFeatureNotSupportedWhenGettingArrayByName() throws Exception {
    try (CallableStatement cs = createCallableStatement("CALL proc(?)")) {
      assertThrows(SQLFeatureNotSupportedException.class, () -> cs.getArray("p"));
    }
  }

  @Test
  void shouldThrowFeatureNotSupportedWhenGettingUrlByIndex() throws Exception {
    try (CallableStatement cs = createCallableStatement("CALL proc(?)")) {
      assertThrows(SQLFeatureNotSupportedException.class, () -> cs.getURL(1));
    }
  }

  @Test
  void shouldThrowFeatureNotSupportedWhenGettingUrlByName() throws Exception {
    try (CallableStatement cs = createCallableStatement("CALL proc(?)")) {
      assertThrows(SQLFeatureNotSupportedException.class, () -> cs.getURL("p"));
    }
  }

  @Test
  void shouldThrowFeatureNotSupportedWhenGettingRowIdByIndex() throws Exception {
    try (CallableStatement cs = createCallableStatement("CALL proc(?)")) {
      assertThrows(SQLFeatureNotSupportedException.class, () -> cs.getRowId(1));
    }
  }

  @Test
  void shouldThrowFeatureNotSupportedWhenGettingRowIdByName() throws Exception {
    try (CallableStatement cs = createCallableStatement("CALL proc(?)")) {
      assertThrows(SQLFeatureNotSupportedException.class, () -> cs.getRowId("p"));
    }
  }

  @Test
  void shouldThrowFeatureNotSupportedWhenGettingNClobByIndex() throws Exception {
    try (CallableStatement cs = createCallableStatement("CALL proc(?)")) {
      assertThrows(SQLFeatureNotSupportedException.class, () -> cs.getNClob(1));
    }
  }

  @Test
  void shouldThrowFeatureNotSupportedWhenGettingNClobByName() throws Exception {
    try (CallableStatement cs = createCallableStatement("CALL proc(?)")) {
      assertThrows(SQLFeatureNotSupportedException.class, () -> cs.getNClob("p"));
    }
  }

  @Test
  void shouldThrowFeatureNotSupportedWhenGettingSqlXmlByIndex() throws Exception {
    try (CallableStatement cs = createCallableStatement("CALL proc(?)")) {
      assertThrows(SQLFeatureNotSupportedException.class, () -> cs.getSQLXML(1));
    }
  }

  @Test
  void shouldThrowFeatureNotSupportedWhenGettingSqlXmlByName() throws Exception {
    try (CallableStatement cs = createCallableStatement("CALL proc(?)")) {
      assertThrows(SQLFeatureNotSupportedException.class, () -> cs.getSQLXML("p"));
    }
  }

  @Test
  void shouldThrowFeatureNotSupportedWhenGettingNStringByIndex() throws Exception {
    try (CallableStatement cs = createCallableStatement("CALL proc(?)")) {
      assertThrows(SQLFeatureNotSupportedException.class, () -> cs.getNString(1));
    }
  }

  @Test
  void shouldThrowFeatureNotSupportedWhenGettingNStringByName() throws Exception {
    try (CallableStatement cs = createCallableStatement("CALL proc(?)")) {
      assertThrows(SQLFeatureNotSupportedException.class, () -> cs.getNString("p"));
    }
  }

  @Test
  void shouldThrowFeatureNotSupportedWhenGettingNCharacterStreamByIndex() throws Exception {
    try (CallableStatement cs = createCallableStatement("CALL proc(?)")) {
      assertThrows(SQLFeatureNotSupportedException.class, () -> cs.getNCharacterStream(1));
    }
  }

  @Test
  void shouldThrowFeatureNotSupportedWhenGettingNCharacterStreamByName() throws Exception {
    try (CallableStatement cs = createCallableStatement("CALL proc(?)")) {
      assertThrows(SQLFeatureNotSupportedException.class, () -> cs.getNCharacterStream("p"));
    }
  }

  @Test
  void shouldThrowFeatureNotSupportedWhenGettingCharacterStreamByIndex() throws Exception {
    try (CallableStatement cs = createCallableStatement("CALL proc(?)")) {
      assertThrows(SQLFeatureNotSupportedException.class, () -> cs.getCharacterStream(1));
    }
  }

  @Test
  void shouldThrowFeatureNotSupportedWhenGettingCharacterStreamByName() throws Exception {
    try (CallableStatement cs = createCallableStatement("CALL proc(?)")) {
      assertThrows(SQLFeatureNotSupportedException.class, () -> cs.getCharacterStream("p"));
    }
  }

  // ── Name-based setters ────────────────────────────────────────────────────

  @Test
  void shouldThrowFeatureNotSupportedWhenSettingNullByName() throws Exception {
    try (CallableStatement cs = createCallableStatement("CALL proc(?)")) {
      assertThrows(SQLFeatureNotSupportedException.class, () -> cs.setNull("p", Types.NULL));
    }
  }

  @Test
  void shouldThrowFeatureNotSupportedWhenSettingNullByNameWithTypeName() throws Exception {
    try (CallableStatement cs = createCallableStatement("CALL proc(?)")) {
      assertThrows(
          SQLFeatureNotSupportedException.class, () -> cs.setNull("p", Types.NULL, "null"));
    }
  }

  @Test
  void shouldThrowFeatureNotSupportedWhenSettingBooleanByName() throws Exception {
    try (CallableStatement cs = createCallableStatement("CALL proc(?)")) {
      assertThrows(SQLFeatureNotSupportedException.class, () -> cs.setBoolean("p", true));
    }
  }

  @Test
  void shouldThrowFeatureNotSupportedWhenSettingByteByName() throws Exception {
    try (CallableStatement cs = createCallableStatement("CALL proc(?)")) {
      assertThrows(SQLFeatureNotSupportedException.class, () -> cs.setByte("p", (byte) 1));
    }
  }

  @Test
  void shouldThrowFeatureNotSupportedWhenSettingShortByName() throws Exception {
    try (CallableStatement cs = createCallableStatement("CALL proc(?)")) {
      assertThrows(SQLFeatureNotSupportedException.class, () -> cs.setShort("p", (short) 1));
    }
  }

  @Test
  void shouldThrowFeatureNotSupportedWhenSettingIntByName() throws Exception {
    try (CallableStatement cs = createCallableStatement("CALL proc(?)")) {
      assertThrows(SQLFeatureNotSupportedException.class, () -> cs.setInt("p", 1));
    }
  }

  @Test
  void shouldThrowFeatureNotSupportedWhenSettingLongByName() throws Exception {
    try (CallableStatement cs = createCallableStatement("CALL proc(?)")) {
      assertThrows(SQLFeatureNotSupportedException.class, () -> cs.setLong("p", 1L));
    }
  }

  @Test
  void shouldThrowFeatureNotSupportedWhenSettingFloatByName() throws Exception {
    try (CallableStatement cs = createCallableStatement("CALL proc(?)")) {
      assertThrows(SQLFeatureNotSupportedException.class, () -> cs.setFloat("p", 1.0f));
    }
  }

  @Test
  void shouldThrowFeatureNotSupportedWhenSettingDoubleByName() throws Exception {
    try (CallableStatement cs = createCallableStatement("CALL proc(?)")) {
      assertThrows(SQLFeatureNotSupportedException.class, () -> cs.setDouble("p", 1.0));
    }
  }

  @Test
  void shouldThrowFeatureNotSupportedWhenSettingBigDecimalByName() throws Exception {
    try (CallableStatement cs = createCallableStatement("CALL proc(?)")) {
      assertThrows(
          SQLFeatureNotSupportedException.class, () -> cs.setBigDecimal("p", BigDecimal.ONE));
    }
  }

  @Test
  void shouldThrowFeatureNotSupportedWhenSettingStringByName() throws Exception {
    try (CallableStatement cs = createCallableStatement("CALL proc(?)")) {
      assertThrows(SQLFeatureNotSupportedException.class, () -> cs.setString("p", "x"));
    }
  }

  @Test
  void shouldThrowFeatureNotSupportedWhenSettingBytesByName() throws Exception {
    try (CallableStatement cs = createCallableStatement("CALL proc(?)")) {
      assertThrows(SQLFeatureNotSupportedException.class, () -> cs.setBytes("p", new byte[] {1}));
    }
  }

  @Test
  void shouldThrowFeatureNotSupportedWhenSettingDateByName() throws Exception {
    try (CallableStatement cs = createCallableStatement("CALL proc(?)")) {
      assertThrows(
          SQLFeatureNotSupportedException.class, () -> cs.setDate("p", Date.valueOf("2024-01-01")));
    }
  }

  @Test
  void shouldThrowFeatureNotSupportedWhenSettingDateByNameWithCalendar() throws Exception {
    try (CallableStatement cs = createCallableStatement("CALL proc(?)")) {
      assertThrows(
          SQLFeatureNotSupportedException.class,
          () -> cs.setDate("p", Date.valueOf("2024-01-01"), Calendar.getInstance()));
    }
  }

  @Test
  void shouldThrowFeatureNotSupportedWhenSettingTimeByName() throws Exception {
    try (CallableStatement cs = createCallableStatement("CALL proc(?)")) {
      assertThrows(SQLFeatureNotSupportedException.class, () -> cs.setTime("p", new Time(0)));
    }
  }

  @Test
  void shouldThrowFeatureNotSupportedWhenSettingTimeByNameWithCalendar() throws Exception {
    try (CallableStatement cs = createCallableStatement("CALL proc(?)")) {
      assertThrows(
          SQLFeatureNotSupportedException.class,
          () -> cs.setTime("p", new Time(0), Calendar.getInstance()));
    }
  }

  @Test
  void shouldThrowFeatureNotSupportedWhenSettingTimestampByName() throws Exception {
    try (CallableStatement cs = createCallableStatement("CALL proc(?)")) {
      assertThrows(
          SQLFeatureNotSupportedException.class, () -> cs.setTimestamp("p", new Timestamp(0)));
    }
  }

  @Test
  void shouldThrowFeatureNotSupportedWhenSettingTimestampByNameWithCalendar() throws Exception {
    try (CallableStatement cs = createCallableStatement("CALL proc(?)")) {
      assertThrows(
          SQLFeatureNotSupportedException.class,
          () -> cs.setTimestamp("p", new Timestamp(0), Calendar.getInstance()));
    }
  }

  @Test
  void shouldThrowFeatureNotSupportedWhenSettingAsciiStreamByName() throws Exception {
    try (CallableStatement cs = createCallableStatement("CALL proc(?)")) {
      assertThrows(
          SQLFeatureNotSupportedException.class,
          () -> cs.setAsciiStream("p", new ByteArrayInputStream(new byte[] {1})));
    }
  }

  @Test
  void shouldThrowFeatureNotSupportedWhenSettingAsciiStreamByNameWithIntLength() throws Exception {
    try (CallableStatement cs = createCallableStatement("CALL proc(?)")) {
      assertThrows(
          SQLFeatureNotSupportedException.class,
          () -> cs.setAsciiStream("p", new ByteArrayInputStream(new byte[] {1}), 1));
    }
  }

  @Test
  void shouldThrowFeatureNotSupportedWhenSettingAsciiStreamByNameWithLongLength() throws Exception {
    try (CallableStatement cs = createCallableStatement("CALL proc(?)")) {
      assertThrows(
          SQLFeatureNotSupportedException.class,
          () -> cs.setAsciiStream("p", new ByteArrayInputStream(new byte[] {1}), 1L));
    }
  }

  @Test
  void shouldThrowFeatureNotSupportedWhenSettingBinaryStreamByName() throws Exception {
    try (CallableStatement cs = createCallableStatement("CALL proc(?)")) {
      assertThrows(
          SQLFeatureNotSupportedException.class,
          () -> cs.setBinaryStream("p", new ByteArrayInputStream(new byte[] {1})));
    }
  }

  @Test
  void shouldThrowFeatureNotSupportedWhenSettingBinaryStreamByNameWithIntLength() throws Exception {
    try (CallableStatement cs = createCallableStatement("CALL proc(?)")) {
      assertThrows(
          SQLFeatureNotSupportedException.class,
          () -> cs.setBinaryStream("p", new ByteArrayInputStream(new byte[] {1}), 1));
    }
  }

  @Test
  void shouldThrowFeatureNotSupportedWhenSettingBinaryStreamByNameWithLongLength()
      throws Exception {
    try (CallableStatement cs = createCallableStatement("CALL proc(?)")) {
      assertThrows(
          SQLFeatureNotSupportedException.class,
          () -> cs.setBinaryStream("p", new ByteArrayInputStream(new byte[] {1}), 1L));
    }
  }

  @Test
  void shouldThrowFeatureNotSupportedWhenSettingCharacterStreamByName() throws Exception {
    try (CallableStatement cs = createCallableStatement("CALL proc(?)")) {
      assertThrows(
          SQLFeatureNotSupportedException.class,
          () -> cs.setCharacterStream("p", new StringReader("x")));
    }
  }

  @Test
  void shouldThrowFeatureNotSupportedWhenSettingCharacterStreamByNameWithIntLength()
      throws Exception {
    try (CallableStatement cs = createCallableStatement("CALL proc(?)")) {
      assertThrows(
          SQLFeatureNotSupportedException.class,
          () -> cs.setCharacterStream("p", new StringReader("x"), 1));
    }
  }

  @Test
  void shouldThrowFeatureNotSupportedWhenSettingCharacterStreamByNameWithLongLength()
      throws Exception {
    try (CallableStatement cs = createCallableStatement("CALL proc(?)")) {
      assertThrows(
          SQLFeatureNotSupportedException.class,
          () -> cs.setCharacterStream("p", new StringReader("x"), 1L));
    }
  }

  @Test
  void shouldThrowFeatureNotSupportedWhenSettingObjectByName() throws Exception {
    try (CallableStatement cs = createCallableStatement("CALL proc(?)")) {
      assertThrows(SQLFeatureNotSupportedException.class, () -> cs.setObject("p", new Object()));
    }
  }

  @Test
  void shouldThrowFeatureNotSupportedWhenSettingObjectByNameWithTargetSqlType() throws Exception {
    try (CallableStatement cs = createCallableStatement("CALL proc(?)")) {
      assertThrows(
          SQLFeatureNotSupportedException.class,
          () -> cs.setObject("p", new Object(), Types.JAVA_OBJECT));
    }
  }

  @Test
  void shouldThrowFeatureNotSupportedWhenSettingObjectByNameWithTargetSqlTypeAndScale()
      throws Exception {
    try (CallableStatement cs = createCallableStatement("CALL proc(?)")) {
      assertThrows(
          SQLFeatureNotSupportedException.class,
          () -> cs.setObject("p", new Object(), Types.JAVA_OBJECT, 0));
    }
  }

  @Test
  void shouldThrowFeatureNotSupportedWhenSettingUrlByName() throws Exception {
    try (CallableStatement cs = createCallableStatement("CALL proc(?)")) {
      URL fakeURL = new URL("http://localhost:8888/");
      assertThrows(SQLFeatureNotSupportedException.class, () -> cs.setURL("p", fakeURL));
    }
  }

  @Test
  void shouldThrowFeatureNotSupportedWhenSettingRowIdByName() throws Exception {
    try (CallableStatement cs = createCallableStatement("CALL proc(?)")) {
      assertThrows(SQLFeatureNotSupportedException.class, () -> cs.setRowId("p", null));
    }
  }

  @Test
  void shouldThrowFeatureNotSupportedWhenSettingSqlXmlByName() throws Exception {
    try (CallableStatement cs = createCallableStatement("CALL proc(?)")) {
      assertThrows(SQLFeatureNotSupportedException.class, () -> cs.setSQLXML("p", null));
    }
  }

  @Test
  void shouldThrowFeatureNotSupportedWhenSettingNStringByName() throws Exception {
    try (CallableStatement cs = createCallableStatement("CALL proc(?)")) {
      assertThrows(SQLFeatureNotSupportedException.class, () -> cs.setNString("p", "x"));
    }
  }

  @Test
  void shouldThrowFeatureNotSupportedWhenSettingNCharacterStreamByName() throws Exception {
    try (CallableStatement cs = createCallableStatement("CALL proc(?)")) {
      assertThrows(
          SQLFeatureNotSupportedException.class,
          () -> cs.setNCharacterStream("p", new StringReader("x")));
    }
  }

  @Test
  void shouldThrowFeatureNotSupportedWhenSettingNCharacterStreamByNameWithLength()
      throws Exception {
    try (CallableStatement cs = createCallableStatement("CALL proc(?)")) {
      assertThrows(
          SQLFeatureNotSupportedException.class,
          () -> cs.setNCharacterStream("p", new StringReader("x"), 1L));
    }
  }

  @Test
  void shouldThrowFeatureNotSupportedWhenSettingNClobByNameWithNClob() throws Exception {
    try (CallableStatement cs = createCallableStatement("CALL proc(?)")) {
      assertThrows(
          SQLFeatureNotSupportedException.class, () -> cs.setNClob("p", (java.sql.NClob) null));
    }
  }

  @Test
  void shouldThrowFeatureNotSupportedWhenSettingNClobByNameWithReader() throws Exception {
    try (CallableStatement cs = createCallableStatement("CALL proc(?)")) {
      assertThrows(
          SQLFeatureNotSupportedException.class, () -> cs.setNClob("p", new StringReader("x")));
    }
  }

  @Test
  void shouldThrowFeatureNotSupportedWhenSettingNClobByNameWithReaderAndLength() throws Exception {
    try (CallableStatement cs = createCallableStatement("CALL proc(?)")) {
      assertThrows(
          SQLFeatureNotSupportedException.class, () -> cs.setNClob("p", new StringReader("x"), 1L));
    }
  }

  @Test
  void shouldThrowFeatureNotSupportedWhenSettingClobByNameWithClob() throws Exception {
    try (CallableStatement cs = createCallableStatement("CALL proc(?)")) {
      assertThrows(
          SQLFeatureNotSupportedException.class, () -> cs.setClob("p", (java.sql.Clob) null));
    }
  }

  @Test
  void shouldThrowFeatureNotSupportedWhenSettingClobByNameWithReader() throws Exception {
    try (CallableStatement cs = createCallableStatement("CALL proc(?)")) {
      assertThrows(
          SQLFeatureNotSupportedException.class, () -> cs.setClob("p", new StringReader("x")));
    }
  }

  @Test
  void shouldThrowFeatureNotSupportedWhenSettingClobByNameWithReaderAndLength() throws Exception {
    try (CallableStatement cs = createCallableStatement("CALL proc(?)")) {
      assertThrows(
          SQLFeatureNotSupportedException.class, () -> cs.setClob("p", new StringReader("x"), 1L));
    }
  }

  @Test
  void shouldThrowFeatureNotSupportedWhenSettingBlobByNameWithBlob() throws Exception {
    try (CallableStatement cs = createCallableStatement("CALL proc(?)")) {
      assertThrows(
          SQLFeatureNotSupportedException.class, () -> cs.setBlob("p", (java.sql.Blob) null));
    }
  }

  @Test
  void shouldThrowFeatureNotSupportedWhenSettingBlobByNameWithInputStream() throws Exception {
    try (CallableStatement cs = createCallableStatement("CALL proc(?)")) {
      assertThrows(
          SQLFeatureNotSupportedException.class,
          () -> cs.setBlob("p", new ByteArrayInputStream(new byte[] {1})));
    }
  }

  @Test
  void shouldThrowFeatureNotSupportedWhenSettingBlobByNameWithInputStreamAndLength()
      throws Exception {
    try (CallableStatement cs = createCallableStatement("CALL proc(?)")) {
      assertThrows(
          SQLFeatureNotSupportedException.class,
          () -> cs.setBlob("p", new ByteArrayInputStream(new byte[] {1}), 1L));
    }
  }

  // ── getParameterMetaData ──────────────────────────────────────────────────

  @Nested
  class GetParameterMetaData {

    private void stubDescribe(int numberOfBinds) throws Exception {
      // The server returns one metaDataOfBinds entry per bind; mirror that so the
      // reported parameter count (derived from the bind list) matches.
      ColumnMetadata[] binds = new ColumnMetadata[numberOfBinds];
      for (int i = 0; i < numberOfBinds; i++) {
        binds[i] = bind("text", true, 0, 0);
      }
      stubDescribe(numberOfBinds, binds);
    }

    private void stubDescribe(int numberOfBinds, ColumnMetadata... binds) throws Exception {
      when(mockCoreApi.statementSetSqlQuery(any(), any()))
          .thenReturn(StatementSetSqlQueryResponse.getDefaultInstance());
      PrepareResult.Builder result = PrepareResult.newBuilder().setNumberOfBinds(numberOfBinds);
      for (ColumnMetadata bind : binds) {
        result.addBinds(bind);
      }
      when(mockCoreApi.statementPrepare(any()))
          .thenReturn(StatementPrepareResponse.newBuilder().setResult(result.build()).build());
    }

    private ColumnMetadata bind(String type, boolean nullable, long precision, long scale) {
      return ColumnMetadata.newBuilder()
          .setType(type)
          .setNullable(nullable)
          .setPrecision(precision)
          .setScale(scale)
          .build();
    }

    @Test
    void shouldReportBindCountFromDescribe() throws Exception {
      stubDescribe(2);
      try (CallableStatement cs = createCallableStatement("call add_nums(?,?)")) {
        ParameterMetaData metaData = cs.getParameterMetaData();
        assertEquals(2, metaData.getParameterCount());
      }
    }

    @Test
    void shouldReportZeroBindsForLiteralCall() throws Exception {
      stubDescribe(0);
      try (CallableStatement cs = createCallableStatement("call square_it(5)")) {
        ParameterMetaData metaData = cs.getParameterMetaData();
        assertEquals(0, metaData.getParameterCount());
      }
    }

    @Test
    void shouldDescribeOnceAndCacheResult() throws Exception {
      stubDescribe(2);
      try (CallableStatement cs = createCallableStatement("CALL add_nums(?,?)")) {
        assertEquals(2, cs.getParameterMetaData().getParameterCount());
        assertEquals(2, cs.getParameterMetaData().getParameterCount());
        assertEquals(2, cs.getParameterMetaData().getParameterCount());

        verify(mockCoreApi, times(1)).statementPrepare(any());
        verify(mockCoreApi, times(1)).statementSetSqlQuery(any(), any());
      }
    }

    @Test
    void shouldThrowFeatureNotSupportedForUnsupportedPerParameterMethods() throws Exception {
      stubDescribe(1, bind("FIXED", true, 38, 0));
      try (CallableStatement cs = createCallableStatement("call square_it(?)")) {
        ParameterMetaData metaData = cs.getParameterMetaData();
        assertThrows(SQLFeatureNotSupportedException.class, () -> metaData.isSigned(1));
        assertThrows(
            SQLFeatureNotSupportedException.class, () -> metaData.getParameterClassName(1));
        assertThrows(SQLFeatureNotSupportedException.class, () -> metaData.getParameterMode(1));
      }
    }

    @Test
    void shouldReportTypeForFixedBind() throws Exception {
      stubDescribe(1, bind("FIXED", false, 38, 2));
      try (CallableStatement cs = createCallableStatement("call square_it(?)")) {
        ParameterMetaData metaData = cs.getParameterMetaData();
        assertEquals(Types.OTHER, metaData.getParameterType(1));
        assertEquals("FIXED", metaData.getParameterTypeName(1));
        assertEquals(38, metaData.getPrecision(1));
        assertEquals(2, metaData.getScale(1));
        assertEquals(ParameterMetaData.parameterNoNulls, metaData.isNullable(1));
      }
    }

    @Test
    void shouldReportTypeForTextBind() throws Exception {
      stubDescribe(1, bind("TEXT", true, 0, 0));
      try (CallableStatement cs = createCallableStatement("call echo(?)")) {
        ParameterMetaData metaData = cs.getParameterMetaData();
        assertEquals(Types.VARCHAR, metaData.getParameterType(1));
        assertEquals("TEXT", metaData.getParameterTypeName(1));
        assertEquals(ParameterMetaData.parameterNullable, metaData.isNullable(1));
      }
    }

    @Test
    void shouldMapEachBindIndependently() throws Exception {
      stubDescribe(2, bind("FIXED", false, 10, 0), bind("BOOLEAN", true, 0, 0));
      try (CallableStatement cs = createCallableStatement("call add_nums(?,?)")) {
        ParameterMetaData metaData = cs.getParameterMetaData();
        assertEquals(Types.OTHER, metaData.getParameterType(1));
        assertEquals(Types.BOOLEAN, metaData.getParameterType(2));
      }
    }

    @Test
    void shouldMapUnknownTypeToOther() throws Exception {
      stubDescribe(1, bind("SOME_FUTURE_TYPE", true, 0, 0));
      try (CallableStatement cs = createCallableStatement("call f(?)")) {
        ParameterMetaData metaData = cs.getParameterMetaData();
        assertEquals(Types.OTHER, metaData.getParameterType(1));
      }
    }

    @Test
    void shouldMapLowercaseServerTypeNamesCaseInsensitively() throws Exception {
      // The server reports bind type names in lowercase (verified against a live
      // describe-only response: e.g. {"type": "text"}). The SQL-type mapping is
      // case-insensitive, but getParameterTypeName returns the value verbatim.
      stubDescribe(1, bind("text", true, 0, 0));
      try (CallableStatement cs = createCallableStatement("call echo(?)")) {
        ParameterMetaData metaData = cs.getParameterMetaData();
        assertEquals(Types.VARCHAR, metaData.getParameterType(1));
        assertEquals("text", metaData.getParameterTypeName(1));
      }
    }

    @Test
    void shouldThrowForOutOfRangeIndex() throws Exception {
      stubDescribe(1, bind("FIXED", true, 38, 0));
      try (CallableStatement cs = createCallableStatement("call square_it(?)")) {
        ParameterMetaData metaData = cs.getParameterMetaData();
        assertThrows(java.sql.SQLException.class, () -> metaData.getParameterType(2));
        assertThrows(java.sql.SQLException.class, () -> metaData.getParameterType(0));
      }
    }
  }
}
