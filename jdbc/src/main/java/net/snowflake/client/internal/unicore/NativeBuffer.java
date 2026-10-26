package net.snowflake.client.internal.unicore;

import java.nio.ByteBuffer;
import lombok.AccessLevel;
import lombok.AllArgsConstructor;
import net.snowflake.client.internal.api.implementation.exception.SFSQLException;

@AllArgsConstructor(access = AccessLevel.PRIVATE)
public final class NativeBuffer implements AutoCloseable {
  private long handle;
  private final long address;

  public static NativeBuffer copyOf(byte[] bytes) {
    NativeLibraryLoader.init();
    long[] allocation = new long[2];
    String error = nativeAllocate(bytes, allocation);
    if (error != null) {
      throw new SFSQLException(error);
    }
    if (allocation[0] == 0 || allocation[1] == 0) {
      throw new SFSQLException("Failed to allocate native memory for binding data");
    }
    return new NativeBuffer(allocation[0], allocation[1]);
  }

  public long address() {
    if (handle == 0) {
      throw new SFSQLException("Native buffer is closed");
    }
    return address;
  }

  /**
   * Aliases {@code length} bytes at {@code address} without copying or checking where the memory
   * came from. The caller keeps {@code [address, address + length)} readable for the life of the
   * returned {@link ByteBuffer}.
   */
  public static ByteBuffer view(long address, int length) {
    if (address == 0 || length < 0) {
      throw new SFSQLException("Native buffer view is not valid");
    }
    NativeLibraryLoader.init();
    ByteBuffer buffer = nativeView(address, length);
    if (buffer == null) {
      throw new SFSQLException("Failed to view native buffer");
    }
    return buffer;
  }

  @Override
  public void close() {
    if (handle == 0) {
      return;
    }
    nativeRelease(handle);
    handle = 0;
  }

  /**
   * Total bytes held by live {@link NativeBuffer} handles across the process. Exposed for leak
   * assertions in test suites that run in a package other than this one.
   */
  public static long allocatedBytes() {
    NativeLibraryLoader.init();
    return nativeAllocatedBytes();
  }

  private static native String nativeAllocate(byte[] bytes, long[] out);

  private static native void nativeRelease(long handle);

  private static native long nativeAllocatedBytes();

  private static native ByteBuffer nativeView(long address, int length);
}
