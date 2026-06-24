package net.snowflake.client.internal.api.implementation.resultset;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertNull;

import java.sql.Types;
import org.junit.jupiter.api.Test;

class SnowflakeResultSetMetaDataImplTest {

  @Test
  void getQueryIdReturnsValueProvidedAtConstruction() throws Exception {
    SnowflakeResultSetMetaDataImpl meta =
        new SnowflakeResultSetMetaDataImpl(new String[] {"C"}, new int[] {Types.INTEGER}, "qid-1");
    assertEquals("qid-1", meta.getQueryID());
  }

  @Test
  void getQueryIdReturnsNullWhenConstructedWithoutOne() throws Exception {
    SnowflakeResultSetMetaDataImpl meta =
        new SnowflakeResultSetMetaDataImpl(new String[] {"C"}, new int[] {Types.INTEGER}, null);
    assertNull(meta.getQueryID());
  }

  @Test
  void shouldReportTypeNameClassNameScaleAndStringLengthForTimeColumn() throws Exception {
    SnowflakeResultSetMetaDataImpl meta =
        new SnowflakeResultSetMetaDataImpl(
            new String[] {"T"}, new int[] {Types.TIME}, new int[] {9}, 18, "qid-2");

    assertEquals(Types.TIME, meta.getColumnType(1));
    assertEquals("TIME", meta.getColumnTypeName(1));
    assertEquals("java.sql.Time", meta.getColumnClassName(1));
    assertEquals(9, meta.getScale(1));
    // precision and display size both equal the formatted time-string length.
    assertEquals(18, meta.getPrecision(1));
    assertEquals(18, meta.getColumnDisplaySize(1));
  }

  @Test
  void shouldDefaultToStringLengthEightForTimeColumnWithoutFormat() throws Exception {
    SnowflakeResultSetMetaDataImpl meta =
        new SnowflakeResultSetMetaDataImpl(new String[] {"T"}, new int[] {Types.TIME}, "qid-3");
    assertEquals(8, meta.getPrecision(1));
    assertEquals(8, meta.getColumnDisplaySize(1));
  }
}
