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
import java.sql.SQLXML;
import java.sql.Time;
import java.sql.Timestamp;
import java.util.Calendar;
import java.util.Map;
import net.snowflake.client.internal.api.implementation.connection.InternalSnowflakeConnection;
import net.snowflake.client.internal.api.implementation.exception.SFSQLFeatureNotSupportedException;
import net.snowflake.client.internal.codegen.JdbcBoundary;
import net.snowflake.client.internal.log.SFLogger;
import net.snowflake.client.internal.log.SFLoggerFactory;
import net.snowflake.client.internal.unicore.CoreDriverApi;

@JdbcBoundary
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
      logger.debug("Curly brackets {} removed before sending sql to server.");
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
  public void registerOutParameter(int parameterIndex, int sqlType) {
    throw new SFSQLFeatureNotSupportedException("registerOutParameter not supported");
  }

  @Override
  public void registerOutParameter(int parameterIndex, int sqlType, int scale) {
    throw new SFSQLFeatureNotSupportedException("registerOutParameter not supported");
  }

  @Override
  public void registerOutParameter(int parameterIndex, int sqlType, String typeName) {
    throw new SFSQLFeatureNotSupportedException("registerOutParameter not supported");
  }

  @Override
  public void registerOutParameter(String parameterName, int sqlType) {
    throw new SFSQLFeatureNotSupportedException("registerOutParameter not supported");
  }

  @Override
  public void registerOutParameter(String parameterName, int sqlType, int scale) {
    throw new SFSQLFeatureNotSupportedException("registerOutParameter not supported");
  }

  @Override
  public void registerOutParameter(String parameterName, int sqlType, String typeName) {
    throw new SFSQLFeatureNotSupportedException("registerOutParameter not supported");
  }

  @Override
  public boolean wasNull() {
    throw new SFSQLFeatureNotSupportedException("wasNull not supported");
  }

  @Override
  public String getString(int parameterIndex) {
    throw new SFSQLFeatureNotSupportedException("getString not supported");
  }

  @Override
  public String getString(String parameterName) {
    throw new SFSQLFeatureNotSupportedException("getString not supported");
  }

  @Override
  public boolean getBoolean(int parameterIndex) {
    throw new SFSQLFeatureNotSupportedException("getBoolean not supported");
  }

  @Override
  public boolean getBoolean(String parameterName) {
    throw new SFSQLFeatureNotSupportedException("getBoolean not supported");
  }

  @Override
  public byte getByte(int parameterIndex) {
    throw new SFSQLFeatureNotSupportedException("getByte not supported");
  }

  @Override
  public byte getByte(String parameterName) {
    throw new SFSQLFeatureNotSupportedException("getByte not supported");
  }

  @Override
  public short getShort(int parameterIndex) {
    throw new SFSQLFeatureNotSupportedException("getShort not supported");
  }

  @Override
  public short getShort(String parameterName) {
    throw new SFSQLFeatureNotSupportedException("getShort not supported");
  }

  @Override
  public int getInt(int parameterIndex) {
    throw new SFSQLFeatureNotSupportedException("getInt not supported");
  }

  @Override
  public int getInt(String parameterName) {
    throw new SFSQLFeatureNotSupportedException("getInt not supported");
  }

  @Override
  public long getLong(int parameterIndex) {
    throw new SFSQLFeatureNotSupportedException("getLong not supported");
  }

  @Override
  public long getLong(String parameterName) {
    throw new SFSQLFeatureNotSupportedException("getLong not supported");
  }

  @Override
  public float getFloat(int parameterIndex) {
    throw new SFSQLFeatureNotSupportedException("getFloat not supported");
  }

  @Override
  public float getFloat(String parameterName) {
    throw new SFSQLFeatureNotSupportedException("getFloat not supported");
  }

  @Override
  public double getDouble(int parameterIndex) {
    throw new SFSQLFeatureNotSupportedException("getDouble not supported");
  }

  @Override
  public double getDouble(String parameterName) {
    throw new SFSQLFeatureNotSupportedException("getDouble not supported");
  }

  @Override
  @Deprecated
  public BigDecimal getBigDecimal(int parameterIndex, int scale) {
    throw new SFSQLFeatureNotSupportedException("getBigDecimal not supported");
  }

  @Override
  @Deprecated
  public BigDecimal getBigDecimal(String parameterName) {
    throw new SFSQLFeatureNotSupportedException("getBigDecimal not supported");
  }

  @Override
  public BigDecimal getBigDecimal(int parameterIndex) {
    throw new SFSQLFeatureNotSupportedException("getBigDecimal not supported");
  }

  @Override
  public byte[] getBytes(int parameterIndex) {
    throw new SFSQLFeatureNotSupportedException("getBytes not supported");
  }

  @Override
  public byte[] getBytes(String parameterName) {
    throw new SFSQLFeatureNotSupportedException("getBytes not supported");
  }

  @Override
  public Date getDate(int parameterIndex) {
    throw new SFSQLFeatureNotSupportedException("getDate not supported");
  }

  @Override
  public Date getDate(String parameterName) {
    throw new SFSQLFeatureNotSupportedException("getDate not supported");
  }

  @Override
  public Date getDate(int parameterIndex, Calendar cal) {
    throw new SFSQLFeatureNotSupportedException("getDate not supported");
  }

  @Override
  public Date getDate(String parameterName, Calendar cal) {
    throw new SFSQLFeatureNotSupportedException("getDate not supported");
  }

  @Override
  public Time getTime(int parameterIndex) {
    throw new SFSQLFeatureNotSupportedException("getTime not supported");
  }

  @Override
  public Time getTime(String parameterName) {
    throw new SFSQLFeatureNotSupportedException("getTime not supported");
  }

  @Override
  public Time getTime(int parameterIndex, Calendar cal) {
    throw new SFSQLFeatureNotSupportedException("getTime not supported");
  }

  @Override
  public Time getTime(String parameterName, Calendar cal) {
    throw new SFSQLFeatureNotSupportedException("getTime not supported");
  }

  @Override
  public Timestamp getTimestamp(int parameterIndex) {
    throw new SFSQLFeatureNotSupportedException("getTimestamp not supported");
  }

  @Override
  public Timestamp getTimestamp(String parameterName) {
    throw new SFSQLFeatureNotSupportedException("getTimestamp not supported");
  }

  @Override
  public Timestamp getTimestamp(int parameterIndex, Calendar cal) {
    throw new SFSQLFeatureNotSupportedException("getTimestamp not supported");
  }

  @Override
  public Timestamp getTimestamp(String parameterName, Calendar cal) {
    throw new SFSQLFeatureNotSupportedException("getTimestamp not supported");
  }

  @Override
  public Object getObject(int parameterIndex) {
    throw new SFSQLFeatureNotSupportedException("getObject not supported");
  }

  @Override
  public Object getObject(int parameterIndex, Map<String, Class<?>> map) {
    throw new SFSQLFeatureNotSupportedException("getObject not supported");
  }

  @Override
  public Object getObject(String parameterName) {
    throw new SFSQLFeatureNotSupportedException("getObject not supported");
  }

  @Override
  public Object getObject(String parameterName, Map<String, Class<?>> map) {
    throw new SFSQLFeatureNotSupportedException("getObject not supported");
  }

  @Override
  public <T> T getObject(int parameterIndex, Class<T> type) {
    throw new SFSQLFeatureNotSupportedException("getObject not supported");
  }

  @Override
  public <T> T getObject(String parameterName, Class<T> type) {
    throw new SFSQLFeatureNotSupportedException("getObject not supported");
  }

  @Override
  public Ref getRef(int parameterIndex) {
    throw new SFSQLFeatureNotSupportedException("getRef not supported");
  }

  @Override
  public Ref getRef(String parameterName) {
    throw new SFSQLFeatureNotSupportedException("getRef not supported");
  }

  @Override
  public Blob getBlob(int parameterIndex) {
    throw new SFSQLFeatureNotSupportedException("getBlob not supported");
  }

  @Override
  public Blob getBlob(String parameterName) {
    throw new SFSQLFeatureNotSupportedException("getBlob not supported");
  }

  @Override
  public Clob getClob(int parameterIndex) {
    throw new SFSQLFeatureNotSupportedException("getClob not supported");
  }

  @Override
  public Clob getClob(String parameterName) {
    throw new SFSQLFeatureNotSupportedException("getClob not supported");
  }

  @Override
  public Array getArray(int parameterIndex) {
    throw new SFSQLFeatureNotSupportedException("getArray not supported");
  }

  @Override
  public Array getArray(String parameterName) {
    throw new SFSQLFeatureNotSupportedException("getArray not supported");
  }

  @Override
  public URL getURL(int parameterIndex) {
    throw new SFSQLFeatureNotSupportedException("getURL not supported");
  }

  @Override
  public URL getURL(String parameterName) {
    throw new SFSQLFeatureNotSupportedException("getURL not supported");
  }

  @Override
  public RowId getRowId(int parameterIndex) {
    throw new SFSQLFeatureNotSupportedException("getRowId not supported");
  }

  @Override
  public RowId getRowId(String parameterName) {
    throw new SFSQLFeatureNotSupportedException("getRowId not supported");
  }

  @Override
  public NClob getNClob(int parameterIndex) {
    throw new SFSQLFeatureNotSupportedException("getNClob not supported");
  }

  @Override
  public NClob getNClob(String parameterName) {
    throw new SFSQLFeatureNotSupportedException("getNClob not supported");
  }

  @Override
  public SQLXML getSQLXML(int parameterIndex) {
    throw new SFSQLFeatureNotSupportedException("getSQLXML not supported");
  }

  @Override
  public SQLXML getSQLXML(String parameterName) {
    throw new SFSQLFeatureNotSupportedException("getSQLXML not supported");
  }

  @Override
  public String getNString(int parameterIndex) {
    throw new SFSQLFeatureNotSupportedException("getNString not supported");
  }

  @Override
  public String getNString(String parameterName) {
    throw new SFSQLFeatureNotSupportedException("getNString not supported");
  }

  @Override
  public Reader getNCharacterStream(int parameterIndex) {
    throw new SFSQLFeatureNotSupportedException("getNCharacterStream not supported");
  }

  @Override
  public Reader getNCharacterStream(String parameterName) {
    throw new SFSQLFeatureNotSupportedException("getNCharacterStream not supported");
  }

  @Override
  public Reader getCharacterStream(int parameterIndex) {
    throw new SFSQLFeatureNotSupportedException("getCharacterStream not supported");
  }

  @Override
  public Reader getCharacterStream(String parameterName) {
    throw new SFSQLFeatureNotSupportedException("getCharacterStream not supported");
  }

  /*
   * JDBC does not store parameter names, only parameter indices. Name-based setters are therefore
   * not supported.
   */

  @Override
  public void setSQLXML(String parameterName, SQLXML xmlObject) {
    throw new SFSQLFeatureNotSupportedException("setSQLXML by name not supported");
  }

  @Override
  public void setRowId(String parameterName, RowId x) {
    throw new SFSQLFeatureNotSupportedException("setRowId by name not supported");
  }

  @Override
  public void setNString(String parameterName, String value) {
    throw new SFSQLFeatureNotSupportedException("setNString by name not supported");
  }

  @Override
  public void setNCharacterStream(String parameterName, Reader value) {
    throw new SFSQLFeatureNotSupportedException("setNCharacterStream by name not supported");
  }

  @Override
  public void setNCharacterStream(String parameterName, Reader value, long length) {
    throw new SFSQLFeatureNotSupportedException("setNCharacterStream by name not supported");
  }

  @Override
  public void setNClob(String parameterName, NClob value) {
    throw new SFSQLFeatureNotSupportedException("setNClob by name not supported");
  }

  @Override
  public void setNClob(String parameterName, Reader reader) {
    throw new SFSQLFeatureNotSupportedException("setNClob by name not supported");
  }

  @Override
  public void setNClob(String parameterName, Reader reader, long length) {
    throw new SFSQLFeatureNotSupportedException("setNClob by name not supported");
  }

  @Override
  public void setClob(String parameterName, Clob x) {
    throw new SFSQLFeatureNotSupportedException("setClob by name not supported");
  }

  @Override
  public void setClob(String parameterName, Reader reader) {
    throw new SFSQLFeatureNotSupportedException("setClob by name not supported");
  }

  @Override
  public void setClob(String parameterName, Reader reader, long length) {
    throw new SFSQLFeatureNotSupportedException("setClob by name not supported");
  }

  @Override
  public void setBlob(String parameterName, Blob x) {
    throw new SFSQLFeatureNotSupportedException("setBlob by name not supported");
  }

  @Override
  public void setBlob(String parameterName, InputStream inputStream) {
    throw new SFSQLFeatureNotSupportedException("setBlob by name not supported");
  }

  @Override
  public void setBlob(String parameterName, InputStream inputStream, long length) {
    throw new SFSQLFeatureNotSupportedException("setBlob by name not supported");
  }

  @Override
  public void setURL(String parameterName, URL val) {
    throw new SFSQLFeatureNotSupportedException("setURL by name not supported");
  }

  @Override
  public void setNull(String parameterName, int sqlType) {
    throw new SFSQLFeatureNotSupportedException("setNull by name not supported");
  }

  @Override
  public void setNull(String parameterName, int sqlType, String typeName) {
    throw new SFSQLFeatureNotSupportedException("setNull by name not supported");
  }

  @Override
  public void setBoolean(String parameterName, boolean x) {
    throw new SFSQLFeatureNotSupportedException("setBoolean by name not supported");
  }

  @Override
  public void setByte(String parameterName, byte x) {
    throw new SFSQLFeatureNotSupportedException("setByte by name not supported");
  }

  @Override
  public void setShort(String parameterName, short x) {
    throw new SFSQLFeatureNotSupportedException("setShort by name not supported");
  }

  @Override
  public void setInt(String parameterName, int x) {
    throw new SFSQLFeatureNotSupportedException("setInt by name not supported");
  }

  @Override
  public void setLong(String parameterName, long x) {
    throw new SFSQLFeatureNotSupportedException("setLong by name not supported");
  }

  @Override
  public void setFloat(String parameterName, float x) {
    throw new SFSQLFeatureNotSupportedException("setFloat by name not supported");
  }

  @Override
  public void setDouble(String parameterName, double x) {
    throw new SFSQLFeatureNotSupportedException("setDouble by name not supported");
  }

  @Override
  public void setBigDecimal(String parameterName, BigDecimal x) {
    throw new SFSQLFeatureNotSupportedException("setBigDecimal by name not supported");
  }

  @Override
  public void setString(String parameterName, String x) {
    throw new SFSQLFeatureNotSupportedException("setString by name not supported");
  }

  @Override
  public void setBytes(String parameterName, byte[] x) {
    throw new SFSQLFeatureNotSupportedException("setBytes by name not supported");
  }

  @Override
  public void setDate(String parameterName, Date x) {
    throw new SFSQLFeatureNotSupportedException("setDate by name not supported");
  }

  @Override
  public void setDate(String parameterName, Date x, Calendar cal) {
    throw new SFSQLFeatureNotSupportedException("setDate by name not supported");
  }

  @Override
  public void setTime(String parameterName, Time x) {
    throw new SFSQLFeatureNotSupportedException("setTime by name not supported");
  }

  @Override
  public void setTime(String parameterName, Time x, Calendar cal) {
    throw new SFSQLFeatureNotSupportedException("setTime by name not supported");
  }

  @Override
  public void setTimestamp(String parameterName, Timestamp x) {
    throw new SFSQLFeatureNotSupportedException("setTimestamp by name not supported");
  }

  @Override
  public void setTimestamp(String parameterName, Timestamp x, Calendar cal) {
    throw new SFSQLFeatureNotSupportedException("setTimestamp by name not supported");
  }

  @Override
  public void setAsciiStream(String parameterName, InputStream x) {
    throw new SFSQLFeatureNotSupportedException("setAsciiStream by name not supported");
  }

  @Override
  public void setAsciiStream(String parameterName, InputStream x, int length) {
    throw new SFSQLFeatureNotSupportedException("setAsciiStream by name not supported");
  }

  @Override
  public void setAsciiStream(String parameterName, InputStream x, long length) {
    throw new SFSQLFeatureNotSupportedException("setAsciiStream by name not supported");
  }

  @Override
  public void setBinaryStream(String parameterName, InputStream x) {
    throw new SFSQLFeatureNotSupportedException("setBinaryStream by name not supported");
  }

  @Override
  public void setBinaryStream(String parameterName, InputStream x, int length) {
    throw new SFSQLFeatureNotSupportedException("setBinaryStream by name not supported");
  }

  @Override
  public void setBinaryStream(String parameterName, InputStream x, long length) {
    throw new SFSQLFeatureNotSupportedException("setBinaryStream by name not supported");
  }

  @Override
  public void setObject(String parameterName, Object x, int targetSqlType, int scale) {
    throw new SFSQLFeatureNotSupportedException("setObject by name not supported");
  }

  @Override
  public void setObject(String parameterName, Object x, int targetSqlType) {
    throw new SFSQLFeatureNotSupportedException("setObject by name not supported");
  }

  @Override
  public void setObject(String parameterName, Object x) {
    throw new SFSQLFeatureNotSupportedException("setObject by name not supported");
  }

  @Override
  public void setCharacterStream(String parameterName, Reader reader, int length) {
    throw new SFSQLFeatureNotSupportedException("setCharacterStream by name not supported");
  }

  @Override
  public void setCharacterStream(String parameterName, Reader reader, long length) {
    throw new SFSQLFeatureNotSupportedException("setCharacterStream by name not supported");
  }

  @Override
  public void setCharacterStream(String parameterName, Reader reader) {
    throw new SFSQLFeatureNotSupportedException("setCharacterStream by name not supported");
  }
}
