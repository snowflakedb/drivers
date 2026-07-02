package net.snowflake.client.internal.api.implementation.metadata.objects;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertNull;
import static org.mockito.Mockito.mock;
import static org.mockito.Mockito.when;

import java.sql.Types;
import java.util.stream.Stream;
import net.snowflake.client.api.resultset.SnowflakeType;
import net.snowflake.client.internal.api.implementation.resultset.metadata.SnowflakeColumnMetadata;
import org.junit.jupiter.api.Test;
import org.junit.jupiter.params.ParameterizedTest;
import org.junit.jupiter.params.provider.Arguments;
import org.junit.jupiter.params.provider.MethodSource;
import org.junit.jupiter.params.provider.ValueSource;

class MetaDataObjectsColumnSizeTest {

  static Stream<Arguments> lengthBasedTypes() {
    return Stream.of(
        Arguments.of(Types.VARCHAR, 100),
        Arguments.of(Types.CHAR, 50),
        Arguments.of(Types.BINARY, 200),
        Arguments.of(Types.VARBINARY, 250));
  }

  @ParameterizedTest
  @MethodSource("lengthBasedTypes")
  void shouldReturnLengthForLengthBasedColumnTypes(int type, int length) {
    SnowflakeColumnMetadata metadata = mock(SnowflakeColumnMetadata.class);
    when(metadata.getType()).thenReturn(type);
    when(metadata.getLength()).thenReturn(length);

    assertEquals(length, MetaDataObjects.getColumnSize(metadata));
  }

  static Stream<Integer> precisionBasedTypes() {
    return Stream.of(
        Types.DECIMAL,
        Types.NUMERIC,
        Types.BIGINT,
        Types.INTEGER,
        Types.SMALLINT,
        Types.TINYINT,
        Types.FLOAT,
        Types.DOUBLE,
        Types.REAL,
        SnowflakeType.EXTRA_TYPES_DECFLOAT,
        Types.DATE,
        Types.TIME,
        Types.TIMESTAMP,
        Types.TIMESTAMP_WITH_TIMEZONE,
        SnowflakeType.EXTRA_TYPES_TIMESTAMP_LTZ,
        SnowflakeType.EXTRA_TYPES_TIMESTAMP_TZ,
        SnowflakeType.EXTRA_TYPES_TIMESTAMP_NTZ);
  }

  @ParameterizedTest
  @MethodSource("precisionBasedTypes")
  void shouldReturnPrecisionForPrecisionBasedColumnTypes(int type) {
    int precision = 38;
    SnowflakeColumnMetadata metadata = mock(SnowflakeColumnMetadata.class);
    when(metadata.getType()).thenReturn(type);
    when(metadata.getPrecision()).thenReturn(precision);

    assertEquals(precision, MetaDataObjects.getColumnSize(metadata));
  }

  @Test
  void shouldReturnDimensionForVectorColumnType() {
    SnowflakeColumnMetadata metadata = mock(SnowflakeColumnMetadata.class);
    when(metadata.getType()).thenReturn(SnowflakeType.EXTRA_TYPES_VECTOR);
    when(metadata.getDimension()).thenReturn(128);

    assertEquals(128, MetaDataObjects.getColumnSize(metadata));
  }

  @ParameterizedTest
  @ValueSource(ints = {Types.BOOLEAN, Types.ARRAY, Types.STRUCT})
  void shouldReturnNullForUnsupportedColumnTypes(int type) {
    SnowflakeColumnMetadata metadata = mock(SnowflakeColumnMetadata.class);
    when(metadata.getType()).thenReturn(type);
    assertNull(MetaDataObjects.getColumnSize(metadata));
  }
}
