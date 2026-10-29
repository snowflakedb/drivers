package net.snowflake.client.internal.core.arrow.cursor;

import java.nio.ByteBuffer;

/**
 * Supplies encoded schema bytes and Arrow C batches to {@link NativeBatchReader}. Production uses
 * {@link net.snowflake.client.internal.unicore.NativeArrowStream}; tests supply a recording
 * implementation without JNI.
 */
interface NativeBatchSource extends AutoCloseable {
  byte[] schema();

  /**
   * Fills {@code slots} with the next batch's buffers and returns its row count, or {@link
   * net.snowflake.client.internal.unicore.NativeArrowStream#END_OF_STREAM} when the stream is
   * drained. A zero row count is a legal batch.
   */
  int nextBatch(ByteBuffer[] slots);

  @Override
  void close();
}
