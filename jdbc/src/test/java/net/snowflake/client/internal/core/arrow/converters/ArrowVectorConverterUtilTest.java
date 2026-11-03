package net.snowflake.client.internal.core.arrow.converters;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertInstanceOf;
import static org.junit.jupiter.api.Assertions.assertNull;
import static org.junit.jupiter.api.Assertions.assertThrows;

import java.sql.SQLException;
import java.util.Collections;
import java.util.HashMap;
import java.util.Map;
import net.snowflake.client.api.exception.ErrorCode;
import net.snowflake.client.api.resultset.SnowflakeType;
import net.snowflake.client.internal.api.implementation.exception.SFSQLException;
import net.snowflake.client.internal.api.implementation.parameters.FrozenParametersRegistry;
import net.snowflake.client.internal.api.implementation.parameters.Parameter;
import net.snowflake.client.internal.core.arrow.vector.Column;
import net.snowflake.client.internal.core.arrow.vector.Column.PhysicalKind;
import net.snowflake.client.internal.core.arrow.vector.ColumnBuilder;
import net.snowflake.client.internal.unicore.ConfigSettingFactory;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.ConfigSetting;
import org.junit.jupiter.api.Test;
import org.junit.jupiter.params.ParameterizedTest;
import org.junit.jupiter.params.provider.CsvSource;
import org.junit.jupiter.params.provider.ValueSource;

class ArrowVectorConverterUtilTest extends BaseConverterTest {

  @ParameterizedTest
  @CsvSource({
    "timestamp_ntz, TIMESTAMP_NTZ",
    "TIMESTAMP_NTZ, TIMESTAMP_NTZ",
    "timestamp_ltz, TIMESTAMP_LTZ",
    "TIMESTAMP_LTZ, TIMESTAMP_LTZ",
    "timestamp_tz, TIMESTAMP_TZ",
    "TIMESTAMP_TZ, TIMESTAMP_TZ"
  })
  void shouldResolveLogicalTypeMetadataCaseInsensitively(String logicalType, String expected) {
    Map<String, String> metadata = Collections.singletonMap("logicalType", logicalType);
    try (Column column = ColumnBuilder.of("col", PhysicalKind.UTF8, metadata)) {
      assertEquals(
          SnowflakeType.valueOf(expected),
          ArrowVectorConverterUtil.getSnowflakeTypeFromFieldMetadata(column.getField()));
    }
  }

  @Test
  void shouldReturnNullWhenLogicalTypeMetadataIsAbsent() {
    try (Column column = ColumnBuilder.of("col", PhysicalKind.UTF8, Collections.emptyMap())) {
      assertNull(ArrowVectorConverterUtil.getSnowflakeTypeFromFieldMetadata(column.getField()));
    }
  }

  @Test
  void shouldDispatchBareTimestampArrowColumnToNtzConverterWhenSessionMappingIsLowercaseWireName() {
    DataConversionContext context = contextWithTimestampMapping("timestamp_ntz");

    try (Column vector = bareTimestampColumn()) {
      ArrowVectorConverter converter = ArrowVectorConverterUtil.initConverter(vector, context, 0);
      assertInstanceOf(BigIntToTimestampNTZConverter.class, converter);
    }
  }

  @Test
  void shouldDispatchBareTimestampArrowColumnToLtzConverterWhenSessionMappingIsLowercaseWireName() {
    DataConversionContext context = contextWithTimestampMapping("timestamp_ltz");

    try (Column vector = bareTimestampColumn()) {
      ArrowVectorConverter converter = ArrowVectorConverterUtil.initConverter(vector, context, 0);
      assertInstanceOf(BigIntToTimestampLTZConverter.class, converter);
    }
  }

  @Test
  void shouldDispatchTwoFieldBareTimestampToLtzConverterWhenSessionMappingIsLowercaseWireName() {
    DataConversionContext context = contextWithTimestampMapping("timestamp_ltz");

    try (Column vector = bareTimestampStruct()) {
      ArrowVectorConverter converter = ArrowVectorConverterUtil.initConverter(vector, context, 0);
      assertInstanceOf(TwoFieldStructToTimestampLTZConverter.class, converter);
    }
  }

  @Test
  void shouldDispatchTwoFieldBareTimestampToNtzConverterWhenSessionMappingIsLowercaseWireName() {
    DataConversionContext context = contextWithTimestampMapping("timestamp_ntz");

    try (Column vector = bareTimestampStruct()) {
      ArrowVectorConverter converter = ArrowVectorConverterUtil.initConverter(vector, context, 0);
      assertInstanceOf(TwoFieldStructToTimestampNTZConverter.class, converter);
    }
  }

  @Test
  void shouldRejectBareTimestampArrowColumnWhenSessionMappingIsNeitherNtzNorLtz() {
    DataConversionContext context = contextWithTimestampMapping("not_a_timestamp");

    try (Column vector = bareTimestampColumn()) {
      SFSQLException ex =
          assertThrows(
              SFSQLException.class,
              () -> ArrowVectorConverterUtil.initConverter(vector, context, 0));
      assertEquals(
          "Unsupported TIMESTAMP mapping for bare TIMESTAMP: not_a_timestamp", ex.getMessage());
    }
  }

  @Test
  void shouldRejectFloat32Column() {
    try (Column vector = ColumnBuilder.of("col", PhysicalKind.FLOAT32, Collections.emptyMap())) {
      SFSQLException failure =
          assertThrows(
              SFSQLException.class, () -> ArrowVectorConverterUtil.initConverter(vector, this, 0));
      SQLException sql = failure.toSQLException();
      assertEquals(ErrorCode.INTERNAL_ERROR.getMessageCode(), sql.getErrorCode());
      assertEquals(ErrorCode.INTERNAL_ERROR.getSqlState(), sql.getSQLState());
      assertEquals(
          "JDBC driver internal error: Unexpected Arrow Field for FLOAT32.", failure.getMessage());
    }
  }

  @ParameterizedTest
  @ValueSource(strings = {"text", "variant"})
  void shouldParseLowercaseArrowLogicalTypeNames(String logicalType) {
    Map<String, String> meta = new HashMap<>();
    meta.put("logicalType", logicalType);
    try (Column vector = ColumnBuilder.of("col", PhysicalKind.UTF8, meta)) {
      ArrowVectorConverter converter = ArrowVectorConverterUtil.initConverter(vector, this, 0);
      assertInstanceOf(VarCharConverter.class, converter);
    }
  }

  @Test
  void shouldInitConverterForFixedAndScaledFixedAndStructuredTypes() {
    Map<String, String> fixedZero = new HashMap<>();
    fixedZero.put("logicalType", "FIXED");
    fixedZero.put("precision", "10");
    fixedZero.put("scale", "0");

    try (Column int8 = ColumnBuilder.of("c", PhysicalKind.INT8, fixedZero);
        Column int16 = ColumnBuilder.of("c", PhysicalKind.INT16, fixedZero);
        Column int32 = ColumnBuilder.of("c", PhysicalKind.INT32, fixedZero);
        Column int64 = ColumnBuilder.of("c", PhysicalKind.INT64, fixedZero)) {
      assertInstanceOf(
          TinyIntToFixedConverter.class, ArrowVectorConverterUtil.initConverter(int8, this, 0));
      assertInstanceOf(
          SmallIntToFixedConverter.class, ArrowVectorConverterUtil.initConverter(int16, this, 0));
      assertInstanceOf(
          IntToFixedConverter.class, ArrowVectorConverterUtil.initConverter(int32, this, 0));
      assertInstanceOf(
          BigIntToFixedConverter.class, ArrowVectorConverterUtil.initConverter(int64, this, 0));
    }

    Map<String, String> fixedScaled = new HashMap<>();
    fixedScaled.put("logicalType", "FIXED");
    fixedScaled.put("precision", "10");
    fixedScaled.put("scale", "2");

    try (Column int8 = ColumnBuilder.of("c", PhysicalKind.INT8, fixedScaled);
        Column int16 = ColumnBuilder.of("c", PhysicalKind.INT16, fixedScaled);
        Column int32 = ColumnBuilder.of("c", PhysicalKind.INT32, fixedScaled);
        Column int64 = ColumnBuilder.of("c", PhysicalKind.INT64, fixedScaled);
        Column dec128 = ColumnBuilder.of("c", PhysicalKind.DECIMAL128, fixedScaled)) {
      assertInstanceOf(
          TinyIntToScaledFixedConverter.class,
          ArrowVectorConverterUtil.initConverter(int8, this, 0));
      assertInstanceOf(
          SmallIntToScaledFixedConverter.class,
          ArrowVectorConverterUtil.initConverter(int16, this, 0));
      assertInstanceOf(
          IntToScaledFixedConverter.class, ArrowVectorConverterUtil.initConverter(int32, this, 0));
      assertInstanceOf(
          BigIntToScaledFixedConverter.class,
          ArrowVectorConverterUtil.initConverter(int64, this, 0));
      assertInstanceOf(
          DecimalToScaledFixedConverter.class,
          ArrowVectorConverterUtil.initConverter(dec128, this, 0));
    }

    for (String structuredType : new String[] {"MAP", "ARRAY", "OBJECT", "CHAR", "ANY"}) {
      Map<String, String> meta = Collections.singletonMap("logicalType", structuredType);
      try (Column col = ColumnBuilder.of("c", PhysicalKind.UTF8, meta)) {
        assertInstanceOf(
            VarCharConverter.class, ArrowVectorConverterUtil.initConverter(col, this, 0));
      }
    }
  }

  @Test
  void shouldRejectUnsupportedPhysicalTypesAndStructLayoutsInInitConverter() {
    Map<String, String> timeMeta = Collections.singletonMap("logicalType", "TIME");
    try (Column badTime = ColumnBuilder.of("c", PhysicalKind.UTF8, timeMeta)) {
      SFSQLException ex =
          assertThrows(
              SFSQLException.class, () -> ArrowVectorConverterUtil.initConverter(badTime, this, 0));
      assertEquals("Unsupported Arrow physical type for TIME: UTF8", ex.getMessage());
    }

    Map<String, String> vectorMeta = Collections.singletonMap("logicalType", "VECTOR");
    try (Column badVector = ColumnBuilder.of("c", PhysicalKind.INT32, vectorMeta)) {
      SFSQLException ex =
          assertThrows(
              SFSQLException.class,
              () -> ArrowVectorConverterUtil.initConverter(badVector, this, 0));
      assertEquals("Unsupported Arrow physical type for VECTOR: INT32", ex.getMessage());
    }

    Map<String, String> ntzMeta = Collections.singletonMap("logicalType", "TIMESTAMP_NTZ");
    try (Column badNtz =
        ColumnBuilder.struct(
            "c", ntzMeta, ColumnBuilder.of("epoch", PhysicalKind.INT64, Collections.emptyMap()))) {
      SFSQLException ex =
          assertThrows(
              SFSQLException.class, () -> ArrowVectorConverterUtil.initConverter(badNtz, this, 0));
      assertEquals(
          "Unsupported Arrow physical layout for TIMESTAMP_NTZ: 1 struct children",
          ex.getMessage());
    }

    Map<String, String> ltzMeta = Collections.singletonMap("logicalType", "TIMESTAMP_LTZ");
    try (Column badLtz =
        ColumnBuilder.struct(
            "c", ltzMeta, ColumnBuilder.of("epoch", PhysicalKind.INT64, Collections.emptyMap()))) {
      SFSQLException ex =
          assertThrows(
              SFSQLException.class, () -> ArrowVectorConverterUtil.initConverter(badLtz, this, 0));
      assertEquals(
          "Unsupported Arrow physical layout for TIMESTAMP_LTZ: 1 struct children",
          ex.getMessage());
    }

    Map<String, String> tzMeta = Collections.singletonMap("logicalType", "TIMESTAMP_TZ");
    try (Column badTz = ColumnBuilder.of("c", PhysicalKind.INT64, tzMeta)) {
      SFSQLException ex =
          assertThrows(
              SFSQLException.class, () -> ArrowVectorConverterUtil.initConverter(badTz, this, 0));
      assertEquals(
          "Unsupported Arrow physical layout for TIMESTAMP_TZ: 0 struct children", ex.getMessage());
    }

    Map<String, String> geoMeta = Collections.singletonMap("logicalType", "GEOGRAPHY");
    try (Column badLogical = ColumnBuilder.of("c", PhysicalKind.UTF8, geoMeta)) {
      SFSQLException ex =
          assertThrows(
              SFSQLException.class,
              () -> ArrowVectorConverterUtil.initConverter(badLogical, this, 0));
      assertEquals("Unsupported Arrow logical type: GEOGRAPHY", ex.getMessage());
    }

    try (Column unmappedField = ColumnBuilder.of("c", PhysicalKind.UTF8, Collections.emptyMap())) {
      SFSQLException ex =
          assertThrows(
              SFSQLException.class,
              () -> ArrowVectorConverterUtil.initConverter(unmappedField, this, 0));
      assertEquals("Unsupported Arrow field type: UTF8", ex.getMessage());
    }
  }

  private static DataConversionContext contextWithTimestampMapping(String mapping) {
    Map<String, ConfigSetting> parameters = new HashMap<>();
    parameters.put(
        Parameter.CLIENT_TIMESTAMP_TYPE_MAPPING.getKey(), ConfigSettingFactory.from(mapping));
    return SessionDataConversionContext.from(new FrozenParametersRegistry(parameters));
  }

  private static Column bareTimestampColumn() {
    Map<String, String> metadata = new HashMap<>();
    metadata.put("logicalType", "TIMESTAMP");
    metadata.put("scale", "0");
    return ColumnBuilder.of("col_ts", PhysicalKind.INT64, metadata);
  }

  private static Column bareTimestampStruct() {
    Map<String, String> metadata = new HashMap<>();
    metadata.put("logicalType", "TIMESTAMP");
    metadata.put("scale", "0");
    Column epochs =
        ColumnBuilder.of(AbstractArrowVectorConverter.FIELD_NAME_EPOCH, PhysicalKind.INT64, null);
    Column fractions =
        ColumnBuilder.of(
            AbstractArrowVectorConverter.FIELD_NAME_FRACTION, PhysicalKind.INT32, null);
    return ColumnBuilder.struct("col", metadata, epochs, fractions);
  }
}
