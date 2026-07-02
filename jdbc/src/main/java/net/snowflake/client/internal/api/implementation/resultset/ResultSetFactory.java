package net.snowflake.client.internal.api.implementation.resultset;

import java.sql.SQLException;
import java.util.List;
import lombok.AccessLevel;
import lombok.NoArgsConstructor;
import net.snowflake.client.internal.api.implementation.connection.InternalSnowflakeConnection;
import net.snowflake.client.internal.api.implementation.resultset.metadata.SnowflakeResultSetMetaDataImpl;
import net.snowflake.client.internal.api.implementation.statement.SnowflakeStatementImpl;
import net.snowflake.client.internal.core.arrow.ArrowStreamFactory;
import net.snowflake.client.internal.core.arrow.converters.DataConversionContext;
import net.snowflake.client.internal.core.arrow.converters.SessionDataConversionContext;
import net.snowflake.client.internal.core.arrow.cursor.ArrowResources;
import net.snowflake.client.internal.log.SFLogger;
import net.snowflake.client.internal.log.SFLoggerFactory;
import net.snowflake.client.internal.unicore.CoreDriverApi;
import net.snowflake.client.internal.unicore.ProtobufApis;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.ColumnMetadata;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.ResultSetGetStreamResponse;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.ResultSetHandle;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.ResultSetResponse;

/**
 * Central factory for all ResultSet creation.
 *
 * <p>For synchronous results, encapsulates the fetch-stream-and-release lifecycle: {@code
 * resultSetGetStream} takes ownership of the prebuilt Arrow stream (one-shot), so the handle is
 * released immediately after, regardless of success or failure.
 *
 * <p>For asynchronous results, creates an {@link SnowflakeAsyncResultSetImpl} that lazily
 * materializes on first data access.
 */
@NoArgsConstructor(access = AccessLevel.PRIVATE)
public class ResultSetFactory {

  private static final SFLogger logger = SFLoggerFactory.getLogger(ResultSetFactory.class);

  public static InternalResultSet create(
      CoreDriverApi coreDriverApi,
      SnowflakeStatementImpl statement,
      String queryId,
      ResultSetResponse rs)
      throws SQLException {
    ResultSetGetStreamResponse response =
        fetchStreamAndRelease(coreDriverApi, rs.getResultSetHandle());
    return resultSetFromResponse(
        statement, queryId, response, rs.getResultDescriptor().getColumnsList());
  }

  public static InternalAsyncResultSet createAsync(
      String queryId,
      InternalSnowflakeConnection connection,
      SnowflakeStatementImpl statement,
      boolean ownsStatement) {
    return new SnowflakeAsyncResultSetImpl(queryId, connection, statement, ownsStatement);
  }

  public static InternalResultSet createIfHasStream(
      CoreDriverApi coreDriverApi,
      SnowflakeStatementImpl statement,
      String queryId,
      ResultSetResponse rs)
      throws SQLException {
    ResultSetGetStreamResponse response =
        fetchStreamAndRelease(coreDriverApi, rs.getResultSetHandle());
    if (response.hasStream() && !response.getStream().getValue().isEmpty()) {
      return resultSetFromResponse(
          statement, queryId, response, rs.getResultDescriptor().getColumnsList());
    }
    return null;
  }

  /**
   * Wraps an existing result set with a {@link ConvertingRowReader} that projects/filters rows into
   * a new column layout. The source result set is detached (marked closed and unregistered from the
   * statement) and its {@link RowReader} ownership transfers to the new result set.
   */
  public static InternalResultSet wrapWithConverter(
      SnowflakeStatementImpl statement,
      SnowflakeResultSetImpl resultSet,
      SnowflakeResultSetMetaDataImpl metaData,
      RowConverter converter)
      throws SQLException {
    String queryID = metaData.getQueryID();
    RowReader sourceReader = resultSet.detachRowReader();
    String[] names = metaData.getColumnNames().toArray(new String[metaData.getColumnCount()]);
    ConvertingRowReader convertingReader = new ConvertingRowReader(sourceReader, names, converter);

    return new SnowflakeResultSetImpl(statement, queryID, convertingReader, metaData, true);
  }

  /** Creates a result set backed by pre-built in-memory rows. */
  public static InternalResultSet createFromRows(
      SnowflakeStatementImpl statement,
      SnowflakeResultSetMetaDataImpl metaData,
      Object[][] rows,
      boolean ownsStatement)
      throws SQLException {
    String queryId = metaData.getQueryID();
    String[] names = metaData.getColumnNames().toArray(new String[metaData.getColumnCount()]);
    InMemoryRowReader rowReader = new InMemoryRowReader(names, rows);
    return new SnowflakeResultSetImpl(statement, queryId, rowReader, metaData, ownsStatement);
  }

  /** Creates a result set with the given metadata and no rows. */
  public static InternalResultSet createEmpty(
      SnowflakeStatementImpl statement,
      SnowflakeResultSetMetaDataImpl metaData,
      boolean ownsStatement)
      throws SQLException {
    String queryId = metaData.getQueryID();
    String[] names = metaData.getColumnNames().toArray(new String[metaData.getColumnCount()]);
    InMemoryRowReader rowReader = new InMemoryRowReader(names, new Object[][] {});
    return new SnowflakeResultSetImpl(statement, queryId, rowReader, metaData, ownsStatement);
  }

  private static InternalResultSet resultSetFromResponse(
      SnowflakeStatementImpl statement,
      String queryId,
      ResultSetGetStreamResponse response,
      List<ColumnMetadata> columns)
      throws SQLException {
    byte[] streamPointerBytes = response.getStream().getValue().toByteArray();
    long pointer = ArrowStreamFactory.pointerFromBytes(streamPointerBytes);
    ArrowResources arrowResources = ArrowStreamFactory.createFromPointer(pointer);
    SnowflakeResultSetMetaDataImpl metaData = SnowflakeResultSetMetaDataImpl.from(queryId, columns);
    DataConversionContext conversionContext = buildConversionContext(statement);
    ArrowRowReader rowReader = new ArrowRowReader(arrowResources, conversionContext);
    return new SnowflakeResultSetImpl(statement, queryId, rowReader, metaData, false);
  }

  private static DataConversionContext buildConversionContext(SnowflakeStatementImpl statement)
      throws SQLException {
    return SessionDataConversionContext.fromConnection(
        ProtobufApis.coreDriverApi,
        statement.getConnection().unwrap(InternalSnowflakeConnection.class).getHandle());
  }

  private static ResultSetGetStreamResponse fetchStreamAndRelease(
      CoreDriverApi coreDriverApi, ResultSetHandle handle) throws SQLException {
    try {
      ResultSetGetStreamResponse response = coreDriverApi.resultSetGetStream(handle);
      releaseHandle(coreDriverApi, handle);
      return response;
    } catch (SQLException e) {
      releaseHandle(coreDriverApi, handle);
      throw e;
    }
  }

  private static void releaseHandle(CoreDriverApi coreDriverApi, ResultSetHandle handle) {
    try {
      coreDriverApi.resultSetRelease(handle);
    } catch (SQLException e) {
      logger.warn("Failed to release ResultSet handle", e);
    }
  }
}
