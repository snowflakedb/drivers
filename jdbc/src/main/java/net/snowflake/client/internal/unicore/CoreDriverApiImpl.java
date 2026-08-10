package net.snowflake.client.internal.unicore;

import com.google.protobuf.UnsafeByteOperations;
import java.util.List;
import java.util.Map;
import java.util.concurrent.ExecutionException;
import java.util.concurrent.Future;
import lombok.RequiredArgsConstructor;
import net.snowflake.client.internal.api.implementation.exception.CoreException;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverService;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.ColumnMetadata;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.ConfigSetting;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.ConnectionAbortQueryRequest;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.ConnectionAbortQueryResponse;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.ConnectionCloseRequest;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.ConnectionCloseResponse;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.ConnectionCommitRequest;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.ConnectionCommitResponse;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.ConnectionDownloadStreamBeginRequest;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.ConnectionDownloadStreamBeginResponse;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.ConnectionDownloadStreamChunkRequest;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.ConnectionDownloadStreamChunkResponse;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.ConnectionDownloadStreamCloseRequest;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.ConnectionDownloadStreamCloseResponse;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.ConnectionGetAllParametersRequest;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.ConnectionGetAllParametersResponse;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.ConnectionGetInfoRequest;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.ConnectionGetInfoResponse;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.ConnectionGetParameterRequest;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.ConnectionGetParameterResponse;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.ConnectionGetQueryResultRequest;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.ConnectionGetQueryStatusRequest;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.ConnectionGetQueryStatusResponse;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.ConnectionGetResultSetRequest;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.ConnectionHandle;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.ConnectionHeartbeatRequest;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.ConnectionHeartbeatResponse;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.ConnectionInitRequest;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.ConnectionInitResponse;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.ConnectionIsClosedRequest;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.ConnectionIsClosedResponse;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.ConnectionNewRequest;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.ConnectionNewResponse;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.ConnectionReleaseRequest;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.ConnectionReleaseResponse;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.ConnectionRollbackRequest;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.ConnectionRollbackResponse;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.ConnectionSendHttpRequest;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.ConnectionSendHttpResponse;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.ConnectionSetAutocommitRequest;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.ConnectionSetAutocommitResponse;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.ConnectionSetOptionsRequest;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.ConnectionSetOptionsResponse;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.ConnectionSetSessionParametersRequest;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.ConnectionSetSessionParametersResponse;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.ConnectionTokenRequest;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.ConnectionTokenResponse;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.ConnectionUploadStreamAbortRequest;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.ConnectionUploadStreamAbortResponse;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.ConnectionUploadStreamBeginRequest;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.ConnectionUploadStreamBeginResponse;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.ConnectionUploadStreamChunkRequest;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.ConnectionUploadStreamChunkResponse;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.ConnectionUploadStreamFinishRequest;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.ConnectionUploadStreamFinishResponse;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.ConnectionUseDatabaseRequest;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.ConnectionUseDatabaseResponse;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.ConnectionUseSchemaRequest;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.ConnectionUseSchemaResponse;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.DatabaseFetchChunkRequest;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.DatabaseFetchChunkResponse;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.DatabaseHandle;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.DatabaseInitRequest;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.DatabaseInitResponse;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.DatabaseNewRequest;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.DatabaseNewResponse;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.DatabaseReleaseRequest;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.DatabaseReleaseResponse;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.DownloadStreamHandle;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.DriverException;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.ExecuteQueryResponse;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.QueryBindings;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.ResultChunk;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.ResultSetGetChunksRequest;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.ResultSetGetChunksResponse;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.ResultSetGetStreamRequest;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.ResultSetGetStreamResponse;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.ResultSetHandle;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.ResultSetReleaseRequest;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.ResultSetReleaseResponse;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.ResultSetResponse;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.StatementExecuteAsyncRequest;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.StatementExecuteAsyncResponse;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.StatementExecuteQueryRequest;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.StatementHandle;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.StatementNewRequest;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.StatementNewResponse;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.StatementPrepareRequest;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.StatementPrepareResponse;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.StatementReleaseRequest;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.StatementReleaseResponse;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.StatementSetOptionsRequest;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.StatementSetOptionsResponse;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.StatementSetSqlQueryRequest;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.StatementSetSqlQueryResponse;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.TelemetrySendApiUsageRequest;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.TelemetrySendResponse;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.TelemetrySendWrapperErrorRequest;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.TokenRequestType;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.UploadStreamHandle;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.WrapperIdentity;

/**
 * Facade over {@link DatabaseDriverService} that encapsulates protobuf request construction and
 * provides centralized error conversion from core driver exceptions to a runtime {@link
 * net.snowflake.client.internal.api.implementation.exception.CoreException} carrier.
 *
 * <p>Callers interact with domain-level parameters (handles, strings, maps) and never need to
 * import or construct {@code *Request} protobuf objects directly.
 */
@RequiredArgsConstructor
class CoreDriverApiImpl implements CoreDriverApi {

  private final DatabaseDriverService client;

  // =========================================================================
  // Database lifecycle
  // =========================================================================

  public DatabaseNewResponse databaseNew() {
    DatabaseNewRequest request = DatabaseNewRequest.getDefaultInstance();
    return invoke(() -> client.databaseNew(request));
  }

  public DatabaseInitResponse databaseInit(DatabaseHandle dbHandle) {
    DatabaseInitRequest request = DatabaseInitRequest.newBuilder().setDbHandle(dbHandle).build();
    return invoke(() -> client.databaseInit(request));
  }

  public DatabaseReleaseResponse databaseRelease(DatabaseHandle dbHandle) {
    DatabaseReleaseRequest request =
        DatabaseReleaseRequest.newBuilder().setDbHandle(dbHandle).build();
    return invoke(() -> client.databaseRelease(request));
  }

  // =========================================================================
  // Connection lifecycle
  // =========================================================================

  public ConnectionNewResponse connectionNew() {
    ConnectionNewRequest request = ConnectionNewRequest.getDefaultInstance();
    return invoke(() -> client.connectionNew(request));
  }

  public ConnectionInitResponse connectionInit(
      ConnectionHandle connHandle, DatabaseHandle dbHandle, WrapperIdentity wrapperIdentity) {
    ConnectionInitRequest request =
        ConnectionInitRequest.newBuilder()
            .setConnHandle(connHandle)
            .setDbHandle(dbHandle)
            .setWrapperIdentity(wrapperIdentity)
            .build();
    // connectionInit is async-first (returns a Future); block on it here. There
    // is no user-facing cancel trigger yet, so this stays a plain get().
    return await(client.connectionInit(request));
  }

  public ConnectionSetOptionsResponse connectionSetOptions(
      ConnectionHandle connHandle, Map<String, ConfigSetting> options) {
    ConnectionSetOptionsRequest request =
        ConnectionSetOptionsRequest.newBuilder()
            .setConnHandle(connHandle)
            .putAllOptions(options)
            .build();
    return invoke(() -> client.connectionSetOptions(request));
  }

  public ConnectionSetAutocommitResponse connectionSetAutocommit(
      ConnectionHandle connHandle, boolean autocommit) {
    ConnectionSetAutocommitRequest request =
        ConnectionSetAutocommitRequest.newBuilder()
            .setConnHandle(connHandle)
            .setAutocommit(autocommit)
            .build();
    return invoke(() -> client.connectionSetAutocommit(request));
  }

  public ConnectionCommitResponse connectionCommit(ConnectionHandle connHandle) {
    ConnectionCommitRequest request =
        ConnectionCommitRequest.newBuilder().setConnHandle(connHandle).build();
    return invoke(() -> client.connectionCommit(request));
  }

  public ConnectionRollbackResponse connectionRollback(ConnectionHandle connHandle) {
    ConnectionRollbackRequest request =
        ConnectionRollbackRequest.newBuilder().setConnHandle(connHandle).build();
    return invoke(() -> client.connectionRollback(request));
  }

  public ConnectionSetSessionParametersResponse connectionSetSessionParameters(
      ConnectionHandle connHandle, Map<String, String> parameters) {
    ConnectionSetSessionParametersRequest request =
        ConnectionSetSessionParametersRequest.newBuilder()
            .setConnHandle(connHandle)
            .putAllParameters(parameters)
            .build();
    return invoke(() -> client.connectionSetSessionParameters(request));
  }

  public ConnectionCloseResponse connectionClose(ConnectionHandle connHandle) {
    ConnectionCloseRequest request =
        ConnectionCloseRequest.newBuilder().setConnHandle(connHandle).build();
    return invoke(() -> client.connectionClose(request));
  }

  public ConnectionReleaseResponse connectionRelease(ConnectionHandle connHandle) {
    ConnectionReleaseRequest request =
        ConnectionReleaseRequest.newBuilder().setConnHandle(connHandle).build();
    return invoke(() -> client.connectionRelease(request));
  }

  public ConnectionIsClosedResponse connectionIsClosed(ConnectionHandle connHandle) {
    ConnectionIsClosedRequest request =
        ConnectionIsClosedRequest.newBuilder().setConnHandle(connHandle).build();
    return invoke(() -> client.connectionIsClosed(request));
  }

  public ConnectionHeartbeatResponse connectionHeartbeat(
      ConnectionHandle connHandle, int timeoutSeconds) {
    ConnectionHeartbeatRequest.Builder builder =
        ConnectionHeartbeatRequest.newBuilder().setConnHandle(connHandle);
    if (timeoutSeconds > 0) {
      builder.setTimeoutSeconds(timeoutSeconds);
    }
    return invoke(() -> client.connectionHeartbeat(builder.build()));
  }

  public ConnectionGetInfoResponse connectionGetInfo(ConnectionHandle connHandle) {
    ConnectionGetInfoRequest request =
        ConnectionGetInfoRequest.newBuilder().setConnHandle(connHandle).build();
    return invoke(() -> client.connectionGetInfo(request));
  }

  public ConnectionUseDatabaseResponse connectionUseDatabase(
      ConnectionHandle connHandle, String database) {
    ConnectionUseDatabaseRequest request =
        ConnectionUseDatabaseRequest.newBuilder()
            .setConnHandle(connHandle)
            .setDatabase(database)
            .build();
    return invoke(() -> client.connectionUseDatabase(request));
  }

  public ConnectionUseSchemaResponse connectionUseSchema(
      ConnectionHandle connHandle, String schema) {
    ConnectionUseSchemaRequest request =
        ConnectionUseSchemaRequest.newBuilder().setConnHandle(connHandle).setSchema(schema).build();
    return invoke(() -> client.connectionUseSchema(request));
  }

  public ConnectionGetQueryStatusResponse connectionGetQueryStatus(
      ConnectionHandle connHandle, String queryId) {
    ConnectionGetQueryStatusRequest request =
        ConnectionGetQueryStatusRequest.newBuilder()
            .setConnHandle(connHandle)
            .setQueryId(queryId)
            .build();
    return invoke(() -> client.connectionGetQueryStatus(request));
  }

  // =========================================================================
  // Connection data & queries
  // =========================================================================

  public ResultSetResponse connectionGetResultSet(ConnectionHandle connHandle, String queryId) {
    ConnectionGetResultSetRequest request =
        ConnectionGetResultSetRequest.newBuilder()
            .setConnHandle(connHandle)
            .setQueryId(queryId)
            .build();
    return invoke(() -> client.connectionGetResultSet(request));
  }

  public ExecuteQueryResponse connectionGetQueryResult(
      ConnectionHandle connHandle, String queryId) {
    ConnectionGetQueryResultRequest request =
        ConnectionGetQueryResultRequest.newBuilder()
            .setConnHandle(connHandle)
            .setQueryId(queryId)
            .build();
    return invoke(() -> client.connectionGetQueryResult(request));
  }

  public ConnectionAbortQueryResponse connectionAbortQuery(
      ConnectionHandle connHandle, String queryId) {
    ConnectionAbortQueryRequest request =
        ConnectionAbortQueryRequest.newBuilder()
            .setConnHandle(connHandle)
            .setQueryId(queryId)
            .build();
    return invoke(() -> client.connectionAbortQuery(request));
  }

  public ConnectionSendHttpResponse connectionSendHttp(ConnectionSendHttpRequest request) {
    return invoke(() -> client.connectionSendHttp(request));
  }

  // =========================================================================
  // Connection tokens & parameters
  // =========================================================================

  public ConnectionTokenResponse connectionRequestToken(
      ConnectionHandle connHandle, TokenRequestType requestType) {
    ConnectionTokenRequest request =
        ConnectionTokenRequest.newBuilder()
            .setConnHandle(connHandle)
            .setRequestType(requestType)
            .build();
    return invoke(() -> client.connectionRequestToken(request));
  }

  public ConnectionGetParameterResponse connectionGetParameter(
      ConnectionHandle connHandle, String key) {
    ConnectionGetParameterRequest request =
        ConnectionGetParameterRequest.newBuilder().setConnHandle(connHandle).setKey(key).build();
    return invoke(() -> client.connectionGetParameter(request));
  }

  public ConnectionGetAllParametersResponse connectionGetAllParameters(
      ConnectionHandle connHandle) {
    ConnectionGetAllParametersRequest request =
        ConnectionGetAllParametersRequest.newBuilder().setConnHandle(connHandle).build();
    return invoke(() -> client.connectionGetAllParameters(request));
  }

  // =========================================================================
  // Statement lifecycle
  // =========================================================================

  public StatementNewResponse statementNew(ConnectionHandle connHandle) {
    StatementNewRequest request =
        StatementNewRequest.newBuilder().setConnHandle(connHandle).build();
    return invoke(() -> client.statementNew(request));
  }

  public StatementSetSqlQueryResponse statementSetSqlQuery(StatementHandle stmtHandle, String sql) {
    StatementSetSqlQueryRequest request =
        StatementSetSqlQueryRequest.newBuilder().setStmtHandle(stmtHandle).setQuery(sql).build();
    return invoke(() -> client.statementSetSqlQuery(request));
  }

  public StatementPrepareResponse statementPrepare(StatementHandle stmtHandle) {
    StatementPrepareRequest request =
        StatementPrepareRequest.newBuilder().setStmtHandle(stmtHandle).build();
    return invoke(() -> client.statementPrepare(request));
  }

  public StatementSetOptionsResponse statementSetOptions(
      StatementHandle stmtHandle, Map<String, ConfigSetting> options) {
    StatementSetOptionsRequest request =
        StatementSetOptionsRequest.newBuilder()
            .setStmtHandle(stmtHandle)
            .putAllOptions(options)
            .build();
    return invoke(() -> client.statementSetOptions(request));
  }

  public ExecuteQueryResponse statementExecuteQuery(
      StatementHandle stmtHandle, QueryBindings bindings) {
    StatementExecuteQueryRequest.Builder builder =
        StatementExecuteQueryRequest.newBuilder().setStmtHandle(stmtHandle);
    if (bindings != null) {
      builder.setBindings(bindings);
    }
    StatementExecuteQueryRequest request = builder.build();
    return invoke(() -> client.statementExecuteQuery(request));
  }

  public StatementExecuteAsyncResponse statementExecuteAsync(
      StatementHandle stmtHandle, QueryBindings bindings) {
    StatementExecuteAsyncRequest.Builder builder =
        StatementExecuteAsyncRequest.newBuilder().setStmtHandle(stmtHandle);
    if (bindings != null) {
      builder.setBindings(bindings);
    }
    StatementExecuteAsyncRequest request = builder.build();
    return invoke(() -> client.statementExecuteAsync(request));
  }

  public StatementReleaseResponse statementRelease(StatementHandle stmtHandle) {
    StatementReleaseRequest request =
        StatementReleaseRequest.newBuilder().setStmtHandle(stmtHandle).build();
    return invoke(() -> client.statementRelease(request));
  }

  // =========================================================================
  // Result set
  // =========================================================================

  public ResultSetGetStreamResponse resultSetGetStream(ResultSetHandle resultSetHandle) {
    ResultSetGetStreamRequest request =
        ResultSetGetStreamRequest.newBuilder().setResultSetHandle(resultSetHandle).build();
    return invoke(() -> client.resultSetGetStream(request));
  }

  public ResultSetGetChunksResponse resultSetGetChunks(ResultSetHandle resultSetHandle) {
    ResultSetGetChunksRequest request =
        ResultSetGetChunksRequest.newBuilder().setResultSetHandle(resultSetHandle).build();
    return invoke(() -> client.resultSetGetChunks(request));
  }

  public ResultSetReleaseResponse resultSetRelease(ResultSetHandle resultSetHandle) {
    ResultSetReleaseRequest request =
        ResultSetReleaseRequest.newBuilder().setResultSetHandle(resultSetHandle).build();
    return invoke(() -> client.resultSetRelease(request));
  }

  @Override
  public DatabaseFetchChunkResponse databaseFetchChunk(
      List<ResultChunk> chunks, List<ColumnMetadata> columnMetadata) {
    DatabaseFetchChunkRequest request =
        DatabaseFetchChunkRequest.newBuilder()
            .addAllChunks(chunks)
            .addAllColumns(columnMetadata)
            .build();
    return invoke(() -> client.databaseFetchChunk(request));
  }

  // =========================================================================
  // Telemetry
  // =========================================================================

  public TelemetrySendResponse telemetrySendApiUsage(
      ConnectionHandle connHandle, String apiMethod) {
    TelemetrySendApiUsageRequest request =
        TelemetrySendApiUsageRequest.newBuilder()
            .setConnHandle(connHandle)
            .setApiMethod(apiMethod)
            .build();
    return invoke(() -> client.telemetrySendApiUsage(request));
  }

  public TelemetrySendResponse telemetrySendWrapperError(
      ConnectionHandle connHandle, String exceptionType, String errorSource) {
    TelemetrySendWrapperErrorRequest request =
        TelemetrySendWrapperErrorRequest.newBuilder()
            .setConnHandle(connHandle)
            .setExceptionType(exceptionType)
            .setErrorSource(errorSource)
            .build();
    return invoke(() -> client.telemetrySendWrapperError(request));
  }

  // =========================================================================
  // Stream-based file transfer (gap 4)
  // =========================================================================

  @Override
  public ConnectionUploadStreamBeginResponse connectionUploadStreamBegin(
      ConnectionHandle connHandle, String sql) {
    ConnectionUploadStreamBeginRequest request =
        ConnectionUploadStreamBeginRequest.newBuilder()
            .setConnHandle(connHandle)
            .setSql(sql)
            .build();
    return invoke(() -> client.connectionUploadStreamBegin(request));
  }

  @Override
  public ConnectionUploadStreamChunkResponse connectionUploadStreamChunk(
      UploadStreamHandle uploadHandle, byte[] data, int offset, int length) {
    ConnectionUploadStreamChunkRequest request =
        ConnectionUploadStreamChunkRequest.newBuilder()
            .setUploadHandle(uploadHandle)
            .setData(UnsafeByteOperations.unsafeWrap(data, offset, length))
            .build();
    return invoke(() -> client.connectionUploadStreamChunk(request));
  }

  @Override
  public ConnectionUploadStreamFinishResponse connectionUploadStreamFinish(
      UploadStreamHandle uploadHandle) {
    ConnectionUploadStreamFinishRequest request =
        ConnectionUploadStreamFinishRequest.newBuilder().setUploadHandle(uploadHandle).build();
    return invoke(() -> client.connectionUploadStreamFinish(request));
  }

  @Override
  public ConnectionUploadStreamAbortResponse connectionUploadStreamAbort(
      UploadStreamHandle uploadHandle) {
    ConnectionUploadStreamAbortRequest request =
        ConnectionUploadStreamAbortRequest.newBuilder().setUploadHandle(uploadHandle).build();
    return invoke(() -> client.connectionUploadStreamAbort(request));
  }

  @Override
  public ConnectionDownloadStreamBeginResponse connectionDownloadStreamBegin(
      ConnectionHandle connHandle, String stageName, String sourceFilename, boolean decompress) {
    ConnectionDownloadStreamBeginRequest request =
        ConnectionDownloadStreamBeginRequest.newBuilder()
            .setConnHandle(connHandle)
            .setStageName(stageName)
            .setSourceFilename(sourceFilename)
            .setDecompress(decompress)
            .build();
    return invoke(() -> client.connectionDownloadStreamBegin(request));
  }

  @Override
  public ConnectionDownloadStreamChunkResponse connectionDownloadStreamChunk(
      DownloadStreamHandle downloadHandle, long maxLen) {
    ConnectionDownloadStreamChunkRequest request =
        ConnectionDownloadStreamChunkRequest.newBuilder()
            .setDownloadHandle(downloadHandle)
            .setMaxLen(maxLen)
            .build();
    return invoke(() -> client.connectionDownloadStreamChunk(request));
  }

  @Override
  public ConnectionDownloadStreamCloseResponse connectionDownloadStreamClose(
      DownloadStreamHandle downloadHandle) {
    ConnectionDownloadStreamCloseRequest request =
        ConnectionDownloadStreamCloseRequest.newBuilder().setDownloadHandle(downloadHandle).build();
    return invoke(() -> client.connectionDownloadStreamClose(request));
  }

  // =========================================================================
  // Error handling
  // =========================================================================

  @FunctionalInterface
  private interface ServiceCall<T> {
    T call() throws ServiceException, TransportException;
  }

  private <T> T invoke(ServiceCall<T> callable) {
    try {
      return callable.call();
    } catch (ServiceException | TransportException e) {
      throw toCoreException(e);
    }
  }

  /**
   * Block on an async-first RPC future, mapping its failure to a {@link CoreException} runtime
   * carrier. {@link CoreFuture#get()} wraps the decoder's {@code ServiceException} / {@code
   * TransportException} in an {@link ExecutionException}, so unwrap the cause here.
   */
  private <T> T await(Future<T> future) {
    try {
      return future.get();
    } catch (ExecutionException e) {
      Throwable cause = e.getCause();
      if (cause instanceof ServiceException || cause instanceof TransportException) {
        throw toCoreException((RuntimeException) cause);
      }
      throw new CoreException(
          "Driver error: " + (cause != null ? cause.getMessage() : "unknown"),
          cause != null ? cause : e);
    } catch (InterruptedException e) {
      Thread.currentThread().interrupt();
      throw new CoreException("Interrupted while waiting for connection init", e);
    }
  }

  /**
   * Map an RPC failure — thrown directly by a blocking call or unwrapped from an {@link
   * ExecutionException} on the async path — to a {@link CoreException} runtime carrier. Any {@link
   * DriverException} payload is preserved verbatim so the decorator boundary can reconstruct the
   * core-reported SQLState / vendor code / query id.
   */
  private static CoreException toCoreException(RuntimeException e) {
    if (e instanceof ServiceException) {
      Object error = ((ServiceException) e).error;
      if (error instanceof DriverException) {
        return new CoreException((DriverException) error, e);
      }
      return new CoreException(e.getMessage(), e);
    }
    // TransportException
    return new CoreException("Driver communication error: " + e.getMessage(), e);
  }
}
