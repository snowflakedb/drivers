package com.snowflake.unicore.protobuf_gen;


import com.snowflake.unicore.TransportException;

/**
 * Service interface for DatabaseDriver
 * This file is auto-generated. Do not edit manually.
 */
public interface DatabaseDriverService {
    /**
     * Method: databaseNew
     */
    DatabaseDriverV1.DatabaseNewResponse databaseNew(DatabaseDriverV1.DatabaseNewRequest request) throws ServiceException, TransportException;

    /**
     * Method: databaseSetOptionString
     */
    DatabaseDriverV1.DatabaseSetOptionStringResponse databaseSetOptionString(DatabaseDriverV1.DatabaseSetOptionStringRequest request) throws ServiceException, TransportException;

    /**
     * Method: databaseSetOptionBytes
     */
    DatabaseDriverV1.DatabaseSetOptionBytesResponse databaseSetOptionBytes(DatabaseDriverV1.DatabaseSetOptionBytesRequest request) throws ServiceException, TransportException;

    /**
     * Method: databaseSetOptionInt
     */
    DatabaseDriverV1.DatabaseSetOptionIntResponse databaseSetOptionInt(DatabaseDriverV1.DatabaseSetOptionIntRequest request) throws ServiceException, TransportException;

    /**
     * Method: databaseSetOptionDouble
     */
    DatabaseDriverV1.DatabaseSetOptionDoubleResponse databaseSetOptionDouble(DatabaseDriverV1.DatabaseSetOptionDoubleRequest request) throws ServiceException, TransportException;

    /**
     * Method: databaseInit
     */
    DatabaseDriverV1.DatabaseInitResponse databaseInit(DatabaseDriverV1.DatabaseInitRequest request) throws ServiceException, TransportException;

    /**
     * Method: databaseRelease
     */
    DatabaseDriverV1.DatabaseReleaseResponse databaseRelease(DatabaseDriverV1.DatabaseReleaseRequest request) throws ServiceException, TransportException;

    /**
     * Method: connectionNew
     */
    DatabaseDriverV1.ConnectionNewResponse connectionNew(DatabaseDriverV1.ConnectionNewRequest request) throws ServiceException, TransportException;

    /**
     * Method: connectionSetOptionString
     */
    DatabaseDriverV1.ConnectionSetOptionStringResponse connectionSetOptionString(DatabaseDriverV1.ConnectionSetOptionStringRequest request) throws ServiceException, TransportException;

    /**
     * Method: connectionSetOptionBytes
     */
    DatabaseDriverV1.ConnectionSetOptionBytesResponse connectionSetOptionBytes(DatabaseDriverV1.ConnectionSetOptionBytesRequest request) throws ServiceException, TransportException;

    /**
     * Method: connectionSetOptionInt
     */
    DatabaseDriverV1.ConnectionSetOptionIntResponse connectionSetOptionInt(DatabaseDriverV1.ConnectionSetOptionIntRequest request) throws ServiceException, TransportException;

    /**
     * Method: connectionSetOptionDouble
     */
    DatabaseDriverV1.ConnectionSetOptionDoubleResponse connectionSetOptionDouble(DatabaseDriverV1.ConnectionSetOptionDoubleRequest request) throws ServiceException, TransportException;

    /**
     * Method: connectionInit
     */
    DatabaseDriverV1.ConnectionInitResponse connectionInit(DatabaseDriverV1.ConnectionInitRequest request) throws ServiceException, TransportException;

    /**
     * Method: connectionRelease
     */
    DatabaseDriverV1.ConnectionReleaseResponse connectionRelease(DatabaseDriverV1.ConnectionReleaseRequest request) throws ServiceException, TransportException;

    /**
     * Method: connectionGetInfo
     */
    DatabaseDriverV1.ConnectionGetInfoResponse connectionGetInfo(DatabaseDriverV1.ConnectionGetInfoRequest request) throws ServiceException, TransportException;

    /**
     * Method: connectionGetObjects
     */
    DatabaseDriverV1.ConnectionGetObjectsResponse connectionGetObjects(DatabaseDriverV1.ConnectionGetObjectsRequest request) throws ServiceException, TransportException;

    /**
     * Method: connectionGetTableSchema
     */
    DatabaseDriverV1.ConnectionGetTableSchemaResponse connectionGetTableSchema(DatabaseDriverV1.ConnectionGetTableSchemaRequest request) throws ServiceException, TransportException;

    /**
     * Method: connectionGetTableTypes
     */
    DatabaseDriverV1.ConnectionGetTableTypesResponse connectionGetTableTypes(DatabaseDriverV1.ConnectionGetTableTypesRequest request) throws ServiceException, TransportException;

    /**
     * Method: connectionCommit
     */
    DatabaseDriverV1.ConnectionCommitResponse connectionCommit(DatabaseDriverV1.ConnectionCommitRequest request) throws ServiceException, TransportException;

    /**
     * Method: connectionRollback
     */
    DatabaseDriverV1.ConnectionRollbackResponse connectionRollback(DatabaseDriverV1.ConnectionRollbackRequest request) throws ServiceException, TransportException;

    /**
     * Method: statementNew
     */
    DatabaseDriverV1.StatementNewResponse statementNew(DatabaseDriverV1.StatementNewRequest request) throws ServiceException, TransportException;

    /**
     * Method: statementRelease
     */
    DatabaseDriverV1.StatementReleaseResponse statementRelease(DatabaseDriverV1.StatementReleaseRequest request) throws ServiceException, TransportException;

    /**
     * Method: statementSetSqlQuery
     */
    DatabaseDriverV1.StatementSetSqlQueryResponse statementSetSqlQuery(DatabaseDriverV1.StatementSetSqlQueryRequest request) throws ServiceException, TransportException;

    /**
     * Method: statementSetSubstraitPlan
     */
    DatabaseDriverV1.StatementSetSubstraitPlanResponse statementSetSubstraitPlan(DatabaseDriverV1.StatementSetSubstraitPlanRequest request) throws ServiceException, TransportException;

    /**
     * Method: statementPrepare
     */
    DatabaseDriverV1.StatementPrepareResponse statementPrepare(DatabaseDriverV1.StatementPrepareRequest request) throws ServiceException, TransportException;

    /**
     * Method: statementSetOptionString
     */
    DatabaseDriverV1.StatementSetOptionStringResponse statementSetOptionString(DatabaseDriverV1.StatementSetOptionStringRequest request) throws ServiceException, TransportException;

    /**
     * Method: statementSetOptionBytes
     */
    DatabaseDriverV1.StatementSetOptionBytesResponse statementSetOptionBytes(DatabaseDriverV1.StatementSetOptionBytesRequest request) throws ServiceException, TransportException;

    /**
     * Method: statementSetOptionInt
     */
    DatabaseDriverV1.StatementSetOptionIntResponse statementSetOptionInt(DatabaseDriverV1.StatementSetOptionIntRequest request) throws ServiceException, TransportException;

    /**
     * Method: statementSetOptionDouble
     */
    DatabaseDriverV1.StatementSetOptionDoubleResponse statementSetOptionDouble(DatabaseDriverV1.StatementSetOptionDoubleRequest request) throws ServiceException, TransportException;

    /**
     * Method: statementGetParameterSchema
     */
    DatabaseDriverV1.StatementGetParameterSchemaResponse statementGetParameterSchema(DatabaseDriverV1.StatementGetParameterSchemaRequest request) throws ServiceException, TransportException;

    /**
     * Method: statementBind
     */
    DatabaseDriverV1.StatementBindResponse statementBind(DatabaseDriverV1.StatementBindRequest request) throws ServiceException, TransportException;

    /**
     * Method: statementBindStream
     */
    DatabaseDriverV1.StatementBindStreamResponse statementBindStream(DatabaseDriverV1.StatementBindStreamRequest request) throws ServiceException, TransportException;

    /**
     * Method: statementExecuteQuery
     */
    DatabaseDriverV1.StatementExecuteQueryResponse statementExecuteQuery(DatabaseDriverV1.StatementExecuteQueryRequest request) throws ServiceException, TransportException;

    /**
     * Method: statementExecutePartitions
     */
    DatabaseDriverV1.StatementExecutePartitionsResponse statementExecutePartitions(DatabaseDriverV1.StatementExecutePartitionsRequest request) throws ServiceException, TransportException;

    /**
     * Method: statementReadPartition
     */
    DatabaseDriverV1.StatementReadPartitionResponse statementReadPartition(DatabaseDriverV1.StatementReadPartitionRequest request) throws ServiceException, TransportException;


    class ServiceException extends RuntimeException {
        public final DatabaseDriverV1.DriverException error;
        public ServiceException(DatabaseDriverV1.DriverException error) {
            super(error.toString());
            this.error = error;
        }
    }
}
