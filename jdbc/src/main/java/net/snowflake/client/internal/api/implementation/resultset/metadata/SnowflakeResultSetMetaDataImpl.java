package net.snowflake.client.internal.api.implementation.resultset.metadata;

import java.sql.ResultSetMetaData;
import java.sql.SQLException;
import java.sql.Types;
import java.util.ArrayList;
import java.util.List;
import lombok.AccessLevel;
import lombok.RequiredArgsConstructor;
import net.snowflake.client.api.exception.SFException;
import net.snowflake.client.api.exception.SnowflakeSQLException;
import net.snowflake.client.api.resultset.FieldMetadata;
import net.snowflake.client.api.resultset.SnowflakeResultSetMetaData;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.ColumnMetadata;
import net.snowflake.client.internal.util.DelegatingWrapper;
import net.snowflake.client.internal.util.SnowflakeUtil;

@RequiredArgsConstructor(access = AccessLevel.PRIVATE)
public class SnowflakeResultSetMetaDataImpl
    implements ResultSetMetaData, SnowflakeResultSetMetaData, DelegatingWrapper {

  public enum QueryType {
    ASYNC,
    SYNC
  }

  private final SFResultSetMetaData resultSetMetaData;
  private final String queryId;
  private final QueryType queryType;

  /**
   * Builds result-set metadata from the protobuf result-column descriptions returned by core.
   *
   * @param queryId the query id this metadata belongs to
   * @param columns the protobuf result-column metadata
   * @return the assembled metadata
   */
  public static SnowflakeResultSetMetaDataImpl from(String queryId, List<ColumnMetadata> columns)
      throws SnowflakeSQLException {
    // TODO(SNOW-3695645): source jdbcTreatDecimalAsInt, isResultColumnCaseInsensitive,
    //  enableReturnTimestampWithTimeZone and the date/time formatters from connection parameters
    boolean jdbcTreatDecimalAsInt = false;
    List<SnowflakeColumnMetadata> columnMetadata = new ArrayList<>(columns.size());
    for (ColumnMetadata column : columns) {
      columnMetadata.add(new SnowflakeColumnMetadata(column, jdbcTreatDecimalAsInt));
    }
    SFResultSetMetaData sfResultSetMetaData =
        new SFResultSetMetaData(
            columnMetadata,
            queryId,
            /* isResultColumnCaseInsensitive= */ false,
            /* enableReturnTimestampWithTimeZone= */ true,
            /* timestampNTZFormatter= */ null,
            /* timestampLTZFormatter= */ null,
            /* timestampTZFormatter= */ null,
            /* dateFormatter= */ null,
            /* timeFormatter= */ null);
    return new SnowflakeResultSetMetaDataImpl(sfResultSetMetaData, queryId, QueryType.SYNC);
  }

  // TODO(SNOW-3695645): the minimal SFResultSetMetaData constructor leaves precisions, scales,
  //  displaySizes, and source table/schema/catalog null, so accessors fall back to defaults
  //  (precision 9, scale 9, displaySize 25, tableName "T", etc.). These match the old driver's
  //  behavior for fabricated metadata result sets but should be revisited.
  public static SnowflakeResultSetMetaDataImpl fromColumnSpec(
      String queryId,
      List<String> columnNames,
      List<String> columnTypeNames,
      List<Integer> columnTypes) {
    SFResultSetMetaData sfResultSetMetaData =
        new SFResultSetMetaData(columnNames.size(), columnNames, columnTypeNames, columnTypes);
    return new SnowflakeResultSetMetaDataImpl(sfResultSetMetaData, queryId, QueryType.SYNC);
  }

  /**
   * Returns an async view of {@code sync}: same column layout, {@link QueryType#ASYNC} (which
   * suppresses catalog/schema/table names), and {@code asyncQueryId} as the reported query ID. Does
   * not mutate {@code sync}.
   */
  public static SnowflakeResultSetMetaDataImpl toAsync(
      SnowflakeResultSetMetaDataImpl sync, String queryId) {
    return new SnowflakeResultSetMetaDataImpl(sync.resultSetMetaData, queryId, QueryType.ASYNC);
  }

  @Override
  public int getColumnCount() throws SQLException {
    return resultSetMetaData.getColumnCount();
  }

  @Override
  public boolean isAutoIncrement(int column) throws SQLException {
    return resultSetMetaData.getIsAutoIncrement(column);
  }

  @Override
  public boolean isCaseSensitive(int column) throws SQLException {
    int colType = getColumnType(column);

    switch (colType) {
        // Note: SF types GEOGRAPHY, GEOMETRY are also represented as VARCHAR.
      case Types.VARCHAR:
      case Types.CHAR:
      case Types.STRUCT:
      case Types.ARRAY:
        return true;

      case Types.INTEGER:
      case Types.BIGINT:
      case Types.DECIMAL:
      case Types.DOUBLE:
      case Types.BOOLEAN:
      case Types.TIMESTAMP:
      case Types.TIMESTAMP_WITH_TIMEZONE:
      case Types.DATE:
      case Types.TIME:
      case Types.BINARY:
      default:
        return false;
    }
  }

  @Override
  public boolean isSearchable(int column) throws SQLException {
    return true;
  }

  @Override
  public boolean isCurrency(int column) throws SQLException {
    return false;
  }

  @Override
  public int isNullable(int column) throws SQLException {
    return resultSetMetaData.isNullable(column);
  }

  @Override
  public boolean isSigned(int column) throws SQLException {
    return resultSetMetaData.isSigned(column);
  }

  @Override
  public int getColumnDisplaySize(int column) throws SQLException {
    return resultSetMetaData.getColumnDisplaySize(column);
  }

  @Override
  public String getColumnLabel(int column) throws SQLException {
    return resultSetMetaData.getColumnLabel(column);
  }

  @Override
  public String getColumnName(int column) throws SQLException {
    return resultSetMetaData.getColumnName(column);
  }

  @Override
  public String getSchemaName(int column) throws SQLException {
    // TODO(SNOW-3695645): : is it correct behavior?
    if (this.queryType == QueryType.SYNC) {
      return resultSetMetaData.getSchemaName(column);
    }
    return "";
  }

  @Override
  public int getPrecision(int column) throws SQLException {
    return resultSetMetaData.getPrecision(column);
  }

  @Override
  public int getScale(int column) throws SQLException {
    return resultSetMetaData.getScale(column);
  }

  @Override
  public String getTableName(int column) throws SQLException {
    // TODO(SNOW-3695645): : is it correct behavior?
    if (this.queryType == QueryType.SYNC) {
      return resultSetMetaData.getTableName(column);
    }
    return "";
  }

  @Override
  public String getCatalogName(int column) throws SQLException {
    // TODO(SNOW-3695645): : is it correct behavior?
    if (this.queryType == QueryType.SYNC) {
      return resultSetMetaData.getCatalogName(column);
    }
    return "";
  }

  @Override
  public int getColumnType(int column) throws SQLException {
    try {
      return resultSetMetaData.getColumnType(column);
    } catch (SFException ex) {
      throw new SnowflakeSQLException(ex.getErrorCode(), ex.getMessage());
    }
  }

  @Override
  public String getColumnTypeName(int column) throws SQLException {
    try {
      return resultSetMetaData.getColumnTypeName(column);
    } catch (SFException ex) {
      throw new SnowflakeSQLException(ex.getErrorCode(), ex.getMessage());
    }
  }

  @Override
  public boolean isReadOnly(int column) throws SQLException {
    return true; // metadata column is always readonly
  }

  @Override
  public boolean isWritable(int column) throws SQLException {
    return false; // never writable
  }

  @Override
  public boolean isDefinitelyWritable(int column) throws SQLException {
    return false; // never writable
  }

  @Override
  public String getColumnClassName(int column) throws SQLException {
    int type = this.getColumnType(column);

    return SnowflakeUtil.javaTypeToClassName(type);
  }

  @Override
  public String getQueryID() throws SQLException {
    return queryId;
  }

  @Override
  public List<String> getColumnNames() throws SQLException {
    return resultSetMetaData.getColumnNames();
  }

  @Override
  public int getColumnIndex(String columnName) throws SQLException {
    return resultSetMetaData.getColumnIndex(columnName);
  }

  @Override
  public int getInternalColumnType(int column) throws SQLException {
    try {
      return resultSetMetaData.getInternalColumnType(column);
    } catch (SFException ex) {
      throw new SnowflakeSQLException(ex.getErrorCode(), ex.getMessage());
    }
  }

  @Override
  public List<FieldMetadata> getColumnFields(int column) throws SQLException {
    try {
      return resultSetMetaData.getColumnFields(column);
    } catch (SFException ex) {
      throw new SnowflakeSQLException(ex.getErrorCode(), ex.getMessage());
    }
  }

  @Override
  public int getVectorDimension(int column) throws SQLException {
    return resultSetMetaData.getDimension(column);
  }

  @Override
  public int getVectorDimension(String columnName) throws SQLException {
    return resultSetMetaData.getDimension(getColumnIndex(columnName) + 1);
  }
}
