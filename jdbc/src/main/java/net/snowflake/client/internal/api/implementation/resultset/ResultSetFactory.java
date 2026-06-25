package net.snowflake.client.internal.api.implementation.resultset;

import java.sql.SQLException;
import java.util.List;
import lombok.AccessLevel;
import lombok.NoArgsConstructor;
import net.snowflake.client.internal.api.implementation.connection.InternalSnowflakeConnection;
import net.snowflake.client.internal.api.implementation.statement.SnowflakeStatementImpl;
import net.snowflake.client.internal.core.arrow.ArrowStreamFactory;
import net.snowflake.client.internal.log.SFLogger;
import net.snowflake.client.internal.log.SFLoggerFactory;
import net.snowflake.client.internal.unicore.CoreDriverApi;
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

  private static InternalResultSet resultSetFromResponse(
      SnowflakeStatementImpl statement,
      String queryId,
      ResultSetGetStreamResponse response,
      List<ColumnMetadata> columns)
      throws SQLException {
    byte[] streamPointerBytes = response.getStream().getValue().toByteArray();
    long pointer = ArrowStreamFactory.pointerFromBytes(streamPointerBytes);
    return new SnowflakeResultSetImpl(statement, queryId, pointer, columns);
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
