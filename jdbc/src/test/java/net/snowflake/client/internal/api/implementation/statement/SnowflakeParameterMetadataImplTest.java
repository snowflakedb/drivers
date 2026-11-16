package net.snowflake.client.internal.api.implementation.statement;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertThrows;

import java.sql.ParameterMetaData;
import java.sql.SQLException;
import java.sql.Types;
import java.util.Arrays;
import java.util.Collections;
import net.snowflake.client.internal.api.implementation.exception.SFSQLException;
import net.snowflake.client.internal.api.implementation.exception.SFSQLFeatureNotSupportedException;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.ColumnMetadata;
import org.junit.jupiter.api.Test;

class SnowflakeParameterMetadataImplTest {

  private static final ColumnMetadata FIXED_NOT_NULL =
      ColumnMetadata.newBuilder()
          .setType("fixed")
          .setNullable(false)
          .setPrecision(10)
          .setScale(2)
          .build();

  private static final ColumnMetadata TEXT_NULLABLE =
      ColumnMetadata.newBuilder().setType("text").setNullable(true).build();

  @Test
  void shouldExposeNullabilityPrecisionScaleAndTypePerParameter() throws SQLException {
    ParameterMetaData meta =
        SnowflakeParameterMetadataImpl.from(Arrays.asList(FIXED_NOT_NULL, TEXT_NULLABLE));

    assertEquals(2, meta.getParameterCount());
    assertEquals(ParameterMetaData.parameterNoNulls, meta.isNullable(1));
    assertEquals(ParameterMetaData.parameterNullable, meta.isNullable(2));
    assertEquals(10, meta.getPrecision(1));
    assertEquals(2, meta.getScale(1));
    assertEquals(Types.OTHER, meta.getParameterType(1));
    assertEquals("fixed", meta.getParameterTypeName(1));
    assertEquals(Types.VARCHAR, meta.getParameterType(2));
    assertEquals("text", meta.getParameterTypeName(2));
  }

  @Test
  void shouldRejectUnsupportedParameterMetadataCalls() throws SQLException {
    ParameterMetaData meta = SnowflakeParameterMetadataImpl.from(Arrays.asList(FIXED_NOT_NULL));

    assertEquals(
        "isSigned not supported",
        assertThrows(SFSQLFeatureNotSupportedException.class, () -> meta.isSigned(1)).getMessage());
    assertEquals(
        "getParameterClassName not supported",
        assertThrows(SFSQLFeatureNotSupportedException.class, () -> meta.getParameterClassName(1))
            .getMessage());
    assertEquals(
        "getParameterMode not supported",
        assertThrows(SFSQLFeatureNotSupportedException.class, () -> meta.getParameterMode(1))
            .getMessage());
  }

  @Test
  void shouldRejectParameterIndexOutsideBoundParameters() throws SQLException {
    ParameterMetaData meta =
        SnowflakeParameterMetadataImpl.from(Arrays.asList(FIXED_NOT_NULL, TEXT_NULLABLE));
    ParameterMetaData empty = SnowflakeParameterMetadataImpl.from(Collections.emptyList());

    assertEquals(
        "Invalid parameter index: 0 (parameter count: 2)",
        assertThrows(SFSQLException.class, () -> meta.getPrecision(0)).getMessage());
    assertEquals(
        "Invalid parameter index: 3 (parameter count: 2)",
        assertThrows(SFSQLException.class, () -> meta.getScale(3)).getMessage());
    assertEquals(0, empty.getParameterCount());
    assertEquals(
        "Invalid parameter index: 1 (parameter count: 0)",
        assertThrows(SFSQLException.class, () -> empty.isNullable(1)).getMessage());
  }
}
