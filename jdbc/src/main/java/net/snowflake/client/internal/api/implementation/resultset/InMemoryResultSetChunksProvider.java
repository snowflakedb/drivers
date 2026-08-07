package net.snowflake.client.internal.api.implementation.resultset;

import java.util.List;
import lombok.RequiredArgsConstructor;
import net.snowflake.client.api.resultset.SnowflakeResultSetSerializable;
import net.snowflake.client.internal.api.implementation.parameters.ParametersRegistry;
import net.snowflake.client.internal.unicore.CoreDriverApi;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.ColumnMetadata;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.ResultChunk;

/**
 * Chunk metadata held in memory by a sessionless result set that was rehydrated from a {@link
 * SnowflakeResultSetSerializable}. Re-slicing these chunks re-partitions the same underlying data
 * (no core handle, no backend re-fetch), mirroring snowflake-jdbc where the derived result set
 * keeps a reference to the serializable it was built from.
 *
 * <p>The lists are the serializable's own immutable fields, so they are held by reference rather
 * than re-copied here.
 */
@RequiredArgsConstructor
class InMemoryResultSetChunksProvider implements ResultSetChunksProvider {

  private final CoreDriverApi coreDriverApi;
  private final List<ResultChunk> chunks;
  private final List<ColumnMetadata> columns;
  private final String queryId;
  private final ParametersRegistry parameters;

  @Override
  public List<SnowflakeResultSetSerializable> getChunks(long maxSizeInBytes) {
    return SnowflakeResultSetSerializableImpl.splitBySize(
        coreDriverApi, chunks, columns, queryId, parameters, maxSizeInBytes);
  }

  @Override
  public void release() {
    // In-memory chunks own no core-side resources.
  }
}
