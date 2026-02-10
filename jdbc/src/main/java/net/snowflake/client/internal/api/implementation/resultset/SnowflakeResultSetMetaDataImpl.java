package net.snowflake.client.internal.api.implementation.resultset;

import java.sql.ResultSetMetaData;
import java.sql.SQLException;
import java.sql.SQLFeatureNotSupportedException;
import java.sql.Types;
import java.util.List;
import net.snowflake.client.api.resultset.FieldMetadata;
import net.snowflake.client.api.resultset.SnowflakeResultSetMetaData;

/** Simple ResultSetMetaData implementation */
public class SnowflakeResultSetMetaDataImpl
    implements ResultSetMetaData, SnowflakeResultSetMetaData {
  private final String[] columnNames;
  private final int[] columnTypes;

  public SnowflakeResultSetMetaDataImpl(String[] columnNames, int[] columnTypes) {
    this.columnNames = columnNames;
    this.columnTypes = columnTypes;
  }

  @Override
  public int getColumnCount() throws SQLException {
    return columnNames.length;
  }

  @Override
  public boolean isAutoIncrement(int column) throws SQLException {
    return false;
  }

  @Override
  public boolean isCaseSensitive(int column) throws SQLException {
    return false;
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
    return columnNullable;
  }

  @Override
  public boolean isSigned(int column) throws SQLException {
    int type = getColumnType(column);
    return type == Types.INTEGER
        || type == Types.BIGINT
        || type == Types.SMALLINT
        || type == Types.TINYINT
        || type == Types.FLOAT
        || type == Types.DOUBLE
        || type == Types.DECIMAL
        || type == Types.NUMERIC;
  }

  @Override
  public int getColumnDisplaySize(int column) throws SQLException {
    return 255;
  }

  @Override
  public String getColumnLabel(int column) throws SQLException {
    checkColumnIndex(column);
    return columnNames[column - 1];
  }

  @Override
  public String getColumnName(int column) throws SQLException {
    return getColumnLabel(column);
  }

  @Override
  public String getSchemaName(int column) throws SQLException {
    return "PUBLIC";
  }

  @Override
  public int getPrecision(int column) throws SQLException {
    return 0;
  }

  @Override
  public int getScale(int column) throws SQLException {
    return 0;
  }

  @Override
  public String getTableName(int column) throws SQLException {
    return "STUB_TABLE";
  }

  @Override
  public String getCatalogName(int column) throws SQLException {
    return "STUB_DB";
  }

  @Override
  public int getColumnType(int column) throws SQLException {
    checkColumnIndex(column);
    return columnTypes[column - 1];
  }

  @Override
  public String getColumnTypeName(int column) throws SQLException {
    int type = getColumnType(column);
    switch (type) {
      case Types.INTEGER:
        return "INTEGER";
      case Types.VARCHAR:
        return "VARCHAR";
      case Types.DATE:
        return "DATE";
      case Types.TIMESTAMP:
        return "TIMESTAMP";
      default:
        return "VARCHAR";
    }
  }

  @Override
  public boolean isReadOnly(int column) throws SQLException {
    return true;
  }

  @Override
  public boolean isWritable(int column) throws SQLException {
    return false;
  }

  @Override
  public boolean isDefinitelyWritable(int column) throws SQLException {
    return false;
  }

  @Override
  public String getColumnClassName(int column) throws SQLException {
    int type = getColumnType(column);
    switch (type) {
      case Types.INTEGER:
        return "java.lang.Integer";
      case Types.VARCHAR:
        return "java.lang.String";
      case Types.DATE:
        return "java.sql.Date";
      case Types.TIMESTAMP:
        return "java.sql.Timestamp";
      default:
        return "java.lang.String";
    }
  }

  @Override
  public <T> T unwrap(Class<T> iface) throws SQLException {
    if (iface.isAssignableFrom(getClass())) {
      return iface.cast(this);
    }
    throw new SQLException("Cannot unwrap to " + iface.getName());
  }

  @Override
  public boolean isWrapperFor(Class<?> iface) throws SQLException {
    return iface.isAssignableFrom(getClass());
  }

  private void checkColumnIndex(int column) throws SQLException {
    if (column < 1 || column > columnNames.length) {
      throw new SQLException("Invalid column index: " + column);
    }
  }

  @Override
  public String getQueryID() throws SQLException {
    throw new SQLFeatureNotSupportedException("getQueryID not supported");
  }

  @Override
  public List<String> getColumnNames() throws SQLException {
    throw new SQLFeatureNotSupportedException("getColumnNames not supported");
  }

  @Override
  public int getColumnIndex(String columnName) throws SQLException {
    throw new SQLFeatureNotSupportedException("getColumnIndex not supported");
  }

  @Override
  public int getInternalColumnType(int column) throws SQLException {
    throw new SQLFeatureNotSupportedException("getInternalColumnType not supported");
  }

  @Override
  public List<FieldMetadata> getColumnFields(int column) throws SQLException {
    throw new SQLFeatureNotSupportedException("getColumnFields not supported");
  }

  @Override
  public int getVectorDimension(int column) throws SQLException {
    throw new SQLFeatureNotSupportedException("getVectorDimension not supported");
  }

  @Override
  public int getVectorDimension(String columnName) throws SQLException {
    throw new SQLFeatureNotSupportedException("getVectorDimension not supported");
  }
}
