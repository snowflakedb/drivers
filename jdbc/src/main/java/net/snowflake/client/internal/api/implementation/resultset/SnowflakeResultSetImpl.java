package net.snowflake.client.internal.api.implementation.resultset;

import java.io.InputStream;
import java.io.Reader;
import java.io.StringReader;
import java.math.BigDecimal;
import java.math.RoundingMode;
import java.net.URL;
import java.sql.Array;
import java.sql.Blob;
import java.sql.Clob;
import java.sql.Date;
import java.sql.NClob;
import java.sql.Ref;
import java.sql.ResultSetMetaData;
import java.sql.RowId;
import java.sql.SQLWarning;
import java.sql.SQLXML;
import java.sql.Statement;
import java.sql.Time;
import java.sql.Timestamp;
import java.time.Duration;
import java.time.Period;
import java.util.Calendar;
import java.util.List;
import java.util.Map;
import java.util.TimeZone;
import lombok.AccessLevel;
import lombok.RequiredArgsConstructor;
import net.snowflake.client.api.resultset.SnowflakeResultSetSerializable;
import net.snowflake.client.api.resultset.SnowflakeType;
import net.snowflake.client.internal.api.decorator.Telemetry;
import net.snowflake.client.internal.api.implementation.Decorators;
import net.snowflake.client.internal.api.implementation.connection.SnowflakeClob;
import net.snowflake.client.internal.api.implementation.exception.CoreException;
import net.snowflake.client.internal.api.implementation.exception.SFSQLFeatureNotSupportedException;
import net.snowflake.client.internal.api.implementation.resultset.metadata.DecoratedSnowflakeResultSetMetaDataImpl;
import net.snowflake.client.internal.api.implementation.resultset.metadata.SnowflakeResultSetMetaDataImpl;
import net.snowflake.client.internal.api.implementation.statement.SnowflakeStatementImpl;
import net.snowflake.client.internal.codegen.JdbcBoundary;
import net.snowflake.client.internal.codegen.NoTelemetry;
import net.snowflake.client.internal.util.DelegatingWrapper;
import net.snowflake.client.internal.util.NotImplementedException;

@JdbcBoundary
@RequiredArgsConstructor(access = AccessLevel.PACKAGE)
public class SnowflakeResultSetImpl implements InternalResultSet, DelegatingWrapper {

  private final SnowflakeStatementImpl statement;
  private final String queryId;
  private final RowReader rowReader;
  private final SnowflakeResultSetMetaDataImpl resultSetMetaData;
  private final boolean ownsStatement;
  private final ResultSetChunksProvider resultSetChunksProvider;

  private boolean closed = false;
  private int fetchSize = 0;
  private int fetchDirection = FETCH_FORWARD;

  SnowflakeResultSetImpl(
      SnowflakeStatementImpl statement,
      String queryId,
      RowReader rowReader,
      SnowflakeResultSetMetaDataImpl resultSetMetaData,
      boolean ownsStatement) {
    this(statement, queryId, rowReader, resultSetMetaData, ownsStatement, null);
  }

  @Override
  @NoTelemetry
  public boolean next() {
    if (closed) {
      return false;
    }
    return rowReader.next();
  }

  @Override
  public void close() {
    if (closed) {
      return;
    }
    try {
      rowReader.close();
    } finally {
      if (resultSetChunksProvider != null) {
        resultSetChunksProvider.release();
      }
      closed = true;
      if (statement != null) {
        statement.removeClosedResultSet(this);
      }
    }

    if (ownsStatement && statement != null) {
      try {
        if (!statement.isClosed()) {
          statement.close();
        }
      } catch (CoreException ignored) {
        // closing the owning statement is best-effort
      }
    }
  }

  @Override
  @NoTelemetry
  public boolean wasNull() {
    checkClosed();
    return rowReader.wasNull();
  }

  @Override
  @NoTelemetry
  public String getString(int columnIndex) {
    checkClosed();
    return rowReader.getString(columnIndex);
  }

  @Override
  @NoTelemetry
  public boolean getBoolean(int columnIndex) {
    checkClosed();
    return rowReader.getBoolean(columnIndex);
  }

  @Override
  @NoTelemetry
  public byte getByte(int columnIndex) {
    checkClosed();
    return rowReader.getByte(columnIndex);
  }

  @Override
  @NoTelemetry
  public short getShort(int columnIndex) {
    checkClosed();
    return rowReader.getShort(columnIndex);
  }

  @Override
  @NoTelemetry
  public int getInt(int columnIndex) {
    checkClosed();
    return rowReader.getInt(columnIndex);
  }

  @Override
  @NoTelemetry
  public long getLong(int columnIndex) {
    checkClosed();
    return rowReader.getLong(columnIndex);
  }

  @Override
  @NoTelemetry
  public float getFloat(int columnIndex) {
    checkClosed();
    return rowReader.getFloat(columnIndex);
  }

  @Override
  @NoTelemetry
  public double getDouble(int columnIndex) {
    checkClosed();
    return rowReader.getDouble(columnIndex);
  }

  @Override
  @NoTelemetry
  public BigDecimal getBigDecimal(int columnIndex, int scale) {
    BigDecimal value = getBigDecimal(columnIndex);
    if (value == null) {
      return null;
    }
    return value.setScale(scale, RoundingMode.HALF_UP);
  }

  @Override
  @NoTelemetry
  public byte[] getBytes(int columnIndex) {
    checkClosed();
    return rowReader.getBytes(columnIndex);
  }

  @Override
  @NoTelemetry
  public Date getDate(int columnIndex) {
    checkClosed();
    // Mirrors snowflake-jdbc's SnowflakeBaseResultSet.getDate(int): JDBC_GET_DATE_USE_NULL_TIMEZONE
    // (default true) selects a null timezone (raw epoch-day date); when false the JVM default
    // timezone is used, which the converter shifts only if JDBC_FORMAT_DATE_WITH_TIMEZONE is set.
    TimeZone tz =
        rowReader.getConversionContext().isGetDateUseNullTimezone() ? null : TimeZone.getDefault();
    return getDate(columnIndex, tz);
  }

  /**
   * Shared DATE materialization mirroring snowflake-jdbc's {@code SFArrowResultSet.getDate(int,
   * TimeZone)}: the caller timezone and the runtime {@code JDBC_FORMAT_DATE_WITH_TIMEZONE} flag are
   * threaded into the converter, which applies the session-vs-caller timezone shift only when both
   * are present and the flag is set.
   */
  private Date getDate(int columnIndex, TimeZone tz) {
    checkClosed();
    return rowReader.getDate(columnIndex, tz);
  }

  @Override
  @NoTelemetry
  public Time getTime(int columnIndex) {
    checkClosed();
    return rowReader.getTime(columnIndex);
  }

  @Override
  @NoTelemetry
  public Timestamp getTimestamp(int columnIndex) {
    checkClosed();
    return rowReader.getTimestamp(columnIndex);
  }

  /** Backs {@code getObject(col, Period.class)} for INTERVAL YEAR TO MONTH columns. */
  private Period getPeriod(int columnIndex) {
    checkClosed();
    return rowReader.getPeriod(columnIndex);
  }

  /** Backs {@code getObject(col, Duration.class)} for INTERVAL DAY TO SECOND columns. */
  private Duration getDuration(int columnIndex) {
    checkClosed();
    return rowReader.getDuration(columnIndex);
  }

  @Override
  public InputStream getAsciiStream(int columnIndex) {
    throw new SFSQLFeatureNotSupportedException("getAsciiStream not supported");
  }

  @Override
  public InputStream getUnicodeStream(int columnIndex) {
    throw new SFSQLFeatureNotSupportedException("getUnicodeStream not supported");
  }

  @Override
  public InputStream getBinaryStream(int columnIndex) {
    throw new SFSQLFeatureNotSupportedException("getBinaryStream not supported");
  }

  // String-based column access
  @Override
  @NoTelemetry
  public String getString(String columnLabel) {
    return getString(findColumn(columnLabel));
  }

  @Override
  @NoTelemetry
  public boolean getBoolean(String columnLabel) {
    return getBoolean(findColumn(columnLabel));
  }

  @Override
  @NoTelemetry
  public byte getByte(String columnLabel) {
    return getByte(findColumn(columnLabel));
  }

  @Override
  @NoTelemetry
  public short getShort(String columnLabel) {
    return getShort(findColumn(columnLabel));
  }

  @Override
  @NoTelemetry
  public int getInt(String columnLabel) {
    return getInt(findColumn(columnLabel));
  }

  @Override
  @NoTelemetry
  public long getLong(String columnLabel) {
    return getLong(findColumn(columnLabel));
  }

  @Override
  @NoTelemetry
  public float getFloat(String columnLabel) {
    return getFloat(findColumn(columnLabel));
  }

  @Override
  @NoTelemetry
  public double getDouble(String columnLabel) {
    return getDouble(findColumn(columnLabel));
  }

  @Override
  @NoTelemetry
  public BigDecimal getBigDecimal(String columnLabel, int scale) {
    return getBigDecimal(findColumn(columnLabel), scale);
  }

  @Override
  @NoTelemetry
  public byte[] getBytes(String columnLabel) {
    return getBytes(findColumn(columnLabel));
  }

  @Override
  @NoTelemetry
  public Date getDate(String columnLabel) {
    return getDate(findColumn(columnLabel));
  }

  @Override
  @NoTelemetry
  public Time getTime(String columnLabel) {
    return getTime(findColumn(columnLabel));
  }

  @Override
  @NoTelemetry
  public Timestamp getTimestamp(String columnLabel) {
    return getTimestamp(findColumn(columnLabel));
  }

  @Override
  public InputStream getAsciiStream(String columnLabel) {
    throw new SFSQLFeatureNotSupportedException("getAsciiStream not supported");
  }

  @Override
  public InputStream getUnicodeStream(String columnLabel) {
    throw new SFSQLFeatureNotSupportedException("getUnicodeStream not supported");
  }

  @Override
  public InputStream getBinaryStream(String columnLabel) {
    throw new SFSQLFeatureNotSupportedException("getAsciiStream not supported");
  }

  @Override
  public SQLWarning getWarnings() {
    checkClosed();
    return null;
  }

  @Override
  public void clearWarnings() {
    checkClosed();
  }

  @Override
  public String getCursorName() {
    throw new SFSQLFeatureNotSupportedException("getCursorName not supported");
  }

  @Override
  public ResultSetMetaData getMetaData() {
    checkClosed();
    // No connection in scope for serializable-derived result sets (statement == null) — NOOP then.
    Telemetry telemetry =
        statement != null ? statement.getConnectionInternal().getTelemetry() : Telemetry.NOOP;
    return new DecoratedSnowflakeResultSetMetaDataImpl(resultSetMetaData, telemetry);
  }

  /**
   * The concrete metadata for intra-package callers (e.g. the async view building its own decorated
   * projection). Returns the impl directly rather than the decorated {@link #getMetaData()} view,
   * so callers avoid the checked {@code unwrap} that the decorator boundary re-exposes.
   */
  SnowflakeResultSetMetaDataImpl getMetaDataImpl() {
    return resultSetMetaData;
  }

  @Override
  @NoTelemetry
  public Object getObject(int columnIndex) {
    checkClosed();
    return rowReader.getObject(columnIndex);
  }

  @Override
  @NoTelemetry
  public Object getObject(String columnLabel) {
    return getObject(findColumn(columnLabel));
  }

  @Override
  @NoTelemetry
  public int findColumn(String columnLabel) {
    // TODO(SNOW-3695645): in SnowflakeResultSetMetaDataImpl::getColumnIndex session parameter
    //  "isResultColumnCaseInsensitive" is respect during the search, should we respect it here?

    checkClosed();
    List<String> columnNames = resultSetMetaData.columnNames();
    for (int i = 0; i < columnNames.size(); i++) {
      if (columnNames.get(i).equalsIgnoreCase(columnLabel)) {
        return i + 1; // JDBC columns are 1-based
      }
    }
    throw new IllegalArgumentException("Column not found: " + columnLabel);
  }

  @Override
  @NoTelemetry
  public Reader getCharacterStream(int columnIndex) {
    String value = getString(columnIndex);
    return value == null ? null : new StringReader(value);
  }

  @Override
  @NoTelemetry
  public Reader getCharacterStream(String columnLabel) {
    return getCharacterStream(findColumn(columnLabel));
  }

  @Override
  @NoTelemetry
  public BigDecimal getBigDecimal(int columnIndex) {
    checkClosed();
    return rowReader.getBigDecimal(columnIndex);
  }

  @Override
  @NoTelemetry
  public BigDecimal getBigDecimal(String columnLabel) {
    return getBigDecimal(findColumn(columnLabel));
  }

  @Override
  @NoTelemetry
  public boolean isBeforeFirst() {
    checkClosed();
    return rowReader.isBeforeFirst();
  }

  @Override
  @NoTelemetry
  public boolean isAfterLast() {
    checkClosed();
    return rowReader.isAfterLast();
  }

  @Override
  @NoTelemetry
  public boolean isFirst() {
    checkClosed();
    return rowReader.isFirst();
  }

  @Override
  @NoTelemetry
  public boolean isLast() {
    checkClosed();
    return rowReader.isLast();
  }

  @Override
  public void beforeFirst() {
    throw new SFSQLFeatureNotSupportedException("beforeFirst not supported");
  }

  @Override
  public void afterLast() {
    throw new SFSQLFeatureNotSupportedException("afterLast not supported");
  }

  @Override
  public boolean first() {
    throw new SFSQLFeatureNotSupportedException("first not supported (forward-only)");
  }

  @Override
  public boolean last() {
    throw new SFSQLFeatureNotSupportedException("last not supported (forward-only)");
  }

  @Override
  @NoTelemetry
  public int getRow() {
    checkClosed();
    int currentRow = rowReader.getCurrentRow();
    if (currentRow < 0 || rowReader.isAfterLast()) {
      return 0;
    }
    return currentRow + 1; // JDBC rows are 1-based
  }

  @Override
  public boolean absolute(int row) {
    throw new SFSQLFeatureNotSupportedException("absolute not supported (forward-only)");
  }

  @Override
  public boolean relative(int rows) {
    throw new SFSQLFeatureNotSupportedException("relative not supported (forward-only)");
  }

  @Override
  public boolean previous() {
    throw new SFSQLFeatureNotSupportedException("previous not supported (forward-only)");
  }

  @Override
  public void setFetchDirection(int direction) {
    checkClosed();
    if (direction != FETCH_FORWARD) {
      throw new SFSQLFeatureNotSupportedException("Only FETCH_FORWARD supported");
    }
    this.fetchDirection = direction;
  }

  @Override
  public int getFetchDirection() {
    checkClosed();
    return fetchDirection;
  }

  @Override
  public void setFetchSize(int rows) {
    checkClosed();
    if (rows < 0) {
      throw new IllegalArgumentException("Fetch size must be >= 0");
    }
    this.fetchSize = rows;
  }

  @Override
  public int getFetchSize() {
    checkClosed();
    return fetchSize;
  }

  @Override
  public int getType() {
    return TYPE_FORWARD_ONLY;
  }

  @Override
  public int getConcurrency() {
    return CONCUR_READ_ONLY;
  }

  // Update methods (not supported)
  @Override
  public boolean rowUpdated() {
    throw new SFSQLFeatureNotSupportedException("Updates not supported");
  }

  @Override
  public boolean rowInserted() {
    throw new SFSQLFeatureNotSupportedException("Updates not supported");
  }

  @Override
  public boolean rowDeleted() {
    throw new SFSQLFeatureNotSupportedException("Updates not supported");
  }

  @Override
  public void updateNull(int columnIndex) {
    throw new SFSQLFeatureNotSupportedException("Updates not supported");
  }

  @Override
  public void updateBoolean(int columnIndex, boolean x) {
    throw new SFSQLFeatureNotSupportedException("Updates not supported");
  }

  @Override
  public void updateByte(int columnIndex, byte x) {
    throw new SFSQLFeatureNotSupportedException("Updates not supported");
  }

  @Override
  public void updateShort(int columnIndex, short x) {
    throw new SFSQLFeatureNotSupportedException("Updates not supported");
  }

  @Override
  public void updateInt(int columnIndex, int x) {
    throw new SFSQLFeatureNotSupportedException("Updates not supported");
  }

  @Override
  public void updateLong(int columnIndex, long x) {
    throw new SFSQLFeatureNotSupportedException("Updates not supported");
  }

  @Override
  public void updateFloat(int columnIndex, float x) {
    throw new SFSQLFeatureNotSupportedException("Updates not supported");
  }

  @Override
  public void updateDouble(int columnIndex, double x) {
    throw new SFSQLFeatureNotSupportedException("Updates not supported");
  }

  @Override
  public void updateBigDecimal(int columnIndex, BigDecimal x) {
    throw new SFSQLFeatureNotSupportedException("Updates not supported");
  }

  @Override
  public void updateString(int columnIndex, String x) {
    throw new SFSQLFeatureNotSupportedException("Updates not supported");
  }

  @Override
  public void updateBytes(int columnIndex, byte[] x) {
    throw new SFSQLFeatureNotSupportedException("Updates not supported");
  }

  @Override
  public void updateDate(int columnIndex, Date x) {
    throw new SFSQLFeatureNotSupportedException("Updates not supported");
  }

  @Override
  public void updateTime(int columnIndex, Time x) {
    throw new SFSQLFeatureNotSupportedException("Updates not supported");
  }

  @Override
  public void updateTimestamp(int columnIndex, Timestamp x) {
    throw new SFSQLFeatureNotSupportedException("Updates not supported");
  }

  @Override
  public void updateAsciiStream(int columnIndex, InputStream x, int length) {
    throw new SFSQLFeatureNotSupportedException("Updates not supported");
  }

  @Override
  public void updateBinaryStream(int columnIndex, InputStream x, int length) {
    throw new SFSQLFeatureNotSupportedException("Updates not supported");
  }

  @Override
  public void updateCharacterStream(int columnIndex, Reader x, int length) {
    throw new SFSQLFeatureNotSupportedException("Updates not supported");
  }

  @Override
  public void updateObject(int columnIndex, Object x, int scaleOrLength) {
    throw new SFSQLFeatureNotSupportedException("Updates not supported");
  }

  @Override
  public void updateObject(int columnIndex, Object x) {
    throw new SFSQLFeatureNotSupportedException("Updates not supported");
  }

  // String-based update methods
  @Override
  public void updateNull(String columnLabel) {
    throw new SFSQLFeatureNotSupportedException("Updates not supported");
  }

  @Override
  public void updateBoolean(String columnLabel, boolean x) {
    throw new SFSQLFeatureNotSupportedException("Updates not supported");
  }

  @Override
  public void updateByte(String columnLabel, byte x) {
    throw new SFSQLFeatureNotSupportedException("Updates not supported");
  }

  @Override
  public void updateShort(String columnLabel, short x) {
    throw new SFSQLFeatureNotSupportedException("Updates not supported");
  }

  @Override
  public void updateInt(String columnLabel, int x) {
    throw new SFSQLFeatureNotSupportedException("Updates not supported");
  }

  @Override
  public void updateLong(String columnLabel, long x) {
    throw new SFSQLFeatureNotSupportedException("Updates not supported");
  }

  @Override
  public void updateFloat(String columnLabel, float x) {
    throw new SFSQLFeatureNotSupportedException("Updates not supported");
  }

  @Override
  public void updateDouble(String columnLabel, double x) {
    throw new SFSQLFeatureNotSupportedException("Updates not supported");
  }

  @Override
  public void updateBigDecimal(String columnLabel, BigDecimal x) {
    throw new SFSQLFeatureNotSupportedException("Updates not supported");
  }

  @Override
  public void updateString(String columnLabel, String x) {
    throw new SFSQLFeatureNotSupportedException("Updates not supported");
  }

  @Override
  public void updateBytes(String columnLabel, byte[] x) {
    throw new SFSQLFeatureNotSupportedException("Updates not supported");
  }

  @Override
  public void updateDate(String columnLabel, Date x) {
    throw new SFSQLFeatureNotSupportedException("Updates not supported");
  }

  @Override
  public void updateTime(String columnLabel, Time x) {
    throw new SFSQLFeatureNotSupportedException("Updates not supported");
  }

  @Override
  public void updateTimestamp(String columnLabel, Timestamp x) {
    throw new SFSQLFeatureNotSupportedException("Updates not supported");
  }

  @Override
  public void updateAsciiStream(String columnLabel, InputStream x, int length) {
    throw new SFSQLFeatureNotSupportedException("Updates not supported");
  }

  @Override
  public void updateBinaryStream(String columnLabel, InputStream x, int length) {
    throw new SFSQLFeatureNotSupportedException("Updates not supported");
  }

  @Override
  public void updateCharacterStream(String columnLabel, Reader reader, int length) {
    throw new SFSQLFeatureNotSupportedException("Updates not supported");
  }

  @Override
  public void updateObject(String columnLabel, Object x, int scaleOrLength) {
    throw new SFSQLFeatureNotSupportedException("Updates not supported");
  }

  @Override
  public void updateObject(String columnLabel, Object x) {
    throw new SFSQLFeatureNotSupportedException("Updates not supported");
  }

  @Override
  public void insertRow() {
    throw new SFSQLFeatureNotSupportedException("insertRow not supported");
  }

  @Override
  public void updateRow() {
    throw new SFSQLFeatureNotSupportedException("updateRow not supported");
  }

  @Override
  public void deleteRow() {
    throw new SFSQLFeatureNotSupportedException("deleteRow not supported");
  }

  @Override
  public void refreshRow() {
    throw new SFSQLFeatureNotSupportedException("refreshRow not supported");
  }

  @Override
  public void cancelRowUpdates() {
    checkClosed();
  }

  @Override
  public void moveToInsertRow() {
    throw new SFSQLFeatureNotSupportedException("moveToInsertRow not supported");
  }

  @Override
  public void moveToCurrentRow() {
    throw new SFSQLFeatureNotSupportedException("moveToCurrentRow not supported");
  }

  @Override
  public Statement getStatement() {
    checkClosed();
    return statement == null
        ? null
        : Decorators.statement(statement, Decorators.telemetryOf(statement));
  }

  @Override
  @NoTelemetry
  public Object getObject(int columnIndex, Map<String, Class<?>> map) {
    throw new SFSQLFeatureNotSupportedException("getObject not supported");
  }

  @Override
  public Ref getRef(int columnIndex) {
    throw new SFSQLFeatureNotSupportedException("getRef not supported");
  }

  @Override
  public Blob getBlob(int columnIndex) {
    throw new SFSQLFeatureNotSupportedException("getBlob not supported");
  }

  @Override
  public Clob getClob(int columnIndex) {
    String value = getString(columnIndex);
    return value == null ? null : new SnowflakeClob(value);
  }

  @Override
  public Array getArray(int columnIndex) {
    throw new NotImplementedException();
  }

  @Override
  @NoTelemetry
  public Object getObject(String columnLabel, Map<String, Class<?>> map) {
    throw new SFSQLFeatureNotSupportedException("getObject not supported");
  }

  @Override
  public Ref getRef(String columnLabel) {
    throw new SFSQLFeatureNotSupportedException("getRef not supported");
  }

  @Override
  public Blob getBlob(String columnLabel) {
    throw new SFSQLFeatureNotSupportedException("getBlob not supported");
  }

  @Override
  public Clob getClob(String columnLabel) {
    return getClob(findColumn(columnLabel));
  }

  @Override
  public Array getArray(String columnLabel) {
    throw new SFSQLFeatureNotSupportedException("getArray not supported");
  }

  @Override
  @NoTelemetry
  public Date getDate(int columnIndex, Calendar cal) {
    return getDate(columnIndex, cal == null ? null : cal.getTimeZone());
  }

  @Override
  @NoTelemetry
  public Date getDate(String columnLabel, Calendar cal) {
    return getDate(findColumn(columnLabel), cal);
  }

  @Override
  @NoTelemetry
  public Time getTime(int columnIndex, Calendar cal) {
    return getTime(columnIndex);
  }

  @Override
  @NoTelemetry
  public Time getTime(String columnLabel, Calendar cal) {
    return getTime(findColumn(columnLabel), cal);
  }

  @Override
  @NoTelemetry
  public Timestamp getTimestamp(int columnIndex, Calendar cal) {
    checkClosed();
    // Mirrors snowflake-jdbc's SnowflakeBaseResultSet.getTimestamp(int, Calendar): pass the
    // Calendar's timezone to the converter. Only TIMESTAMP_NTZ consumes it (honor-client-TZ
    // re-anchoring); LTZ/TZ ignore it. Note getTime(int, Calendar) intentionally drops the Calendar
    // to match legacy.
    return rowReader.getTimestamp(columnIndex, cal == null ? null : cal.getTimeZone());
  }

  @Override
  @NoTelemetry
  public Timestamp getTimestamp(String columnLabel, Calendar cal) {
    return getTimestamp(findColumn(columnLabel), cal);
  }

  @Override
  public URL getURL(int columnIndex) {
    throw new SFSQLFeatureNotSupportedException("getURL not supported");
  }

  @Override
  public URL getURL(String columnLabel) {
    throw new SFSQLFeatureNotSupportedException("getURL not supported");
  }

  @Override
  public void updateRef(int columnIndex, Ref x) {
    throw new SFSQLFeatureNotSupportedException("Updates not supported");
  }

  @Override
  public void updateRef(String columnLabel, Ref x) {
    throw new SFSQLFeatureNotSupportedException("Updates not supported");
  }

  @Override
  public void updateBlob(int columnIndex, Blob x) {
    throw new SFSQLFeatureNotSupportedException("Updates not supported");
  }

  @Override
  public void updateBlob(String columnLabel, Blob x) {
    throw new SFSQLFeatureNotSupportedException("Updates not supported");
  }

  @Override
  public void updateClob(int columnIndex, Clob x) {
    throw new SFSQLFeatureNotSupportedException("Updates not supported");
  }

  @Override
  public void updateClob(String columnLabel, Clob x) {
    throw new SFSQLFeatureNotSupportedException("Updates not supported");
  }

  @Override
  public void updateArray(int columnIndex, Array x) {
    throw new SFSQLFeatureNotSupportedException("Updates not supported");
  }

  @Override
  public void updateArray(String columnLabel, Array x) {
    throw new SFSQLFeatureNotSupportedException("Updates not supported");
  }

  @Override
  public RowId getRowId(int columnIndex) {
    throw new SFSQLFeatureNotSupportedException("getRowId not supported");
  }

  @Override
  public RowId getRowId(String columnLabel) {
    throw new SFSQLFeatureNotSupportedException("getRowId not supported");
  }

  @Override
  public void updateRowId(int columnIndex, RowId x) {
    throw new SFSQLFeatureNotSupportedException("Updates not supported");
  }

  @Override
  public void updateRowId(String columnLabel, RowId x) {
    throw new SFSQLFeatureNotSupportedException("Updates not supported");
  }

  @Override
  public int getHoldability() {
    return CLOSE_CURSORS_AT_COMMIT;
  }

  @Override
  public boolean isClosed() {
    return closed;
  }

  @Override
  public void updateNString(int columnIndex, String nString) {
    throw new SFSQLFeatureNotSupportedException("Updates not supported");
  }

  @Override
  public void updateNString(String columnLabel, String nString) {
    throw new SFSQLFeatureNotSupportedException("Updates not supported");
  }

  @Override
  public void updateNClob(int columnIndex, NClob nClob) {
    throw new SFSQLFeatureNotSupportedException("Updates not supported");
  }

  @Override
  public void updateNClob(String columnLabel, NClob nClob) {
    throw new SFSQLFeatureNotSupportedException("Updates not supported");
  }

  @Override
  public NClob getNClob(int columnIndex) {
    throw new SFSQLFeatureNotSupportedException("getNClob not supported");
  }

  @Override
  public NClob getNClob(String columnLabel) {
    throw new SFSQLFeatureNotSupportedException("getNClob not supported");
  }

  @Override
  public SQLXML getSQLXML(int columnIndex) {
    throw new SFSQLFeatureNotSupportedException("getSQLXML not supported");
  }

  @Override
  public SQLXML getSQLXML(String columnLabel) {
    throw new SFSQLFeatureNotSupportedException("getSQLXML not supported");
  }

  @Override
  public void updateSQLXML(int columnIndex, SQLXML xmlObject) {
    throw new SFSQLFeatureNotSupportedException("Updates not supported");
  }

  @Override
  public void updateSQLXML(String columnLabel, SQLXML xmlObject) {
    throw new SFSQLFeatureNotSupportedException("Updates not supported");
  }

  @Override
  public String getNString(int columnIndex) {
    throw new SFSQLFeatureNotSupportedException("getNString not supported");
  }

  @Override
  public String getNString(String columnLabel) {
    throw new SFSQLFeatureNotSupportedException("getNString not supported");
  }

  @Override
  public Reader getNCharacterStream(int columnIndex) {
    throw new SFSQLFeatureNotSupportedException("getNCharacterStream not supported");
  }

  @Override
  public Reader getNCharacterStream(String columnLabel) {
    throw new SFSQLFeatureNotSupportedException("getNCharacterStream not supported");
  }

  @Override
  public void updateNCharacterStream(int columnIndex, Reader x, long length) {
    throw new SFSQLFeatureNotSupportedException("Updates not supported");
  }

  @Override
  public void updateNCharacterStream(String columnLabel, Reader reader, long length) {
    throw new SFSQLFeatureNotSupportedException("Updates not supported");
  }

  @Override
  public void updateAsciiStream(int columnIndex, InputStream x, long length) {
    throw new SFSQLFeatureNotSupportedException("Updates not supported");
  }

  @Override
  public void updateBinaryStream(int columnIndex, InputStream x, long length) {
    throw new SFSQLFeatureNotSupportedException("Updates not supported");
  }

  @Override
  public void updateCharacterStream(int columnIndex, Reader x, long length) {
    throw new SFSQLFeatureNotSupportedException("Updates not supported");
  }

  @Override
  public void updateAsciiStream(String columnLabel, InputStream x, long length) {
    throw new SFSQLFeatureNotSupportedException("Updates not supported");
  }

  @Override
  public void updateBinaryStream(String columnLabel, InputStream x, long length) {
    throw new SFSQLFeatureNotSupportedException("Updates not supported");
  }

  @Override
  public void updateCharacterStream(String columnLabel, Reader reader, long length) {
    throw new SFSQLFeatureNotSupportedException("Updates not supported");
  }

  @Override
  public void updateBlob(int columnIndex, InputStream inputStream, long length) {
    throw new SFSQLFeatureNotSupportedException("Updates not supported");
  }

  @Override
  public void updateBlob(String columnLabel, InputStream inputStream, long length) {
    throw new SFSQLFeatureNotSupportedException("Updates not supported");
  }

  @Override
  public void updateClob(int columnIndex, Reader reader, long length) {
    throw new SFSQLFeatureNotSupportedException("Updates not supported");
  }

  @Override
  public void updateClob(String columnLabel, Reader reader, long length) {
    throw new SFSQLFeatureNotSupportedException("Updates not supported");
  }

  @Override
  public void updateNClob(int columnIndex, Reader reader, long length) {
    throw new SFSQLFeatureNotSupportedException("Updates not supported");
  }

  @Override
  public void updateNClob(String columnLabel, Reader reader, long length) {
    throw new SFSQLFeatureNotSupportedException("Updates not supported");
  }

  @Override
  public void updateNCharacterStream(int columnIndex, Reader x) {
    throw new SFSQLFeatureNotSupportedException("Updates not supported");
  }

  @Override
  public void updateNCharacterStream(String columnLabel, Reader reader) {
    throw new SFSQLFeatureNotSupportedException("Updates not supported");
  }

  @Override
  public void updateAsciiStream(int columnIndex, InputStream x) {
    throw new SFSQLFeatureNotSupportedException("Updates not supported");
  }

  @Override
  public void updateBinaryStream(int columnIndex, InputStream x) {
    throw new SFSQLFeatureNotSupportedException("Updates not supported");
  }

  @Override
  public void updateCharacterStream(int columnIndex, Reader x) {
    throw new SFSQLFeatureNotSupportedException("Updates not supported");
  }

  @Override
  public void updateAsciiStream(String columnLabel, InputStream x) {
    throw new SFSQLFeatureNotSupportedException("Updates not supported");
  }

  @Override
  public void updateBinaryStream(String columnLabel, InputStream x) {
    throw new SFSQLFeatureNotSupportedException("Updates not supported");
  }

  @Override
  public void updateCharacterStream(String columnLabel, Reader reader) {
    throw new SFSQLFeatureNotSupportedException("Updates not supported");
  }

  @Override
  public void updateBlob(int columnIndex, InputStream inputStream) {
    throw new SFSQLFeatureNotSupportedException("Updates not supported");
  }

  @Override
  public void updateBlob(String columnLabel, InputStream inputStream) {
    throw new SFSQLFeatureNotSupportedException("Updates not supported");
  }

  @Override
  public void updateClob(int columnIndex, Reader reader) {
    throw new SFSQLFeatureNotSupportedException("Updates not supported");
  }

  @Override
  public void updateClob(String columnLabel, Reader reader) {
    throw new SFSQLFeatureNotSupportedException("Updates not supported");
  }

  @Override
  public void updateNClob(int columnIndex, Reader reader) {
    throw new SFSQLFeatureNotSupportedException("Updates not supported");
  }

  @Override
  public void updateNClob(String columnLabel, Reader reader) {
    throw new SFSQLFeatureNotSupportedException("Updates not supported");
  }

  @Override
  @NoTelemetry
  public <T> T getObject(int columnIndex, Class<T> type) {
    if (type == String.class) {
      return type.cast(getString(columnIndex));
    } else if (type == Integer.class) {
      return type.cast(getInt(columnIndex));
    } else if (type == Long.class) {
      return type.cast(getLong(columnIndex));
    } else if (type == Double.class) {
      return type.cast(getDouble(columnIndex));
    } else if (type == Boolean.class) {
      return type.cast(getBoolean(columnIndex));
    } else if (type == BigDecimal.class) {
      return type.cast(getBigDecimal(columnIndex));
    } else if (type == Date.class) {
      return type.cast(getDate(columnIndex));
    } else if (type == Time.class) {
      return type.cast(getTime(columnIndex));
    } else if (type == Timestamp.class) {
      return type.cast(getTimestamp(columnIndex));
    } else if (type == Period.class) {
      return type.cast(getPeriod(columnIndex));
    } else if (type == Duration.class) {
      return type.cast(getDuration(columnIndex));
    }
    throw new SFSQLFeatureNotSupportedException("Type not supported: " + type.getName());
  }

  @Override
  @NoTelemetry
  public <T> T getObject(String columnLabel, Class<T> type) {
    return getObject(findColumn(columnLabel), type);
  }

  /**
   * Transfers ownership of the {@link RowReader} to the caller. This result set is marked closed
   * and unregistered from the statement, but the reader itself is NOT closed - the caller assumes
   * responsibility for its lifecycle.
   */
  RowReader detachRowReader() {
    closed = true;
    // The converted view exposes a different column shape,
    // release now rather than transferring ownership to the wrapping result set.
    if (resultSetChunksProvider != null) {
      resultSetChunksProvider.release();
    }
    if (statement != null) {
      statement.removeClosedResultSet(this);
    }
    return rowReader;
  }

  private void checkClosed() {
    if (closed) {
      throw new IllegalStateException("ResultSet is closed");
    }
  }

  @Override
  public String getQueryID() {
    return queryId;
  }

  @Override
  public List<SnowflakeResultSetSerializable> getResultSetSerializables(long maxSizeInBytes) {
    checkClosed();
    if (resultSetChunksProvider == null) {
      // Plain in-memory (metadata) and converter-wrapped result sets have no chunk backing to
      // slice, matching snowflake-jdbc whose SnowflakeDatabaseMetaDataResultSet rejects this.
      throw new SFSQLFeatureNotSupportedException(
          "getResultSetSerializables is not supported for this result set");
    }
    return resultSetChunksProvider.getChunks(maxSizeInBytes);
  }

  @Override
  public <T> T[] getArray(int columnIndex, Class<T> type) {
    checkClosed();
    // TODO(SNOW-3445814): getArray supported for structured types (not only VECTOR)
    if (resultSetMetaData.getColumnType(columnIndex) != SnowflakeType.EXTRA_TYPES_VECTOR) {
      throw new SFSQLFeatureNotSupportedException("getArray is only supported for VECTOR columns");
    }
    // Exact int & float match only (snowflake-jdbc does proper type coercion via Converters).
    // TODO(SNOW-3445814): add broad type coercion for structured types
    if (!Integer.class.equals(type) && !Float.class.equals(type)) {
      throw new SFSQLFeatureNotSupportedException(
          "Type passed to 'getArray(int columnIndex, Class<T> type)' is unsupported. Type: "
              + type.getName());
    }
    List<?> elements = rowReader.getList(columnIndex);
    if (elements == null) {
      return null;
    }
    try {
      return elements.toArray((T[]) java.lang.reflect.Array.newInstance(type, elements.size()));
    } catch (ArrayStoreException e) {
      throw new SFSQLFeatureNotSupportedException(
          "VECTOR elements cannot be converted to " + type.getName());
    }
  }

  @Override
  public <T> List<T> getList(int columnIndex, Class<T> type) {
    throw new NotImplementedException();
  }

  @Override
  public <T> Map<String, T> getMap(int columnIndex, Class<T> type) {
    throw new NotImplementedException();
  }
}
