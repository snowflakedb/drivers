package net.snowflake.client.internal.api.implementation.resultset.metadata;

import java.sql.ResultSetMetaData;
import java.sql.Types;
import java.util.ArrayList;
import java.util.List;
import lombok.AccessLevel;
import lombok.RequiredArgsConstructor;
import net.snowflake.client.api.resultset.FieldMetadata;
import net.snowflake.client.api.resultset.SnowflakeResultSetMetaData;
import net.snowflake.client.internal.codegen.JdbcBoundary;
import net.snowflake.client.internal.core.arrow.converters.DataConversionContext;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.ColumnMetadata;
import net.snowflake.client.internal.util.DelegatingWrapper;
import net.snowflake.client.internal.util.SnowflakeUtil;

@JdbcBoundary
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
   * @param conversionContext the data conversion context providing formatters for display size
   * @return the assembled metadata
   */
  public static SnowflakeResultSetMetaDataImpl from(
      String queryId, List<ColumnMetadata> columns, DataConversionContext conversionContext) {
    // TODO(SNOW-3740746): source isResultColumnCaseInsensitive and
    //  enableReturnTimestampWithTimeZone from connection parameters
    boolean jdbcTreatDecimalAsInt = conversionContext.isTreatDecimalAsInt();
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
            conversionContext.getTimestampNTZFormatter(),
            conversionContext.getTimestampLTZFormatter(),
            conversionContext.getTimestampTZFormatter(),
            conversionContext.getDateFormatter(),
            conversionContext.getTimeFormatter());
    return new SnowflakeResultSetMetaDataImpl(sfResultSetMetaData, queryId, QueryType.SYNC);
  }

  // TODO(SNOW-3740746): the minimal SFResultSetMetaData constructor leaves precisions, scales,
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
  public int getColumnCount() {
    return resultSetMetaData.getColumnCount();
  }

  @Override
  public boolean isAutoIncrement(int column) {
    return resultSetMetaData.getIsAutoIncrement(column);
  }

  @Override
  public boolean isCaseSensitive(int column) {
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
  public boolean isSearchable(int column) {
    return true;
  }

  @Override
  public boolean isCurrency(int column) {
    return false;
  }

  @Override
  public int isNullable(int column) {
    return resultSetMetaData.isNullable(column);
  }

  @Override
  public boolean isSigned(int column) {
    return resultSetMetaData.isSigned(column);
  }

  @Override
  public int getColumnDisplaySize(int column) {
    return resultSetMetaData.getColumnDisplaySize(column);
  }

  @Override
  public String getColumnLabel(int column) {
    return resultSetMetaData.getColumnLabel(column);
  }

  @Override
  public String getColumnName(int column) {
    return resultSetMetaData.getColumnName(column);
  }

  @Override
  public String getSchemaName(int column) {
    // TODO(SNOW-3740747): : is it correct behavior?
    if (this.queryType == QueryType.SYNC) {
      return resultSetMetaData.getSchemaName(column);
    }
    return "";
  }

  @Override
  public int getPrecision(int column) {
    return resultSetMetaData.getPrecision(column);
  }

  @Override
  public int getScale(int column) {
    return resultSetMetaData.getScale(column);
  }

  @Override
  public String getTableName(int column) {
    // TODO(SNOW-3740747): : is it correct behavior?
    if (this.queryType == QueryType.SYNC) {
      return resultSetMetaData.getTableName(column);
    }
    return "";
  }

  @Override
  public String getCatalogName(int column) {
    // TODO(SNOW-3740747): : is it correct behavior?
    if (this.queryType == QueryType.SYNC) {
      return resultSetMetaData.getCatalogName(column);
    }
    return "";
  }

  @Override
  public int getColumnType(int column) {
    // Any SFSQLException propagates untranslated to the decorator boundary.
    return resultSetMetaData.getColumnType(column);
  }

  @Override
  public String getColumnTypeName(int column) {
    return resultSetMetaData.getColumnTypeName(column);
  }

  @Override
  public boolean isReadOnly(int column) {
    return true; // metadata column is always readonly
  }

  @Override
  public boolean isWritable(int column) {
    return false; // never writable
  }

  @Override
  public boolean isDefinitelyWritable(int column) {
    return false; // never writable
  }

  @Override
  public String getColumnClassName(int column) {
    int type = this.getColumnType(column);

    return SnowflakeUtil.javaTypeToClassName(type);
  }

  @Override
  public String getQueryID() {
    return queryId;
  }

  @Override
  public List<String> getColumnNames() {
    return resultSetMetaData.getColumnNames();
  }

  @Override
  public int getColumnIndex(String columnName) {
    return resultSetMetaData.getColumnIndex(columnName);
  }

  /**
   * The in-memory column names without the checked {@link #getColumnNames()} interface signature.
   * For same-driver internal callers (e.g. {@code SnowflakeResultSetImpl#findColumn}) that scan the
   * names directly: the backing list is materialized at construction, so this never fails and needs
   * no boundary translation.
   */
  public List<String> columnNames() {
    return resultSetMetaData.getColumnNames();
  }

  @Override
  public int getInternalColumnType(int column) {
    return resultSetMetaData.getInternalColumnType(column);
  }

  @Override
  public List<FieldMetadata> getColumnFields(int column) {
    return resultSetMetaData.getColumnFields(column);
  }

  @Override
  public int getVectorDimension(int column) {
    return resultSetMetaData.getDimension(column);
  }

  @Override
  public int getVectorDimension(String columnName) {
    return resultSetMetaData.getDimension(getColumnIndex(columnName) + 1);
  }
}
