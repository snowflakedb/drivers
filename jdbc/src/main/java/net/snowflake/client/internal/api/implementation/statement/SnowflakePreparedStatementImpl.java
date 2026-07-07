package net.snowflake.client.internal.api.implementation.statement;

import static java.lang.Integer.MAX_VALUE;
import static java.sql.Types.CLOB;
import static net.snowflake.client.internal.util.StringUtil.isNullOrEmpty;

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
import java.util.HashMap;
import java.util.HashSet;
import java.util.Map;
import java.util.Objects;
import java.util.Set;
import java.util.TimeZone;
import java.util.function.Function;
import net.snowflake.client.api.exception.SnowflakeSQLException;
import net.snowflake.client.api.resultset.SnowflakeType;
import net.snowflake.client.api.statement.SnowflakePreparedStatement;
import net.snowflake.client.internal.api.implementation.connection.InternalSnowflakeConnection;
import net.snowflake.client.internal.api.implementation.resultset.metadata.SnowflakeResultSetMetaDataImpl;
import net.snowflake.client.internal.core.arrow.ArrowDateUtil;
import net.snowflake.client.internal.core.arrow.converters.DataConversionContext;
import net.snowflake.client.internal.core.arrow.converters.SessionDataConversionContext;
import net.snowflake.client.internal.log.SFLogger;
import net.snowflake.client.internal.log.SFLoggerFactory;
import net.snowflake.client.internal.unicore.CoreDriverApi;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.PrepareResult;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.StatementPrepareResponse;
import net.snowflake.client.internal.util.HexUtil;

public class SnowflakePreparedStatementImpl extends SnowflakeStatementImpl
    implements PreparedStatement, SnowflakePreparedStatement {
  private static final SFLogger logger =
      SFLoggerFactory.getLogger(SnowflakePreparedStatementImpl.class);

  private final String sql;
  private final SqlPlaceholderMetadata placeholderMetadata;
  private final Map<Integer, PreparedStatementBindingSerializer.ParameterValue> parameterValues;
  private final PreparedBatch batch = new PreparedBatch();

  /** Cached prepare result from the server, populated lazily on the first metadata request. */
  // TODO(SNOW-3740751): should we replace proto message with POJO?
  private PrepareResult prepareResult;

  /** Lazily resolved session conversion context, used for TIME binding semantics. */
  private DataConversionContext conversionContext;

  public SnowflakePreparedStatementImpl(
      InternalSnowflakeConnection connection, String sql, CoreDriverApi coreDriverApi) {
    super(connection, coreDriverApi);
    this.sql = sql;
    this.placeholderMetadata = SqlPlaceholderMetadata.analyze(sql);
    this.parameterValues = new HashMap<>();
  }

  /** Prepares the statement on the server once and caches the result. */
  private PrepareResult getPrepareResult() throws SQLException {
    checkClosed();
    if (prepareResult == null) {
      try {
        coreDriverApi.statementSetSqlQuery(statementHandle, sql);
        StatementPrepareResponse response = coreDriverApi.statementPrepare(statementHandle);
        prepareResult = response.getResult();
      } catch (SnowflakeSQLException e) {
        // Mirror snowflake-jdbc: some describe failures (DDL, unset bind variables, etc.) are
        // expected and fall back to empty metadata instead of re-issuing describe on every call.
        if (!ERROR_CODES_IGNORED_IN_DESCRIBE_MODE.contains(e.getErrorCode())) {
          throw e;
        }
        PrepareResult.Builder builder = PrepareResult.newBuilder();
        if (!isNullOrEmpty(e.getQueryId())) {
          builder.setQueryId(e.getQueryId());
        }
        prepareResult = builder.build();
      }
    }
    return prepareResult;
  }

  @Override
  public ResultSet executeQuery() throws SQLException {
    checkClosed();
    invalidateConversionContext();
    try (PreparedStatementBindingSerializer.NativeBindings nativeBindings =
        PreparedStatementBindingSerializer.serialize(placeholderMetadata, parameterValues)) {
      return executeQueryWithBindings(sql, nativeBindings);
    }
  }

  @Override
  public int executeUpdate() throws SQLException {
    checkClosed();
    invalidateConversionContext();
    try (PreparedStatementBindingSerializer.NativeBindings nativeBindings =
        PreparedStatementBindingSerializer.serialize(placeholderMetadata, parameterValues)) {
      return executeUpdateWithBindings(sql, nativeBindings);
    }
  }

  @Override
  public void setNull(int parameterIndex, int sqlType) throws SQLException {
    checkClosed();
    // ANY is the sentinel; addBatch promotes the column to a real type on first non-null.
    setParameter(parameterIndex, SnowflakeType.ANY, null);
  }

  @Override
  public void setBoolean(int parameterIndex, boolean x) throws SQLException {
    checkClosed();
    setParameter(parameterIndex, SnowflakeType.BOOLEAN, String.valueOf(x));
  }

  @Override
  public void setByte(int parameterIndex, byte x) throws SQLException {
    checkClosed();
    setParameter(parameterIndex, SnowflakeType.FIXED, String.valueOf(x));
  }

  @Override
  public void setShort(int parameterIndex, short x) throws SQLException {
    checkClosed();
    setParameter(parameterIndex, SnowflakeType.FIXED, String.valueOf(x));
  }

  @Override
  public void setInt(int parameterIndex, int x) throws SQLException {
    checkClosed();
    setParameter(parameterIndex, SnowflakeType.FIXED, String.valueOf(x));
  }

  @Override
  public void setLong(int parameterIndex, long x) throws SQLException {
    checkClosed();
    setParameter(parameterIndex, SnowflakeType.FIXED, String.valueOf(x));
  }

  @Override
  public void setFloat(int parameterIndex, float x) throws SQLException {
    checkClosed();
    setParameter(parameterIndex, SnowflakeType.REAL, String.valueOf(x));
  }

  @Override
  public void setDouble(int parameterIndex, double x) throws SQLException {
    checkClosed();
    setParameter(parameterIndex, SnowflakeType.REAL, String.valueOf(x));
  }

  @Override
  public void setBigDecimal(int parameterIndex, BigDecimal x) throws SQLException {
    checkClosed();
    setNullableParameter(
        parameterIndex, Types.DECIMAL, SnowflakeType.FIXED, x, decimal -> String.valueOf(decimal));
  }

  @Override
  public void setString(int parameterIndex, String x) throws SQLException {
    checkClosed();
    setNullableParameter(
        parameterIndex, Types.VARCHAR, SnowflakeType.TEXT, x, stringValue -> stringValue);
  }

  @Override
  public void setBytes(int parameterIndex, byte[] x) throws SQLException {
    checkClosed();
    setNullableParameter(
        parameterIndex, Types.BINARY, SnowflakeType.BINARY, x, bytes -> HexUtil.bytesToHex(bytes));
  }

  @Override
  public void setDate(int parameterIndex, Date x) throws SQLException {
    checkClosed();
    setDate(parameterIndex, x, TimeZone.getDefault());
  }

  @Override
  public void setTime(int parameterIndex, Time x) throws SQLException {
    checkClosed();
    DataConversionContext context = conversionContext();
    setNullableParameter(
        parameterIndex,
        Types.TIME,
        SnowflakeType.TIME,
        x,
        time -> String.valueOf(context.timeToNanosOfDay(time)));
  }

  /**
   * Returns this statement's session conversion context, lazily fetching (and caching) the session
   * parameters on first use. The cache is dropped at every execute boundary (see {@link
   * #invalidateConversionContext()}) so a reused statement picks up session-parameter changes,
   * while a single bind cycle of N values still costs only one fetch.
   */
  private DataConversionContext conversionContext() {
    if (conversionContext == null) {
      conversionContext =
          SessionDataConversionContext.fromConnection(
              coreDriverApi, connection.getHandle(), connection.getResolvedProperties());
    }
    return conversionContext;
  }

  /**
   * Drops the cached conversion context so the next bind cycle re-reads session parameters,
   * matching the old driver's refresh-at-execution granularity.
   *
   * <p>TODO(SNOW-2872484): conversion is currently eager in the typed setters, so we have to
   * invalidate in every execute entry point. The cleaner shape is to store the raw value and
   * convert once inside {@link PreparedStatementBindingSerializer#serialize} (the single bind
   * chokepoint), which would remove the need for these explicit invalidations.
   */
  private void invalidateConversionContext() {
    conversionContext = null;
  }

  @Override
  public void setTimestamp(int parameterIndex, Timestamp x) throws SQLException {
    checkClosed();
    setTimestampWithType(parameterIndex, x, Types.TIMESTAMP);
  }

  /**
   * Binds a TIMESTAMP value with an explicit Snowflake target type, mirroring snowflake-jdbc's
   * {@code SnowflakePreparedStatementV1.setTimestampWithType}. The value is the instant as an
   * epoch-nanoseconds decimal string (see {@link ArrowDateUtil#timestampToBindString}); {@code
   * TIMESTAMP_LTZ} and {@code TIMESTAMP_NTZ} share the identical numeric string and differ only in
   * the bind type name. A bare {@link Types#TIMESTAMP} resolves to the session's {@code
   * CLIENT_TIMESTAMP_TYPE_MAPPING} (default {@code TIMESTAMP_LTZ}).
   *
   * @param snowflakeType {@link Types#TIMESTAMP} for the mapped default, or {@link
   *     SnowflakeType#EXTRA_TYPES_TIMESTAMP_LTZ} / {@link SnowflakeType#EXTRA_TYPES_TIMESTAMP_NTZ}
   *     to force that type
   */
  private void setTimestampWithType(int parameterIndex, Timestamp x, int snowflakeType)
      throws SQLException {
    SnowflakeType bindType;
    switch (snowflakeType) {
      case SnowflakeType.EXTRA_TYPES_TIMESTAMP_LTZ:
        bindType = SnowflakeType.TIMESTAMP_LTZ;
        break;
      case SnowflakeType.EXTRA_TYPES_TIMESTAMP_NTZ:
        bindType = SnowflakeType.TIMESTAMP_NTZ;
        break;
      default:
        bindType = mappedTimestampBindType();
        break;
    }
    setNullableParameter(
        parameterIndex, Types.TIMESTAMP, bindType, x, ArrowDateUtil::timestampToBindString);
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
    logger.trace(
        "Clearing prepared parameters: placeholders={}", placeholderMetadata.placeholderCount());
    parameterValues.clear();
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
    // EXTRA_TYPES_TIMESTAMP_LTZ / _NTZ force that concrete timestamp type. Legacy has no
    // EXTRA_TYPES_TIMESTAMP_TZ branch here — a TZ is only bindable via the Calendar overload — so
    // it is intentionally absent (SnowflakePreparedStatementV1.setObject :470-472).
    if (targetSqlType == SnowflakeType.EXTRA_TYPES_TIMESTAMP_LTZ
        || targetSqlType == SnowflakeType.EXTRA_TYPES_TIMESTAMP_NTZ) {
      if (!(x instanceof Timestamp)) {
        throw new SQLException(
            "Invalid parameter type for TIMESTAMP at index "
                + parameterIndex
                + ": "
                + x.getClass().getCanonicalName());
      }
      setTimestampWithType(parameterIndex, (Timestamp) x, targetSqlType);
      return;
    }
    SnowflakeType bindType = sqlTypeToBindType(targetSqlType);
    if (bindType == SnowflakeType.BINARY && x instanceof byte[]) {
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
    if (x instanceof Date) {
      setDate(parameterIndex, (Date) x);
      return;
    }
    if (x instanceof Time) {
      setTime(parameterIndex, (Time) x);
      return;
    }
    // java.sql.Timestamp is NOT a java.sql.Date (both extend java.util.Date), so it does not match
    // the instanceof Date branch above; it binds as the mapped TIMESTAMP type, mirroring legacy's
    // dedicated instanceof Timestamp branch (SnowflakePreparedStatementV1.setObject :553).
    if (x instanceof Timestamp) {
      setTimestamp(parameterIndex, (Timestamp) x);
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
    invalidateConversionContext();
    try (PreparedStatementBindingSerializer.NativeBindings nativeBindings =
        PreparedStatementBindingSerializer.serialize(placeholderMetadata, parameterValues)) {
      return executeWithBindings(sql, nativeBindings);
    }
  }

  @Override
  public void addBatch() throws SQLException {
    checkClosed();
    batch.addRow(placeholderMetadata, parameterValues);
  }

  @Override
  public void clearBatch() throws SQLException {
    super.clearBatch();
    batch.clear();
  }

  @Override
  public void addBatch(String sql) throws SQLException {
    checkClosed();
    throw new SQLFeatureNotSupportedException(
        "addBatch(String) is not allowed on PreparedStatement");
  }

  @Override
  public int[] executeBatch() throws SQLException {
    checkClosed();
    invalidateConversionContext();
    long[] expanded = batch.executeAll(this, sql, placeholderMetadata);
    int[] result = new int[expanded.length];
    for (int i = 0; i < expanded.length; i++) {
      result[i] = toBatchInt(expanded[i]);
    }
    return result;
  }

  @Override
  public long[] executeLargeBatch() throws SQLException {
    checkClosed();
    invalidateConversionContext();
    return batch.executeAll(this, sql, placeholderMetadata);
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
    if (x == null) {
      setNull(parameterIndex, CLOB);
    } else {
      long length = x.length();
      if (length > MAX_VALUE) {
        throw new SQLException("CLOB length " + length + " exceeds the maximum supported size.");
      }
      // SerialClob (and most Clob impls) reject getSubString(1, 0) on an empty CLOB, so bind an
      // empty string directly instead of calling getSubString for a zero-length value.
      setString(parameterIndex, length == 0 ? "" : x.getSubString(1, (int) length));
    }
  }

  @Override
  public void setArray(int parameterIndex, Array x) throws SQLException {
    throw new SQLFeatureNotSupportedException("setArray not supported");
  }

  @Override
  public ResultSetMetaData getMetaData() throws SQLException {
    PrepareResult result = getPrepareResult();
    return SnowflakeResultSetMetaDataImpl.from(
        result.getQueryId(), result.getColumnsList(), conversionContext());
  }

  @Override
  public void setDate(int parameterIndex, Date x, Calendar cal) throws SQLException {
    checkClosed();
    setDate(parameterIndex, x, cal == null ? TimeZone.getDefault() : cal.getTimeZone());
  }

  /**
   * Binds a DATE value, mirroring snowflake-jdbc's {@code setDate}: the server receives sfType
   * {@code "DATE"} with the value being milliseconds-since-epoch in {@code tz}, after applying the
   * Julian→Gregorian correction for pre-1582-10-05 dates. {@code tz} is the JVM default for {@code
   * setDate(int, Date)} and the Calendar's timezone for {@code setDate(int, Date, Calendar)}.
   */
  private void setDate(int parameterIndex, Date x, TimeZone tz) throws SQLException {
    setNullableParameter(
        parameterIndex,
        Types.DATE,
        SnowflakeType.DATE,
        x,
        date -> String.valueOf(ArrowDateUtil.dateToBindMillis(date, tz)));
  }

  @Override
  public void setTime(int parameterIndex, Time x, Calendar cal) throws SQLException {
    setTime(parameterIndex, x);
  }

  /**
   * Binds a TIMESTAMP value using the given Calendar's timezone, mirroring snowflake-jdbc's {@code
   * SnowflakePreparedStatementV1.setTimestamp(int, Timestamp, Calendar)}. The target type is the
   * session's {@code CLIENT_TIMESTAMP_TYPE_MAPPING} (default {@code TIMESTAMP_LTZ}); the Calendar
   * overload never accepts an explicit type. For {@code TIMESTAMP_TZ} the instant is kept as-is and
   * the Calendar offset is stored as a separate offset code (the only way to bind a TZ); for {@code
   * TIMESTAMP_LTZ}/{@code TIMESTAMP_NTZ} the instant is shifted by the Calendar offset. Neither
   * branch applies the Julian→Gregorian correction. A null Calendar falls back to the JVM default
   * zone, matching {@link #setDate(int, Date, Calendar)}.
   */
  @Override
  public void setTimestamp(int parameterIndex, Timestamp x, Calendar cal) throws SQLException {
    checkClosed();
    SnowflakeType bindType = mappedTimestampBindType();
    TimeZone tz = cal == null ? TimeZone.getDefault() : cal.getTimeZone();
    String value;
    if (x == null) {
      value = null;
    } else if (bindType == SnowflakeType.TIMESTAMP_TZ) {
      value = ArrowDateUtil.timestampTzToBindString(x, tz);
    } else {
      value = ArrowDateUtil.timestampWithCalendarToBindString(x, tz);
    }
    setParameter(parameterIndex, bindType, value);
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
    PrepareResult prepareResult = getPrepareResult();
    return SnowflakeParameterMetadataImpl.from(prepareResult.getBindsList());
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

  private void setParameter(int parameterIndex, SnowflakeType bindType, Object value)
      throws SQLException {
    if (parameterIndex < 1) {
      logger.warn(
          "Invalid prepared parameter index: index={}, placeholders={}",
          parameterIndex,
          placeholderMetadata.placeholderCount());
      throw new SQLException("Invalid parameter index: " + parameterIndex);
    }
    if (placeholderMetadata.hasMixedPlaceholderStyles()) {
      throw new SQLException("Mixed positional and numeric placeholders are not supported");
    }
    if (!placeholderMetadata.referencesParameterIndex(parameterIndex)) {
      logger.debug(
          "Ignoring extra prepared parameter to preserve legacy JDBC behavior: index={}, placeholders={}",
          parameterIndex,
          placeholderMetadata.placeholderCount());
      return;
    }
    // Boxed primitives passed through setObject must be stringified before they reach the
    // serializer (which rejects non-String values).
    String normalized = Objects.toString(value, null);
    parameterValues.put(
        parameterIndex,
        new PreparedStatementBindingSerializer.ParameterValue(bindType, normalized));
    logger.debug(
        "Prepared parameter set: index={}, bindType={}, isNull={}, placeholders={}",
        parameterIndex,
        bindType,
        value == null,
        placeholderMetadata.placeholderCount());
  }

  private <T> void setNullableParameter(
      int parameterIndex,
      int sqlType,
      SnowflakeType bindType,
      T value,
      Function<T, String> serializer)
      throws SQLException {
    if (value == null) {
      // Preserve the typed bind type for the typed setX-with-null path. The generic
      // setNull(idx, sqlType) entry point still maps to "ANY" (matches reference); this avoids
      // silently widening every typed null to ANY when the user clearly intended a typed
      // column.
      setParameter(parameterIndex, bindType, null);
      return;
    }
    setParameter(parameterIndex, bindType, serializer.apply(value));
  }

  /**
   * Resolves the session's {@code CLIENT_TIMESTAMP_TYPE_MAPPING} to the bind type for a bare {@code
   * TIMESTAMP} (only ever {@link SnowflakeType#TIMESTAMP_LTZ} or {@link
   * SnowflakeType#TIMESTAMP_NTZ}; defaults to LTZ).
   */
  private SnowflakeType mappedTimestampBindType() {
    return SnowflakeType.valueOf(conversionContext().getTimestampMappedType());
  }

  private static SnowflakeType sqlTypeToBindType(int sqlType) {
    switch (sqlType) {
      case Types.BOOLEAN:
      case Types.BIT:
        return SnowflakeType.BOOLEAN;
      case Types.TINYINT:
      case Types.SMALLINT:
      case Types.INTEGER:
      case Types.BIGINT:
      case Types.NUMERIC:
      case Types.DECIMAL:
        return SnowflakeType.FIXED;
      case Types.FLOAT:
      case Types.REAL:
      case Types.DOUBLE:
        return SnowflakeType.REAL;
      case Types.BINARY:
      case Types.VARBINARY:
      case Types.LONGVARBINARY:
      case Types.BLOB:
        return SnowflakeType.BINARY;
      case SnowflakeType.EXTRA_TYPES_TIMESTAMP_LTZ:
        return SnowflakeType.TIMESTAMP_LTZ;
      case SnowflakeType.EXTRA_TYPES_TIMESTAMP_NTZ:
        return SnowflakeType.TIMESTAMP_NTZ;
      case SnowflakeType.EXTRA_TYPES_TIMESTAMP_TZ:
        return SnowflakeType.TIMESTAMP_TZ;
      default:
        return SnowflakeType.TEXT;
    }
  }

  @Override
  public ResultSet executeAsyncQuery() throws SQLException {
    checkClosed();
    invalidateConversionContext();
    try (PreparedStatementBindingSerializer.NativeBindings nativeBindings =
        PreparedStatementBindingSerializer.serialize(placeholderMetadata, parameterValues)) {
      return executeAsyncQueryWithBindings(sql, nativeBindings.bindings());
    }
  }

  @Override
  public void setBigInteger(int parameterIndex, BigInteger x) throws SQLException {
    checkClosed();
    throw new SQLFeatureNotSupportedException("setBigInteger not supported");
  }

  @Override
  public <T> void setMap(int parameterIndex, Map<String, T> map, int type) throws SQLException {
    throw new SQLFeatureNotSupportedException("setMap not supported");
  }

  // =========================================================================
  // Constants ported from snowflake-jdbc
  // =========================================================================

  /** Error code returned when describing a statement that is binding table name */
  private static final int ERROR_CODE_TABLE_BIND_VARIABLE_NOT_SET = 2128;

  /** Error code when preparing statement with binding object names */
  private static final int ERROR_CODE_OBJECT_BIND_NOT_SET = 2129;

  /** Error code returned when describing a ddl command */
  private static final int ERROR_CODE_STATEMENT_CANNOT_BE_PREPARED = 7;

  /** snow-44393 Workaround for compiler cannot prepare to_timestamp(?, 3) */
  private static final int ERROR_CODE_FORMAT_ARGUMENT_NOT_STRING = 1026;

  /** Error codes that should not lead to an exception in describe mode. */
  private static final Set<Integer> ERROR_CODES_IGNORED_IN_DESCRIBE_MODE =
      new HashSet<>(
          Arrays.asList(
              ERROR_CODE_TABLE_BIND_VARIABLE_NOT_SET,
              ERROR_CODE_STATEMENT_CANNOT_BE_PREPARED,
              ERROR_CODE_OBJECT_BIND_NOT_SET,
              ERROR_CODE_FORMAT_ARGUMENT_NOT_STRING));
}
