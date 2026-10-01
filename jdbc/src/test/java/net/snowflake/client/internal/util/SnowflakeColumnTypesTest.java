package net.snowflake.client.internal.util;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertNull;
import static org.junit.jupiter.api.Assertions.assertTrue;

import java.sql.Types;
import net.snowflake.client.api.resultset.SnowflakeType;
import net.snowflake.client.internal.util.SnowflakeColumnTypes.ColumnTypeInfo;
import org.junit.jupiter.api.Test;
import org.junit.jupiter.params.ParameterizedTest;
import org.junit.jupiter.params.provider.CsvSource;

class SnowflakeColumnTypesTest {

  @ParameterizedTest
  @CsvSource({"INTEGER, INTEGER", "timestamp_ntz, TIMESTAMP_NTZ", "unknown_type, ", "'', "})
  void shouldParseInternalTypeName(String name, String expectedEnumName) {
    SnowflakeType parsed = SnowflakeColumnTypes.fromStringOrNull(name);
    if (expectedEnumName == null || expectedEnumName.isEmpty()) {
      assertNull(parsed);
    } else {
      assertEquals(SnowflakeType.valueOf(expectedEnumName), parsed);
    }
  }

  @Test
  void shouldReturnNullForNullInternalTypeName() {
    assertNull(SnowflakeColumnTypes.fromStringOrNull(null));
  }

  @Test
  void shouldMapIntegerColumnToJdbcInteger() {
    ColumnTypeInfo info =
        SnowflakeColumnTypes.getSnowflakeType("INTEGER", null, null, Types.NUMERIC, false, false);
    assertEquals(Types.INTEGER, info.getColumnType());
    assertEquals("INTEGER", info.getExtColTypeName());
    assertEquals(SnowflakeType.INTEGER, info.getSnowflakeType());
  }

  @Test
  void shouldMapTextColumnToVarchar() {
    ColumnTypeInfo info =
        SnowflakeColumnTypes.getSnowflakeType("TEXT", null, null, Types.NUMERIC, false, false);
    assertEquals(Types.VARCHAR, info.getColumnType());
    assertEquals("VARCHAR", info.getExtColTypeName());
    assertEquals(SnowflakeType.TEXT, info.getSnowflakeType());
  }

  @Test
  void shouldDetectVectorTypeName() {
    assertTrue(SnowflakeColumnTypes.isVectorType("VECTOR"));
    assertTrue(SnowflakeColumnTypes.isVectorType("vector"));
  }

  @Test
  void shouldMapGeographyInternalTypeToBinaryWhenUdtOutputIsBinary() {
    ColumnTypeInfo info =
        SnowflakeColumnTypes.getSnowflakeType(
            "GEOGRAPHY", "GEOGRAPHY", "binary", Types.NUMERIC, false, false);
    assertEquals(Types.BINARY, info.getColumnType());
    assertEquals("GEOGRAPHY", info.getExtColTypeName());
    assertEquals(SnowflakeType.GEOGRAPHY, info.getSnowflakeType());
  }

  @Test
  void shouldMapGeometryInternalTypeToVarcharWhenUdtOutputIsText() {
    ColumnTypeInfo info =
        SnowflakeColumnTypes.getSnowflakeType(
            "GEOMETRY", "GEOMETRY", "text", Types.NUMERIC, false, false);
    assertEquals(Types.VARCHAR, info.getColumnType());
    assertEquals("GEOMETRY", info.getExtColTypeName());
    assertEquals(SnowflakeType.GEOMETRY, info.getSnowflakeType());
  }

  @Test
  void shouldReportUnknownInternalTypeAsJdbcOtherWithExtName() {
    ColumnTypeInfo info =
        SnowflakeColumnTypes.getSnowflakeType(
            "uuid", "CUSTOM_EXT", null, Types.NUMERIC, false, false);
    assertEquals(Types.OTHER, info.getColumnType());
    assertEquals("CUSTOM_EXT", info.getExtColTypeName());
    assertEquals(SnowflakeType.ANY, info.getSnowflakeType());
  }
}
