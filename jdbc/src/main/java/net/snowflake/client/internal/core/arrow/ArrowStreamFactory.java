package net.snowflake.client.internal.core.arrow;

import java.nio.ByteBuffer;
import java.nio.ByteOrder;
import net.snowflake.client.internal.core.arrow.cursor.ArrowResources;
import org.apache.arrow.c.ArrowArrayStream;
import org.apache.arrow.c.Data;
import org.apache.arrow.memory.RootAllocator;

public class ArrowStreamFactory {

  public static long pointerFromBytes(byte[] bytes) {
    // TODO Check how will this behave on AIX (Big Endian)
    return ByteBuffer.wrap(bytes).order(ByteOrder.LITTLE_ENDIAN).getLong();
  }

  public static ArrowResources createFromPointer(long arrowStreamPointer) {
    ArrowArrayStream stream = ArrowArrayStream.wrap(arrowStreamPointer);
    RootAllocator allocator = new RootAllocator();
    return new ArrowResources(stream, allocator, Data.importArrayStream(allocator, stream));
  }
}
