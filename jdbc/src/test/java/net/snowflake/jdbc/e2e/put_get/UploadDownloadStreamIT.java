package net.snowflake.jdbc.e2e.put_get;

import static net.snowflake.jdbc.utils.IoTestUtils.readAllBytes;
import static org.junit.jupiter.api.Assertions.assertArrayEquals;
import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertThrows;
import static org.junit.jupiter.api.Assertions.assertTrue;

import java.io.ByteArrayInputStream;
import java.io.IOException;
import java.io.InputStream;
import java.nio.charset.StandardCharsets;
import java.sql.Connection;
import java.sql.SQLException;
import java.util.zip.GZIPInputStream;
import net.snowflake.client.api.connection.DownloadStreamConfig;
import net.snowflake.client.api.connection.SnowflakeConnection;
import net.snowflake.client.api.connection.UploadStreamConfig;
import net.snowflake.jdbc.utils.SnowflakeIntegrationTestBase;
import org.junit.jupiter.api.Test;

public class UploadDownloadStreamIT extends SnowflakeIntegrationTestBase implements WithPutGet {

  private static final byte[] PAYLOAD =
      "stream-roundtrip-payload\n".getBytes(StandardCharsets.UTF_8);
  private static final String DEST_FILE = "payload.csv";
  private static final String DEST_PREFIX = "inbox";
  private static final byte GZIP_MAGIC_0 = 0x1f;
  private static final byte GZIP_MAGIC_1 = (byte) 0x8b;

  @Test
  public void shouldRoundTripCompressedStreamWithDestPrefix() throws Exception {
    Connection connection = getDefaultConnection();
    SnowflakeConnection sf = connection.unwrap(SnowflakeConnection.class);

    // Given A temporary stage and a dest prefix
    String stageRef = "@" + createTemporaryStage(connection, "TEST_STREAM_COMPRESS");
    String stagedPath = DEST_PREFIX + "/" + DEST_FILE + ".gz";
    UploadStreamConfig uploadConfig =
        UploadStreamConfig.builder().setDestPrefix(DEST_PREFIX).build();

    // When Data is uploaded via uploadStream with default compression and destPrefix
    try (InputStream in = new ByteArrayInputStream(PAYLOAD)) {
      sf.uploadStream(stageRef, DEST_FILE, in, uploadConfig);
    }

    // Then downloadStream with decompress=true returns the original bytes
    try (InputStream out =
        sf.downloadStream(
            stageRef, stagedPath, DownloadStreamConfig.builder().setDecompress(true).build())) {
      assertArrayEquals(
          PAYLOAD, readAllBytes(out), "decompressed download should match the uploaded payload");
    }

    // And downloadStream with default decompress=false returns gzip of those bytes
    try (InputStream out = sf.downloadStream(stageRef, stagedPath)) {
      assertGzipOf(PAYLOAD, readAllBytes(out));
    }
  }

  @Test
  public void shouldRoundTripUncompressedStreamWithoutPrefix() throws Exception {
    Connection connection = getDefaultConnection();
    SnowflakeConnection sf = connection.unwrap(SnowflakeConnection.class);

    // Given A temporary stage
    String stageRef = "@" + createTemporaryStage(connection, "TEST_STREAM_UNCOMPRESS");
    UploadStreamConfig uploadConfig = UploadStreamConfig.builder().setCompressData(false).build();

    // When Data is uploaded via uploadStream with compressData=false and no destPrefix
    try (InputStream in = new ByteArrayInputStream(PAYLOAD)) {
      sf.uploadStream(stageRef, DEST_FILE, in, uploadConfig);
    }

    // Then downloadStream with default decompress=false returns the original bytes
    try (InputStream out = sf.downloadStream(stageRef, DEST_FILE)) {
      assertArrayEquals(
          PAYLOAD, readAllBytes(out), "uncompressed download should match the uploaded payload");
    }
  }

  @Test
  public void shouldThrowWhenDownloadingMissingStageFile() throws Exception {
    Connection connection = getDefaultConnection();
    SnowflakeConnection sf = connection.unwrap(SnowflakeConnection.class);

    // Given An empty temporary stage
    String stageRef = "@" + createTemporaryStage(connection, "TEST_STREAM_MISSING");

    // When downloadStream is called for a file that does not exist on the stage
    SQLException thrown =
        assertThrows(
            SQLException.class,
            () -> sf.downloadStream(stageRef, "missing.csv"),
            "Expected downloadStream of a missing stage file to fail");

    // Then it fails with vendor FILE_NOT_FOUND (200008), SQLSTATE 02000, and a "File not found"
    // message. The literal 200008 is asserted rather than ErrorCode.FILE_NOT_FOUND.getMessageCode()
    // so the assertion also runs against the old driver, whose ErrorCode lacks that accessor.
    assertEquals(200008, thrown.getErrorCode());
    assertEquals("02000", thrown.getSQLState(), "Unexpected SQLSTATE");
    assertTrue(
        thrown.getMessage().contains("File not found"),
        "Unexpected message: " + thrown.getMessage());
  }

  private static void assertGzipOf(byte[] expectedPlaintext, byte[] actual) throws IOException {
    assertTrue(actual.length >= 2, "decompress=false should return gzip-compressed bytes");
    assertEquals(GZIP_MAGIC_0, actual[0], "expected gzip magic byte 0x1f");
    assertEquals(GZIP_MAGIC_1, actual[1], "expected gzip magic byte 0x8b");
    assertArrayEquals(
        expectedPlaintext,
        gunzip(actual),
        "gunzip of staged object should match the uploaded payload");
  }

  private static byte[] gunzip(byte[] compressed) throws IOException {
    try (GZIPInputStream in = new GZIPInputStream(new ByteArrayInputStream(compressed))) {
      return readAllBytes(in);
    }
  }
}
