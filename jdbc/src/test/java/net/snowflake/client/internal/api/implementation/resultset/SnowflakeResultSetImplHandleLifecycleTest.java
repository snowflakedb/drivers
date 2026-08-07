package net.snowflake.client.internal.api.implementation.resultset;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertSame;
import static org.junit.jupiter.api.Assertions.assertThrows;
import static org.mockito.ArgumentMatchers.any;
import static org.mockito.Mockito.doThrow;
import static org.mockito.Mockito.mock;
import static org.mockito.Mockito.never;
import static org.mockito.Mockito.verify;
import static org.mockito.Mockito.when;

import java.sql.SQLException;
import java.util.Collections;
import java.util.List;
import net.snowflake.client.api.resultset.SnowflakeResultSetSerializable;
import net.snowflake.client.internal.api.implementation.exception.CoreException;
import net.snowflake.client.internal.api.implementation.exception.SFSQLFeatureNotSupportedException;
import net.snowflake.client.internal.api.implementation.parameters.FrozenParametersRegistry;
import net.snowflake.client.internal.api.implementation.resultset.metadata.SnowflakeResultSetMetaDataImpl;
import net.snowflake.client.internal.api.implementation.statement.SnowflakeStatementImpl;
import net.snowflake.client.internal.unicore.CoreDriverApi;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.ResultChunk;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.ResultSetGetChunksResponse;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.ResultSetHandle;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.ResultSetReleaseResponse;
import org.junit.jupiter.api.BeforeEach;
import org.junit.jupiter.api.Test;

class SnowflakeResultSetImplHandleLifecycleTest {

  private static final ResultSetHandle HANDLE =
      ResultSetHandle.newBuilder().setId(42).setMagic(99).build();
  private static final String QUERY_ID = "01ab-cdef-0000-0000";

  private CoreDriverApi mockCoreApi;
  private RowReader mockRowReader;
  private SnowflakeResultSetMetaDataImpl mockMetaData;

  @BeforeEach
  void setUp() throws Exception {
    mockCoreApi = mock(CoreDriverApi.class);
    mockRowReader = mock(RowReader.class);
    mockMetaData = mock(SnowflakeResultSetMetaDataImpl.class);
    when(mockCoreApi.resultSetRelease(any()))
        .thenReturn(ResultSetReleaseResponse.getDefaultInstance());
  }

  private SnowflakeResultSetImpl handleBackedResultSet() {
    return new SnowflakeResultSetImpl(
        null,
        QUERY_ID,
        mockRowReader,
        mockMetaData,
        false,
        new CoreResultSetProvider(mockCoreApi, HANDLE, QUERY_ID, FrozenParametersRegistry.EMPTY));
  }

  private SnowflakeResultSetImpl inMemoryResultSet() {
    return new SnowflakeResultSetImpl(null, QUERY_ID, mockRowReader, mockMetaData, false);
  }

  @Test
  void shouldReleaseHandleOnClose() throws Exception {
    SnowflakeResultSetImpl resultSet = handleBackedResultSet();

    resultSet.close();

    verify(mockRowReader).close();
    verify(mockCoreApi).resultSetRelease(HANDLE);
  }

  @Test
  void shouldReleaseHandleOnlyOnceAcrossRepeatedClose() throws Exception {
    SnowflakeResultSetImpl resultSet = handleBackedResultSet();

    resultSet.close();
    resultSet.close();

    verify(mockCoreApi).resultSetRelease(HANDLE);
  }

  @Test
  void shouldSwallowReleaseFailureOnClose() throws Exception {
    // CoreDriverApi reports every core failure as a CoreException, and release() logs those.
    when(mockCoreApi.resultSetRelease(HANDLE)).thenThrow(new CoreException("release failed"));
    SnowflakeResultSetImpl resultSet = handleBackedResultSet();

    // A failed handle release must not surface from close().
    resultSet.close();

    verify(mockCoreApi).resultSetRelease(HANDLE);
  }

  @Test
  void shouldNotReleaseWhenNoBackingHandleOnClose() throws Exception {
    SnowflakeResultSetImpl resultSet = inMemoryResultSet();

    resultSet.close();

    verify(mockRowReader).close();
    verify(mockCoreApi, never()).resultSetRelease(any());
  }

  @Test
  void shouldReleaseHandleWhenRowReaderCloseThrows() throws Exception {
    RowReader throwingReader = mock(RowReader.class);
    doThrow(new SQLException("reader close failed")).when(throwingReader).close();
    SnowflakeResultSetImpl resultSet =
        new SnowflakeResultSetImpl(
            null,
            QUERY_ID,
            throwingReader,
            mockMetaData,
            false,
            new CoreResultSetProvider(
                mockCoreApi, HANDLE, QUERY_ID, FrozenParametersRegistry.EMPTY));

    assertThrows(SQLException.class, resultSet::close);

    verify(mockCoreApi).resultSetRelease(HANDLE);
  }

  @Test
  void shouldReleaseHandleOnDetachRowReader() throws Exception {
    SnowflakeStatementImpl mockStatement = mock(SnowflakeStatementImpl.class);
    SnowflakeResultSetImpl resultSet =
        new SnowflakeResultSetImpl(
            mockStatement,
            QUERY_ID,
            mockRowReader,
            mockMetaData,
            false,
            new CoreResultSetProvider(
                mockCoreApi, HANDLE, QUERY_ID, FrozenParametersRegistry.EMPTY));

    RowReader detached = resultSet.detachRowReader();

    assertSame(mockRowReader, detached);
    verify(mockCoreApi).resultSetRelease(HANDLE);
    verify(mockStatement).removeClosedResultSet(resultSet);
  }

  @Test
  void shouldReleaseHandleOnDetachRowReaderWithoutStatement() throws Exception {
    // Serializable-derived result sets carry a null statement; detaching must not NPE on
    // unregister.
    SnowflakeResultSetImpl resultSet = handleBackedResultSet();

    RowReader detached = resultSet.detachRowReader();

    assertSame(mockRowReader, detached);
    verify(mockCoreApi).resultSetRelease(HANDLE);
  }

  @Test
  void shouldBuildSerializablesFromStoredHandleWithoutBackendRefetch() throws Exception {
    ResultChunk inlineChunk = ResultChunk.newBuilder().setInline("Zm9v").setRowCount(2).build();
    when(mockCoreApi.resultSetGetChunks(HANDLE))
        .thenReturn(ResultSetGetChunksResponse.newBuilder().addChunks(inlineChunk).build());

    try (SnowflakeResultSetImpl resultSet = handleBackedResultSet()) {

      List<SnowflakeResultSetSerializable> serializables =
          resultSet.getResultSetSerializables(Long.MAX_VALUE);

      assertEquals(1, serializables.size());
      assertEquals(2, serializables.get(0).getRowCount());
      verify(mockCoreApi).resultSetGetChunks(HANDLE);
      verify(mockCoreApi, never()).connectionGetResultSet(any(), any());
    }
  }

  @Test
  void shouldRejectSerializablesWhenResultSetClosed() throws Exception {
    SnowflakeResultSetImpl resultSet = handleBackedResultSet();

    resultSet.close();

    assertThrows(
        IllegalStateException.class, () -> resultSet.getResultSetSerializables(Long.MAX_VALUE));
  }

  @Test
  void shouldRejectSerializablesWhenNoBackingHandle() throws SQLException {
    try (SnowflakeResultSetImpl resultSet = inMemoryResultSet()) {

      assertThrows(
          SFSQLFeatureNotSupportedException.class,
          () -> resultSet.getResultSetSerializables(Long.MAX_VALUE));
    }
  }

  @Test
  void shouldReSliceInMemoryChunksWithoutAnyCoreCall() throws Exception {
    // A result set rehydrated from a serializable keeps its chunks, so it can be re-sliced into
    // serializables again (ResultSet -> serializables -> ResultSet -> serializables) with no core
    // round-trip.
    ResultChunk inlineChunk = ResultChunk.newBuilder().setInline("Zm9v").setRowCount(3).build();
    ResultSetChunksProvider chunks =
        new InMemoryResultSetChunksProvider(
            mockCoreApi,
            Collections.singletonList(inlineChunk),
            Collections.emptyList(),
            QUERY_ID,
            FrozenParametersRegistry.EMPTY);
    try (SnowflakeResultSetImpl resultSet =
        new SnowflakeResultSetImpl(null, QUERY_ID, mockRowReader, mockMetaData, false, chunks)) {

      List<SnowflakeResultSetSerializable> serializables =
          resultSet.getResultSetSerializables(Long.MAX_VALUE);

      assertEquals(1, serializables.size());
      assertEquals(3, serializables.get(0).getRowCount());
      verify(mockCoreApi, never()).resultSetGetChunks(any());
      verify(mockCoreApi, never()).connectionGetResultSet(any(), any());
    }
  }

  @Test
  void shouldNotReleaseAnythingForInMemoryChunksOnClose() throws Exception {
    ResultChunk inlineChunk = ResultChunk.newBuilder().setInline("Zm9v").setRowCount(1).build();
    ResultSetChunksProvider chunks =
        new InMemoryResultSetChunksProvider(
            mockCoreApi,
            Collections.singletonList(inlineChunk),
            Collections.emptyList(),
            QUERY_ID,
            FrozenParametersRegistry.EMPTY);
    SnowflakeResultSetImpl resultSet =
        new SnowflakeResultSetImpl(null, QUERY_ID, mockRowReader, mockMetaData, false, chunks);

    resultSet.close();

    verify(mockRowReader).close();
    verify(mockCoreApi, never()).resultSetRelease(any());
  }
}
