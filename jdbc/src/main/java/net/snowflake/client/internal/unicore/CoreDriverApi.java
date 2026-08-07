package net.snowflake.client.internal.unicore;

import java.util.List;
import java.util.Map;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.ColumnMetadata;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.ConfigSetting;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.ConnectionAbortQueryResponse;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.ConnectionCloseResponse;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.ConnectionCommitResponse;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.ConnectionDownloadStreamBeginResponse;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.ConnectionDownloadStreamChunkResponse;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.ConnectionDownloadStreamCloseResponse;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.ConnectionGetAllParametersResponse;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.ConnectionGetInfoResponse;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.ConnectionGetParameterResponse;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.ConnectionGetQueryStatusResponse;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.ConnectionHandle;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.ConnectionHeartbeatResponse;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.ConnectionInitResponse;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.ConnectionIsClosedResponse;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.ConnectionNewResponse;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.ConnectionReleaseResponse;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.ConnectionRollbackResponse;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.ConnectionSendHttpRequest;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.ConnectionSendHttpResponse;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.ConnectionSetAutocommitResponse;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.ConnectionSetOptionsResponse;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.ConnectionSetSessionParametersResponse;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.ConnectionTokenResponse;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.ConnectionUploadStreamAbortResponse;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.ConnectionUploadStreamBeginResponse;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.ConnectionUploadStreamChunkResponse;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.ConnectionUploadStreamFinishResponse;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.ConnectionUseDatabaseResponse;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.ConnectionUseSchemaResponse;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.DatabaseFetchChunkResponse;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.DatabaseHandle;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.DatabaseInitResponse;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.DatabaseNewResponse;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.DatabaseReleaseResponse;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.DownloadStreamHandle;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.ExecuteQueryResponse;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.QueryBindings;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.ResultChunk;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.ResultSetGetChunksResponse;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.ResultSetGetStreamResponse;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.ResultSetHandle;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.ResultSetReleaseResponse;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.ResultSetResponse;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.StatementExecuteAsyncResponse;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.StatementHandle;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.StatementNewResponse;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.StatementPrepareResponse;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.StatementReleaseResponse;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.StatementSetOptionsResponse;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.StatementSetSqlQueryResponse;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.TelemetrySendResponse;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.TokenRequestType;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.UploadStreamHandle;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.WrapperIdentity;

public interface CoreDriverApi {

  // Database lifecycle

  DatabaseNewResponse databaseNew();

  DatabaseInitResponse databaseInit(DatabaseHandle dbHandle);

  DatabaseReleaseResponse databaseRelease(DatabaseHandle dbHandle);

  // Connection lifecycle

  ConnectionNewResponse connectionNew();

  ConnectionInitResponse connectionInit(
      ConnectionHandle connHandle, DatabaseHandle dbHandle, WrapperIdentity wrapperIdentity);

  ConnectionSetOptionsResponse connectionSetOptions(
      ConnectionHandle connHandle, Map<String, ConfigSetting> options);

  ConnectionSetAutocommitResponse connectionSetAutocommit(
      ConnectionHandle connHandle, boolean autocommit);

  ConnectionCommitResponse connectionCommit(ConnectionHandle connHandle);

  ConnectionRollbackResponse connectionRollback(ConnectionHandle connHandle);

  ConnectionSetSessionParametersResponse connectionSetSessionParameters(
      ConnectionHandle connHandle, Map<String, String> parameters);

  ConnectionCloseResponse connectionClose(ConnectionHandle connHandle);

  ConnectionReleaseResponse connectionRelease(ConnectionHandle connHandle);

  ConnectionIsClosedResponse connectionIsClosed(ConnectionHandle connHandle);

  ConnectionHeartbeatResponse connectionHeartbeat(ConnectionHandle connHandle, int timeoutSeconds);

  ConnectionGetInfoResponse connectionGetInfo(ConnectionHandle connHandle);

  ConnectionUseDatabaseResponse connectionUseDatabase(ConnectionHandle connHandle, String database);

  ConnectionUseSchemaResponse connectionUseSchema(ConnectionHandle connHandle, String schema);

  ConnectionGetQueryStatusResponse connectionGetQueryStatus(
      ConnectionHandle connHandle, String queryId);

  // Connection data & queries

  ResultSetResponse connectionGetResultSet(ConnectionHandle connHandle, String queryId);

  ExecuteQueryResponse connectionGetQueryResult(ConnectionHandle connHandle, String queryId);

  ConnectionAbortQueryResponse connectionAbortQuery(ConnectionHandle connHandle, String queryId);

  ConnectionSendHttpResponse connectionSendHttp(ConnectionSendHttpRequest request);

  // Connection tokens & parameters

  ConnectionTokenResponse connectionRequestToken(
      ConnectionHandle connHandle, TokenRequestType requestType);

  ConnectionGetParameterResponse connectionGetParameter(ConnectionHandle connHandle, String key);

  ConnectionGetAllParametersResponse connectionGetAllParameters(ConnectionHandle connHandle);

  // Statement lifecycle

  StatementNewResponse statementNew(ConnectionHandle connHandle);

  StatementSetSqlQueryResponse statementSetSqlQuery(StatementHandle stmtHandle, String sql);

  StatementPrepareResponse statementPrepare(StatementHandle stmtHandle);

  StatementSetOptionsResponse statementSetOptions(
      StatementHandle stmtHandle, Map<String, ConfigSetting> options);

  ExecuteQueryResponse statementExecuteQuery(StatementHandle stmtHandle, QueryBindings bindings);

  StatementExecuteAsyncResponse statementExecuteAsync(
      StatementHandle stmtHandle, QueryBindings bindings);

  StatementReleaseResponse statementRelease(StatementHandle stmtHandle);

  // Result set

  ResultSetGetStreamResponse resultSetGetStream(ResultSetHandle resultSetHandle);

  ResultSetGetChunksResponse resultSetGetChunks(ResultSetHandle resultSetHandle);

  ResultSetReleaseResponse resultSetRelease(ResultSetHandle resultSetHandle);

  DatabaseFetchChunkResponse databaseFetchChunk(
      List<ResultChunk> chunk, List<ColumnMetadata> columnMetadata);

  // Telemetry

  TelemetrySendResponse telemetrySendApiUsage(ConnectionHandle connHandle, String apiMethod);

  TelemetrySendResponse telemetrySendWrapperError(
      ConnectionHandle connHandle, String exceptionType, String errorSource);

  // Stream-based file transfer (gap 4)

  /**
   * Begins a chunked upload: registers the PUT SQL (including AUTO_COMPRESS / OVERWRITE clauses,
   * already synthesized by the wrapper) with core, which does not contact GS until {@link
   * #connectionUploadStreamFinish}. Bounds wrapper memory to ~one chunk regardless of source size.
   */
  ConnectionUploadStreamBeginResponse connectionUploadStreamBegin(
      ConnectionHandle connHandle, String sql);

  /**
   * Appends one chunk of the upload source to the session opened by {@link
   * #connectionUploadStreamBegin}. Only {@code data[offset, offset + length)} is sent — callers
   * with a reusable read buffer do not need to trim it into a fresh array first.
   */
  ConnectionUploadStreamChunkResponse connectionUploadStreamChunk(
      UploadStreamHandle uploadHandle, byte[] data, int offset, int length);

  /**
   * Finishes a chunked upload: closes the session, reassembles the buffered chunks, and runs the
   * PUT exactly as a file-path PUT would.
   */
  ConnectionUploadStreamFinishResponse connectionUploadStreamFinish(
      UploadStreamHandle uploadHandle);

  /**
   * Aborts a chunked upload without running the PUT (e.g. the caller's source stream failed
   * mid-read). Any buffered bytes are discarded.
   */
  ConnectionUploadStreamAbortResponse connectionUploadStreamAbort(UploadStreamHandle uploadHandle);

  /**
   * Begins a chunked, zero-disk download: resolves {@code stageName} + {@code sourceFilename}
   * against GS and opens a streaming GET directly against cloud storage. Core synthesizes the GET
   * SQL internally because the GET protocol requires a local destination path that is meaningless
   * across the JNI boundary.
   */
  ConnectionDownloadStreamBeginResponse connectionDownloadStreamBegin(
      ConnectionHandle connHandle, String stageName, String sourceFilename, boolean decompress);

  /**
   * Pulls up to {@code maxLen} bytes from the session opened by {@link
   * #connectionDownloadStreamBegin}. The response's {@code eof} flag is set once the producer has
   * finished and no more bytes remain.
   */
  ConnectionDownloadStreamChunkResponse connectionDownloadStreamChunk(
      DownloadStreamHandle downloadHandle, long maxLen);

  /**
   * Closes the session opened by {@link #connectionDownloadStreamBegin}, aborting the in-flight
   * download if it has not already finished. Safe to call after eof, or early (e.g. the caller's
   * consumer failed mid-read).
   */
  ConnectionDownloadStreamCloseResponse connectionDownloadStreamClose(
      DownloadStreamHandle downloadHandle);
}
