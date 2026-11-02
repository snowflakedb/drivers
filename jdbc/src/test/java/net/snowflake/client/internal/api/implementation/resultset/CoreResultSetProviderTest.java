package net.snowflake.client.internal.api.implementation.resultset;

import static org.junit.jupiter.api.Assertions.assertDoesNotThrow;
import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertSame;
import static org.mockito.Mockito.mock;
import static org.mockito.Mockito.times;
import static org.mockito.Mockito.verify;
import static org.mockito.Mockito.verifyNoInteractions;
import static org.mockito.Mockito.when;

import java.sql.SQLException;
import java.util.Collections;
import java.util.List;
import java.util.concurrent.CountDownLatch;
import java.util.concurrent.TimeUnit;
import net.snowflake.client.api.resultset.SnowflakeResultSetSerializable;
import net.snowflake.client.internal.api.implementation.exception.CoreException;
import net.snowflake.client.internal.api.implementation.parameters.ParametersRegistry;
import net.snowflake.client.internal.unicore.CoreDriverApi;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.ColumnMetadata;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.ResultChunk;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.ResultSetGetChunksResponse;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.ResultSetGetStreamResponse;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.ResultSetHandle;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.ResultSetReleaseResponse;
import org.junit.jupiter.api.BeforeEach;
import org.junit.jupiter.api.Test;

class CoreResultSetProviderTest {

  private static final ResultSetHandle HANDLE =
      ResultSetHandle.newBuilder().setId(42).setMagic(99).build();
  private static final String QUERY_ID = "01ab-cdef-0000-0000";

  private CoreDriverApi mockCoreApi;

  @BeforeEach
  void setUp() {
    mockCoreApi = mock(CoreDriverApi.class);
  }

  @Test
  void shouldReleaseHandleOnceWhenReleasedMoreThanOnce() {
    when(mockCoreApi.resultSetRelease(HANDLE))
        .thenReturn(ResultSetReleaseResponse.getDefaultInstance());
    CoreResultSetProvider provider = createProvider();

    provider.release();
    provider.release();

    verify(mockCoreApi, times(1)).resultSetRelease(HANDLE);
  }

  @Test
  void shouldReleaseHandleWhenUnreleasedProviderIsGarbageCollected() throws Exception {
    CountDownLatch released = new CountDownLatch(1);
    when(mockCoreApi.resultSetRelease(HANDLE))
        .thenAnswer(
            invocation -> {
              released.countDown();
              return ResultSetReleaseResponse.getDefaultInstance();
            });

    createProvider();

    long deadline = System.nanoTime() + TimeUnit.SECONDS.toNanos(10);
    while (released.getCount() > 0 && System.nanoTime() < deadline) {
      System.gc();
      released.await(100, TimeUnit.MILLISECONDS);
    }
    assertEquals(0, released.getCount());
  }

  @Test
  void shouldReturnCoreStreamResponseAndSplitCoreChunksIntoSlices() throws SQLException {
    ResultSetGetStreamResponse streamResponse = ResultSetGetStreamResponse.getDefaultInstance();
    when(mockCoreApi.resultSetGetStream(HANDLE)).thenReturn(streamResponse);
    ResultSetGetChunksResponse chunksResponse =
        ResultSetGetChunksResponse.newBuilder()
            .addChunks(ResultChunk.newBuilder().setInline("aGVsbG8=").setRowCount(2))
            .addColumns(ColumnMetadata.newBuilder().setName("COL1"))
            .build();
    when(mockCoreApi.resultSetGetChunks(HANDLE)).thenReturn(chunksResponse);
    CoreResultSetProvider provider = createProvider();

    assertSame(streamResponse, provider.getStream());
    List<SnowflakeResultSetSerializable> slices = provider.getChunks(1024L);

    assertEquals(1, slices.size());
    assertEquals(2L, slices.get(0).getRowCount());
  }

  @Test
  void shouldSwallowCoreReleaseFailure() {
    when(mockCoreApi.resultSetRelease(HANDLE)).thenThrow(new CoreException("release failed"));
    CoreResultSetProvider provider = createProvider();

    assertDoesNotThrow(provider::release);
    verify(mockCoreApi).resultSetRelease(HANDLE);
  }

  @Test
  void shouldSplitInMemoryChunksWithoutCallingCoreDriverApi() throws SQLException {
    InMemoryResultSetChunksProvider provider =
        new InMemoryResultSetChunksProvider(
            mockCoreApi,
            Collections.singletonList(
                ResultChunk.newBuilder().setInline("YWJj").setRowCount(3).build()),
            Collections.singletonList(ColumnMetadata.newBuilder().setName("ID").build()),
            QUERY_ID,
            ParametersRegistry.EMPTY);

    List<SnowflakeResultSetSerializable> slices = provider.getChunks(512L);
    provider.release();

    assertEquals(1, slices.size());
    assertEquals(3L, slices.get(0).getRowCount());
    verifyNoInteractions(mockCoreApi);
  }

  private CoreResultSetProvider createProvider() {
    return new CoreResultSetProvider(mockCoreApi, HANDLE, QUERY_ID, ParametersRegistry.EMPTY);
  }
}
