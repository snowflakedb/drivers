package net.snowflake.client.internal.api.implementation.resultset.metadata;

import java.sql.ResultSetMetaData;
import java.sql.Types;
import java.util.ArrayList;
import java.util.HashMap;
import java.util.List;
import java.util.Map;
import net.snowflake.client.api.exception.ErrorCode;
import net.snowflake.client.api.exception.SFException;
import net.snowflake.client.api.resultset.FieldMetadata;
import net.snowflake.client.api.resultset.SnowflakeType;
import net.snowflake.client.internal.common.core.SnowflakeDateTimeFormat;
import net.snowflake.client.internal.log.SFLogger;
import net.snowflake.client.internal.log.SFLoggerFactory;

/** Snowflake ResultSetMetaData */
// ported from snowflake-jdbc
class SFResultSetMetaData {
  // TODO(SNOW-3695645): maybe we should validate column index on all accessors and throw
  // SFException
  //   with COLUMN_DOES_NOT_EXIST (surfaced as SQLException by the wrapper)
  //   instead of letting out-of-range indices propagate as IndexOutOfBoundsException
  //   Current implementation is ported from snowflake-jdbc.

  private static final SFLogger logger = SFLoggerFactory.getLogger(SFResultSetMetaData.class);

  private int columnCount = 0;

  private List<String> columnNames;

  private List<String> columnTypeNames;

  private List<Integer> columnTypes;

  private List<Integer> precisions;

  private List<Integer> dimensions;

  private List<Integer> scales;

  private List<Integer> nullables;

  private List<String> columnSrcTables;

  private List<String> columnSrcSchemas;

  private List<String> columnSrcDatabases;

  private List<Integer> columnDisplaySizes;

  private List<SnowflakeColumnMetadata> columnMetadata = new ArrayList<>();
  private String queryId;

  private Map<String, Integer> columnNamePositionMap = new HashMap<>();

  private Map<String, Integer> columnNameUpperCasePositionMap = new HashMap<>();

  // Controls whether TIMESTAMP_TZ columns are reported as TIMESTAMP_WITH_TIMEZONE or TIMESTAMP.
  // Sourced from connection parameters by the caller (was SFBaseSession in the legacy driver).
  private boolean enableReturnTimestampWithTimeZone = true;

  // Date time formatter for calculating the display size
  private SnowflakeDateTimeFormat timestampNTZFormatter;

  private SnowflakeDateTimeFormat timestampLTZFormatter;

  private SnowflakeDateTimeFormat timestampTZFormatter;

  private SnowflakeDateTimeFormat timeFormatter;

  private SnowflakeDateTimeFormat dateFormatter;

  // provide default display size for databasemetadata result set.
  // i.e. result set returned calling getTables etc
  private int timestampNTZStringLength = 30;

  private int timestampLTZStringLength = 30;

  private int timestampTZStringLength = 30;

  private int timeStringLength = 18;

  private int dateStringLength = 10;

  private boolean isResultColumnCaseInsensitive = false;

  private List<Boolean> isAutoIncrementList;

  SFResultSetMetaData(
      List<SnowflakeColumnMetadata> columnMetadata,
      String queryId,
      boolean isResultColumnCaseInsensitive,
      boolean enableReturnTimestampWithTimeZone,
      SnowflakeDateTimeFormat timestampNTZFormatter,
      SnowflakeDateTimeFormat timestampLTZFormatter,
      SnowflakeDateTimeFormat timestampTZFormatter,
      SnowflakeDateTimeFormat dateFormatter,
      SnowflakeDateTimeFormat timeFormatter) {
    this.columnCount = columnMetadata.size();
    this.columnMetadata = columnMetadata;
    this.queryId = queryId;
    this.timestampNTZFormatter = timestampNTZFormatter;
    this.timestampLTZFormatter = timestampLTZFormatter;
    this.timestampTZFormatter = timestampTZFormatter;
    this.dateFormatter = dateFormatter;
    this.timeFormatter = timeFormatter;
    this.enableReturnTimestampWithTimeZone = enableReturnTimestampWithTimeZone;
    // TODO(SNOW-3695645): port calculateDateTimeStringLength() to compute accurate display sizes
    // from the configured formatters. It previously built SFTimestamp/SFTime samples
    // and measured their formatted length. For now we keep the default lengths (30/30/30/18/10).

    this.columnNames = new ArrayList<>(this.columnCount);
    this.columnTypeNames = new ArrayList<>(this.columnCount);
    this.columnTypes = new ArrayList<>(this.columnCount);
    this.precisions = new ArrayList<>(this.columnCount);
    this.dimensions = new ArrayList<>(this.columnCount);
    this.scales = new ArrayList<>(this.columnCount);
    this.nullables = new ArrayList<>(this.columnCount);
    this.columnSrcDatabases = new ArrayList<>(this.columnCount);
    this.columnSrcSchemas = new ArrayList<>(this.columnCount);
    this.columnSrcTables = new ArrayList<>(this.columnCount);
    this.columnDisplaySizes = new ArrayList<>(this.columnCount);
    this.isAutoIncrementList = new ArrayList<>(this.columnCount);
    this.isResultColumnCaseInsensitive = isResultColumnCaseInsensitive;

    for (int colIdx = 0; colIdx < columnCount; colIdx++) {
      columnNames.add(columnMetadata.get(colIdx).getName());
      columnTypeNames.add(columnMetadata.get(colIdx).getTypeName());
      precisions.add(calculatePrecision(columnMetadata.get(colIdx)));
      dimensions.add(calculateDimension(columnMetadata.get(colIdx)));
      columnTypes.add(columnMetadata.get(colIdx).getType());
      scales.add(columnMetadata.get(colIdx).getScale());
      nullables.add(
          columnMetadata.get(colIdx).isNullable()
              ? ResultSetMetaData.columnNullable
              : ResultSetMetaData.columnNoNulls);
      columnSrcDatabases.add(columnMetadata.get(colIdx).getColumnSrcDatabase());
      columnSrcSchemas.add(columnMetadata.get(colIdx).getColumnSrcSchema());
      columnSrcTables.add(columnMetadata.get(colIdx).getColumnSrcTable());
      columnDisplaySizes.add(calculateDisplaySize(columnMetadata.get(colIdx)));
      isAutoIncrementList.add(columnMetadata.get(colIdx).isAutoIncrement());
    }
  }

  private Integer calculatePrecision(SnowflakeColumnMetadata columnMetadata) {
    int columnType = columnMetadata.getType();
    switch (columnType) {
      case Types.CHAR:
      case Types.VARCHAR:
      case Types.BINARY:
        return columnMetadata.getLength();
      case Types.INTEGER:
      case Types.DECIMAL:
      case Types.BIGINT:
        return columnMetadata.getPrecision();
      case Types.DATE:
        return dateStringLength;
      case Types.TIME:
        return timeStringLength;
      case SnowflakeType.EXTRA_TYPES_TIMESTAMP_LTZ:
        return timestampLTZStringLength;
      case SnowflakeType.EXTRA_TYPES_TIMESTAMP_TZ:
        return timestampTZStringLength;
      case Types.TIMESTAMP:
        return timestampNTZStringLength;
        // for double and boolean
        // Precision is not applicable hence return 0
      default:
        return 0;
    }
  }

  private Integer calculateDimension(SnowflakeColumnMetadata columnMetadata) {
    int columnType = columnMetadata.getType();
    if (columnType == SnowflakeType.EXTRA_TYPES_VECTOR) {
      return columnMetadata.getDimension();
    }
    return 0;
  }

  private Integer calculateDisplaySize(SnowflakeColumnMetadata columnMetadata) {
    int columnType = columnMetadata.getType();
    switch (columnType) {
      case Types.CHAR:
      case Types.VARCHAR:
      case Types.BINARY:
        return columnMetadata.getLength();
      case Types.INTEGER:
      case Types.BIGINT:
        // + 1 because number can be negative, it could be -20 for number(2,0)
        return columnMetadata.getPrecision() + 1;
      case Types.DECIMAL:
        // first + 1 because number can be negative, second + 1 because it always
        // include decimal point.
        // i.e. number(2, 1) could be -1.3
        return columnMetadata.getPrecision() + 1 + 1;
      case Types.DOUBLE:
        // Hard code as 24 since the longest float
        // represented in char is
        // -2.2250738585072020E−308
        return 24;
      case Types.DATE:
        return dateStringLength;
      case Types.TIME:
        return timeStringLength;
      case SnowflakeType.EXTRA_TYPES_TIMESTAMP_LTZ:
        return timestampLTZStringLength;
      case SnowflakeType.EXTRA_TYPES_TIMESTAMP_TZ:
        return timestampTZStringLength;
      case Types.TIMESTAMP:
        return timestampNTZStringLength;
      case Types.BOOLEAN:
        // Hard code as 5 since the longest char to represent
        // a boolean would be false, which is 5.
        return 5;
      default:
        return 25;
    }
  }

  /**
   * get the query id
   *
   * @return query id
   */
  String getQueryId() {
    return queryId;
  }

  /**
   * Get the list of column names
   *
   * @return column names in list
   */
  List<String> getColumnNames() {
    return columnNames;
  }

  /**
   * Get the index of the column by name
   *
   * @param columnName column name
   * @return index of the column that names matches the column name
   */
  int getColumnIndex(String columnName) {
    columnName = isResultColumnCaseInsensitive ? columnName.toUpperCase() : columnName;
    Map<String, Integer> nameToIndexMap =
        isResultColumnCaseInsensitive ? columnNameUpperCasePositionMap : columnNamePositionMap;

    if (nameToIndexMap.get(columnName) != null) {
      return nameToIndexMap.get(columnName);
    } else {
      int columnIndex =
          isResultColumnCaseInsensitive
              ? listSearchCaseInsensitive(columnNames, columnName)
              : columnNames.indexOf(columnName);
      nameToIndexMap.put(columnName, columnIndex);
      return columnIndex;
    }
  }

  /**
   * Get number of columns
   *
   * @return column count
   */
  int getColumnCount() {
    return columnCount;
  }

  int getColumnType(int column) throws SFException {
    return ColumnTypeHelper.getColumnType(
        getInternalColumnType(column), enableReturnTimestampWithTimeZone);
  }

  /**
   * Returns the index of the first element in {@code source} that case-insensitively equals {@code
   * target}, or {@code -1} if none match. Inlined from the legacy {@code
   * ResultUtil.listSearchCaseInsensitive}.
   */
  private static int listSearchCaseInsensitive(List<String> source, String target) {
    for (int i = 0; i < source.size(); i++) {
      if (target.equalsIgnoreCase(source.get(i))) {
        return i;
      }
    }
    return -1;
  }

  int getInternalColumnType(int column) throws SFException {
    int columnIdx = column - 1;
    if (column < 1 || column > columnTypes.size()) {
      throw new SFException(ErrorCode.COLUMN_DOES_NOT_EXIST, column);
    }

    if (columnTypes.get(columnIdx) == null) {
      throw new SFException(ErrorCode.INTERNAL_ERROR, "Missing column type for column " + column);
    }

    return columnTypes.get(columnIdx);
  }

  String getColumnTypeName(int column) throws SFException {
    if (column < 1 || column > columnTypeNames.size()) {
      throw new SFException(ErrorCode.COLUMN_DOES_NOT_EXIST, column);
    }

    if (columnTypeNames.get(column - 1) == null) {
      throw new SFException(ErrorCode.INTERNAL_ERROR, "Missing column type for column " + column);
    }

    return columnTypeNames.get(column - 1);
  }

  int getScale(int column) {
    if (scales != null && scales.size() >= column) {
      return scales.get(column - 1);
    } else {
      // TODO: fix this later to use different defaults for number or timestamp
      return 9;
    }
  }

  int getPrecision(int column) {
    if (precisions != null && precisions.size() >= column) {
      return precisions.get(column - 1);
    } else {
      // TODO: fix this later to use different defaults for number or timestamp
      return 9;
    }
  }

  int getDimension(int column) {
    if (dimensions != null && dimensions.size() >= column && column > 0) {
      return dimensions.get(column - 1);
    } else {
      return 0;
    }
  }

  boolean isSigned(int column) {
    return (columnTypes.get(column - 1) == Types.INTEGER
        || columnTypes.get(column - 1) == Types.DECIMAL
        || columnTypes.get(column - 1) == Types.BIGINT
        || columnTypes.get(column - 1) == Types.DOUBLE);
  }

  String getColumnLabel(int column) {
    if (columnNames != null) {
      return columnNames.get(column - 1);
    } else {
      return "C" + Integer.toString(column - 1);
    }
  }

  String getColumnName(int column) {
    if (columnNames != null) {
      return columnNames.get(column - 1);
    } else {
      return "C" + Integer.toString(column - 1);
    }
  }

  int isNullable(int column) {
    if (nullables != null) {
      return nullables.get(column - 1);
    } else {
      return ResultSetMetaData.columnNullableUnknown;
    }
  }

  String getCatalogName(int column) {
    if (columnSrcDatabases == null) {
      return "";
    }
    return columnSrcDatabases.get(column - 1);
  }

  String getSchemaName(int column) {
    if (columnSrcDatabases == null) {
      return "";
    }
    return columnSrcSchemas.get(column - 1);
  }

  String getTableName(int column) {
    if (columnSrcDatabases == null) {
      return "T";
    }
    return columnSrcTables.get(column - 1);
  }

  Integer getColumnDisplaySize(int column) {
    if (columnDisplaySizes == null) {
      return 25;
    }
    return columnDisplaySizes.get(column - 1);
  }

  boolean getIsAutoIncrement(int column) {
    if (isAutoIncrementList == null || isAutoIncrementList.size() == 0) {
      return false;
    }

    return isAutoIncrementList.get(column - 1);
  }

  List<Boolean> getIsAutoIncrementList() {
    return isAutoIncrementList;
  }

  List<FieldMetadata> getColumnFields(int column) throws SFException {
    if (column < 1 || column > columnMetadata.size()) {
      throw new SFException(ErrorCode.COLUMN_DOES_NOT_EXIST, column);
    }

    if (columnMetadata.get(column - 1) == null) {
      throw new SFException(ErrorCode.INTERNAL_ERROR, "Missing column fields for column " + column);
    }

    return columnMetadata.get(column - 1).getFields();
  }

  boolean isStructuredTypeColumn(int columnIndex) {
    return columnMetadata.get(columnIndex - 1).getFields() != null
        && !columnMetadata.get(columnIndex - 1).getFields().isEmpty();
  }
}
