package net.snowflake.client.internal.api.implementation.statement;

import java.io.InputStream;
import java.io.Reader;
import java.math.BigDecimal;
import java.net.URL;
import java.sql.Array;
import java.sql.Blob;
import java.sql.CallableStatement;
import java.sql.Clob;
import java.sql.Date;
import java.sql.NClob;
import java.sql.Ref;
import java.sql.RowId;
import java.sql.SQLException;
import java.sql.SQLFeatureNotSupportedException;
import java.sql.SQLXML;
import java.sql.Time;
import java.sql.Timestamp;
import java.util.Calendar;
import java.util.Map;
import net.snowflake.client.internal.api.implementation.connection.InternalSnowflakeConnection;
import net.snowflake.client.internal.log.SFLogger;
import net.snowflake.client.internal.log.SFLoggerFactory;
import net.snowflake.client.internal.unicore.CoreDriverApi;

public final class SnowflakeCallableStatementImpl extends SnowflakePreparedStatementImpl
    implements CallableStatement {
  private static final SFLogger logger =
      SFLoggerFactory.getLogger(SnowflakeCallableStatementImpl.class);

  public SnowflakeCallableStatementImpl(
      InternalSnowflakeConnection connection, String sql, CoreDriverApi coreDriverApi) {
    super(connection, parseSqlEscapeSyntax(sql), coreDriverApi);
  }

  /**
   * Removes JDBC curly-bracket escape syntax before sending SQL to the server, since the GS parser
   * does not support it.
   *
   * @param originalSql original SQL text, possibly wrapped in curly brackets
   * @return SQL text with outer curly brackets removed
   */
  public static String parseSqlEscapeSyntax(String originalSql) {
    originalSql = originalSql.trim();
    if (originalSql.startsWith("{") && originalSql.endsWith("}")) {
      logger.debug("Curly brackets {} removed before sending sql to server.", false);
      return originalSql.substring(1, originalSql.length() - 1);
    }
    return originalSql;
  }

  /*
   * The Snowflake database does not accept OUT or INOUT parameters, so the registerOutParameter
   * functions and the get functions (which get values of OUT parameters) will remain not
   * implemented.
   */

  @Override
  public void registerOutParameter(int parameterIndex, int sqlType) throws SQLException {
    throw new SQLFeatureNotSupportedException("registerOutParameter not supported");
  }

  @Override
  public void registerOutParameter(int parameterIndex, int sqlType, int scale) throws SQLException {
    throw new SQLFeatureNotSupportedException("registerOutParameter not supported");
  }

  @Override
  public void registerOutParameter(int parameterIndex, int sqlType, String typeName)
      throws SQLException {
    throw new SQLFeatureNotSupportedException("registerOutParameter not supported");
  }

  @Override
  public void registerOutParameter(String parameterName, int sqlType) throws SQLException {
    throw new SQLFeatureNotSupportedException("registerOutParameter not supported");
  }

  @Override
  public void registerOutParameter(String parameterName, int sqlType, int scale)
      throws SQLException {
    throw new SQLFeatureNotSupportedException("registerOutParameter not supported");
  }

  @Override
  public void registerOutParameter(String parameterName, int sqlType, String typeName)
      throws SQLException {
    throw new SQLFeatureNotSupportedException("registerOutParameter not supported");
  }

  @Override
  public boolean wasNull() throws SQLException {
    throw new SQLFeatureNotSupportedException("wasNull not supported");
  }

  @Override
  public String getString(int parameterIndex) throws SQLException {
    throw new SQLFeatureNotSupportedException("getString not supported");
  }

  @Override
  public String getString(String parameterName) throws SQLException {
    throw new SQLFeatureNotSupportedException("getString not supported");
  }

  @Override
  public boolean getBoolean(int parameterIndex) throws SQLException {
    throw new SQLFeatureNotSupportedException("getBoolean not supported");
  }

  @Override
  public boolean getBoolean(String parameterName) throws SQLException {
    throw new SQLFeatureNotSupportedException("getBoolean not supported");
  }

  @Override
  public byte getByte(int parameterIndex) throws SQLException {
    throw new SQLFeatureNotSupportedException("getByte not supported");
  }

  @Override
  public byte getByte(String parameterName) throws SQLException {
    throw new SQLFeatureNotSupportedException("getByte not supported");
  }

  @Override
  public short getShort(int parameterIndex) throws SQLException {
    throw new SQLFeatureNotSupportedException("getShort not supported");
  }

  @Override
  public short getShort(String parameterName) throws SQLException {
    throw new SQLFeatureNotSupportedException("getShort not supported");
  }

  @Override
  public int getInt(int parameterIndex) throws SQLException {
    throw new SQLFeatureNotSupportedException("getInt not supported");
  }

  @Override
  public int getInt(String parameterName) throws SQLException {
    throw new SQLFeatureNotSupportedException("getInt not supported");
  }

  @Override
  public long getLong(int parameterIndex) throws SQLException {
    throw new SQLFeatureNotSupportedException("getLong not supported");
  }

  @Override
  public long getLong(String parameterName) throws SQLException {
    throw new SQLFeatureNotSupportedException("getLong not supported");
  }

  @Override
  public float getFloat(int parameterIndex) throws SQLException {
    throw new SQLFeatureNotSupportedException("getFloat not supported");
  }

  @Override
  public float getFloat(String parameterName) throws SQLException {
    throw new SQLFeatureNotSupportedException("getFloat not supported");
  }

  @Override
  public double getDouble(int parameterIndex) throws SQLException {
    throw new SQLFeatureNotSupportedException("getDouble not supported");
  }

  @Override
  public double getDouble(String parameterName) throws SQLException {
    throw new SQLFeatureNotSupportedException("getDouble not supported");
  }

  @Override
  @Deprecated
  public BigDecimal getBigDecimal(int parameterIndex, int scale) throws SQLException {
    throw new SQLFeatureNotSupportedException("getBigDecimal not supported");
  }

  @Override
  @Deprecated
  public BigDecimal getBigDecimal(String parameterName) throws SQLException {
    throw new SQLFeatureNotSupportedException("getBigDecimal not supported");
  }

  @Override
  public BigDecimal getBigDecimal(int parameterIndex) throws SQLException {
    throw new SQLFeatureNotSupportedException("getBigDecimal not supported");
  }

  @Override
  public byte[] getBytes(int parameterIndex) throws SQLException {
    throw new SQLFeatureNotSupportedException("getBytes not supported");
  }

  @Override
  public byte[] getBytes(String parameterName) throws SQLException {
    throw new SQLFeatureNotSupportedException("getBytes not supported");
  }

  @Override
  public Date getDate(int parameterIndex) throws SQLException {
    throw new SQLFeatureNotSupportedException("getDate not supported");
  }

  @Override
  public Date getDate(String parameterName) throws SQLException {
    throw new SQLFeatureNotSupportedException("getDate not supported");
  }

  @Override
  public Date getDate(int parameterIndex, Calendar cal) throws SQLException {
    throw new SQLFeatureNotSupportedException("getDate not supported");
  }

  @Override
  public Date getDate(String parameterName, Calendar cal) throws SQLException {
    throw new SQLFeatureNotSupportedException("getDate not supported");
  }

  @Override
  public Time getTime(int parameterIndex) throws SQLException {
    throw new SQLFeatureNotSupportedException("getTime not supported");
  }

  @Override
  public Time getTime(String parameterName) throws SQLException {
    throw new SQLFeatureNotSupportedException("getTime not supported");
  }

  @Override
  public Time getTime(int parameterIndex, Calendar cal) throws SQLException {
    throw new SQLFeatureNotSupportedException("getTime not supported");
  }

  @Override
  public Time getTime(String parameterName, Calendar cal) throws SQLException {
    throw new SQLFeatureNotSupportedException("getTime not supported");
  }

  @Override
  public Timestamp getTimestamp(int parameterIndex) throws SQLException {
    throw new SQLFeatureNotSupportedException("getTimestamp not supported");
  }

  @Override
  public Timestamp getTimestamp(String parameterName) throws SQLException {
    throw new SQLFeatureNotSupportedException("getTimestamp not supported");
  }

  @Override
  public Timestamp getTimestamp(int parameterIndex, Calendar cal) throws SQLException {
    throw new SQLFeatureNotSupportedException("getTimestamp not supported");
  }

  @Override
  public Timestamp getTimestamp(String parameterName, Calendar cal) throws SQLException {
    throw new SQLFeatureNotSupportedException("getTimestamp not supported");
  }

  @Override
  public Object getObject(int parameterIndex) throws SQLException {
    throw new SQLFeatureNotSupportedException("getObject not supported");
  }

  @Override
  public Object getObject(int parameterIndex, Map<String, Class<?>> map) throws SQLException {
    throw new SQLFeatureNotSupportedException("getObject not supported");
  }

  @Override
  public Object getObject(String parameterName) throws SQLException {
    throw new SQLFeatureNotSupportedException("getObject not supported");
  }

  @Override
  public Object getObject(String parameterName, Map<String, Class<?>> map) throws SQLException {
    throw new SQLFeatureNotSupportedException("getObject not supported");
  }

  @Override
  public <T> T getObject(int parameterIndex, Class<T> type) throws SQLException {
    throw new SQLFeatureNotSupportedException("getObject not supported");
  }

  @Override
  public <T> T getObject(String parameterName, Class<T> type) throws SQLException {
    throw new SQLFeatureNotSupportedException("getObject not supported");
  }

  @Override
  public Ref getRef(int parameterIndex) throws SQLException {
    throw new SQLFeatureNotSupportedException("getRef not supported");
  }

  @Override
  public Ref getRef(String parameterName) throws SQLException {
    throw new SQLFeatureNotSupportedException("getRef not supported");
  }

  @Override
  public Blob getBlob(int parameterIndex) throws SQLException {
    throw new SQLFeatureNotSupportedException("getBlob not supported");
  }

  @Override
  public Blob getBlob(String parameterName) throws SQLException {
    throw new SQLFeatureNotSupportedException("getBlob not supported");
  }

  @Override
  public Clob getClob(int parameterIndex) throws SQLException {
    throw new SQLFeatureNotSupportedException("getClob not supported");
  }

  @Override
  public Clob getClob(String parameterName) throws SQLException {
    throw new SQLFeatureNotSupportedException("getClob not supported");
  }

  @Override
  public Array getArray(int parameterIndex) throws SQLException {
    throw new SQLFeatureNotSupportedException("getArray not supported");
  }

  @Override
  public Array getArray(String parameterName) throws SQLException {
    throw new SQLFeatureNotSupportedException("getArray not supported");
  }

  @Override
  public URL getURL(int parameterIndex) throws SQLException {
    throw new SQLFeatureNotSupportedException("getURL not supported");
  }

  @Override
  public URL getURL(String parameterName) throws SQLException {
    throw new SQLFeatureNotSupportedException("getURL not supported");
  }

  @Override
  public RowId getRowId(int parameterIndex) throws SQLException {
    throw new SQLFeatureNotSupportedException("getRowId not supported");
  }

  @Override
  public RowId getRowId(String parameterName) throws SQLException {
    throw new SQLFeatureNotSupportedException("getRowId not supported");
  }

  @Override
  public NClob getNClob(int parameterIndex) throws SQLException {
    throw new SQLFeatureNotSupportedException("getNClob not supported");
  }

  @Override
  public NClob getNClob(String parameterName) throws SQLException {
    throw new SQLFeatureNotSupportedException("getNClob not supported");
  }

  @Override
  public SQLXML getSQLXML(int parameterIndex) throws SQLException {
    throw new SQLFeatureNotSupportedException("getSQLXML not supported");
  }

  @Override
  public SQLXML getSQLXML(String parameterName) throws SQLException {
    throw new SQLFeatureNotSupportedException("getSQLXML not supported");
  }

  @Override
  public String getNString(int parameterIndex) throws SQLException {
    throw new SQLFeatureNotSupportedException("getNString not supported");
  }

  @Override
  public String getNString(String parameterName) throws SQLException {
    throw new SQLFeatureNotSupportedException("getNString not supported");
  }

  @Override
  public Reader getNCharacterStream(int parameterIndex) throws SQLException {
    throw new SQLFeatureNotSupportedException("getNCharacterStream not supported");
  }

  @Override
  public Reader getNCharacterStream(String parameterName) throws SQLException {
    throw new SQLFeatureNotSupportedException("getNCharacterStream not supported");
  }

  @Override
  public Reader getCharacterStream(int parameterIndex) throws SQLException {
    throw new SQLFeatureNotSupportedException("getCharacterStream not supported");
  }

  @Override
  public Reader getCharacterStream(String parameterName) throws SQLException {
    throw new SQLFeatureNotSupportedException("getCharacterStream not supported");
  }

  /*
   * JDBC does not store parameter names, only parameter indices. Name-based setters are therefore
   * not supported.
   */

  @Override
  public void setSQLXML(String parameterName, SQLXML xmlObject) throws SQLException {
    throw new SQLFeatureNotSupportedException("setSQLXML by name not supported");
  }

  @Override
  public void setRowId(String parameterName, RowId x) throws SQLException {
    throw new SQLFeatureNotSupportedException("setRowId by name not supported");
  }

  @Override
  public void setNString(String parameterName, String value) throws SQLException {
    throw new SQLFeatureNotSupportedException("setNString by name not supported");
  }

  @Override
  public void setNCharacterStream(String parameterName, Reader value) throws SQLException {
    throw new SQLFeatureNotSupportedException("setNCharacterStream by name not supported");
  }

  @Override
  public void setNCharacterStream(String parameterName, Reader value, long length)
      throws SQLException {
    throw new SQLFeatureNotSupportedException("setNCharacterStream by name not supported");
  }

  @Override
  public void setNClob(String parameterName, NClob value) throws SQLException {
    throw new SQLFeatureNotSupportedException("setNClob by name not supported");
  }

  @Override
  public void setNClob(String parameterName, Reader reader) throws SQLException {
    throw new SQLFeatureNotSupportedException("setNClob by name not supported");
  }

  @Override
  public void setNClob(String parameterName, Reader reader, long length) throws SQLException {
    throw new SQLFeatureNotSupportedException("setNClob by name not supported");
  }

  @Override
  public void setClob(String parameterName, Clob x) throws SQLException {
    throw new SQLFeatureNotSupportedException("setClob by name not supported");
  }

  @Override
  public void setClob(String parameterName, Reader reader) throws SQLException {
    throw new SQLFeatureNotSupportedException("setClob by name not supported");
  }

  @Override
  public void setClob(String parameterName, Reader reader, long length) throws SQLException {
    throw new SQLFeatureNotSupportedException("setClob by name not supported");
  }

  @Override
  public void setBlob(String parameterName, Blob x) throws SQLException {
    throw new SQLFeatureNotSupportedException("setBlob by name not supported");
  }

  @Override
  public void setBlob(String parameterName, InputStream inputStream) throws SQLException {
    throw new SQLFeatureNotSupportedException("setBlob by name not supported");
  }

  @Override
  public void setBlob(String parameterName, InputStream inputStream, long length)
      throws SQLException {
    throw new SQLFeatureNotSupportedException("setBlob by name not supported");
  }

  @Override
  public void setURL(String parameterName, URL val) throws SQLException {
    throw new SQLFeatureNotSupportedException("setURL by name not supported");
  }

  @Override
  public void setNull(String parameterName, int sqlType) throws SQLException {
    throw new SQLFeatureNotSupportedException("setNull by name not supported");
  }

  @Override
  public void setNull(String parameterName, int sqlType, String typeName) throws SQLException {
    throw new SQLFeatureNotSupportedException("setNull by name not supported");
  }

  @Override
  public void setBoolean(String parameterName, boolean x) throws SQLException {
    throw new SQLFeatureNotSupportedException("setBoolean by name not supported");
  }

  @Override
  public void setByte(String parameterName, byte x) throws SQLException {
    throw new SQLFeatureNotSupportedException("setByte by name not supported");
  }

  @Override
  public void setShort(String parameterName, short x) throws SQLException {
    throw new SQLFeatureNotSupportedException("setShort by name not supported");
  }

  @Override
  public void setInt(String parameterName, int x) throws SQLException {
    throw new SQLFeatureNotSupportedException("setInt by name not supported");
  }

  @Override
  public void setLong(String parameterName, long x) throws SQLException {
    throw new SQLFeatureNotSupportedException("setLong by name not supported");
  }

  @Override
  public void setFloat(String parameterName, float x) throws SQLException {
    throw new SQLFeatureNotSupportedException("setFloat by name not supported");
  }

  @Override
  public void setDouble(String parameterName, double x) throws SQLException {
    throw new SQLFeatureNotSupportedException("setDouble by name not supported");
  }

  @Override
  public void setBigDecimal(String parameterName, BigDecimal x) throws SQLException {
    throw new SQLFeatureNotSupportedException("setBigDecimal by name not supported");
  }

  @Override
  public void setString(String parameterName, String x) throws SQLException {
    throw new SQLFeatureNotSupportedException("setString by name not supported");
  }

  @Override
  public void setBytes(String parameterName, byte[] x) throws SQLException {
    throw new SQLFeatureNotSupportedException("setBytes by name not supported");
  }

  @Override
  public void setDate(String parameterName, Date x) throws SQLException {
    throw new SQLFeatureNotSupportedException("setDate by name not supported");
  }

  @Override
  public void setDate(String parameterName, Date x, Calendar cal) throws SQLException {
    throw new SQLFeatureNotSupportedException("setDate by name not supported");
  }

  @Override
  public void setTime(String parameterName, Time x) throws SQLException {
    throw new SQLFeatureNotSupportedException("setTime by name not supported");
  }

  @Override
  public void setTime(String parameterName, Time x, Calendar cal) throws SQLException {
    throw new SQLFeatureNotSupportedException("setTime by name not supported");
  }

  @Override
  public void setTimestamp(String parameterName, Timestamp x) throws SQLException {
    throw new SQLFeatureNotSupportedException("setTimestamp by name not supported");
  }

  @Override
  public void setTimestamp(String parameterName, Timestamp x, Calendar cal) throws SQLException {
    throw new SQLFeatureNotSupportedException("setTimestamp by name not supported");
  }

  @Override
  public void setAsciiStream(String parameterName, InputStream x) throws SQLException {
    throw new SQLFeatureNotSupportedException("setAsciiStream by name not supported");
  }

  @Override
  public void setAsciiStream(String parameterName, InputStream x, int length) throws SQLException {
    throw new SQLFeatureNotSupportedException("setAsciiStream by name not supported");
  }

  @Override
  public void setAsciiStream(String parameterName, InputStream x, long length) throws SQLException {
    throw new SQLFeatureNotSupportedException("setAsciiStream by name not supported");
  }

  @Override
  public void setBinaryStream(String parameterName, InputStream x) throws SQLException {
    throw new SQLFeatureNotSupportedException("setBinaryStream by name not supported");
  }

  @Override
  public void setBinaryStream(String parameterName, InputStream x, int length) throws SQLException {
    throw new SQLFeatureNotSupportedException("setBinaryStream by name not supported");
  }

  @Override
  public void setBinaryStream(String parameterName, InputStream x, long length)
      throws SQLException {
    throw new SQLFeatureNotSupportedException("setBinaryStream by name not supported");
  }

  @Override
  public void setObject(String parameterName, Object x, int targetSqlType, int scale)
      throws SQLException {
    throw new SQLFeatureNotSupportedException("setObject by name not supported");
  }

  @Override
  public void setObject(String parameterName, Object x, int targetSqlType) throws SQLException {
    throw new SQLFeatureNotSupportedException("setObject by name not supported");
  }

  @Override
  public void setObject(String parameterName, Object x) throws SQLException {
    throw new SQLFeatureNotSupportedException("setObject by name not supported");
  }

  @Override
  public void setCharacterStream(String parameterName, Reader reader, int length)
      throws SQLException {
    throw new SQLFeatureNotSupportedException("setCharacterStream by name not supported");
  }

  @Override
  public void setCharacterStream(String parameterName, Reader reader, long length)
      throws SQLException {
    throw new SQLFeatureNotSupportedException("setCharacterStream by name not supported");
  }

  @Override
  public void setCharacterStream(String parameterName, Reader reader) throws SQLException {
    throw new SQLFeatureNotSupportedException("setCharacterStream by name not supported");
  }
}
