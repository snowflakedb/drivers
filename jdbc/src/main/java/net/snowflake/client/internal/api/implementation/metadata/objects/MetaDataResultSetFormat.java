package net.snowflake.client.internal.api.implementation.metadata.objects;

import java.sql.Types;
import java.util.ArrayList;
import java.util.Arrays;
import java.util.Collections;
import java.util.List;
import lombok.Getter;
import lombok.RequiredArgsConstructor;
import net.snowflake.client.internal.api.implementation.resultset.metadata.SnowflakeResultSetMetaDataImpl;

/**
 * Column shape for {@link java.sql.DatabaseMetaData} result sets. Ported from the snowflake-jdbc.
 */
@Getter
@RequiredArgsConstructor
enum MetaDataResultSetFormat {
  GET_CATALOGS(
      Collections.singletonList("TABLE_CAT"),
      Collections.singletonList("TEXT"),
      Collections.singletonList(Types.VARCHAR)),

  GET_SCHEMAS(
      Arrays.asList("TABLE_SCHEM", "TABLE_CATALOG"),
      Arrays.asList("TEXT", "TEXT"),
      Arrays.asList(Types.VARCHAR, Types.VARCHAR)),

  GET_TABLES(
      Arrays.asList(
          "TABLE_CAT",
          "TABLE_SCHEM",
          "TABLE_NAME",
          "TABLE_TYPE",
          "REMARKS",
          "TYPE_CAT",
          "TYPE_SCHEM",
          "TYPE_NAME",
          "SELF_REFERENCING_COL_NAME",
          "REF_GENERATION"),
      Arrays.asList("TEXT", "TEXT", "TEXT", "TEXT", "TEXT", "TEXT", "TEXT", "TEXT", "TEXT", "TEXT"),
      Arrays.asList(
          Types.VARCHAR,
          Types.VARCHAR,
          Types.VARCHAR,
          Types.VARCHAR,
          Types.VARCHAR,
          Types.VARCHAR,
          Types.VARCHAR,
          Types.VARCHAR,
          Types.VARCHAR,
          Types.VARCHAR)),

  GET_COLUMNS(
      Arrays.asList(
          "TABLE_CAT",
          "TABLE_SCHEM",
          "TABLE_NAME",
          "COLUMN_NAME",
          "DATA_TYPE",
          "TYPE_NAME",
          "COLUMN_SIZE",
          "BUFFER_LENGTH",
          "DECIMAL_DIGITS",
          "NUM_PREC_RADIX",
          "NULLABLE",
          "REMARKS",
          "COLUMN_DEF",
          "SQL_DATA_TYPE",
          "SQL_DATETIME_SUB",
          "CHAR_OCTET_LENGTH",
          "ORDINAL_POSITION",
          "IS_NULLABLE",
          "SCOPE_CATALOG",
          "SCOPE_SCHEMA",
          "SCOPE_TABLE",
          "SOURCE_DATA_TYPE",
          "IS_AUTOINCREMENT",
          "IS_GENERATEDCOLUMN"),
      Arrays.asList(
          "TEXT", "TEXT", "TEXT", "TEXT", "INTEGER", "TEXT", "INTEGER", "INTEGER", "INTEGER",
          "INTEGER", "INTEGER", "TEXT", "TEXT", "INTEGER", "INTEGER", "INTEGER", "INTEGER", "TEXT",
          "TEXT", "TEXT", "TEXT", "SHORT", "TEXT", "TEXT"),
      Arrays.asList(
          Types.VARCHAR,
          Types.VARCHAR,
          Types.VARCHAR,
          Types.VARCHAR,
          Types.INTEGER,
          Types.VARCHAR,
          Types.INTEGER,
          Types.INTEGER,
          Types.INTEGER,
          Types.INTEGER,
          Types.INTEGER,
          Types.VARCHAR,
          Types.VARCHAR,
          Types.INTEGER,
          Types.INTEGER,
          Types.INTEGER,
          Types.INTEGER,
          Types.VARCHAR,
          Types.VARCHAR,
          Types.VARCHAR,
          Types.VARCHAR,
          Types.SMALLINT,
          Types.VARCHAR,
          Types.VARCHAR)),

  GET_COLUMNS_EXTENDED_SET(
      GET_COLUMNS,
      Collections.singletonList("BASE_TYPE"),
      Collections.singletonList("TEXT"),
      Collections.singletonList(Types.VARCHAR));

  private final List<String> columnNames;
  private final List<String> columnTypeNames;
  private final List<Integer> columnTypes;

  MetaDataResultSetFormat(
      MetaDataResultSetFormat base,
      List<String> additionalColumnNames,
      List<String> additionalColumnTypeNames,
      List<Integer> additionalColumnTypes) {
    columnNames = new ArrayList<>(base.getColumnNames());
    columnTypeNames = new ArrayList<>(base.getColumnTypeNames());
    columnTypes = new ArrayList<>(base.getColumnTypes());
    columnNames.addAll(additionalColumnNames);
    columnTypeNames.addAll(additionalColumnTypeNames);
    columnTypes.addAll(additionalColumnTypes);
  }

  SnowflakeResultSetMetaDataImpl metaData(String queryId) {
    return SnowflakeResultSetMetaDataImpl.fromColumnSpec(
        queryId, columnNames, columnTypeNames, columnTypes);
  }
}
