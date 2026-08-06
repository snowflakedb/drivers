package net.snowflake.client.internal.api.implementation.resultset;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertFalse;
import static org.junit.jupiter.api.Assertions.assertNull;
import static org.junit.jupiter.api.Assertions.assertSame;
import static org.junit.jupiter.api.Assertions.assertThrows;
import static org.junit.jupiter.api.Assertions.assertTrue;
import static org.mockito.ArgumentMatchers.any;
import static org.mockito.Mockito.mock;
import static org.mockito.Mockito.verify;
import static org.mockito.Mockito.when;

import com.google.protobuf.ByteString;
import java.sql.SQLException;
import java.sql.Types;
import java.util.Arrays;
import net.snowflake.client.internal.api.decorator.Telemetry;
import net.snowflake.client.internal.api.implementation.connection.InternalSnowflakeConnection;
import net.snowflake.client.internal.api.implementation.parameters.FrozenParametersRegistry;
import net.snowflake.client.internal.api.implementation.resultset.metadata.SnowflakeResultSetMetaDataImpl;
import net.snowflake.client.internal.api.implementation.statement.SnowflakeStatementImpl;
import net.snowflake.client.internal.unicore.CoreDriverApi;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.ArrowArrayStreamPtr;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.ResultSetGetStreamResponse;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.ResultSetHandle;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.ResultSetReleaseResponse;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.ResultSetResponse;
import org.junit.jupiter.api.BeforeEach;
import org.junit.jupiter.api.Test;

class ResultSetFactoryTest {

  private static final ResultSetHandle HANDLE =
      ResultSetHandle.newBuilder().setId(42).setMagic(99).build();
  private static final ResultSetResponse RESPONSE =
      ResultSetResponse.newBuilder().setResultSetHandle(HANDLE).build();
  private static final String QUERY_ID = "01ab-cdef-0000-0000";

  private CoreDriverApi mockCoreApi;
  private SnowflakeStatementImpl mockStatement;

  @BeforeEach
  void setUp() throws Exception {
    mockCoreApi = mock(CoreDriverApi.class);
    mockStatement = mock(SnowflakeStatementImpl.class);
    InternalSnowflakeConnection mockConnection = mock(InternalSnowflakeConnection.class);
    when(mockStatement.getConnectionInternal()).thenReturn(mockConnection);
    when(mockConnection.getParameters()).thenReturn(FrozenParametersRegistry.EMPTY);
    when(mockConnection.getTelemetry()).thenReturn(Telemetry.NOOP);
    when(mockCoreApi.resultSetRelease(any()))
        .thenReturn(ResultSetReleaseResponse.getDefaultInstance());
  }

  // =========================================================================
  // create() — handle lifecycle
  // =========================================================================

  @Test
  void shouldCreateReleasesHandleWhenGetStreamThrows() throws Exception {
    SQLException fetchError = new SQLException("stream fetch failed");
    when(mockCoreApi.resultSetGetStream(HANDLE)).thenThrow(fetchError);

    SQLException thrown =
        assertThrows(
            SQLException.class,
            () -> ResultSetFactory.create(mockCoreApi, mockStatement, QUERY_ID, RESPONSE));

    assertSame(fetchError, thrown);
    verify(mockCoreApi).resultSetRelease(HANDLE);
  }

  @Test
  void shouldCreatePropagatesOriginalExceptionWhenBothGetStreamAndReleaseThrow() throws Exception {
    SQLException fetchError = new SQLException("stream fetch failed");
    when(mockCoreApi.resultSetGetStream(HANDLE)).thenThrow(fetchError);
    when(mockCoreApi.resultSetRelease(HANDLE)).thenThrow(new SQLException("release also failed"));

    SQLException thrown =
        assertThrows(
            SQLException.class,
            () -> ResultSetFactory.create(mockCoreApi, mockStatement, QUERY_ID, RESPONSE));

    assertSame(
        fetchError, thrown, "Original fetch exception should propagate, not the release one");
    verify(mockCoreApi).resultSetRelease(HANDLE);
  }

  // =========================================================================
  // createIfHasStream() — null returns for missing/empty streams
  // =========================================================================

  @Test
  void shouldCreateIfHasStreamReturnsNullWhenNoStreamField() throws Exception {
    when(mockCoreApi.resultSetGetStream(HANDLE))
        .thenReturn(ResultSetGetStreamResponse.getDefaultInstance());

    InternalResultSet result =
        ResultSetFactory.createIfHasStream(mockCoreApi, mockStatement, QUERY_ID, RESPONSE);

    assertNull(result);
    verify(mockCoreApi).resultSetRelease(HANDLE);
  }

  @Test
  void shouldCreateIfHasStreamReturnsNullForZeroLengthStream() throws Exception {
    ResultSetGetStreamResponse emptyStreamResponse =
        ResultSetGetStreamResponse.newBuilder()
            .setStream(ArrowArrayStreamPtr.newBuilder().setValue(ByteString.EMPTY))
            .build();
    when(mockCoreApi.resultSetGetStream(HANDLE)).thenReturn(emptyStreamResponse);

    InternalResultSet result =
        ResultSetFactory.createIfHasStream(mockCoreApi, mockStatement, QUERY_ID, RESPONSE);

    assertNull(result);
    verify(mockCoreApi).resultSetRelease(HANDLE);
  }

  // =========================================================================
  // createIfHasStream() — handle lifecycle
  // =========================================================================

  @Test
  void shouldCreateIfHasStreamReleasesHandleWhenGetStreamThrows() throws Exception {
    SQLException fetchError = new SQLException("stream fetch failed");
    when(mockCoreApi.resultSetGetStream(HANDLE)).thenThrow(fetchError);

    SQLException thrown =
        assertThrows(
            SQLException.class,
            () ->
                ResultSetFactory.createIfHasStream(mockCoreApi, mockStatement, QUERY_ID, RESPONSE));

    assertSame(fetchError, thrown);
    verify(mockCoreApi).resultSetRelease(HANDLE);
  }

  @Test
  void shouldCreateIfHasStreamReleasesHandleOnNullReturn() throws Exception {
    when(mockCoreApi.resultSetGetStream(HANDLE))
        .thenReturn(ResultSetGetStreamResponse.getDefaultInstance());

    ResultSetFactory.createIfHasStream(mockCoreApi, mockStatement, QUERY_ID, RESPONSE);

    verify(mockCoreApi).resultSetGetStream(HANDLE);
    verify(mockCoreApi).resultSetRelease(HANDLE);
  }

  // =========================================================================
  // createEmpty()
  // =========================================================================

  @Test
  void shouldCreateEmptyResultSetWithMetadataColumnsAndNoRows() throws Exception {
    SnowflakeResultSetMetaDataImpl metaData =
        SnowflakeResultSetMetaDataImpl.fromColumnSpec(
            null,
            Arrays.asList("TABLE_SCHEM", "TABLE_CATALOG"),
            Arrays.asList("TEXT", "TEXT"),
            Arrays.asList(Types.VARCHAR, Types.VARCHAR));

    InternalResultSet result = ResultSetFactory.createEmpty(mockStatement, metaData, false);

    assertTrue(result.isBeforeFirst());
    assertFalse(result.next());
    assertTrue(result.isAfterLast());
    assertEquals(2, result.getMetaData().getColumnCount());
    assertEquals("TABLE_SCHEM", result.getMetaData().getColumnName(1));
  }
}
