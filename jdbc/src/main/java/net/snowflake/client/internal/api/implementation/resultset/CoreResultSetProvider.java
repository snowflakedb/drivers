package net.snowflake.client.internal.api.implementation.resultset;

import java.util.List;
import lombok.AccessLevel;
import lombok.RequiredArgsConstructor;
import net.snowflake.client.api.resultset.SnowflakeResultSetSerializable;
import net.snowflake.client.internal.api.implementation.exception.CoreException;
import net.snowflake.client.internal.api.implementation.parameters.ParametersRegistry;
import net.snowflake.client.internal.log.SFLogger;
import net.snowflake.client.internal.log.SFLoggerFactory;
import net.snowflake.client.internal.unicore.CoreDriverApi;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.ResultSetGetChunksResponse;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.ResultSetGetStreamResponse;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.ResultSetHandle;

/**
 * Abstraction over core-owned result set.
 *
 * <p>The handle keeps the query's data alive in core, so both the Arrow stream and the chunk
 * metadata can be requested repeatedly until {@link #release()} is called.
 */
@RequiredArgsConstructor(access = AccessLevel.PACKAGE)
class CoreResultSetProvider implements ResultSetChunksProvider {

  private static final SFLogger logger = SFLoggerFactory.getLogger(CoreResultSetProvider.class);

  private final CoreDriverApi coreDriverApi;
  private final ResultSetHandle handle;
  private final String queryId;
  private final ParametersRegistry parameters;

  ResultSetGetStreamResponse getStream() {
    return coreDriverApi.resultSetGetStream(handle);
  }

  @Override
  public List<SnowflakeResultSetSerializable> getChunks(long maxSizeInBytes) {
    // The handle still holds the query's chunk metadata in core, so this reads it locally instead
    // of re-fetching the result from the backend.
    ResultSetGetChunksResponse chunks = coreDriverApi.resultSetGetChunks(handle);
    return SnowflakeResultSetSerializableImpl.splitBySize(
        coreDriverApi,
        chunks.getChunksList(),
        chunks.getColumnsList(),
        queryId,
        parameters,
        maxSizeInBytes);
  }

  /** Best-effort release of the backing handle; logs rather than throwing on failure. */
  @Override
  public void release() {
    try {
      coreDriverApi.resultSetRelease(handle);
    } catch (CoreException e) {
      logger.warn("Failed to release ResultSet handle: {}", e.getClass().getName());
      logger.debug("Failed to release ResultSet handle", e);
    }
  }
}
