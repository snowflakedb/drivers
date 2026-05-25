package net.snowflake.client.internal.api.implementation.resultset;

import com.google.protobuf.ByteString;
import java.nio.ByteBuffer;
import java.nio.ByteOrder;
import java.sql.SQLException;
import lombok.AccessLevel;
import lombok.NoArgsConstructor;
import net.snowflake.client.internal.api.implementation.statement.SnowflakeStatementImpl;
import net.snowflake.client.internal.log.SFLogger;
import net.snowflake.client.internal.log.SFLoggerFactory;
import net.snowflake.client.internal.unicore.CoreDriverApi;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.ResultSetGetStreamResponse;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.ResultSetHandle;

/**
 * Creates {@link SnowflakeResultSetImpl} instances from a core-driver {@link ResultSetHandle}.
 *
 * <p>Encapsulates the fetch-stream-and-release lifecycle: {@code resultSetGetStream} takes
 * ownership of the prebuilt Arrow stream (one-shot), so the handle is released immediately after,
 * regardless of success or failure.
 */
@NoArgsConstructor(access = AccessLevel.PRIVATE)
public class ResultSetFactory {

  private static final SFLogger logger = SFLoggerFactory.getLogger(ResultSetFactory.class);

  public static InternalResultSet create(
      CoreDriverApi coreDriverApi, SnowflakeStatementImpl statement, ResultSetHandle handle)
      throws SQLException {
    ResultSetGetStreamResponse response = fetchStreamAndRelease(coreDriverApi, handle);
    return resultSetFromResponse(statement, response);
  }

  public static InternalResultSet createIfHasStream(
      CoreDriverApi coreDriverApi, SnowflakeStatementImpl statement, ResultSetHandle handle)
      throws SQLException {
    ResultSetGetStreamResponse response = fetchStreamAndRelease(coreDriverApi, handle);
    if (response.hasStream() && !response.getStream().getValue().isEmpty()) {
      return resultSetFromResponse(statement, response);
    }
    return null;
  }

  private static InternalResultSet resultSetFromResponse(
      SnowflakeStatementImpl statement, ResultSetGetStreamResponse response) throws SQLException {
    ByteString streamPointerBytes = response.getStream().getValue();
    // TODO Check how will this behave on AIX (Big Endian)
    long pointer =
        ByteBuffer.wrap(streamPointerBytes.toByteArray()).order(ByteOrder.LITTLE_ENDIAN).getLong();
    return new SnowflakeResultSetImpl(statement, pointer);
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
