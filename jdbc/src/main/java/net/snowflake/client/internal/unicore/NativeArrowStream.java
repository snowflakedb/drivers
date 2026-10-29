package net.snowflake.client.internal.unicore;

import java.nio.ByteBuffer;
import java.util.concurrent.atomic.AtomicLong;
import java.util.function.Supplier;
import net.snowflake.client.api.exception.ErrorCode;
import net.snowflake.client.internal.api.implementation.exception.SFSQLException;

/**
 * Owns an Arrow C stream inside {@code jdbc_bridge}. Direct buffers from {@link
 * #nextBatch(ByteBuffer[])} alias the current native {@code RecordBatch} and are invalid after the
 * next {@code nextBatch} or {@link #close()}.
 *
 * <p>{@link #close()} does not wait for an in-flight {@link #nextBatch(ByteBuffer[])}: it detaches
 * the native stream without waiting for the download, and the racing fetch either fails or returns
 * buffers the native side still keeps alive. That covers a concurrent fetch only. Buffers an
 * earlier fetch already handed out are freed by {@code close()} with no synchronization against a
 * reader, so reading cells on one thread while another closes this stream reads freed memory.
 */
public final class NativeArrowStream implements AutoCloseable {
  /** Returned by {@link #nextBatch(ByteBuffer[])} when the stream is drained. */
  public static final int END_OF_STREAM = -1;

  /** Zero once closed, so the native handle is released exactly once. */
  private final AtomicLong handle;

  private NativeArrowStream(long handle) {
    this.handle = new AtomicLong(handle);
  }

  /**
   * Registers {@code pointer}. The caller passes {@code 0} or the result of {@code Box::into_raw}
   * for an {@code FFI_ArrowArrayStream}, and does not free {@code pointer} after this returns. This
   * consumes the allocation, including when the native reader fails. A null or misaligned pointer
   * is not consumed.
   */
  public static NativeArrowStream open(long pointer) {
    NativeLibraryLoader.init();
    long handle = wrap(() -> nativeOpen(pointer));
    if (handle == 0) {
      throw arrowFailure(new IllegalStateException("Native Arrow stream did not return a handle"));
    }
    return new NativeArrowStream(handle);
  }

  /**
   * Returns the schema in the private little-endian layout from {@code encode_schema}: a field
   * count, then each field's name, metadata, nullability, and type tag, followed by the decimal,
   * struct, or fixed-size list payload. These bytes are not an Arrow IPC message.
   */
  public byte[] schema() {
    final long open = openHandle();
    return wrap(() -> nativeSchema(open));
  }

  /**
   * Fills {@code slots} with the next batch's buffers and returns its row count, or {@link
   * #END_OF_STREAM} once the stream is drained. A zero row count is a legal batch.
   *
   * <p>{@code slots} holds {@code Column.BUFFERS_PER_NODE} entries per Arrow node and is reused for
   * every batch, so the buffers it held before this call become invalid whether or not it succeeds.
   */
  public int nextBatch(ByteBuffer[] slots) {
    // Not routed through wrap(): a Supplier would box the row count on every batch,
    // which is the cost the flat slot hand-off exists to remove.
    try {
      return nativeNextBatch(openHandle(), slots);
    } catch (IllegalStateException failure) {
      throw arrowFailure(failure);
    }
  }

  @Override
  public void close() {
    long open = handle.getAndSet(0);
    if (open != 0) {
      nativeClose(open);
    }
  }

  /**
   * Returns the native handle, or throws when this stream is already closed. A {@link #close()}
   * landing after this read is not an error here: the native registry has dropped the handle, so
   * the call fails with a closed-stream exception instead of touching freed memory.
   */
  private long openHandle() {
    long open = handle.get();
    if (open == 0) {
      throw closedStream();
    }
    return open;
  }

  private static <T> T wrap(Supplier<T> action) {
    try {
      return action.get();
    } catch (IllegalStateException failure) {
      throw arrowFailure(failure);
    }
  }

  private static native long nativeOpen(long pointer);

  private static native byte[] nativeSchema(long handle);

  private static native int nativeNextBatch(long handle, ByteBuffer[] slots);

  private static native void nativeClose(long handle);

  // Matches arrow_stream::UNSUPPORTED_COLUMN_TAG.
  private static final String UNSUPPORTED_COLUMN_TAG = "UNSUPPORTED_COLUMN:";

  private static SFSQLException closedStream() {
    return arrowFailure(new IllegalStateException("Native Arrow stream is closed"));
  }

  static SFSQLException arrowFailure(IllegalStateException failure) {
    String message = failure.getMessage() == null ? "Arrow stream failed" : failure.getMessage();
    ErrorCode code;
    String detail;
    if (message.startsWith(UNSUPPORTED_COLUMN_TAG)) {
      code = ErrorCode.FEATURE_UNSUPPORTED;
      detail = message.substring(UNSUPPORTED_COLUMN_TAG.length());
    } else {
      code = ErrorCode.INTERNAL_ERROR;
      detail = message;
    }
    SFSQLException translated = new SFSQLException(code, detail);
    translated.initCause(failure);
    return translated;
  }
}
