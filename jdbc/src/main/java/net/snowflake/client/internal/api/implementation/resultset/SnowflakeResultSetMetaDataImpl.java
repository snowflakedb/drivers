package net.snowflake.client.internal.api.implementation.resultset;

import java.sql.ResultSetMetaData;
import java.sql.SQLException;
import java.sql.SQLFeatureNotSupportedException;
import java.util.List;
import net.snowflake.client.api.resultset.FieldMetadata;
import net.snowflake.client.api.resultset.SnowflakeResultSetMetaData;
import net.snowflake.client.internal.util.NotImplementedException;

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
    throw new NotImplementedException();
  }

  @Override
  public boolean isAutoIncrement(int column) throws SQLException {
    throw new NotImplementedException();
  }

  @Override
  public boolean isCaseSensitive(int column) throws SQLException {
    throw new NotImplementedException();
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
    throw new NotImplementedException();
  }

  @Override
  public boolean isSigned(int column) throws SQLException {
    throw new NotImplementedException();
  }

  @Override
  public int getColumnDisplaySize(int column) throws SQLException {
    throw new NotImplementedException();
  }

  @Override
  public String getColumnLabel(int column) throws SQLException {
    throw new NotImplementedException();
  }

  @Override
  public String getColumnName(int column) throws SQLException {
    throw new NotImplementedException();
  }

  @Override
  public String getSchemaName(int column) throws SQLException {
    throw new NotImplementedException();
  }

  @Override
  public int getPrecision(int column) throws SQLException {
    throw new NotImplementedException();
  }

  @Override
  public int getScale(int column) throws SQLException {
    throw new NotImplementedException();
  }

  @Override
  public String getTableName(int column) throws SQLException {
    throw new NotImplementedException();
  }

  @Override
  public String getCatalogName(int column) throws SQLException {
    throw new NotImplementedException();
  }

  @Override
  public int getColumnType(int column) throws SQLException {
    throw new NotImplementedException();
  }

  @Override
  public String getColumnTypeName(int column) throws SQLException {
    throw new NotImplementedException();
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
    throw new NotImplementedException();
  }

  @Override
  public <T> T unwrap(Class<T> iface) throws SQLException {
    if (!iface.isInstance(this)) {
      throw new SQLException(
          this.getClass().getName() + " not unwrappable from " + iface.getName());
    }
    return (T) this;
  }

  @Override
  public boolean isWrapperFor(Class<?> iface) throws SQLException {
    return iface.isInstance(this);
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
    throw new NotImplementedException();
  }

  @Override
  public int getVectorDimension(int column) throws SQLException {
    throw new NotImplementedException();
  }

  @Override
  public int getVectorDimension(String columnName) throws SQLException {
    throw new NotImplementedException();
  }
}
