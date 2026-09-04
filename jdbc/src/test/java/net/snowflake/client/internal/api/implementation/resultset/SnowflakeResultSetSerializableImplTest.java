package net.snowflake.client.internal.api.implementation.resultset;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertThrows;
import static org.mockito.ArgumentMatchers.eq;
import static org.mockito.Mockito.mock;
import static org.mockito.Mockito.verify;
import static org.mockito.Mockito.when;

import com.google.protobuf.ByteString;
import java.io.ByteArrayInputStream;
import java.io.ByteArrayOutputStream;
import java.io.IOException;
import java.io.ObjectInputStream;
import java.io.ObjectOutputStream;
import java.sql.SQLException;
import java.util.Arrays;
import java.util.Collections;
import java.util.HashMap;
import java.util.List;
import java.util.Map;
import net.snowflake.client.api.resultset.SnowflakeResultSetSerializable;
import net.snowflake.client.internal.api.implementation.exception.SFSQLException;
import net.snowflake.client.internal.api.implementation.parameters.FrozenParametersRegistry;
import net.snowflake.client.internal.unicore.ConfigSettingFactory;
import net.snowflake.client.internal.unicore.CoreDriverApi;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.ArrowArrayStreamPtr;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.ColumnMetadata;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.ConfigSetting;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.DatabaseFetchChunkResponse;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.RemoteChunk;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.ResultChunk;
import org.junit.jupiter.api.BeforeEach;
import org.junit.jupiter.api.Test;

class SnowflakeResultSetSerializableImplTest {

  private static final String QUERY_ID = "01ab-cdef-0000-0000";
  private static final String PLACEHOLDER_URL = "https://snowflake.snowflakecomputing.com";
  private static final FrozenParametersRegistry NO_PARAMS = FrozenParametersRegistry.EMPTY;

  private CoreDriverApi mockCoreApi;

  @BeforeEach
  void setUp() {
    mockCoreApi = mock(CoreDriverApi.class);
  }

  @Test
  void shouldReturnSingleSerializableForInlineOnlyChunks() throws Exception {
    ResultChunk inline = inlineChunk("Zm9v", 2, 4, 4);

    List<SnowflakeResultSetSerializable> slices =
        SnowflakeResultSetSerializableImpl.splitBySize(
            mockCoreApi,
            Collections.singletonList(inline),
            Collections.emptyList(),
            QUERY_ID,
            NO_PARAMS,
            100);

    assertEquals(1, slices.size());
    assertEquals(2, slices.get(0).getRowCount());
    assertEquals(4, slices.get(0).getCompressedDataSizeInBytes());
    assertEquals(4, slices.get(0).getUncompressedDataSizeInBytes());
  }

  @Test
  void shouldSplitRemoteChunksWhenMaxSizeExceeded() throws Exception {
    ResultChunk inline = inlineChunk("AAAA", 1, 4, 4);
    ResultChunk remote1 = remoteChunk(1, 5, 5);
    ResultChunk remote2 = remoteChunk(2, 5, 5);

    List<SnowflakeResultSetSerializable> slices =
        SnowflakeResultSetSerializableImpl.splitBySize(
            mockCoreApi,
            Arrays.asList(inline, remote1, remote2),
            Collections.emptyList(),
            QUERY_ID,
            NO_PARAMS,
            10);

    assertEquals(2, slices.size());
    assertEquals(2, slices.get(0).getRowCount());
    assertEquals(2, slices.get(1).getRowCount());
    assertEquals(9, slices.get(0).getUncompressedDataSizeInBytes());
    assertEquals(5, slices.get(1).getUncompressedDataSizeInBytes());
  }

  @Test
  void shouldSplitRemoteOnlyChunksWithoutInlineChunk() throws Exception {
    ResultChunk remote1 = remoteChunk(3, 6, 6);
    ResultChunk remote2 = remoteChunk(4, 6, 6);

    List<SnowflakeResultSetSerializable> slices =
        SnowflakeResultSetSerializableImpl.splitBySize(
            mockCoreApi,
            Arrays.asList(remote1, remote2),
            Collections.emptyList(),
            QUERY_ID,
            NO_PARAMS,
            10);

    assertEquals(2, slices.size());
    assertEquals(3, slices.get(0).getRowCount());
    assertEquals(4, slices.get(1).getRowCount());
  }

  @Test
  void shouldRejectSplitWhenChunkListIsEmpty() {
    // splitBySize is internal plumbing (static, not a decorated boundary), so it surfaces the raw
    // carrier; only getResultSet() below translates to a checked SQLException.
    SFSQLException ex =
        assertThrows(
            SFSQLException.class,
            () ->
                SnowflakeResultSetSerializableImpl.splitBySize(
                    mockCoreApi,
                    Collections.emptyList(),
                    Collections.emptyList(),
                    QUERY_ID,
                    NO_PARAMS,
                    100));

    assertEquals("The Result Set serializable is invalid.", ex.getMessage());
  }

  @Test
  void shouldRejectSplitWhenChunksHaveNoInlineOrRemoteData() {
    ResultChunk invalidChunk = ResultChunk.newBuilder().setRowCount(1).build();

    SFSQLException ex =
        assertThrows(
            SFSQLException.class,
            () ->
                SnowflakeResultSetSerializableImpl.splitBySize(
                    mockCoreApi,
                    Collections.singletonList(invalidChunk),
                    Collections.emptyList(),
                    QUERY_ID,
                    NO_PARAMS,
                    100));

    assertEquals("The Result Set serializable is invalid.", ex.getMessage());
  }

  @Test
  void shouldRoundTripJavaSerializationPreservingMetadata() throws Exception {
    ResultChunk inline = inlineChunk("Zm9v", 2, 4, 4);
    ResultChunk remote = remoteChunk(3, 10, 20);
    ColumnMetadata column =
        ColumnMetadata.newBuilder().setName("ID").setType("FIXED").setScale(0).build();
    SnowflakeResultSetSerializableImpl original =
        new SnowflakeResultSetSerializableImpl(
            mockCoreApi,
            QUERY_ID,
            Arrays.asList(inline, remote),
            Collections.singletonList(column),
            NO_PARAMS);

    SnowflakeResultSetSerializableImpl restored = deserialize(serialize(original));

    assertEquals(5, restored.getRowCount());
    assertEquals(14, restored.getCompressedDataSizeInBytes());
    assertEquals(24, restored.getUncompressedDataSizeInBytes());
  }

  @Test
  void shouldPreserveFrozenParametersThroughSplitAndSerialization() throws Exception {
    ResultChunk inline = inlineChunk("Zm9v", 1, 4, 4);
    Map<String, ConfigSetting> params = new HashMap<>();
    params.put("TIMEZONE", ConfigSettingFactory.from("Europe/Warsaw"));
    params.put(
        "TIMESTAMP_LTZ_OUTPUT_FORMAT",
        ConfigSettingFactory.from("YYYY-MM-DD HH24:MI:SS.FF3 TZH:TZM"));

    FrozenParametersRegistry frozen = new FrozenParametersRegistry(params);
    List<SnowflakeResultSetSerializable> slices =
        SnowflakeResultSetSerializableImpl.splitBySize(
            mockCoreApi,
            Collections.singletonList(inline),
            Collections.emptyList(),
            QUERY_ID,
            frozen,
            Long.MAX_VALUE);

    SnowflakeResultSetSerializableImpl restored =
        deserialize(serialize((SnowflakeResultSetSerializableImpl) slices.get(0)));

    assertEquals(frozen, restored.getParameters());
  }

  @Test
  void shouldRoundTripNullQueryIdThroughJavaSerialization() throws Exception {
    ResultChunk inline = inlineChunk("Zm9v", 1, 4, 4);
    SnowflakeResultSetSerializableImpl original =
        new SnowflakeResultSetSerializableImpl(
            mockCoreApi,
            null,
            Collections.singletonList(inline),
            Collections.emptyList(),
            NO_PARAMS);

    SnowflakeResultSetSerializableImpl restored = deserialize(serialize(original));

    assertEquals(1, restored.getRowCount());
  }

  @Test
  void shouldRejectGetResultSetWhenChunksAreEmpty() {
    SnowflakeResultSetSerializableImpl serializable =
        new SnowflakeResultSetSerializableImpl(
            mockCoreApi, QUERY_ID, Collections.emptyList(), Collections.emptyList(), NO_PARAMS);

    SQLException ex =
        assertThrows(SQLException.class, () -> serializable.getResultSet(retrieveConfig()));

    assertEquals("The Result Set serializable is invalid.", ex.getMessage());
  }

  @Test
  void shouldFetchChunksThroughCoreApiOnGetResultSet() throws Exception {
    ResultChunk inline = inlineChunk("Zm9v", 1, 4, 4);
    List<ResultChunk> chunks = Collections.singletonList(inline);
    List<ColumnMetadata> columns = Collections.emptyList();
    SnowflakeResultSetSerializableImpl serializable =
        new SnowflakeResultSetSerializableImpl(mockCoreApi, QUERY_ID, chunks, columns, NO_PARAMS);
    DatabaseFetchChunkResponse response =
        DatabaseFetchChunkResponse.newBuilder()
            .setStream(ArrowArrayStreamPtr.newBuilder().setValue(ByteString.copyFromUtf8("bad")))
            .build();
    when(mockCoreApi.databaseFetchChunk(chunks, columns)).thenReturn(response);

    assertThrows(Exception.class, () -> serializable.getResultSet(retrieveConfig()));

    verify(mockCoreApi).databaseFetchChunk(eq(chunks), eq(columns));
  }

  private static ResultChunk inlineChunk(
      String inline, int rowCount, long compressedSize, long uncompressedSize) {
    // Inline payload length drives size helpers; keep values aligned for assertions.
    assertEquals(inline.length(), compressedSize);
    assertEquals(inline.length(), uncompressedSize);
    return ResultChunk.newBuilder().setInline(inline).setRowCount(rowCount).build();
  }

  private static ResultChunk remoteChunk(int rowCount, long compressedSize, long uncompressedSize) {
    return ResultChunk.newBuilder()
        .setRowCount(rowCount)
        .setRemote(
            RemoteChunk.newBuilder()
                .setUrl("https://example.com/chunk")
                .setCompressedSize(compressedSize)
                .setUncompressedSize(uncompressedSize)
                .build())
        .build();
  }

  private static SnowflakeResultSetSerializable.ResultSetRetrieveConfig retrieveConfig()
      throws IllegalArgumentException {
    return SnowflakeResultSetSerializable.ResultSetRetrieveConfig.Builder.newInstance()
        .setSfFullURL(PLACEHOLDER_URL)
        .build();
  }

  private static byte[] serialize(SnowflakeResultSetSerializableImpl serializable)
      throws IOException {
    ByteArrayOutputStream bytes = new ByteArrayOutputStream();
    try (ObjectOutputStream out = new ObjectOutputStream(bytes)) {
      out.writeObject(serializable);
    }
    return bytes.toByteArray();
  }

  private static SnowflakeResultSetSerializableImpl deserialize(byte[] serialized)
      throws IOException, ClassNotFoundException {
    try (ObjectInputStream in = new ObjectInputStream(new ByteArrayInputStream(serialized))) {
      return (SnowflakeResultSetSerializableImpl) in.readObject();
    }
  }
}
