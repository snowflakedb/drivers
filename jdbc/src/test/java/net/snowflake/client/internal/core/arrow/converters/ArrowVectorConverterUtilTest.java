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
