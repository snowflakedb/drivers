package net.snowflake.client.internal.api.implementation.connection;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.mockito.Mockito.mock;
import static org.mockito.Mockito.when;

import com.google.protobuf.ByteString;
import java.io.InputStream;
import net.snowflake.client.internal.unicore.CoreDriverApi;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.ConnectionDownloadStreamChunkResponse;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.ConnectionDownloadStreamCloseResponse;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.DownloadStreamHandle;
import org.junit.jupiter.api.Test;

class ChunkedDownloadInputStreamTest {

  private static final int CHUNK_SIZE = 16;

  @Test
  void shouldSkipEmptyChunksAndReturnUnsignedBytesUntilEof() throws Exception {
    CoreDriverApi api = mock(CoreDriverApi.class);
    DownloadStreamHandle handle = DownloadStreamHandle.newBuilder().setId(7).setMagic(77).build();
    when(api.connectionDownloadStreamChunk(handle, CHUNK_SIZE))
        .thenReturn(chunk(new byte[0], false), chunk(new byte[] {(byte) 0xFF, 0x41, 0x42}, true));
    when(api.connectionDownloadStreamClose(handle))
        .thenReturn(ConnectionDownloadStreamCloseResponse.getDefaultInstance());

    try (InputStream stream =
        new ChunkedDownloadInputStream(
            api, handle, CHUNK_SIZE, mock(SnowflakeConnectionImpl.class))) {
      byte[] buf = new byte[4];
      assertEquals(0, stream.read(buf, 0, 0));
      assertEquals(0xFF, stream.read());
      assertEquals(2, stream.read(buf, 1, 3));
      assertEquals((byte) 0x41, buf[1]);
      assertEquals((byte) 0x42, buf[2]);
      assertEquals(-1, stream.read());
    }
  }

  private static ConnectionDownloadStreamChunkResponse chunk(byte[] data, boolean eof) {
    return ConnectionDownloadStreamChunkResponse.newBuilder()
        .setData(ByteString.copyFrom(data))
        .setEof(eof)
        .build();
  }
}
