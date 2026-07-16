package net.snowflake.client.internal.api.implementation.connection;

import static org.junit.jupiter.api.Assertions.assertArrayEquals;
import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertFalse;
import static org.junit.jupiter.api.Assertions.assertThrows;
import static org.junit.jupiter.api.Assertions.assertTrue;
import static org.mockito.ArgumentMatchers.any;
import static org.mockito.ArgumentMatchers.anyBoolean;
import static org.mockito.ArgumentMatchers.anyString;
import static org.mockito.ArgumentMatchers.eq;
import static org.mockito.Mockito.mock;
import static org.mockito.Mockito.verify;
import static org.mockito.Mockito.when;

import com.google.protobuf.ByteString;
import java.io.ByteArrayInputStream;
import java.io.ByteArrayOutputStream;
import java.io.IOException;
import java.io.InputStream;
import java.sql.SQLException;
import java.util.Properties;
import net.snowflake.client.api.connection.DownloadStreamConfig;
import net.snowflake.client.api.connection.UploadStreamConfig;
import net.snowflake.client.internal.unicore.CoreDriverApi;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.ConnectionDownloadStreamResponse;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.ConnectionHandle;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.ConnectionInitResponse;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.ConnectionNewResponse;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.ConnectionSetOptionsResponse;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.ConnectionUploadStreamResponse;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.DatabaseHandle;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.DatabaseInitResponse;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.DatabaseNewResponse;
import org.junit.jupiter.api.BeforeEach;
import org.junit.jupiter.api.Test;
import org.mockito.ArgumentCaptor;

/**
 * Unit tests for {@link SnowflakeConnectionImpl#uploadStream} and {@link
 * SnowflakeConnectionImpl#downloadStream}.
 *
 * <p>The tests exercise the JDBC wiring: PUT SQL is synthesized on the Java side from the
 * structured uploadStream parameters, the resulting SQL plus the byte payload are forwarded to
 * {@code coreDriverApi.connectionUploadStream(connHandle, sql, data)}, and the download path
 * unwraps the proto response into an {@link InputStream}.
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
  // uploadStream — wiring + SQL forwarded to RPC
  // ---------------------------------------------------------------------------

  @Test
  void shouldForwardSynthesizedSqlAndBytesForDefaultConfig() throws Exception {
    when(mockCoreApi.connectionUploadStream(any(), anyString(), any()))
        .thenReturn(ConnectionUploadStreamResponse.getDefaultInstance());

    byte[] payload = new byte[] {1, 2, 3};
    connection.uploadStream("@my_stage", "data.csv", new ByteArrayInputStream(payload));

    ArgumentCaptor<String> sqlCaptor = ArgumentCaptor.forClass(String.class);
    ArgumentCaptor<byte[]> dataCaptor = ArgumentCaptor.forClass(byte[].class);
    verify(mockCoreApi)
        .connectionUploadStream(eq(CONN_HANDLE), sqlCaptor.capture(), dataCaptor.capture());

    assertEquals("PUT 'file:///data.csv' @my_stage OVERWRITE = TRUE", sqlCaptor.getValue());
    assertArrayEquals(payload, dataCaptor.getValue());
  }

  @Test
  void shouldSynthesizeAutoCompressFalseWhenCompressDataIsFalse() throws Exception {
    when(mockCoreApi.connectionUploadStream(any(), anyString(), any()))
        .thenReturn(ConnectionUploadStreamResponse.getDefaultInstance());

    UploadStreamConfig config = UploadStreamConfig.builder().setCompressData(false).build();
    connection.uploadStream("@my_stage", "data.csv", new ByteArrayInputStream(new byte[0]), config);

    ArgumentCaptor<String> sqlCaptor = ArgumentCaptor.forClass(String.class);
    verify(mockCoreApi).connectionUploadStream(eq(CONN_HANDLE), sqlCaptor.capture(), any());
    assertTrue(
        sqlCaptor.getValue().contains("AUTO_COMPRESS = FALSE"),
        "synthesized SQL must carry AUTO_COMPRESS = FALSE: " + sqlCaptor.getValue());
  }

  @Test
  void shouldAppendDestPrefixToStagePathOnUpload() throws Exception {
    when(mockCoreApi.connectionUploadStream(any(), anyString(), any()))
        .thenReturn(ConnectionUploadStreamResponse.getDefaultInstance());

    UploadStreamConfig config = UploadStreamConfig.builder().setDestPrefix("inbox").build();
    connection.uploadStream("@my_stage", "data.csv", new ByteArrayInputStream(new byte[0]), config);

    ArgumentCaptor<String> sqlCaptor = ArgumentCaptor.forClass(String.class);
    verify(mockCoreApi).connectionUploadStream(eq(CONN_HANDLE), sqlCaptor.capture(), any());
    assertTrue(
        sqlCaptor.getValue().contains("@my_stage/inbox"),
        "stage path must include destPrefix: " + sqlCaptor.getValue());
  }

  @Test
  void shouldReadAllBytesFromInputStream() throws Exception {
    when(mockCoreApi.connectionUploadStream(any(), anyString(), any()))
        .thenReturn(ConnectionUploadStreamResponse.getDefaultInstance());

    byte[] expected = new byte[] {1, 2, 3, 4, 5};
    connection.uploadStream("@s", "f", new ByteArrayInputStream(expected));

    ArgumentCaptor<byte[]> dataCaptor = ArgumentCaptor.forClass(byte[].class);
    verify(mockCoreApi).connectionUploadStream(any(), anyString(), dataCaptor.capture());
    assertArrayEquals(expected, dataCaptor.getValue());
  }

  @Test
  void shouldDefaultToCompressDataTrueWhenConfigIsNull() throws Exception {
    when(mockCoreApi.connectionUploadStream(any(), anyString(), any()))
        .thenReturn(ConnectionUploadStreamResponse.getDefaultInstance());

    connection.uploadStream("@my_stage", "data.csv", new ByteArrayInputStream(new byte[0]), null);

    ArgumentCaptor<String> sqlCaptor = ArgumentCaptor.forClass(String.class);
    verify(mockCoreApi).connectionUploadStream(eq(CONN_HANDLE), sqlCaptor.capture(), any());
    // null config => compressData defaults to true => no AUTO_COMPRESS = FALSE clause.
    assertFalse(
        sqlCaptor.getValue().contains("AUTO_COMPRESS = FALSE"),
        "null config must default to compressData=true: " + sqlCaptor.getValue());
  }

  @Test
  void shouldWrapIoExceptionAsSqlException() {
    InputStream brokenStream =
        new InputStream() {
          @Override
          public int read() throws IOException {
            throw new IOException("simulated IO failure");
          }
        };

    assertThrows(SQLException.class, () -> connection.uploadStream("@s", "f", brokenStream));
  }

  // ---------------------------------------------------------------------------
  // downloadStream
  // ---------------------------------------------------------------------------

  @Test
  void shouldCallCoreApiWithDecompressFalseForDefaultConfig() throws Exception {
    byte[] expected = "downloaded content".getBytes();
    when(mockCoreApi.connectionDownloadStream(any(), anyString(), anyString(), anyBoolean()))
        .thenReturn(
            ConnectionDownloadStreamResponse.newBuilder()
                .setData(ByteString.copyFrom(expected))
                .build());

    InputStream result = connection.downloadStream("@my_stage", "data.csv.gz");

    assertArrayEquals(expected, readAllBytes(result));
    verify(mockCoreApi)
        .connectionDownloadStream(eq(CONN_HANDLE), eq("@my_stage"), eq("data.csv.gz"), eq(false));
  }

  @Test
  void shouldForwardDecompressFlagWhenDecompressIsTrue() throws Exception {
    byte[] expected = "decompressed".getBytes();
    when(mockCoreApi.connectionDownloadStream(any(), anyString(), anyString(), anyBoolean()))
        .thenReturn(
            ConnectionDownloadStreamResponse.newBuilder()
                .setData(ByteString.copyFrom(expected))
                .build());

    DownloadStreamConfig config = DownloadStreamConfig.builder().setDecompress(true).build();
    InputStream result = connection.downloadStream("@my_stage", "data.csv.gz", config);

    assertArrayEquals(expected, readAllBytes(result));
    verify(mockCoreApi)
        .connectionDownloadStream(eq(CONN_HANDLE), eq("@my_stage"), eq("data.csv.gz"), eq(true));
  }

  private static byte[] readAllBytes(InputStream is) throws IOException {
    ByteArrayOutputStream buf = new ByteArrayOutputStream();
    byte[] chunk = new byte[4096];
    int n;
    while ((n = is.read(chunk)) != -1) {
      buf.write(chunk, 0, n);
    }
    return buf.toByteArray();
  }
}
