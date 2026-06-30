package net.snowflake.client.internal.api.implementation.metadata.objects;

import java.sql.Types;
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
      Arrays.asList(Types.VARCHAR, Types.VARCHAR));

  private final List<String> columnNames;
  private final List<String> columnTypeNames;
  private final List<Integer> columnTypes;

  SnowflakeResultSetMetaDataImpl metaData(String queryId) {
    return SnowflakeResultSetMetaDataImpl.fromColumnSpec(
        queryId, columnNames, columnTypeNames, columnTypes);
  }
}
