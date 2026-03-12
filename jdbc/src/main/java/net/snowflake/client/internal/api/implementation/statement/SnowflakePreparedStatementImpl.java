package net.snowflake.client.internal.api.implementation.statement;

import java.io.InputStream;
import java.io.Reader;
import java.math.BigDecimal;
import java.math.BigInteger;
import java.net.URL;
import java.sql.Array;
import java.sql.Blob;
import java.sql.Clob;
import java.sql.Date;
import java.sql.NClob;
import java.sql.ParameterMetaData;
import java.sql.PreparedStatement;
import java.sql.Ref;
import java.sql.ResultSet;
import java.sql.ResultSetMetaData;
import java.sql.RowId;
import java.sql.SQLException;
import java.sql.SQLFeatureNotSupportedException;
import java.sql.SQLXML;
import java.sql.Time;
import java.sql.Timestamp;
import java.sql.Types;
import java.util.Arrays;
import java.util.Calendar;
import java.util.Map;
import java.util.function.Function;
import net.snowflake.client.api.statement.SnowflakePreparedStatement;
import net.snowflake.client.internal.api.implementation.connection.SnowflakeConnectionImpl;
import net.snowflake.client.internal.log.SFLogger;
import net.snowflake.client.internal.log.SFLoggerFactory;
import net.snowflake.client.internal.util.HexUtil;

/**
 * Snowflake JDBC PreparedStatement implementation
 *
 * <p>This is a stub implementation that provides the basic JDBC PreparedStatement interface.
 */
public class SnowflakePreparedStatementImpl extends SnowflakeStatementImpl
    implements PreparedStatement, SnowflakePreparedStatement {
  private static final SFLogger logger =
      SFLoggerFactory.getLogger(SnowflakePreparedStatementImpl.class);

  private final String sql;
  private final PreparedStatementBindingSerializer.ParameterValue[] parameterValues;

  public SnowflakePreparedStatementImpl(SnowflakeConnectionImpl connection, String sql) {
    super(connection);
    this.sql = sql;
    // TODO: Align with snowflake-jdbc by deriving bind count from server-side describe metadata
    // rather than counting '?' directly in raw SQL text.
    int paramCount = sql.length() - sql.replace("?", "").length();
    this.parameterValues = new PreparedStatementBindingSerializer.ParameterValue[paramCount];
  }

  @Override
  public ResultSet executeQuery() throws SQLException {
    checkClosed();
    try (PreparedStatementBindingSerializer.SerializedBindings serializedBindings =
        PreparedStatementBindingSerializer.serialize(parameterValues)) {
      return executeQueryWithBindings(sql, serializedBindings.bindings());
    }
  }

  @Override
  public int executeUpdate() throws SQLException {
    checkClosed();
    execute();
    // TODO return real number of rows affected
    return 0;
  }

  @Override
  public void setNull(int parameterIndex, int sqlType) throws SQLException {
    checkClosed();
    setParameter(parameterIndex, "ANY", null);
  }

  @Override
  public void setBoolean(int parameterIndex, boolean x) throws SQLException {
    checkClosed();
    setParameter(parameterIndex, "BOOLEAN", String.valueOf(x));
  }

  @Override
  public void setByte(int parameterIndex, byte x) throws SQLException {
    checkClosed();
    setParameter(parameterIndex, "FIXED", String.valueOf(x));
  }

  @Override
  public void setShort(int parameterIndex, short x) throws SQLException {
    checkClosed();
    setParameter(parameterIndex, "FIXED", String.valueOf(x));
  }

  @Override
  public void setInt(int parameterIndex, int x) throws SQLException {
    checkClosed();
    setParameter(parameterIndex, "FIXED", String.valueOf(x));
  }

  @Override
  public void setLong(int parameterIndex, long x) throws SQLException {
    checkClosed();
    setParameter(parameterIndex, "FIXED", String.valueOf(x));
  }

  @Override
  public void setFloat(int parameterIndex, float x) throws SQLException {
    checkClosed();
    setParameter(parameterIndex, "REAL", String.valueOf(x));
  }

  @Override
  public void setDouble(int parameterIndex, double x) throws SQLException {
    checkClosed();
    setParameter(parameterIndex, "REAL", String.valueOf(x));
  }

  @Override
  public void setBigDecimal(int parameterIndex, BigDecimal x) throws SQLException {
    checkClosed();
    setNullableParameter(
        parameterIndex, Types.DECIMAL, "FIXED", x, decimal -> String.valueOf(decimal));
  }

  @Override
  public void setString(int parameterIndex, String x) throws SQLException {
    checkClosed();
    setNullableParameter(parameterIndex, Types.VARCHAR, "TEXT", x, stringValue -> stringValue);
  }

  @Override
  public void setBytes(int parameterIndex, byte[] x) throws SQLException {
    checkClosed();
    setNullableParameter(
        parameterIndex, Types.BINARY, "BINARY", x, bytes -> HexUtil.bytesToHex(bytes));
  }

  @Override
  public void setDate(int parameterIndex, Date x) throws SQLException {
    checkClosed();
    setNullableParameter(
        parameterIndex, Types.DATE, "TEXT", x, date -> date.toLocalDate().toString());
  }

  @Override
  public void setTime(int parameterIndex, Time x) throws SQLException {
    checkClosed();
    throw new SQLFeatureNotSupportedException("setTime not supported");
  }

  @Override
  public void setTimestamp(int parameterIndex, Timestamp x) throws SQLException {
    checkClosed();
    throw new SQLFeatureNotSupportedException("setTimestamp not supported");
  }

  @Override
  public void setAsciiStream(int parameterIndex, InputStream x, int length) throws SQLException {
    throw new SQLFeatureNotSupportedException("setAsciiStream not supported");
  }

  @Override
  public void setUnicodeStream(int parameterIndex, InputStream x, int length) throws SQLException {
    throw new SQLFeatureNotSupportedException("setUnicodeStream not supported");
  }

  @Override
  public void setBinaryStream(int parameterIndex, InputStream x, int length) throws SQLException {
    throw new SQLFeatureNotSupportedException("setBinaryStream not supported");
  }

  @Override
  public void clearParameters() throws SQLException {
    checkClosed();
    logger.trace("Clearing prepared parameters: placeholders={}", parameterValues.length);
    Arrays.fill(parameterValues, null);
  }

  @Override
  public void setObject(int parameterIndex, Object x, int targetSqlType) throws SQLException {
    checkClosed();
    if (x == null) {
      setNull(parameterIndex, targetSqlType);
      return;
    }
    if (targetSqlType == Types.DATE) {
      if (!(x instanceof Date)) {
        throw new SQLException(
            "Invalid parameter type for DATE at index "
                + parameterIndex
                + ": "
                + x.getClass().getCanonicalName());
      }
      setDate(parameterIndex, (Date) x);
      return;
    }
    if (targetSqlType == Types.TIME) {
      if (!(x instanceof Time)) {
        throw new SQLException(
            "Invalid parameter type for TIME at index "
                + parameterIndex
                + ": "
                + x.getClass().getCanonicalName());
      }
      setTime(parameterIndex, (Time) x);
      return;
    }
    if (targetSqlType == Types.TIMESTAMP) {
      if (!(x instanceof Timestamp)) {
        throw new SQLException(
            "Invalid parameter type for TIMESTAMP at index "
                + parameterIndex
                + ": "
                + x.getClass().getCanonicalName());
      }
      setTimestamp(parameterIndex, (Timestamp) x);
      return;
    }
    String bindType = sqlTypeToBindType(targetSqlType);
    if ("BINARY".equals(bindType) && x instanceof byte[]) {
      setBytes(parameterIndex, (byte[]) x);
      return;
    }
    setParameter(parameterIndex, bindType, x);
  }

  @Override
  public void setObject(int parameterIndex, Object x) throws SQLException {
    checkClosed();
    if (x == null) {
      setNull(parameterIndex, Types.NULL);
      return;
    }
    if (x instanceof String) {
      setString(parameterIndex, (String) x);
      return;
    }
    if (x instanceof Boolean) {
      setBoolean(parameterIndex, (Boolean) x);
      return;
    }
    if (x instanceof Short) {
      setShort(parameterIndex, (Short) x);
      return;
    }
    if (x instanceof Integer) {
      setInt(parameterIndex, (Integer) x);
      return;
    }
    if (x instanceof Long) {
      setLong(parameterIndex, (Long) x);
      return;
    }
    if (x instanceof Float) {
      setFloat(parameterIndex, (Float) x);
      return;
    }
    if (x instanceof Double) {
      setDouble(parameterIndex, (Double) x);
      return;
    }
    if (x instanceof BigDecimal) {
      setBigDecimal(parameterIndex, (BigDecimal) x);
      return;
    }
    if (x instanceof byte[]) {
      setBytes(parameterIndex, (byte[]) x);
      return;
    }
    logger.warn(
        "Unsupported prepared parameter value type: index={}, type={}",
        parameterIndex,
        x.getClass().getCanonicalName());
    throw new SQLException(
        "Unsupported parameter value type at index "
            + parameterIndex
            + ": "
            + x.getClass().getCanonicalName());
  }

  @Override
  public boolean execute() throws SQLException {
    checkClosed();
    try (PreparedStatementBindingSerializer.SerializedBindings serializedBindings =
        PreparedStatementBindingSerializer.serialize(parameterValues)) {
      try (ResultSet ignored = executeQueryWithBindings(sql, serializedBindings.bindings())) {
        // TODO: Align execute() return value and update-count behavior with snowflake-jdbc by using
        // backend statement-type metadata (true for result sets, false for update counts).
        return true;
      }
    }
  }

  @Override
  public void addBatch() throws SQLException {
    throw new SQLFeatureNotSupportedException("addBatch not supported");
  }

  @Override
  public void setCharacterStream(int parameterIndex, Reader reader, int length)
      throws SQLException {
    throw new SQLFeatureNotSupportedException("setCharacterStream not supported");
  }

  @Override
  public void setRef(int parameterIndex, Ref x) throws SQLException {
    throw new SQLFeatureNotSupportedException("setRef not supported");
  }

  @Override
  public void setBlob(int parameterIndex, Blob x) throws SQLException {
    throw new SQLFeatureNotSupportedException("setBlob not supported");
  }

  @Override
  public void setClob(int parameterIndex, Clob x) throws SQLException {
    throw new SQLFeatureNotSupportedException("setClob not supported");
  }

  @Override
  public void setArray(int parameterIndex, Array x) throws SQLException {
    throw new SQLFeatureNotSupportedException("setArray not supported");
  }

  @Override
  public ResultSetMetaData getMetaData() throws SQLException {
    throw new SQLFeatureNotSupportedException("getMetaData not supported");
  }

  @Override
  public void setDate(int parameterIndex, Date x, Calendar cal) throws SQLException {
    setDate(parameterIndex, x);
  }

  @Override
  public void setTime(int parameterIndex, Time x, Calendar cal) throws SQLException {
    setTime(parameterIndex, x);
  }

  @Override
  public void setTimestamp(int parameterIndex, Timestamp x, Calendar cal) throws SQLException {
    setTimestamp(parameterIndex, x);
  }

  @Override
  public void setNull(int parameterIndex, int sqlType, String typeName) throws SQLException {
    setNull(parameterIndex, sqlType);
  }

  @Override
  public void setURL(int parameterIndex, URL x) throws SQLException {
    throw new SQLFeatureNotSupportedException("setURL not supported");
  }

  @Override
  public ParameterMetaData getParameterMetaData() throws SQLException {
    throw new SQLFeatureNotSupportedException("getParameterMetaData not supported");
  }

  @Override
  public void setRowId(int parameterIndex, RowId x) throws SQLException {
    throw new SQLFeatureNotSupportedException("setRowId not supported");
  }

  @Override
  public void setNString(int parameterIndex, String value) throws SQLException {
    setString(parameterIndex, value);
  }

  @Override
  public void setNCharacterStream(int parameterIndex, Reader value, long length)
      throws SQLException {
    throw new SQLFeatureNotSupportedException("setNCharacterStream not supported");
  }

  @Override
  public void setNClob(int parameterIndex, NClob value) throws SQLException {
    throw new SQLFeatureNotSupportedException("setNClob not supported");
  }

  @Override
  public void setClob(int parameterIndex, Reader reader, long length) throws SQLException {
    throw new SQLFeatureNotSupportedException("setClob not supported");
  }

  @Override
  public void setBlob(int parameterIndex, InputStream inputStream, long length)
      throws SQLException {
    throw new SQLFeatureNotSupportedException("setBlob not supported");
  }

  @Override
  public void setNClob(int parameterIndex, Reader reader, long length) throws SQLException {
    throw new SQLFeatureNotSupportedException("setNClob not supported");
  }

  @Override
  public void setSQLXML(int parameterIndex, SQLXML xmlObject) throws SQLException {
    throw new SQLFeatureNotSupportedException("setSQLXML not supported");
  }

  @Override
  public void setObject(int parameterIndex, Object x, int targetSqlType, int scaleOrLength)
      throws SQLException {
    setObject(parameterIndex, x, targetSqlType);
  }

  @Override
  public void setAsciiStream(int parameterIndex, InputStream x, long length) throws SQLException {
    throw new SQLFeatureNotSupportedException("setAsciiStream not supported");
  }

  @Override
  public void setBinaryStream(int parameterIndex, InputStream x, long length) throws SQLException {
    throw new SQLFeatureNotSupportedException("setBinaryStream not supported");
  }

  @Override
  public void setCharacterStream(int parameterIndex, Reader reader, long length)
      throws SQLException {
    throw new SQLFeatureNotSupportedException("setCharacterStream not supported");
  }

  @Override
  public void setAsciiStream(int parameterIndex, InputStream x) throws SQLException {
    throw new SQLFeatureNotSupportedException("setAsciiStream not supported");
  }

  @Override
  public void setBinaryStream(int parameterIndex, InputStream x) throws SQLException {
    throw new SQLFeatureNotSupportedException("setBinaryStream not supported");
  }

  @Override
  public void setCharacterStream(int parameterIndex, Reader reader) throws SQLException {
    throw new SQLFeatureNotSupportedException("setCharacterStream not supported");
  }

  @Override
  public void setNCharacterStream(int parameterIndex, Reader value) throws SQLException {
    throw new SQLFeatureNotSupportedException("setNCharacterStream not supported");
  }

  @Override
  public void setClob(int parameterIndex, Reader reader) throws SQLException {
    throw new SQLFeatureNotSupportedException("setClob not supported");
  }

  @Override
  public void setBlob(int parameterIndex, InputStream inputStream) throws SQLException {
    throw new SQLFeatureNotSupportedException("setBlob not supported");
  }

  @Override
  public void setNClob(int parameterIndex, Reader reader) throws SQLException {
    throw new SQLFeatureNotSupportedException("setNClob not supported");
  }

  private void setParameter(int parameterIndex, String bindType, Object value) throws SQLException {
    if (parameterIndex < 1 || parameterIndex > parameterValues.length) {
      logger.warn(
          "Invalid prepared parameter index: index={}, placeholders={}",
          parameterIndex,
          parameterValues.length);
      throw new SQLException("Invalid parameter index: " + parameterIndex);
    }
    parameterValues[parameterIndex - 1] =
        new PreparedStatementBindingSerializer.ParameterValue(bindType, value);
    logger.debug(
        "Prepared parameter set: index={}, bindType={}, isNull={}, placeholders={}",
        parameterIndex,
        bindType,
        value == null,
        parameterValues.length);
  }

  private <T> void setNullableParameter(
      int parameterIndex, int sqlType, String bindType, T value, Function<T, String> serializer)
      throws SQLException {
    if (value == null) {
      setNull(parameterIndex, sqlType);
      return;
    }
    setParameter(parameterIndex, bindType, serializer.apply(value));
  }

  private static String sqlTypeToBindType(int sqlType) {
    switch (sqlType) {
      case Types.BOOLEAN:
      case Types.BIT:
        return "BOOLEAN";
      case Types.TINYINT:
      case Types.SMALLINT:
      case Types.INTEGER:
      case Types.BIGINT:
      case Types.NUMERIC:
      case Types.DECIMAL:
        return "FIXED";
      case Types.FLOAT:
      case Types.REAL:
      case Types.DOUBLE:
        return "REAL";
      case Types.BINARY:
      case Types.VARBINARY:
      case Types.LONGVARBINARY:
      case Types.BLOB:
        return "BINARY";
      default:
        return "TEXT";
    }
  }

  @Override
  public ResultSet executeAsyncQuery() throws SQLException {
    throw new SQLFeatureNotSupportedException("executeAsyncQuery not supported");
  }

  @Override
  public void setBigInteger(int parameterIndex, BigInteger x) throws SQLException {
    throw new SQLFeatureNotSupportedException("setBigInteger not supported");
  }

  @Override
  public <T> void setMap(int parameterIndex, Map<String, T> map, int type) throws SQLException {
    throw new SQLFeatureNotSupportedException("setMap not supported");
  }
}
