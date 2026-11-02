package net.snowflake.client.internal.util;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertFalse;
import static org.junit.jupiter.api.Assertions.assertNull;
import static org.junit.jupiter.api.Assertions.assertThrows;
import static org.junit.jupiter.api.Assertions.assertTrue;

import java.sql.Types;
import net.snowflake.client.api.exception.ErrorCode;
import net.snowflake.client.api.resultset.SnowflakeType;
import net.snowflake.client.internal.api.implementation.exception.SFSQLException;
import net.snowflake.client.internal.util.SnowflakeColumnTypes.ColumnTypeInfo;
import org.junit.jupiter.api.Test;
import org.junit.jupiter.params.ParameterizedTest;
import org.junit.jupiter.params.provider.CsvSource;

class SnowflakeColumnTypesTest {

  @ParameterizedTest
  @CsvSource({
    "INTEGER, INTEGER",
    "timestamp_ntz, TIMESTAMP_NTZ",
    "TIMESTAMP_NTZ, TIMESTAMP_NTZ",
    "timestamp_ltz, TIMESTAMP_LTZ",
    "TIMESTAMP_LTZ, TIMESTAMP_LTZ",
    "timestamp_tz, TIMESTAMP_TZ",
    "TIMESTAMP_TZ, TIMESTAMP_TZ",
    "fixed, FIXED",
    "unknown_type, ",
    "'', "
  })
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
  void shouldMapGeographyInternalTypeToVarcharWhenUdtOutputIsObject() {
    ColumnTypeInfo info =
        SnowflakeColumnTypes.getSnowflakeType(
            "GEOGRAPHY", "GEOGRAPHY", "object", Types.NUMERIC, false, false);
    assertEquals(Types.VARCHAR, info.getColumnType());
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

  @Test
  void shouldMapGeographyUdtObjectOutputToVarchar() {
    ColumnTypeInfo info =
        SnowflakeColumnTypes.getSnowflakeType(
            "GEOGRAPHY", "GEOGRAPHY", "object", Types.NUMERIC, false, false);
    assertEquals(Types.VARCHAR, info.getColumnType());
    assertEquals("GEOGRAPHY", info.getExtColTypeName());
    assertEquals(SnowflakeType.GEOGRAPHY, info.getSnowflakeType());
  }

  @Test
  void shouldMapVectorInternalTypeToExtraTypesVector() {
    ColumnTypeInfo info =
        SnowflakeColumnTypes.getSnowflakeType("VECTOR", null, null, Types.NUMERIC, false, false);
    assertEquals(SnowflakeType.EXTRA_TYPES_VECTOR, info.getColumnType());
    assertEquals("VECTOR", info.getExtColTypeName());
    assertEquals(SnowflakeType.VECTOR, info.getSnowflakeType());
  }

  @Test
  void shouldMapFixedColumnOnVectorWireTypeToInteger() {
    ColumnTypeInfo info =
        SnowflakeColumnTypes.getSnowflakeType("fixed", null, null, Types.DECIMAL, false, true);
    assertEquals(Types.INTEGER, info.getColumnType());
    assertEquals(SnowflakeType.FIXED, info.getSnowflakeType());
  }

  @Test
  void shouldMapScalarTemporalAndIntervalInternalTypesToExpectedJdbcTypes() {
    ColumnTypeInfo charInfo =
        SnowflakeColumnTypes.getSnowflakeType("CHAR", null, null, Types.NUMERIC, false, false);
    assertEquals(Types.CHAR, charInfo.getColumnType());
    assertEquals("CHAR", charInfo.getExtColTypeName());
    assertEquals(SnowflakeType.CHAR, charInfo.getSnowflakeType());

    ColumnTypeInfo decfloatInfo =
        SnowflakeColumnTypes.getSnowflakeType(
            "DECFLOAT", "IGNORED_EXT", null, Types.NUMERIC, false, false);
    assertEquals(Types.DECIMAL, decfloatInfo.getColumnType());
    assertEquals("DECFLOAT", decfloatInfo.getExtColTypeName());
    assertEquals(SnowflakeType.DECFLOAT, decfloatInfo.getSnowflakeType());

    ColumnTypeInfo fixedInfo =
        SnowflakeColumnTypes.getSnowflakeType("FIXED", null, null, Types.BIGINT, false, false);
    assertEquals(Types.BIGINT, fixedInfo.getColumnType());
    assertEquals("NUMBER", fixedInfo.getExtColTypeName());
    assertEquals(SnowflakeType.FIXED, fixedInfo.getSnowflakeType());

    ColumnTypeInfo realScalarInfo =
        SnowflakeColumnTypes.getSnowflakeType("REAL", null, null, Types.NUMERIC, false, false);
    assertEquals(Types.DOUBLE, realScalarInfo.getColumnType());
    assertEquals("DOUBLE", realScalarInfo.getExtColTypeName());
    assertEquals(SnowflakeType.REAL, realScalarInfo.getSnowflakeType());

    ColumnTypeInfo realVectorInfo =
        SnowflakeColumnTypes.getSnowflakeType("REAL", null, null, Types.NUMERIC, false, true);
    assertEquals(Types.FLOAT, realVectorInfo.getColumnType());
    assertEquals("FLOAT", realVectorInfo.getExtColTypeName());
    assertEquals(SnowflakeType.REAL, realVectorInfo.getSnowflakeType());

    ColumnTypeInfo timestampInfo =
        SnowflakeColumnTypes.getSnowflakeType("TIMESTAMP", null, null, Types.NUMERIC, false, false);
    assertEquals(SnowflakeType.EXTRA_TYPES_TIMESTAMP_LTZ, timestampInfo.getColumnType());
    assertEquals("TIMESTAMPLTZ", timestampInfo.getExtColTypeName());
    assertEquals(SnowflakeType.TIMESTAMP, timestampInfo.getSnowflakeType());

    ColumnTypeInfo ltzInfo =
        SnowflakeColumnTypes.getSnowflakeType(
            "TIMESTAMP_LTZ", null, null, Types.NUMERIC, false, false);
    assertEquals(SnowflakeType.EXTRA_TYPES_TIMESTAMP_LTZ, ltzInfo.getColumnType());
    assertEquals("TIMESTAMPLTZ", ltzInfo.getExtColTypeName());
    assertEquals(SnowflakeType.TIMESTAMP_LTZ, ltzInfo.getSnowflakeType());

    ColumnTypeInfo ntzInfo =
        SnowflakeColumnTypes.getSnowflakeType(
            "TIMESTAMP_NTZ", null, null, Types.NUMERIC, false, false);
    assertEquals(Types.TIMESTAMP, ntzInfo.getColumnType());
    assertEquals("TIMESTAMPNTZ", ntzInfo.getExtColTypeName());
    assertEquals(SnowflakeType.TIMESTAMP_NTZ, ntzInfo.getSnowflakeType());

    ColumnTypeInfo tzInfo =
        SnowflakeColumnTypes.getSnowflakeType(
            "TIMESTAMP_TZ", null, null, Types.NUMERIC, false, false);
    assertEquals(SnowflakeType.EXTRA_TYPES_TIMESTAMP_TZ, tzInfo.getColumnType());
    assertEquals("TIMESTAMPTZ", tzInfo.getExtColTypeName());
    assertEquals(SnowflakeType.TIMESTAMP_TZ, tzInfo.getSnowflakeType());

    ColumnTypeInfo ymInfo =
        SnowflakeColumnTypes.getSnowflakeType(
            "INTERVAL_YEAR_MONTH", null, null, Types.NUMERIC, false, false);
    assertEquals(SnowflakeType.EXTRA_TYPES_YEAR_MONTH_INTERVAL, ymInfo.getColumnType());
    assertEquals("INTERVAL_YEAR_MONTH", ymInfo.getExtColTypeName());
    assertEquals(SnowflakeType.INTERVAL_YEAR_MONTH, ymInfo.getSnowflakeType());

    ColumnTypeInfo dtInfo =
        SnowflakeColumnTypes.getSnowflakeType(
            "INTERVAL_DAY_TIME", null, null, Types.NUMERIC, false, false);
    assertEquals(SnowflakeType.EXTRA_TYPES_DAY_TIME_INTERVAL, dtInfo.getColumnType());
    assertEquals("INTERVAL_DAY_TIME", dtInfo.getExtColTypeName());
    assertEquals(SnowflakeType.INTERVAL_DAY_TIME, dtInfo.getSnowflakeType());

    ColumnTypeInfo dateInfo =
        SnowflakeColumnTypes.getSnowflakeType("DATE", null, null, Types.NUMERIC, false, false);
    assertEquals(Types.DATE, dateInfo.getColumnType());
    assertEquals("DATE", dateInfo.getExtColTypeName());

    ColumnTypeInfo timeInfo =
        SnowflakeColumnTypes.getSnowflakeType("TIME", null, null, Types.NUMERIC, false, false);
    assertEquals(Types.TIME, timeInfo.getColumnType());
    assertEquals("TIME", timeInfo.getExtColTypeName());

    ColumnTypeInfo boolInfo =
        SnowflakeColumnTypes.getSnowflakeType("BOOLEAN", null, null, Types.NUMERIC, false, false);
    assertEquals(Types.BOOLEAN, boolInfo.getColumnType());
    assertEquals("BOOLEAN", boolInfo.getExtColTypeName());

    ColumnTypeInfo binaryInfo =
        SnowflakeColumnTypes.getSnowflakeType("BINARY", null, null, Types.NUMERIC, false, false);
    assertEquals(Types.BINARY, binaryInfo.getColumnType());
    assertEquals("BINARY", binaryInfo.getExtColTypeName());

    ColumnTypeInfo variantInfo =
        SnowflakeColumnTypes.getSnowflakeType("VARIANT", null, null, Types.NUMERIC, false, false);
    assertEquals(Types.VARCHAR, variantInfo.getColumnType());
    assertEquals("VARIANT", variantInfo.getExtColTypeName());
  }

  @Test
  void shouldMapStructuredAndUnstructuredComplexTypesAndGeoOutputs() {
    ColumnTypeInfo unstructuredArray =
        SnowflakeColumnTypes.getSnowflakeType("ARRAY", null, null, Types.NUMERIC, false, false);
    assertEquals(Types.VARCHAR, unstructuredArray.getColumnType());
    assertEquals("ARRAY", unstructuredArray.getExtColTypeName());

    ColumnTypeInfo structuredArray =
        SnowflakeColumnTypes.getSnowflakeType("ARRAY", null, null, Types.NUMERIC, true, false);
    assertEquals(Types.ARRAY, structuredArray.getColumnType());
    assertEquals("ARRAY", structuredArray.getExtColTypeName());

    ColumnTypeInfo mapInfo =
        SnowflakeColumnTypes.getSnowflakeType("MAP", null, null, Types.NUMERIC, true, false);
    assertEquals(Types.STRUCT, mapInfo.getColumnType());
    assertEquals("OBJECT", mapInfo.getExtColTypeName());
    assertEquals(SnowflakeType.MAP, mapInfo.getSnowflakeType());

    ColumnTypeInfo unstructuredObject =
        SnowflakeColumnTypes.getSnowflakeType("OBJECT", null, null, Types.NUMERIC, false, false);
    assertEquals(Types.VARCHAR, unstructuredObject.getColumnType());
    assertEquals("OBJECT", unstructuredObject.getExtColTypeName());

    ColumnTypeInfo structuredObject =
        SnowflakeColumnTypes.getSnowflakeType("OBJECT", null, null, Types.NUMERIC, true, false);
    assertEquals(Types.STRUCT, structuredObject.getColumnType());
    assertEquals("OBJECT", structuredObject.getExtColTypeName());

    ColumnTypeInfo structuredGeographyObject =
        SnowflakeColumnTypes.getSnowflakeType(
            "OBJECT", "GEOGRAPHY", null, Types.NUMERIC, true, false);
    assertEquals(Types.VARCHAR, structuredGeographyObject.getColumnType());
    assertEquals("GEOGRAPHY", structuredGeographyObject.getExtColTypeName());

    ColumnTypeInfo structuredGeometryObject =
        SnowflakeColumnTypes.getSnowflakeType(
            "OBJECT", "GEOMETRY", null, Types.NUMERIC, true, false);
    assertEquals(Types.VARCHAR, structuredGeometryObject.getColumnType());
    assertEquals("GEOMETRY", structuredGeometryObject.getExtColTypeName());

    ColumnTypeInfo geometryBinary =
        SnowflakeColumnTypes.getSnowflakeType(
            "GEOMETRY", null, "binary", Types.NUMERIC, false, false);
    assertEquals(Types.BINARY, geometryBinary.getColumnType());
    assertEquals("GEOMETRY", geometryBinary.getExtColTypeName());

    ColumnTypeInfo geographyNullUdt =
        SnowflakeColumnTypes.getSnowflakeType("GEOGRAPHY", null, null, Types.NUMERIC, false, false);
    assertEquals(Types.VARCHAR, geographyNullUdt.getColumnType());
    assertEquals("GEOGRAPHY", geographyNullUdt.getExtColTypeName());

    ColumnTypeInfo geometryUnknownUdt =
        SnowflakeColumnTypes.getSnowflakeType(
            "GEOMETRY", null, "unknown_udt", Types.NUMERIC, false, false);
    assertEquals(Types.VARCHAR, geometryUnknownUdt.getColumnType());
    assertEquals("GEOMETRY", geometryUnknownUdt.getExtColTypeName());

    ColumnTypeInfo unknownDefaultExt =
        SnowflakeColumnTypes.getSnowflakeType("uuid", null, null, Types.NUMERIC, false, false);
    assertEquals(Types.OTHER, unknownDefaultExt.getColumnType());
    assertEquals("UUID", unknownDefaultExt.getExtColTypeName());
    assertEquals(SnowflakeType.ANY, unknownDefaultExt.getSnowflakeType());

    assertFalse(SnowflakeColumnTypes.isVectorType("TEXT"));
  }

  @Test
  void shouldThrowInternalErrorWhenInternalTypeIsAny() {
    SFSQLException ex =
        assertThrows(
            SFSQLException.class,
            () ->
                SnowflakeColumnTypes.getSnowflakeType(
                    "ANY", null, null, Types.NUMERIC, false, false));
    assertEquals(ErrorCode.INTERNAL_ERROR, ex.getErrorCode());
    assertEquals(ErrorCode.INTERNAL_ERROR.getSqlState(), ex.toSQLException().getSQLState());
    assertEquals(ErrorCode.INTERNAL_ERROR.getMessageCode(), ex.toSQLException().getErrorCode());
    assertEquals("Unknown column type: ANY", ex.getMessage());
  }
}
