package net.snowflake.client.internal.core.arrow.converters;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertInstanceOf;
import static org.junit.jupiter.api.Assertions.assertNull;
import static org.junit.jupiter.api.Assertions.assertThrows;

import java.nio.charset.StandardCharsets;
import java.util.Collections;
import java.util.HashMap;
import java.util.Map;
import net.snowflake.client.api.resultset.SnowflakeType;
import net.snowflake.client.internal.api.implementation.exception.SFSQLException;
import net.snowflake.client.internal.api.implementation.parameters.FrozenParametersRegistry;
import net.snowflake.client.internal.api.implementation.parameters.Parameter;
import net.snowflake.client.internal.unicore.ConfigSettingFactory;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.ConfigSetting;
import org.apache.arrow.memory.BufferAllocator;
import org.apache.arrow.memory.RootAllocator;
import org.apache.arrow.vector.BigIntVector;
import org.apache.arrow.vector.VarCharVector;
import org.apache.arrow.vector.types.Types;
import org.apache.arrow.vector.types.pojo.ArrowType;
import org.apache.arrow.vector.types.pojo.Field;
import org.apache.arrow.vector.types.pojo.FieldType;
import org.junit.jupiter.api.AfterEach;
import org.junit.jupiter.api.Test;
import org.junit.jupiter.params.ParameterizedTest;
import org.junit.jupiter.params.provider.ValueSource;

class ArrowVectorConverterUtilTest extends BaseConverterTest {

  private final BufferAllocator allocator = new RootAllocator(Long.MAX_VALUE);

  @AfterEach
  void closeAllocator() {
    allocator.close();
  }

  @Test
  void shouldResolveLogicalTypeMetadataCaseInsensitively() {
    Map<String, String> metadata = Collections.singletonMap("logicalType", "timestamp_ntz");
    Field field =
        new Field("col", new FieldType(true, ArrowType.Utf8.INSTANCE, null, metadata), null);

    assertEquals(
        SnowflakeType.TIMESTAMP_NTZ,
        ArrowVectorConverterUtil.getSnowflakeTypeFromFieldMetadata(field));
  }

  @Test
  void shouldReturnNullWhenLogicalTypeMetadataIsAbsent() {
    Field field =
        new Field(
            "col",
            new FieldType(true, ArrowType.Utf8.INSTANCE, null, Collections.emptyMap()),
            null);

    assertNull(ArrowVectorConverterUtil.getSnowflakeTypeFromFieldMetadata(field));
  }

  @Test
  void shouldDispatchBareTimestampArrowColumnToNtzConverterWhenSessionMappingIsLowercaseWireName() {
    DataConversionContext context = contextWithTimestampMapping("timestamp_ntz");

    try (BigIntVector vector = createBareTimestampVector()) {
      ArrowVectorConverter converter = ArrowVectorConverterUtil.initConverter(vector, context, 0);
      assertInstanceOf(BigIntToTimestampNTZConverter.class, converter);
    }
  }

  @Test
  void shouldRejectBareTimestampArrowColumnWhenSessionMappingIsNeitherNtzNorLtz() {
    DataConversionContext context = contextWithTimestampMapping("not_a_timestamp");

    try (BigIntVector vector = createBareTimestampVector()) {
      SFSQLException ex =
          assertThrows(
              SFSQLException.class,
              () -> ArrowVectorConverterUtil.initConverter(vector, context, 0));
      assertEquals(
          "Unsupported TIMESTAMP mapping for bare TIMESTAMP: not_a_timestamp", ex.getMessage());
    }
  }

  @ParameterizedTest
  @ValueSource(strings = {"text", "variant"})
  void shouldParseLowercaseArrowLogicalTypeNames(String logicalType) {
    Map<String, String> meta = new HashMap<>();
    meta.put("logicalType", logicalType);
    FieldType fieldType = new FieldType(true, Types.MinorType.VARCHAR.getType(), null, meta);
    try (VarCharVector vector = new VarCharVector("col", fieldType, allocator)) {
      vector.setSafe(0, "x".getBytes(StandardCharsets.UTF_8));
      vector.setValueCount(1);
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

  private BigIntVector createBareTimestampVector() {
    Map<String, String> metadata = new HashMap<>();
    metadata.put("logicalType", "TIMESTAMP");
    metadata.put("scale", "0");
    FieldType fieldType = new FieldType(true, Types.MinorType.BIGINT.getType(), null, metadata);
    BigIntVector vector = new BigIntVector("col_ts", fieldType, allocator);
    vector.setSafe(0, 0L);
    vector.setValueCount(1);
    return vector;
  }
}
