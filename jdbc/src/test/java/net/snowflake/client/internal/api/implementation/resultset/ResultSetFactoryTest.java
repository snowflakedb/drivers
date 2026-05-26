package net.snowflake.client.internal.api.implementation.resultset;

import static org.junit.jupiter.api.Assertions.assertNull;
import static org.junit.jupiter.api.Assertions.assertSame;
import static org.junit.jupiter.api.Assertions.assertThrows;
import static org.mockito.ArgumentMatchers.any;
import static org.mockito.Mockito.mock;
import static org.mockito.Mockito.verify;
import static org.mockito.Mockito.when;

import com.google.protobuf.ByteString;
import java.sql.SQLException;
import net.snowflake.client.internal.api.implementation.statement.SnowflakeStatementImpl;
import net.snowflake.client.internal.unicore.CoreDriverApi;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.ArrowArrayStreamPtr;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.ResultSetGetStreamResponse;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.ResultSetHandle;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.ResultSetReleaseResponse;
import org.junit.jupiter.api.BeforeEach;
import org.junit.jupiter.api.Test;

class ResultSetFactoryTest {

  private static final ResultSetHandle HANDLE =
      ResultSetHandle.newBuilder().setId(42).setMagic(99).build();

  private CoreDriverApi mockCoreApi;
  private SnowflakeStatementImpl mockStatement;

  @BeforeEach
  void setUp() throws Exception {
    mockCoreApi = mock(CoreDriverApi.class);
    mockStatement = mock(SnowflakeStatementImpl.class);
    when(mockCoreApi.resultSetRelease(any()))
        .thenReturn(ResultSetReleaseResponse.getDefaultInstance());
  }

  // =========================================================================
  // create() — handle lifecycle
  // =========================================================================

  @Test
  void createReleasesHandleWhenGetStreamThrows() throws Exception {
    SQLException fetchError = new SQLException("stream fetch failed");
    when(mockCoreApi.resultSetGetStream(HANDLE)).thenThrow(fetchError);

    SQLException thrown =
        assertThrows(
            SQLException.class, () -> ResultSetFactory.create(mockCoreApi, mockStatement, HANDLE));

    assertSame(fetchError, thrown);
    verify(mockCoreApi).resultSetRelease(HANDLE);
  }

  @Test
  void createPropagatesOriginalExceptionWhenBothGetStreamAndReleaseThrow() throws Exception {
    SQLException fetchError = new SQLException("stream fetch failed");
    when(mockCoreApi.resultSetGetStream(HANDLE)).thenThrow(fetchError);
    when(mockCoreApi.resultSetRelease(HANDLE)).thenThrow(new SQLException("release also failed"));

    SQLException thrown =
        assertThrows(
            SQLException.class, () -> ResultSetFactory.create(mockCoreApi, mockStatement, HANDLE));

    assertSame(
        fetchError, thrown, "Original fetch exception should propagate, not the release one");
    verify(mockCoreApi).resultSetRelease(HANDLE);
  }

  // =========================================================================
  // createIfHasStream() — null returns for missing/empty streams
  // =========================================================================

  @Test
  void createIfHasStreamReturnsNullWhenNoStreamField() throws Exception {
    when(mockCoreApi.resultSetGetStream(HANDLE))
        .thenReturn(ResultSetGetStreamResponse.getDefaultInstance());

    InternalResultSet result =
        ResultSetFactory.createIfHasStream(mockCoreApi, mockStatement, HANDLE);

    assertNull(result);
    verify(mockCoreApi).resultSetRelease(HANDLE);
  }

  @Test
  void createIfHasStreamReturnsNullForZeroLengthStream() throws Exception {
    ResultSetGetStreamResponse emptyStreamResponse =
        ResultSetGetStreamResponse.newBuilder()
            .setStream(ArrowArrayStreamPtr.newBuilder().setValue(ByteString.EMPTY))
            .build();
    when(mockCoreApi.resultSetGetStream(HANDLE)).thenReturn(emptyStreamResponse);

    InternalResultSet result =
        ResultSetFactory.createIfHasStream(mockCoreApi, mockStatement, HANDLE);

    assertNull(result);
    verify(mockCoreApi).resultSetRelease(HANDLE);
  }

  // =========================================================================
  // createIfHasStream() — handle lifecycle
  // =========================================================================

  @Test
  void createIfHasStreamReleasesHandleWhenGetStreamThrows() throws Exception {
    SQLException fetchError = new SQLException("stream fetch failed");
    when(mockCoreApi.resultSetGetStream(HANDLE)).thenThrow(fetchError);

    SQLException thrown =
        assertThrows(
            SQLException.class,
            () -> ResultSetFactory.createIfHasStream(mockCoreApi, mockStatement, HANDLE));

    assertSame(fetchError, thrown);
    verify(mockCoreApi).resultSetRelease(HANDLE);
  }

  @Test
  void createIfHasStreamReleasesHandleOnNullReturn() throws Exception {
    when(mockCoreApi.resultSetGetStream(HANDLE))
        .thenReturn(ResultSetGetStreamResponse.getDefaultInstance());

    ResultSetFactory.createIfHasStream(mockCoreApi, mockStatement, HANDLE);

    verify(mockCoreApi).resultSetGetStream(HANDLE);
    verify(mockCoreApi).resultSetRelease(HANDLE);
  }
}
