package net.snowflake.client.internal.core.arrow.cursor;

import net.snowflake.client.internal.unicore.NativeArrowBatch;

/**
 * Supplies encoded schema bytes and Arrow C batches to {@link NativeBatchReader}. Production uses
 * {@link net.snowflake.client.internal.unicore.NativeArrowStream}; tests supply a recording
 * implementation without JNI.
 */
interface NativeBatchSource extends AutoCloseable {
  byte[] schema();

  NativeArrowBatch nextBatch();

  @Override
  void close();
}
