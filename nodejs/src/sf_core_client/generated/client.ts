// Auto-generated from database_driver_v1.proto -- DO NOT EDIT
import { callProto } from "../transport";
import { database_driver_v1 as proto } from "./proto";

export class SfCoreClient {
  async databaseNew(
    request: proto.IDatabaseNewRequest,
  ): Promise<proto.DatabaseNewResponse> {
    return callProto<proto.IDatabaseNewRequest, proto.DatabaseNewResponse>(
      "database_new",
      "DatabaseNewRequest",
      "DatabaseNewResponse",
      request,
    );
  }

  async databaseSetOptionString(
    request: proto.IDatabaseSetOptionStringRequest,
  ): Promise<proto.DatabaseSetOptionStringResponse> {
    return callProto<proto.IDatabaseSetOptionStringRequest, proto.DatabaseSetOptionStringResponse>(
      "database_set_option_string",
      "DatabaseSetOptionStringRequest",
      "DatabaseSetOptionStringResponse",
      request,
    );
  }

  async databaseSetOptionBytes(
    request: proto.IDatabaseSetOptionBytesRequest,
  ): Promise<proto.DatabaseSetOptionBytesResponse> {
    return callProto<proto.IDatabaseSetOptionBytesRequest, proto.DatabaseSetOptionBytesResponse>(
      "database_set_option_bytes",
      "DatabaseSetOptionBytesRequest",
      "DatabaseSetOptionBytesResponse",
      request,
    );
  }

  async databaseSetOptionInt(
    request: proto.IDatabaseSetOptionIntRequest,
  ): Promise<proto.DatabaseSetOptionIntResponse> {
    return callProto<proto.IDatabaseSetOptionIntRequest, proto.DatabaseSetOptionIntResponse>(
      "database_set_option_int",
      "DatabaseSetOptionIntRequest",
      "DatabaseSetOptionIntResponse",
      request,
    );
  }

  async databaseSetOptionDouble(
    request: proto.IDatabaseSetOptionDoubleRequest,
  ): Promise<proto.DatabaseSetOptionDoubleResponse> {
    return callProto<proto.IDatabaseSetOptionDoubleRequest, proto.DatabaseSetOptionDoubleResponse>(
      "database_set_option_double",
      "DatabaseSetOptionDoubleRequest",
      "DatabaseSetOptionDoubleResponse",
      request,
    );
  }

  async databaseInit(
    request: proto.IDatabaseInitRequest,
  ): Promise<proto.DatabaseInitResponse> {
    return callProto<proto.IDatabaseInitRequest, proto.DatabaseInitResponse>(
      "database_init",
      "DatabaseInitRequest",
      "DatabaseInitResponse",
      request,
    );
  }

  async databaseRelease(
    request: proto.IDatabaseReleaseRequest,
  ): Promise<proto.DatabaseReleaseResponse> {
    return callProto<proto.IDatabaseReleaseRequest, proto.DatabaseReleaseResponse>(
      "database_release",
      "DatabaseReleaseRequest",
      "DatabaseReleaseResponse",
      request,
    );
  }

  async connectionNew(
    request: proto.IConnectionNewRequest,
  ): Promise<proto.ConnectionNewResponse> {
    return callProto<proto.IConnectionNewRequest, proto.ConnectionNewResponse>(
      "connection_new",
      "ConnectionNewRequest",
      "ConnectionNewResponse",
      request,
    );
  }

  async connectionSetOptionString(
    request: proto.IConnectionSetOptionStringRequest,
  ): Promise<proto.ConnectionSetOptionStringResponse> {
    return callProto<proto.IConnectionSetOptionStringRequest, proto.ConnectionSetOptionStringResponse>(
      "connection_set_option_string",
      "ConnectionSetOptionStringRequest",
      "ConnectionSetOptionStringResponse",
      request,
    );
  }

  async connectionSetOptionBytes(
    request: proto.IConnectionSetOptionBytesRequest,
  ): Promise<proto.ConnectionSetOptionBytesResponse> {
    return callProto<proto.IConnectionSetOptionBytesRequest, proto.ConnectionSetOptionBytesResponse>(
      "connection_set_option_bytes",
      "ConnectionSetOptionBytesRequest",
      "ConnectionSetOptionBytesResponse",
      request,
    );
  }

  async connectionSetOptionInt(
    request: proto.IConnectionSetOptionIntRequest,
  ): Promise<proto.ConnectionSetOptionIntResponse> {
    return callProto<proto.IConnectionSetOptionIntRequest, proto.ConnectionSetOptionIntResponse>(
      "connection_set_option_int",
      "ConnectionSetOptionIntRequest",
      "ConnectionSetOptionIntResponse",
      request,
    );
  }

  async connectionSetOptionDouble(
    request: proto.IConnectionSetOptionDoubleRequest,
  ): Promise<proto.ConnectionSetOptionDoubleResponse> {
    return callProto<proto.IConnectionSetOptionDoubleRequest, proto.ConnectionSetOptionDoubleResponse>(
      "connection_set_option_double",
      "ConnectionSetOptionDoubleRequest",
      "ConnectionSetOptionDoubleResponse",
      request,
    );
  }

  async connectionInit(
    request: proto.IConnectionInitRequest,
  ): Promise<proto.ConnectionInitResponse> {
    return callProto<proto.IConnectionInitRequest, proto.ConnectionInitResponse>(
      "connection_init",
      "ConnectionInitRequest",
      "ConnectionInitResponse",
      request,
    );
  }

  async connectionRelease(
    request: proto.IConnectionReleaseRequest,
  ): Promise<proto.ConnectionReleaseResponse> {
    return callProto<proto.IConnectionReleaseRequest, proto.ConnectionReleaseResponse>(
      "connection_release",
      "ConnectionReleaseRequest",
      "ConnectionReleaseResponse",
      request,
    );
  }

  async connectionGetInfo(
    request: proto.IConnectionGetInfoRequest,
  ): Promise<proto.ConnectionGetInfoResponse> {
    return callProto<proto.IConnectionGetInfoRequest, proto.ConnectionGetInfoResponse>(
      "connection_get_info",
      "ConnectionGetInfoRequest",
      "ConnectionGetInfoResponse",
      request,
    );
  }

  async connectionGetObjects(
    request: proto.IConnectionGetObjectsRequest,
  ): Promise<proto.ConnectionGetObjectsResponse> {
    return callProto<proto.IConnectionGetObjectsRequest, proto.ConnectionGetObjectsResponse>(
      "connection_get_objects",
      "ConnectionGetObjectsRequest",
      "ConnectionGetObjectsResponse",
      request,
    );
  }

  async connectionGetTableSchema(
    request: proto.IConnectionGetTableSchemaRequest,
  ): Promise<proto.ConnectionGetTableSchemaResponse> {
    return callProto<proto.IConnectionGetTableSchemaRequest, proto.ConnectionGetTableSchemaResponse>(
      "connection_get_table_schema",
      "ConnectionGetTableSchemaRequest",
      "ConnectionGetTableSchemaResponse",
      request,
    );
  }

  async connectionGetTableTypes(
    request: proto.IConnectionGetTableTypesRequest,
  ): Promise<proto.ConnectionGetTableTypesResponse> {
    return callProto<proto.IConnectionGetTableTypesRequest, proto.ConnectionGetTableTypesResponse>(
      "connection_get_table_types",
      "ConnectionGetTableTypesRequest",
      "ConnectionGetTableTypesResponse",
      request,
    );
  }

  async connectionCommit(
    request: proto.IConnectionCommitRequest,
  ): Promise<proto.ConnectionCommitResponse> {
    return callProto<proto.IConnectionCommitRequest, proto.ConnectionCommitResponse>(
      "connection_commit",
      "ConnectionCommitRequest",
      "ConnectionCommitResponse",
      request,
    );
  }

  async connectionRollback(
    request: proto.IConnectionRollbackRequest,
  ): Promise<proto.ConnectionRollbackResponse> {
    return callProto<proto.IConnectionRollbackRequest, proto.ConnectionRollbackResponse>(
      "connection_rollback",
      "ConnectionRollbackRequest",
      "ConnectionRollbackResponse",
      request,
    );
  }

  async connectionSetSessionParameters(
    request: proto.IConnectionSetSessionParametersRequest,
  ): Promise<proto.ConnectionSetSessionParametersResponse> {
    return callProto<proto.IConnectionSetSessionParametersRequest, proto.ConnectionSetSessionParametersResponse>(
      "connection_set_session_parameters",
      "ConnectionSetSessionParametersRequest",
      "ConnectionSetSessionParametersResponse",
      request,
    );
  }

  async connectionGetParameter(
    request: proto.IConnectionGetParameterRequest,
  ): Promise<proto.ConnectionGetParameterResponse> {
    return callProto<proto.IConnectionGetParameterRequest, proto.ConnectionGetParameterResponse>(
      "connection_get_parameter",
      "ConnectionGetParameterRequest",
      "ConnectionGetParameterResponse",
      request,
    );
  }

  async statementNew(
    request: proto.IStatementNewRequest,
  ): Promise<proto.StatementNewResponse> {
    return callProto<proto.IStatementNewRequest, proto.StatementNewResponse>(
      "statement_new",
      "StatementNewRequest",
      "StatementNewResponse",
      request,
    );
  }

  async statementRelease(
    request: proto.IStatementReleaseRequest,
  ): Promise<proto.StatementReleaseResponse> {
    return callProto<proto.IStatementReleaseRequest, proto.StatementReleaseResponse>(
      "statement_release",
      "StatementReleaseRequest",
      "StatementReleaseResponse",
      request,
    );
  }

  async statementSetSqlQuery(
    request: proto.IStatementSetSqlQueryRequest,
  ): Promise<proto.StatementSetSqlQueryResponse> {
    return callProto<proto.IStatementSetSqlQueryRequest, proto.StatementSetSqlQueryResponse>(
      "statement_set_sql_query",
      "StatementSetSqlQueryRequest",
      "StatementSetSqlQueryResponse",
      request,
    );
  }

  async statementSetSubstraitPlan(
    request: proto.IStatementSetSubstraitPlanRequest,
  ): Promise<proto.StatementSetSubstraitPlanResponse> {
    return callProto<proto.IStatementSetSubstraitPlanRequest, proto.StatementSetSubstraitPlanResponse>(
      "statement_set_substrait_plan",
      "StatementSetSubstraitPlanRequest",
      "StatementSetSubstraitPlanResponse",
      request,
    );
  }

  async statementPrepare(
    request: proto.IStatementPrepareRequest,
  ): Promise<proto.StatementPrepareResponse> {
    return callProto<proto.IStatementPrepareRequest, proto.StatementPrepareResponse>(
      "statement_prepare",
      "StatementPrepareRequest",
      "StatementPrepareResponse",
      request,
    );
  }

  async statementSetOptionString(
    request: proto.IStatementSetOptionStringRequest,
  ): Promise<proto.StatementSetOptionStringResponse> {
    return callProto<proto.IStatementSetOptionStringRequest, proto.StatementSetOptionStringResponse>(
      "statement_set_option_string",
      "StatementSetOptionStringRequest",
      "StatementSetOptionStringResponse",
      request,
    );
  }

  async statementSetOptionBytes(
    request: proto.IStatementSetOptionBytesRequest,
  ): Promise<proto.StatementSetOptionBytesResponse> {
    return callProto<proto.IStatementSetOptionBytesRequest, proto.StatementSetOptionBytesResponse>(
      "statement_set_option_bytes",
      "StatementSetOptionBytesRequest",
      "StatementSetOptionBytesResponse",
      request,
    );
  }

  async statementSetOptionInt(
    request: proto.IStatementSetOptionIntRequest,
  ): Promise<proto.StatementSetOptionIntResponse> {
    return callProto<proto.IStatementSetOptionIntRequest, proto.StatementSetOptionIntResponse>(
      "statement_set_option_int",
      "StatementSetOptionIntRequest",
      "StatementSetOptionIntResponse",
      request,
    );
  }

  async statementSetOptionDouble(
    request: proto.IStatementSetOptionDoubleRequest,
  ): Promise<proto.StatementSetOptionDoubleResponse> {
    return callProto<proto.IStatementSetOptionDoubleRequest, proto.StatementSetOptionDoubleResponse>(
      "statement_set_option_double",
      "StatementSetOptionDoubleRequest",
      "StatementSetOptionDoubleResponse",
      request,
    );
  }

  async statementGetParameterSchema(
    request: proto.IStatementGetParameterSchemaRequest,
  ): Promise<proto.StatementGetParameterSchemaResponse> {
    return callProto<proto.IStatementGetParameterSchemaRequest, proto.StatementGetParameterSchemaResponse>(
      "statement_get_parameter_schema",
      "StatementGetParameterSchemaRequest",
      "StatementGetParameterSchemaResponse",
      request,
    );
  }

  async statementExecuteQuery(
    request: proto.IStatementExecuteQueryRequest,
  ): Promise<proto.StatementExecuteQueryResponse> {
    return callProto<proto.IStatementExecuteQueryRequest, proto.StatementExecuteQueryResponse>(
      "statement_execute_query",
      "StatementExecuteQueryRequest",
      "StatementExecuteQueryResponse",
      request,
    );
  }

  async statementExecutePartitions(
    request: proto.IStatementExecutePartitionsRequest,
  ): Promise<proto.StatementExecutePartitionsResponse> {
    return callProto<proto.IStatementExecutePartitionsRequest, proto.StatementExecutePartitionsResponse>(
      "statement_execute_partitions",
      "StatementExecutePartitionsRequest",
      "StatementExecutePartitionsResponse",
      request,
    );
  }

  async statementReadPartition(
    request: proto.IStatementReadPartitionRequest,
  ): Promise<proto.StatementReadPartitionResponse> {
    return callProto<proto.IStatementReadPartitionRequest, proto.StatementReadPartitionResponse>(
      "statement_read_partition",
      "StatementReadPartitionRequest",
      "StatementReadPartitionResponse",
      request,
    );
  }

  async configLoadAllSections(
    request: proto.IConfigLoadAllSectionsRequest,
  ): Promise<proto.ConfigLoadAllSectionsResponse> {
    return callProto<proto.IConfigLoadAllSectionsRequest, proto.ConfigLoadAllSectionsResponse>(
      "config_load_all_sections",
      "ConfigLoadAllSectionsRequest",
      "ConfigLoadAllSectionsResponse",
      request,
    );
  }

  async configGetPaths(
    request: proto.IConfigGetPathsRequest,
  ): Promise<proto.ConfigGetPathsResponse> {
    return callProto<proto.IConfigGetPathsRequest, proto.ConfigGetPathsResponse>(
      "config_get_paths",
      "ConfigGetPathsRequest",
      "ConfigGetPathsResponse",
      request,
    );
  }

}
