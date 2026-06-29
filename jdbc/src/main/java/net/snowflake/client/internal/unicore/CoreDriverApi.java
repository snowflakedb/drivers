package net.snowflake.client.internal.unicore;

import java.sql.SQLException;
import java.util.Map;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.ConfigSetting;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.ConnectionAbortQueryResponse;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.ConnectionCloseResponse;
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
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.ConnectionSendHttpRequest;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.ConnectionSendHttpResponse;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.ConnectionSetOptionsResponse;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.ConnectionSetSessionParametersResponse;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.ConnectionTokenResponse;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.ConnectionUseDatabaseResponse;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.ConnectionUseSchemaResponse;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.DatabaseHandle;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.DatabaseInitResponse;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.DatabaseNewResponse;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.DatabaseReleaseResponse;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.ExecuteQueryResponse;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.QueryBindings;
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
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.WrapperIdentity;

public interface CoreDriverApi {

  // Database lifecycle

  DatabaseNewResponse databaseNew() throws SQLException;

  DatabaseInitResponse databaseInit(DatabaseHandle dbHandle) throws SQLException;

  DatabaseReleaseResponse databaseRelease(DatabaseHandle dbHandle) throws SQLException;

  // Connection lifecycle

  ConnectionNewResponse connectionNew() throws SQLException;

  ConnectionInitResponse connectionInit(
      ConnectionHandle connHandle, DatabaseHandle dbHandle, WrapperIdentity wrapperIdentity)
      throws SQLException;

  ConnectionSetOptionsResponse connectionSetOptions(
      ConnectionHandle connHandle, Map<String, ConfigSetting> options) throws SQLException;

  ConnectionSetSessionParametersResponse connectionSetSessionParameters(
      ConnectionHandle connHandle, Map<String, String> parameters) throws SQLException;

  ConnectionCloseResponse connectionClose(ConnectionHandle connHandle) throws SQLException;

  ConnectionReleaseResponse connectionRelease(ConnectionHandle connHandle) throws SQLException;

  ConnectionIsClosedResponse connectionIsClosed(ConnectionHandle connHandle) throws SQLException;

  ConnectionHeartbeatResponse connectionHeartbeat(ConnectionHandle connHandle, int timeoutSeconds)
      throws SQLException;

  ConnectionGetInfoResponse connectionGetInfo(ConnectionHandle connHandle) throws SQLException;

  ConnectionUseDatabaseResponse connectionUseDatabase(ConnectionHandle connHandle, String database)
      throws SQLException;

  ConnectionUseSchemaResponse connectionUseSchema(ConnectionHandle connHandle, String schema)
      throws SQLException;

  ConnectionGetQueryStatusResponse connectionGetQueryStatus(
      ConnectionHandle connHandle, String queryId) throws SQLException;

  // Connection data & queries

  ResultSetResponse connectionGetResultSet(ConnectionHandle connHandle, String queryId)
      throws SQLException;

  ExecuteQueryResponse connectionGetQueryResult(ConnectionHandle connHandle, String queryId)
      throws SQLException;

  ConnectionAbortQueryResponse connectionAbortQuery(ConnectionHandle connHandle, String queryId)
      throws SQLException;

  ConnectionSendHttpResponse connectionSendHttp(ConnectionSendHttpRequest request)
      throws SQLException;

  // Connection tokens & parameters

  ConnectionTokenResponse connectionRequestToken(
      ConnectionHandle connHandle, TokenRequestType requestType) throws SQLException;

  ConnectionGetParameterResponse connectionGetParameter(ConnectionHandle connHandle, String key)
      throws SQLException;

  ConnectionGetAllParametersResponse connectionGetAllParameters(ConnectionHandle connHandle)
      throws SQLException;

  // Statement lifecycle

  StatementNewResponse statementNew(ConnectionHandle connHandle) throws SQLException;

  StatementSetSqlQueryResponse statementSetSqlQuery(StatementHandle stmtHandle, String sql)
      throws SQLException;

  StatementPrepareResponse statementPrepare(StatementHandle stmtHandle) throws SQLException;

  StatementSetOptionsResponse statementSetOptions(
      StatementHandle stmtHandle, Map<String, ConfigSetting> options) throws SQLException;

  ExecuteQueryResponse statementExecuteQuery(StatementHandle stmtHandle, QueryBindings bindings)
      throws SQLException;

  StatementExecuteAsyncResponse statementExecuteAsync(
      StatementHandle stmtHandle, QueryBindings bindings) throws SQLException;

  StatementReleaseResponse statementRelease(StatementHandle stmtHandle) throws SQLException;

  // Result set

  ResultSetGetStreamResponse resultSetGetStream(ResultSetHandle resultSetHandle)
      throws SQLException;

  ResultSetGetChunksResponse resultSetGetChunks(ResultSetHandle resultSetHandle)
      throws SQLException;

  ResultSetReleaseResponse resultSetRelease(ResultSetHandle resultSetHandle) throws SQLException;

  // Telemetry

  TelemetrySendResponse telemetrySendApiUsage(ConnectionHandle connHandle, String apiMethod)
      throws SQLException;

  TelemetrySendResponse telemetrySendWrapperError(
      ConnectionHandle connHandle, String exceptionType, String errorSource) throws SQLException;
}
