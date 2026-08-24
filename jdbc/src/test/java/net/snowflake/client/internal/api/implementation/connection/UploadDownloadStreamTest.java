package net.snowflake.client.internal.api.implementation.connection;

import static net.snowflake.jdbc.utils.IoTestUtils.readAllBytes;
import static org.junit.jupiter.api.Assertions.assertArrayEquals;
import static org.junit.jupiter.api.Assertions.assertDoesNotThrow;
import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertFalse;
import static org.junit.jupiter.api.Assertions.assertInstanceOf;
import static org.junit.jupiter.api.Assertions.assertSame;
import static org.junit.jupiter.api.Assertions.assertThrows;
import static org.junit.jupiter.api.Assertions.assertTrue;
import static org.mockito.ArgumentMatchers.any;
import static org.mockito.ArgumentMatchers.anyBoolean;
import static org.mockito.ArgumentMatchers.anyLong;
import static org.mockito.ArgumentMatchers.anyString;
import static org.mockito.ArgumentMatchers.eq;
import static org.mockito.Mockito.mock;
import static org.mockito.Mockito.never;
import static org.mockito.Mockito.times;
import static org.mockito.Mockito.verify;
import static org.mockito.Mockito.when;

import com.google.protobuf.ByteString;
import java.io.ByteArrayInputStream;
import java.io.IOException;
import java.io.InputStream;
import java.sql.SQLException;
import java.util.Arrays;
import java.util.Properties;
import net.snowflake.client.api.connection.DownloadStreamConfig;
import net.snowflake.client.api.connection.SnowflakeConnection;
import net.snowflake.client.api.connection.UploadStreamConfig;
import net.snowflake.client.api.exception.ErrorCode;
import net.snowflake.client.internal.api.decorator.Telemetry;
import net.snowflake.client.internal.api.implementation.exception.CoreException;
import net.snowflake.client.internal.api.implementation.exception.DriverRuntimeException;
import net.snowflake.client.internal.unicore.CoreDriverApi;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.ConnectionDownloadStreamBeginResponse;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.ConnectionDownloadStreamChunkResponse;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.ConnectionDownloadStreamCloseResponse;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.ConnectionHandle;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.ConnectionInitResponse;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.ConnectionNewResponse;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.ConnectionSetOptionsResponse;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.ConnectionUploadStreamAbortResponse;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.ConnectionUploadStreamBeginResponse;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.ConnectionUploadStreamFinishResponse;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.DatabaseHandle;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.DatabaseInitResponse;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.DatabaseNewResponse;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.DownloadStreamHandle;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.DriverException;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.ErrorKind;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.UploadStreamHandle;
import org.junit.jupiter.api.BeforeEach;
import org.junit.jupiter.api.Test;
import org.mockito.ArgumentCaptor;

/**
 * Unit tests for {@link SnowflakeConnectionImpl#uploadStream} and {@link
 * SnowflakeConnectionImpl#downloadStream}.
 *
 * <p>The tests exercise the JDBC wiring: PUT SQL is synthesized on the Java side from the
 * structured uploadStream parameters and forwarded via {@code
 * coreDriverApi.connectionUploadStreamBegin}; the source {@link InputStream} is drained in {@code
 * STREAM_CHUNK_SIZE}-sized pieces, each sent through {@code connectionUploadStreamChunk}, before
 * {@code connectionUploadStreamFinish} closes the session (or {@code connectionUploadStreamAbort}
 * on a read failure). The download path mirrors this: {@code connectionDownloadStreamBegin} opens
 * the session and the returned {@link InputStream} lazily pulls chunks via {@code
 * connectionDownloadStreamChunk} until {@code eof}, closing the session via {@code
 * connectionDownloadStreamClose}.
 */
class UploadDownloadStreamTest {

  private static final DatabaseHandle DB_HANDLE =
      DatabaseHandle.newBuilder().setId(1).setMagic(100).build();
  private static final ConnectionHandle CONN_HANDLE =
      ConnectionHandle.newBuilder().setId(2).setMagic(200).build();
  private static final UploadStreamHandle UPLOAD_HANDLE =
      UploadStreamHandle.newBuilder().setId(10).setMagic(1000).build();
  private static final DownloadStreamHandle DOWNLOAD_HANDLE =
      DownloadStreamHandle.newBuilder().setId(20).setMagic(2000).build();

  private CoreDriverApi mockCoreApi;
  private SnowflakeConnectionImpl connection;

  @BeforeEach
  void setUp() throws Exception {
    mockCoreApi = mock(CoreDriverApi.class);
    when(mockCoreApi.databaseNew())
        .thenReturn(DatabaseNewResponse.newBuilder().setDbHandle(DB_HANDLE).build());
    when(mockCoreApi.databaseInit(any())).thenReturn(DatabaseInitResponse.getDefaultInstance());
    when(mockCoreApi.connectionNew())
        .thenReturn(ConnectionNewResponse.newBuilder().setConnHandle(CONN_HANDLE).build());
    when(mockCoreApi.connectionSetOptions(any(), any()))
        .thenReturn(ConnectionSetOptionsResponse.getDefaultInstance());
    when(mockCoreApi.connectionInit(any(), any(), any()))
        .thenReturn(ConnectionInitResponse.getDefaultInstance());

    when(mockCoreApi.connectionUploadStreamBegin(any(), anyString()))
        .thenReturn(
            ConnectionUploadStreamBeginResponse.newBuilder()
                .setUploadHandle(UPLOAD_HANDLE)
                .build());
    when(mockCoreApi.connectionUploadStreamFinish(any()))
        .thenReturn(ConnectionUploadStreamFinishResponse.getDefaultInstance());
    when(mockCoreApi.connectionUploadStreamAbort(any()))
        .thenReturn(ConnectionUploadStreamAbortResponse.getDefaultInstance());

    when(mockCoreApi.connectionDownloadStreamBegin(any(), anyString(), anyString(), anyBoolean()))
        .thenReturn(
            ConnectionDownloadStreamBeginResponse.newBuilder()
                .setDownloadHandle(DOWNLOAD_HANDLE)
                .build());
    when(mockCoreApi.connectionDownloadStreamClose(any()))
        .thenReturn(ConnectionDownloadStreamCloseResponse.getDefaultInstance());

    Properties props = new Properties();
    props.setProperty("account", "test_account");
    props.setProperty("user", "test_user");
    props.setProperty("password", System.getenv().getOrDefault("SNOWFLAKE_TEST_PASSWORD", "dummy"));
    connection =
        new SnowflakeConnectionImpl(
            "jdbc:snowflake://test.snowflakecomputing.com", props, mockCoreApi);
  }

  // ---------------------------------------------------------------------------
  // SQL synthesis (pure function, no mocks needed)
  // ---------------------------------------------------------------------------

  @Test
  void shouldOmitAutoCompressClauseWhenCompressIsDefault() {
    String sql = SnowflakeConnectionImpl.buildPutSql("@my_stage", "data.csv", null, true);
    assertEquals("PUT 'file:///data.csv' @my_stage OVERWRITE = TRUE", sql);
  }

  @Test
  void shouldAddAutoCompressFalseWhenCompressIsFalse() {
    String sql = SnowflakeConnectionImpl.buildPutSql("@my_stage", "data.csv", null, false);
    assertEquals("PUT 'file:///data.csv' @my_stage AUTO_COMPRESS = FALSE OVERWRITE = TRUE", sql);
  }

  @Test
  void shouldAppendPrefixToStagePath() {
    String sql = SnowflakeConnectionImpl.buildPutSql("@my_stage", "data.csv", "subdir", true);
    assertEquals("PUT 'file:///data.csv' @my_stage/subdir OVERWRITE = TRUE", sql);
  }

  @Test
  void shouldAvoidDoubleSlashWhenStageHasTrailingSlash() {
    String sql = SnowflakeConnectionImpl.buildPutSql("@my_stage/", "data.csv", "subdir", true);
    assertEquals("PUT 'file:///data.csv' @my_stage/subdir OVERWRITE = TRUE", sql);
  }

  @Test
  void shouldTreatEmptyPrefixAsNoPrefix() {
    String sql = SnowflakeConnectionImpl.buildPutSql("@my_stage", "data.csv", "", true);
    assertEquals("PUT 'file:///data.csv' @my_stage OVERWRITE = TRUE", sql);
  }

  // ---------------------------------------------------------------------------
  // uploadStream — wiring: Begin -> Chunk* -> Finish (or Abort on read failure)
  // ---------------------------------------------------------------------------

  @Test
  void shouldForwardSynthesizedSqlAndBytesForDefaultConfig() throws Exception {
    byte[] payload = new byte[] {1, 2, 3};
    connection.uploadStream("@my_stage", "data.csv", new ByteArrayInputStream(payload));

    ArgumentCaptor<String> sqlCaptor = ArgumentCaptor.forClass(String.class);
    verify(mockCoreApi).connectionUploadStreamBegin(eq(CONN_HANDLE), sqlCaptor.capture());
    assertEquals("PUT 'file:///data.csv' @my_stage OVERWRITE = TRUE", sqlCaptor.getValue());

    assertArrayEquals(payload, capturedUploadedChunk());
    verify(mockCoreApi).connectionUploadStreamFinish(UPLOAD_HANDLE);
  }

  @Test
  void shouldSynthesizeAutoCompressFalseWhenCompressDataIsFalse() throws Exception {
    UploadStreamConfig config = UploadStreamConfig.builder().setCompressData(false).build();
    connection.uploadStream("@my_stage", "data.csv", new ByteArrayInputStream(new byte[0]), config);

    ArgumentCaptor<String> sqlCaptor = ArgumentCaptor.forClass(String.class);
    verify(mockCoreApi).connectionUploadStreamBegin(eq(CONN_HANDLE), sqlCaptor.capture());
    assertTrue(
        sqlCaptor.getValue().contains("AUTO_COMPRESS = FALSE"),
        "synthesized SQL must carry AUTO_COMPRESS = FALSE: " + sqlCaptor.getValue());
  }

  @Test
  void shouldAppendDestPrefixToStagePathOnUpload() throws Exception {
    UploadStreamConfig config = UploadStreamConfig.builder().setDestPrefix("inbox").build();
    connection.uploadStream("@my_stage", "data.csv", new ByteArrayInputStream(new byte[0]), config);

    ArgumentCaptor<String> sqlCaptor = ArgumentCaptor.forClass(String.class);
    verify(mockCoreApi).connectionUploadStreamBegin(eq(CONN_HANDLE), sqlCaptor.capture());
    assertTrue(
        sqlCaptor.getValue().contains("@my_stage/inbox"),
        "stage path must include destPrefix: " + sqlCaptor.getValue());
  }

  @Test
  void shouldReadAllBytesFromInputStream() throws Exception {
    byte[] expected = new byte[] {1, 2, 3, 4, 5};
    connection.uploadStream("@s", "f", new ByteArrayInputStream(expected));

    assertArrayEquals(expected, capturedUploadedChunk());
    verify(mockCoreApi).connectionUploadStreamFinish(UPLOAD_HANDLE);
  }

  @Test
  void shouldDefaultToCompressDataTrueWhenConfigIsNull() throws Exception {
    connection.uploadStream("@my_stage", "data.csv", new ByteArrayInputStream(new byte[0]), null);

    ArgumentCaptor<String> sqlCaptor = ArgumentCaptor.forClass(String.class);
    verify(mockCoreApi).connectionUploadStreamBegin(eq(CONN_HANDLE), sqlCaptor.capture());
    // null config => compressData defaults to true => no AUTO_COMPRESS = FALSE clause.
    assertFalse(
        sqlCaptor.getValue().contains("AUTO_COMPRESS = FALSE"),
        "null config must default to compressData=true: " + sqlCaptor.getValue());
  }

  @Test
  void shouldAbortAndWrapIoExceptionAsSqlException() throws Exception {
    InputStream brokenStream =
        new InputStream() {
          @Override
          public int read() throws IOException {
            throw new IOException("simulated IO failure");
          }
        };

    // uploadStream is a @JdbcBoundary method: the raw impl throws an unchecked carrier, and the
    // generated decorator reconstructs the checked SQLException the public contract promises.
    SnowflakeConnection boundary = new DecoratedSnowflakeConnectionImpl(connection, Telemetry.NOOP);
    SQLException thrown =
        assertThrows(SQLException.class, () -> boundary.uploadStream("@s", "f", brokenStream));
    assertTrue(
        thrown.getMessage().contains("simulated IO failure"),
        "exception message must surface the underlying IO failure: " + thrown.getMessage());
    assertInstanceOf(IOException.class, thrown.getCause());

    verify(mockCoreApi).connectionUploadStreamAbort(UPLOAD_HANDLE);
    verify(mockCoreApi, never()).connectionUploadStreamFinish(any());
  }

  @Test
  void shouldAbortAndRethrowRuntimeExceptionFromInputStreamWithoutWrapping() throws Exception {
    RuntimeException readFailure = new IllegalStateException("simulated runtime failure");
    InputStream brokenStream =
        new InputStream() {
          @Override
          public int read() {
            throw readFailure;
          }
        };

    IllegalStateException thrown =
        assertThrows(
            IllegalStateException.class, () -> connection.uploadStream("@s", "f", brokenStream));
    assertSame(
        readFailure,
        thrown,
        "the SQLException|RuntimeException catch arm must rethrow the original exception as-is,"
            + " not wrap it");

    verify(mockCoreApi).connectionUploadStreamAbort(UPLOAD_HANDLE);
    verify(mockCoreApi, never()).connectionUploadStreamFinish(any());
  }

  @Test
  void shouldCoalescePartialReadsIntoFullSizeChunksAndFlushTrailingRemainder() throws Exception {
    // Mirrors the private SnowflakeConnectionImpl.STREAM_CHUNK_SIZE constant.
    final int chunkSize = 8 * 1024 * 1024;
    final int trailing = 100;
    byte[] payload = new byte[chunkSize + trailing];
    for (int i = 0; i < payload.length; i++) {
      payload[i] = (byte) i;
    }
    // Returns at most half of chunkSize per call, forcing the upload loop to accumulate
    // several partial reads before a full chunk is ready to send.
    InputStream partialReadStream =
        new ByteArrayInputStream(payload) {
          @Override
          public synchronized int read(byte[] b, int off, int len) {
            return super.read(b, off, Math.min(len, chunkSize / 2));
          }
        };

    connection.uploadStream("@s", "f", partialReadStream);

    ArgumentCaptor<byte[]> dataCaptor = ArgumentCaptor.forClass(byte[].class);
    ArgumentCaptor<Integer> lengthCaptor = ArgumentCaptor.forClass(Integer.class);
    verify(mockCoreApi, times(2))
        .connectionUploadStreamChunk(
            eq(UPLOAD_HANDLE), dataCaptor.capture(), eq(0), lengthCaptor.capture());

    assertEquals(
        chunkSize,
        lengthCaptor.getAllValues().get(0),
        "first chunk must be full-size, proving partial reads were coalesced");
    assertEquals(
        trailing,
        lengthCaptor.getAllValues().get(1),
        "second chunk must carry only the trailing remainder");
    assertArrayEquals(
        Arrays.copyOfRange(payload, 0, chunkSize),
        Arrays.copyOf(dataCaptor.getAllValues().get(0), chunkSize));
    assertArrayEquals(
        Arrays.copyOfRange(payload, chunkSize, payload.length),
        Arrays.copyOf(dataCaptor.getAllValues().get(1), trailing));
    verify(mockCoreApi).connectionUploadStreamFinish(UPLOAD_HANDLE);
  }

  /**
   * Captures the single {@code connectionUploadStreamChunk} call triggered by the small-payload
   * upload tests above (each writes a payload well under {@code STREAM_CHUNK_SIZE}, so exactly one
   * chunk RPC fires) and returns the bytes actually sent — trimmed from the wrapper's reusable read
   * buffer down to {@code [0, length)}, since the buffer itself is larger than any test payload.
   */
  private byte[] capturedUploadedChunk() throws SQLException {
    ArgumentCaptor<byte[]> dataCaptor = ArgumentCaptor.forClass(byte[].class);
    ArgumentCaptor<Integer> lengthCaptor = ArgumentCaptor.forClass(Integer.class);
    verify(mockCoreApi)
        .connectionUploadStreamChunk(
            eq(UPLOAD_HANDLE), dataCaptor.capture(), eq(0), lengthCaptor.capture());
    return Arrays.copyOf(dataCaptor.getValue(), lengthCaptor.getValue());
  }

  // ---------------------------------------------------------------------------
  // downloadStream — wiring: Begin -> Chunk* (until eof) -> Close
  // ---------------------------------------------------------------------------

  @Test
  void shouldCallCoreApiWithDecompressFalseForDefaultConfig() throws Exception {
    byte[] expected = "downloaded content".getBytes();
    when(mockCoreApi.connectionDownloadStreamChunk(eq(DOWNLOAD_HANDLE), anyLong()))
        .thenReturn(
            ConnectionDownloadStreamChunkResponse.newBuilder()
                .setData(ByteString.copyFrom(expected))
                .setEof(true)
                .build());

    try (InputStream result = connection.downloadStream("@my_stage", "data.csv.gz")) {
      assertArrayEquals(expected, readAllBytes(result));
    }

    verify(mockCoreApi)
        .connectionDownloadStreamBegin(
            eq(CONN_HANDLE), eq("@my_stage"), eq("data.csv.gz"), eq(false));
    verify(mockCoreApi).connectionDownloadStreamClose(DOWNLOAD_HANDLE);
  }

  @Test
  void shouldForwardDecompressFlagWhenDecompressIsTrue() throws Exception {
    byte[] expected = "decompressed".getBytes();
    when(mockCoreApi.connectionDownloadStreamChunk(eq(DOWNLOAD_HANDLE), anyLong()))
        .thenReturn(
            ConnectionDownloadStreamChunkResponse.newBuilder()
                .setData(ByteString.copyFrom(expected))
                .setEof(true)
                .build());

    DownloadStreamConfig config = DownloadStreamConfig.builder().setDecompress(true).build();
    try (InputStream result = connection.downloadStream("@my_stage", "data.csv.gz", config)) {
      assertArrayEquals(expected, readAllBytes(result));
    }

    verify(mockCoreApi)
        .connectionDownloadStreamBegin(
            eq(CONN_HANDLE), eq("@my_stage"), eq("data.csv.gz"), eq(true));
    verify(mockCoreApi).connectionDownloadStreamClose(DOWNLOAD_HANDLE);
  }

  @Test
  void shouldWrapChunkReadFailureAsIoExceptionAndAllowEarlyCloseWithoutReachingEof()
      throws Exception {
    // The core facade surfaces failures as unchecked carriers (never checked SQLException), and
    // read()/close() run outside the decorator boundary — so ChunkedDownloadInputStream catches the
    // carrier base and wraps it as IOException, the only checked type an InputStream may throw.
    when(mockCoreApi.connectionDownloadStreamChunk(eq(DOWNLOAD_HANDLE), anyLong()))
        .thenThrow(new CoreException("chunk rpc failed"));

    // try-with-resources guarantees the stream is released even if an assertion below throws;
    // the ARM close() is a harmless third no-op (close() short-circuits once closed), so the
    // verify(times(1)) below still holds.
    try (InputStream result = connection.downloadStream("@my_stage", "data.csv.gz")) {
      IOException thrown = assertThrows(IOException.class, result::read);
      assertTrue(
          thrown.getMessage().contains("chunk rpc failed"),
          "exception message must surface the underlying chunk failure: " + thrown.getMessage());

      // The caller can still close the stream after a read failure, without ever reaching eof;
      // a second close() call must be a no-op rather than re-invoking the close RPC.
      result.close();
      result.close();

      verify(mockCoreApi, times(1)).connectionDownloadStreamClose(DOWNLOAD_HANDLE);
    }
  }

  @Test
  void shouldMarkDownloadStreamClosedEvenWhenCloseRpcFailsSoSecondCloseIsNoop() throws Exception {
    when(mockCoreApi.connectionDownloadStreamChunk(eq(DOWNLOAD_HANDLE), anyLong()))
        .thenReturn(
            ConnectionDownloadStreamChunkResponse.newBuilder()
                .setData(ByteString.copyFrom(new byte[0]))
                .setEof(true)
                .build());
    when(mockCoreApi.connectionDownloadStreamClose(DOWNLOAD_HANDLE))
        .thenThrow(new CoreException("close rpc failed"));

    // try-with-resources guarantees release even if an assertion below throws. By block exit the
    // stream is already closed by the explicit calls, so the ARM close() is a no-op that neither
    // re-invokes the RPC nor re-throws — verify(times(1)) still holds.
    try (InputStream result = connection.downloadStream("@my_stage", "data.csv.gz")) {
      IOException thrown = assertThrows(IOException.class, result::close);
      assertTrue(
          thrown.getMessage().contains("close rpc failed"),
          "exception message must surface the underlying close failure: " + thrown.getMessage());

      // Regression guard for the close()-ordering fix: the stream must be marked closed (and
      // deregistered from the leak-safety-net set) in a finally block, even though the close RPC
      // threw, so a second close() call is a silent no-op instead of re-invoking the RPC.
      assertDoesNotThrow(result::close);

      verify(mockCoreApi, times(1)).connectionDownloadStreamClose(DOWNLOAD_HANDLE);
    }
  }

  @Test
  void shouldReportMissingRemoteFileAsLegacyNoDataAtDownloadStreamThrowSite() {
    DriverException payload =
        DriverException.newBuilder()
            .setKind(ErrorKind.ERROR_KIND_REMOTE_FILE_NOT_FOUND)
            .setMessage("the file does not exist")
            .build();
    when(mockCoreApi.connectionDownloadStreamBegin(any(), anyString(), anyString(), anyBoolean()))
        .thenThrow(new CoreException(payload, null));

    DriverRuntimeException carrier =
        assertThrows(
            DriverRuntimeException.class,
            () -> connection.downloadStream("@my_stage", "missing.csv"));
    SQLException surfaced = carrier.toSQLException();
    assertEquals(ErrorCode.FILE_NOT_FOUND.getMessageCode(), surfaced.getErrorCode());
    assertEquals("02000", surfaced.getSQLState());
    assertEquals("File not found: missing.csv", surfaced.getMessage());
  }

  @Test
  void shouldPropagateNonMissingFileBeginFailureUnchanged() {
    CoreException original = new CoreException("stage does not exist");
    when(mockCoreApi.connectionDownloadStreamBegin(any(), anyString(), anyString(), anyBoolean()))
        .thenThrow(original);

    CoreException thrown =
        assertThrows(
            CoreException.class, () -> connection.downloadStream("@missing_stage", "data.csv"));
    assertSame(original, thrown, "a non-missing-file begin failure must not be remapped");
  }
}
