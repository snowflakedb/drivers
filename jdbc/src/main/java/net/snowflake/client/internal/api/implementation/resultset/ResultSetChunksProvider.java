package net.snowflake.client.internal.api.implementation.resultset;

import java.util.List;
import net.snowflake.client.api.resultset.SnowflakeResultSetSerializable;

/** The chunk metadata backing a result set, sliceable into pieces for distributed processing. */
interface ResultSetChunksProvider {

  /** Slices the chunks into serializable pieces, each below {@code maxSizeInBytes}. */
  List<SnowflakeResultSetSerializable> getChunks(long maxSizeInBytes);

  /** Releases any core-side resources. No-op when backed by in-memory chunks. */
  void release();
}
