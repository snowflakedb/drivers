package net.snowflake.client.internal.api.implementation.connection;

import static org.junit.jupiter.api.Assertions.assertArrayEquals;
import static org.junit.jupiter.api.Assertions.assertNotNull;
import static org.junit.jupiter.api.Assertions.assertThrows;
import static org.mockito.ArgumentMatchers.any;
import static org.mockito.ArgumentMatchers.anyBoolean;
import static org.mockito.ArgumentMatchers.anyInt;
import static org.mockito.ArgumentMatchers.anyString;
import static org.mockito.ArgumentMatchers.eq;
import static org.mockito.ArgumentMatchers.isNull;
import static org.mockito.Mockito.mock;
import static org.mockito.Mockito.verify;
import static org.mockito.Mockito.when;

import com.google.protobuf.ByteString;
import java.io.ByteArrayInputStream;
import java.io.IOException;
import java.io.InputStream;
import java.sql.SQLException;
import java.util.Properties;
import net.snowflake.client.api.connection.DownloadStreamConfig;
import net.snowflake.client.api.connection.UploadStreamConfig;
import net.snowflake.client.internal.unicore.CoreDriverApi;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.ConnectionDownloadStreamResponse;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.ConnectionHandle;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.ConnectionHeartbeatResponse;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.ConnectionInitResponse;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.ConnectionNewResponse;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.ConnectionReleaseResponse;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.ConnectionSetOptionsResponse;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.ConnectionUploadStreamResponse;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.DatabaseHandle;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.DatabaseInitResponse;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.DatabaseNewResponse;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.DatabaseReleaseResponse;
import org.junit.jupiter.api.BeforeEach;
import org.junit.jupiter.api.Test;
import org.mockito.ArgumentCaptor;

/**
 * Unit tests for SnowflakeConnectionImpl.uploadStream and downloadStream.
 *
 * <p>These tests verify the wiring layer: that the JDBC uploadStream/downloadStream methods
 * correctly delegate to the core API with the expected parameters, and that config options
 * (destPrefix, compressData, decompress) are forwarded faithfully.
 *
 * <p>Integration tests that exercise the full pipeline against a real Snowflake stage live in the
 * integration test suite (see SNOW-3406377 test plan).
 */
class UploadDownloadStreamTest {

  private static final DatabaseHandle DB_HANDLE =
      DatabaseHandle.newBuilder().setId(1).setMagic(100).build();
  private static final ConnectionHandle CONN_HANDLE =
      ConnectionHandle.newBuilder().setId(2).setMagic(200).build();

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
    when(mockCoreApi.connectionHeartbeat(any(), anyInt()))
        .thenReturn(ConnectionHeartbeatResponse.newBuilder().setValid(true).build());
    when(mockCoreApi.connectionRelease(any()))
        .thenReturn(ConnectionReleaseResponse.getDefaultInstance());
    when(mockCoreApi.databaseRelease(any()))
        .thenReturn(DatabaseReleaseResponse.getDefaultInstance());

    Properties props = new Properties();
    props.setProperty("account", "test_account");
    props.setProperty("user", "test_user");
    props.setProperty("password", "test_password");
    connection =
        new SnowflakeConnectionImpl(
            "jdbc:snowflake://test.snowflakecomputing.com", props, mockCoreApi);
  }

  // -------------------------------------------------------------------------
  // uploadStream – default config (compress=true, no prefix)
  // -------------------------------------------------------------------------

  @Test
  void uploadStream_defaultConfig_callsCoreApiWithCompressTrue() throws Exception {
    when(mockCoreApi.connectionUploadStream(
            any(), anyString(), anyString(), any(), any(), anyBoolean()))
        .thenReturn(
            ConnectionUploadStreamResponse.newBuilder()
                .setStatus("UPLOADED")
                .setTargetFilename("data.csv.gz")
                .build());

    byte[] payload = "hello,world".getBytes();
    connection.uploadStream("@my_stage", "data.csv", new ByteArrayInputStream(payload));

    // default: compressData=true, no destPrefix
    verify(mockCoreApi)
        .connectionUploadStream(
            eq(CONN_HANDLE),
            eq("@my_stage"),
            eq("data.csv"),
            eq(payload),
            isNull(), // null destPrefix
            eq(true)); // compressData defaults to true
  }

  @Test
  void uploadStream_withPrefix_forwardsDestPrefix() throws Exception {
    when(mockCoreApi.connectionUploadStream(
            any(), anyString(), anyString(), any(), any(), anyBoolean()))
        .thenReturn(ConnectionUploadStreamResponse.getDefaultInstance());

    UploadStreamConfig config =
        UploadStreamConfig.builder().setDestPrefix("subdir").setCompressData(true).build();
    connection.uploadStream("@my_stage", "data.csv", new ByteArrayInputStream(new byte[0]), config);

    verify(mockCoreApi)
        .connectionUploadStream(
            eq(CONN_HANDLE), eq("@my_stage"), eq("data.csv"), any(), eq("subdir"), eq(true));
  }

  @Test
  void uploadStream_compressDataFalse_forwardsFlag() throws Exception {
    when(mockCoreApi.connectionUploadStream(
            any(), anyString(), anyString(), any(), any(), anyBoolean()))
        .thenReturn(ConnectionUploadStreamResponse.getDefaultInstance());

    UploadStreamConfig config = UploadStreamConfig.builder().setCompressData(false).build();
    connection.uploadStream("@my_stage", "data.csv", new ByteArrayInputStream(new byte[0]), config);

    verify(mockCoreApi)
        .connectionUploadStream(
            eq(CONN_HANDLE), eq("@my_stage"), eq("data.csv"), any(), isNull(), eq(false));
  }

  @Test
  void uploadStream_readsAllBytesFromInputStream() throws Exception {
    when(mockCoreApi.connectionUploadStream(
            any(), anyString(), anyString(), any(), any(), anyBoolean()))
        .thenReturn(ConnectionUploadStreamResponse.getDefaultInstance());

    byte[] expected = new byte[] {1, 2, 3, 4, 5};
    connection.uploadStream("@s", "f", new ByteArrayInputStream(expected));

    ArgumentCaptor<byte[]> dataCaptor = ArgumentCaptor.forClass(byte[].class);
    verify(mockCoreApi)
        .connectionUploadStream(
            any(), anyString(), anyString(), dataCaptor.capture(), any(), anyBoolean());
    assertArrayEquals(expected, dataCaptor.getValue());
  }

  // -------------------------------------------------------------------------
  // downloadStream – default config (decompress=false)
  // -------------------------------------------------------------------------

  @Test
  void downloadStream_defaultConfig_callsCoreApiWithDecompressFalse() throws Exception {
    byte[] expected = "downloaded content".getBytes();
    when(mockCoreApi.connectionDownloadStream(any(), anyString(), anyString(), anyBoolean()))
        .thenReturn(
            ConnectionDownloadStreamResponse.newBuilder()
                .setData(ByteString.copyFrom(expected))
                .build());

    InputStream result = connection.downloadStream("@my_stage", "data.csv.gz");

    assertNotNull(result);
    assertArrayEquals(expected, result.readAllBytes());
    verify(mockCoreApi)
        .connectionDownloadStream(eq(CONN_HANDLE), eq("@my_stage"), eq("data.csv.gz"), eq(false));
  }

  @Test
  void downloadStream_withDecompressTrue_forwardsFlag() throws Exception {
    byte[] expected = "decompressed".getBytes();
    when(mockCoreApi.connectionDownloadStream(any(), anyString(), anyString(), anyBoolean()))
        .thenReturn(
            ConnectionDownloadStreamResponse.newBuilder()
                .setData(ByteString.copyFrom(expected))
                .build());

    DownloadStreamConfig config = DownloadStreamConfig.builder().setDecompress(true).build();
    InputStream result = connection.downloadStream("@my_stage", "data.csv.gz", config);

    assertNotNull(result);
    verify(mockCoreApi)
        .connectionDownloadStream(eq(CONN_HANDLE), eq("@my_stage"), eq("data.csv.gz"), eq(true));
  }

  @Test
  void downloadStream_returnsStreamWithCoreApiBytes() throws Exception {
    byte[] expected = new byte[] {10, 20, 30};
    when(mockCoreApi.connectionDownloadStream(any(), anyString(), anyString(), anyBoolean()))
        .thenReturn(
            ConnectionDownloadStreamResponse.newBuilder()
                .setData(ByteString.copyFrom(expected))
                .build());

    InputStream result = connection.downloadStream("@s", "f");
    byte[] actual = result.readAllBytes();
    assertArrayEquals(expected, actual);
  }

  // -------------------------------------------------------------------------
  // Error path
  // -------------------------------------------------------------------------

  @Test
  void uploadStream_ioExceptionWrappedAsSQLException() {
    InputStream brokenStream =
        new InputStream() {
          @Override
          public int read() throws IOException {
            throw new IOException("simulated IO failure");
          }
        };

    assertThrows(SQLException.class, () -> connection.uploadStream("@s", "f", brokenStream));
  }
}
