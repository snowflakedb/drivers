package net.snowflake.client.api.resultset.metadata;

import static org.junit.jupiter.api.Assertions.assertAll;
import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertFalse;
import static org.junit.jupiter.api.Assertions.assertTrue;

import java.sql.ResultSetMetaData;

interface WithMetaDataAssertions {

  default void assertColumnMetadata(
      ResultSetMetaData meta,
      String label,
      int expectedType,
      String expectedTypeName,
      Class<?> expectedClass,
      int expectedPrecision,
      int expectedScale,
      int expectedDisplaySize,
      boolean expectedSigned,
      boolean expectedCaseSensitive) {
    assertAll(
        label,
        () -> assertEquals(1, meta.getColumnCount(), "column count"),
        () -> assertEquals("COL", meta.getColumnName(1), "column name"),
        () -> assertEquals("COL", meta.getColumnLabel(1), "column label"),
        () -> assertEquals(expectedType, meta.getColumnType(1), "column type"),
        () -> assertEquals(expectedTypeName, meta.getColumnTypeName(1), "column type name"),
        () ->
            assertEquals(expectedClass.getName(), meta.getColumnClassName(1), "column class name"),
        () -> assertEquals(expectedPrecision, meta.getPrecision(1), "precision"),
        () -> assertEquals(expectedScale, meta.getScale(1), "scale"),
        () -> assertEquals(expectedDisplaySize, meta.getColumnDisplaySize(1), "display size"),
        () -> assertEquals(expectedSigned, meta.isSigned(1), "signed"),
        () -> assertEquals(expectedCaseSensitive, meta.isCaseSensitive(1), "case sensitivity"),
        () -> assertTrue(meta.isSearchable(1), "searchable"),
        () -> assertFalse(meta.isCurrency(1), "currency"),
        () -> assertFalse(meta.isAutoIncrement(1), "auto increment"),
        () -> assertTrue(meta.isReadOnly(1), "read only"),
        () -> assertFalse(meta.isWritable(1), "writable"),
        () -> assertFalse(meta.isDefinitelyWritable(1), "definitely writable"));
  }
}
