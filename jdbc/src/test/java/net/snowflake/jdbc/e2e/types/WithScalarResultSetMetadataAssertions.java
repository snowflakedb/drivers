package net.snowflake.jdbc.e2e.types;

import static org.junit.jupiter.api.Assertions.assertAll;
import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertFalse;
import static org.junit.jupiter.api.Assertions.assertTrue;

import java.sql.ResultSetMetaData;
import java.util.List;
import java.util.stream.Collectors;
import lombok.AllArgsConstructor;
import lombok.Value;
import lombok.With;
import net.snowflake.client.api.resultset.SnowflakeResultSetMetaData;

/** Full JDBC and Snowflake-specific ResultSet metadata checks for scalar type e2e tests. */
interface WithScalarResultSetMetadataAssertions {

  @Value
  @AllArgsConstructor
  class ColumnExpectation {
    @With String columnName;
    int jdbcType;
    String typeName;
    String className;
    int precision;
    int scale;
    int displaySize;
    boolean signed;
    boolean caseSensitive;
    int nullable;
  }

  default void assertScalarResultSetMetadata(
      ResultSetMetaData meta, SnowflakeResultSetMetaData sfMeta, List<ColumnExpectation> columns)
      throws Exception {
    List<String> expectedColumnNames =
        columns.stream().map(ColumnExpectation::getColumnName).collect(Collectors.toList());

    assertAll(
        "result set metadata",
        () -> assertEquals(columns.size(), meta.getColumnCount(), "column count"),
        () -> assertEquals(expectedColumnNames, sfMeta.getColumnNames(), "column names"),
        () -> assertFalse(sfMeta.getQueryID().isEmpty(), "query id"));

    for (int column = 1; column <= columns.size(); column++) {
      assertScalarColumnMetadata(
          meta, sfMeta, column, columns.get(column - 1), expectedColumnNames);
    }
  }

  default void assertScalarColumnMetadata(
      ResultSetMetaData meta,
      SnowflakeResultSetMetaData sfMeta,
      int column,
      ColumnExpectation expected,
      List<String> expectedColumnNames)
      throws Exception {
    String columnName = expected.getColumnName();
    assertAll(
        "column " + column + " metadata",
        () -> assertEquals(columnName, meta.getColumnName(column), "column name"),
        () -> assertEquals(columnName, meta.getColumnLabel(column), "column label"),
        () -> assertEquals(expected.getJdbcType(), meta.getColumnType(column), "column type"),
        () ->
            assertEquals(
                expected.getTypeName(), meta.getColumnTypeName(column), "column type name"),
        () ->
            assertEquals(
                expected.getClassName(), meta.getColumnClassName(column), "column class name"),
        () -> assertEquals(expected.getPrecision(), meta.getPrecision(column), "precision"),
        () -> assertEquals(expected.getScale(), meta.getScale(column), "scale"),
        () ->
            assertEquals(
                expected.getDisplaySize(), meta.getColumnDisplaySize(column), "display size"),
        () -> assertEquals(expected.isSigned(), meta.isSigned(column), "signed"),
        () ->
            assertEquals(
                expected.isCaseSensitive(), meta.isCaseSensitive(column), "case sensitivity"),
        () -> assertTrue(meta.isSearchable(column), "searchable"),
        () -> assertFalse(meta.isCurrency(column), "currency"),
        () -> assertEquals(expected.getNullable(), meta.isNullable(column), "nullable"),
        () -> assertFalse(meta.isAutoIncrement(column), "auto increment"),
        () -> assertTrue(meta.isReadOnly(column), "read only"),
        () -> assertFalse(meta.isWritable(column), "writable"),
        () -> assertFalse(meta.isDefinitelyWritable(column), "definitely writable"),
        () -> assertEquals("", meta.getCatalogName(column), "catalog name"),
        () -> assertEquals("", meta.getSchemaName(column), "schema name"),
        () -> assertEquals("", meta.getTableName(column), "table name"),
        () ->
            assertEquals(
                expected.getJdbcType(),
                sfMeta.getInternalColumnType(column),
                "internal column type"),
        () ->
            assertEquals(
                expectedColumnNames.indexOf(columnName),
                sfMeta.getColumnIndex(columnName),
                "column index"),
        () -> assertEquals(0, sfMeta.getVectorDimension(column), "vector dimension"),
        () -> assertEquals(0, sfMeta.getVectorDimension(columnName), "vector dimension by name"));
  }
}
