package net.snowflake.client.internal.api.implementation.connection;

import java.io.IOException;
import java.io.InputStream;
import java.sql.SQLException;
import java.util.Set;
import net.snowflake.client.internal.log.SFLogger;
import net.snowflake.client.internal.log.SFLoggerFactory;
import net.snowflake.client.internal.unicore.CoreDriverApi;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.ConnectionDownloadStreamChunkResponse;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.DownloadStreamHandle;

/**
 * Lazily pulls chunks via {@link CoreDriverApi#connectionDownloadStreamChunk} as the caller reads,
 * bounding JDBC-side memory to ~one chunk regardless of file size — mirroring the chunked upload
 * path rather than materializing the whole file up front.
 */
class ChunkedDownloadInputStream extends InputStream {

  private static final SFLogger logger =
      SFLoggerFactory.getLogger(ChunkedDownloadInputStream.class);

  private final CoreDriverApi coreDriverApi;
  private final DownloadStreamHandle downloadHandle;
  private final int streamChunkSize;
  private final Set<ChunkedDownloadInputStream> openDownloadStreams;

  private byte[] buffer = new byte[0];
  private int bufferPos;
  private boolean eof;
  private boolean closed;

  ChunkedDownloadInputStream(
      CoreDriverApi coreDriverApi,
      DownloadStreamHandle downloadHandle,
      int streamChunkSize,
      Set<ChunkedDownloadInputStream> openDownloadStreams) {
    this.coreDriverApi = coreDriverApi;
    this.downloadHandle = downloadHandle;
    this.streamChunkSize = streamChunkSize;
    this.openDownloadStreams = openDownloadStreams;
  }

  @Override
  public int read() throws IOException {
    byte[] singleByte = new byte[1];
    int n = read(singleByte, 0, 1);
    return n == -1 ? -1 : (singleByte[0] & 0xFF);
  }

  @Override
  public int read(byte[] b, int off, int len) throws IOException {
    if (len == 0) {
      return 0;
    }
    while (bufferPos >= buffer.length) {
      if (eof) {
        return -1;
      }
      fillBuffer();
    }
    int n = Math.min(len, buffer.length - bufferPos);
    System.arraycopy(buffer, bufferPos, b, off, n);
    bufferPos += n;
    return n;
  }

  private void fillBuffer() throws IOException {
    try {
      ConnectionDownloadStreamChunkResponse chunk =
          coreDriverApi.connectionDownloadStreamChunk(downloadHandle, streamChunkSize);
      buffer = chunk.getData().toByteArray();
      bufferPos = 0;
      eof = chunk.getEof();
    } catch (SQLException e) {
      throw new IOException("Failed to read download stream chunk: " + e.getMessage(), e);
    }
  }

  @Override
  public void close() throws IOException {
    logger.info("downloadStream: close entry");
    if (closed) {
      return;
    }
    try {
      coreDriverApi.connectionDownloadStreamClose(downloadHandle);
    } catch (SQLException e) {
      throw new IOException("Failed to close download stream: " + e.getMessage(), e);
    } finally {
      closed = true;
      openDownloadStreams.remove(this);
      logger.info("downloadStream: close exit");
    }
  }
}
