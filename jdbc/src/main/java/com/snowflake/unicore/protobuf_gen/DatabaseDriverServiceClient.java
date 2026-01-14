package com.snowflake.unicore.protobuf_gen;

import com.google.protobuf.Message;
import com.snowflake.unicore.CoreTransport;
import com.snowflake.unicore.CoreTransport.TransportResponse;
import com.snowflake.unicore.TransportException;
import com.google.protobuf.InvalidProtocolBufferException;
import com.snowflake.unicore.protobuf_gen.DatabaseDriverService;

/**
 * Client implementation for DatabaseDriver
 */
public class DatabaseDriverServiceClient implements DatabaseDriverService {

    private final CoreTransport transport;
    
    public DatabaseDriverServiceClient(CoreTransport transport) {
        this.transport = transport;
    }
    
    /**
     * Method: databaseNew
     */
    public DatabaseDriverV1.DatabaseNewResponse databaseNew(DatabaseDriverV1.DatabaseNewRequest request) throws ServiceException, TransportException {
        TransportResponse response = transport.handleMessage(
            "DatabaseDriver",
            "database_new",
            request.toByteArray()
        );
        
        int code = response.getCode();
        byte[] responseBytes = response.getResponseBytes();
        
        if (code == CoreTransport.CODE_SUCCESS) {
            try {
                return DatabaseDriverV1.DatabaseNewResponse.parseFrom(responseBytes);
            } catch (InvalidProtocolBufferException e) {
                throw new TransportException("Invalid protocol buffer exception: " + e.getMessage());
            }
        } else if (code == CoreTransport.CODE_APPLICATION_ERROR) {
            try {
                DatabaseDriverV1.DriverException error = DatabaseDriverV1.DriverException.parseFrom(responseBytes);
                throw new ServiceException(error);
            } catch (InvalidProtocolBufferException e) {
                throw new TransportException("Invalid protocol buffer exception: " + e.getMessage());
            }
        } else if (code == CoreTransport.CODE_TRANSPORT_ERROR) {
            String errorMessage = new String(responseBytes);
            throw new TransportException(errorMessage);
        } else {
            throw new TransportException("Unknown error code: " + code);
        }
    }
    
    /**
     * Method: databaseSetOptionString
     */
    public DatabaseDriverV1.DatabaseSetOptionStringResponse databaseSetOptionString(DatabaseDriverV1.DatabaseSetOptionStringRequest request) throws ServiceException, TransportException {
        TransportResponse response = transport.handleMessage(
            "DatabaseDriver",
            "database_set_option_string",
            request.toByteArray()
        );
        
        int code = response.getCode();
        byte[] responseBytes = response.getResponseBytes();
        
        if (code == CoreTransport.CODE_SUCCESS) {
            try {
                return DatabaseDriverV1.DatabaseSetOptionStringResponse.parseFrom(responseBytes);
            } catch (InvalidProtocolBufferException e) {
                throw new TransportException("Invalid protocol buffer exception: " + e.getMessage());
            }
        } else if (code == CoreTransport.CODE_APPLICATION_ERROR) {
            try {
                DatabaseDriverV1.DriverException error = DatabaseDriverV1.DriverException.parseFrom(responseBytes);
                throw new ServiceException(error);
            } catch (InvalidProtocolBufferException e) {
                throw new TransportException("Invalid protocol buffer exception: " + e.getMessage());
            }
        } else if (code == CoreTransport.CODE_TRANSPORT_ERROR) {
            String errorMessage = new String(responseBytes);
            throw new TransportException(errorMessage);
        } else {
            throw new TransportException("Unknown error code: " + code);
        }
    }
    
    /**
     * Method: databaseSetOptionBytes
     */
    public DatabaseDriverV1.DatabaseSetOptionBytesResponse databaseSetOptionBytes(DatabaseDriverV1.DatabaseSetOptionBytesRequest request) throws ServiceException, TransportException {
        TransportResponse response = transport.handleMessage(
            "DatabaseDriver",
            "database_set_option_bytes",
            request.toByteArray()
        );
        
        int code = response.getCode();
        byte[] responseBytes = response.getResponseBytes();
        
        if (code == CoreTransport.CODE_SUCCESS) {
            try {
                return DatabaseDriverV1.DatabaseSetOptionBytesResponse.parseFrom(responseBytes);
            } catch (InvalidProtocolBufferException e) {
                throw new TransportException("Invalid protocol buffer exception: " + e.getMessage());
            }
        } else if (code == CoreTransport.CODE_APPLICATION_ERROR) {
            try {
                DatabaseDriverV1.DriverException error = DatabaseDriverV1.DriverException.parseFrom(responseBytes);
                throw new ServiceException(error);
            } catch (InvalidProtocolBufferException e) {
                throw new TransportException("Invalid protocol buffer exception: " + e.getMessage());
            }
        } else if (code == CoreTransport.CODE_TRANSPORT_ERROR) {
            String errorMessage = new String(responseBytes);
            throw new TransportException(errorMessage);
        } else {
            throw new TransportException("Unknown error code: " + code);
        }
    }
    
    /**
     * Method: databaseSetOptionInt
     */
    public DatabaseDriverV1.DatabaseSetOptionIntResponse databaseSetOptionInt(DatabaseDriverV1.DatabaseSetOptionIntRequest request) throws ServiceException, TransportException {
        TransportResponse response = transport.handleMessage(
            "DatabaseDriver",
            "database_set_option_int",
            request.toByteArray()
        );
        
        int code = response.getCode();
        byte[] responseBytes = response.getResponseBytes();
        
        if (code == CoreTransport.CODE_SUCCESS) {
            try {
                return DatabaseDriverV1.DatabaseSetOptionIntResponse.parseFrom(responseBytes);
            } catch (InvalidProtocolBufferException e) {
                throw new TransportException("Invalid protocol buffer exception: " + e.getMessage());
            }
        } else if (code == CoreTransport.CODE_APPLICATION_ERROR) {
            try {
                DatabaseDriverV1.DriverException error = DatabaseDriverV1.DriverException.parseFrom(responseBytes);
                throw new ServiceException(error);
            } catch (InvalidProtocolBufferException e) {
                throw new TransportException("Invalid protocol buffer exception: " + e.getMessage());
            }
        } else if (code == CoreTransport.CODE_TRANSPORT_ERROR) {
            String errorMessage = new String(responseBytes);
            throw new TransportException(errorMessage);
        } else {
            throw new TransportException("Unknown error code: " + code);
        }
    }
    
    /**
     * Method: databaseSetOptionDouble
     */
    public DatabaseDriverV1.DatabaseSetOptionDoubleResponse databaseSetOptionDouble(DatabaseDriverV1.DatabaseSetOptionDoubleRequest request) throws ServiceException, TransportException {
        TransportResponse response = transport.handleMessage(
            "DatabaseDriver",
            "database_set_option_double",
            request.toByteArray()
        );
        
        int code = response.getCode();
        byte[] responseBytes = response.getResponseBytes();
        
        if (code == CoreTransport.CODE_SUCCESS) {
            try {
                return DatabaseDriverV1.DatabaseSetOptionDoubleResponse.parseFrom(responseBytes);
            } catch (InvalidProtocolBufferException e) {
                throw new TransportException("Invalid protocol buffer exception: " + e.getMessage());
            }
        } else if (code == CoreTransport.CODE_APPLICATION_ERROR) {
            try {
                DatabaseDriverV1.DriverException error = DatabaseDriverV1.DriverException.parseFrom(responseBytes);
                throw new ServiceException(error);
            } catch (InvalidProtocolBufferException e) {
                throw new TransportException("Invalid protocol buffer exception: " + e.getMessage());
            }
        } else if (code == CoreTransport.CODE_TRANSPORT_ERROR) {
            String errorMessage = new String(responseBytes);
            throw new TransportException(errorMessage);
        } else {
            throw new TransportException("Unknown error code: " + code);
        }
    }
    
    /**
     * Method: databaseInit
     */
    public DatabaseDriverV1.DatabaseInitResponse databaseInit(DatabaseDriverV1.DatabaseInitRequest request) throws ServiceException, TransportException {
        TransportResponse response = transport.handleMessage(
            "DatabaseDriver",
            "database_init",
            request.toByteArray()
        );
        
        int code = response.getCode();
        byte[] responseBytes = response.getResponseBytes();
        
        if (code == CoreTransport.CODE_SUCCESS) {
            try {
                return DatabaseDriverV1.DatabaseInitResponse.parseFrom(responseBytes);
            } catch (InvalidProtocolBufferException e) {
                throw new TransportException("Invalid protocol buffer exception: " + e.getMessage());
            }
        } else if (code == CoreTransport.CODE_APPLICATION_ERROR) {
            try {
                DatabaseDriverV1.DriverException error = DatabaseDriverV1.DriverException.parseFrom(responseBytes);
                throw new ServiceException(error);
            } catch (InvalidProtocolBufferException e) {
                throw new TransportException("Invalid protocol buffer exception: " + e.getMessage());
            }
        } else if (code == CoreTransport.CODE_TRANSPORT_ERROR) {
            String errorMessage = new String(responseBytes);
            throw new TransportException(errorMessage);
        } else {
            throw new TransportException("Unknown error code: " + code);
        }
    }
    
    /**
     * Method: databaseRelease
     */
    public DatabaseDriverV1.DatabaseReleaseResponse databaseRelease(DatabaseDriverV1.DatabaseReleaseRequest request) throws ServiceException, TransportException {
        TransportResponse response = transport.handleMessage(
            "DatabaseDriver",
            "database_release",
            request.toByteArray()
        );
        
        int code = response.getCode();
        byte[] responseBytes = response.getResponseBytes();
        
        if (code == CoreTransport.CODE_SUCCESS) {
            try {
                return DatabaseDriverV1.DatabaseReleaseResponse.parseFrom(responseBytes);
            } catch (InvalidProtocolBufferException e) {
                throw new TransportException("Invalid protocol buffer exception: " + e.getMessage());
            }
        } else if (code == CoreTransport.CODE_APPLICATION_ERROR) {
            try {
                DatabaseDriverV1.DriverException error = DatabaseDriverV1.DriverException.parseFrom(responseBytes);
                throw new ServiceException(error);
            } catch (InvalidProtocolBufferException e) {
                throw new TransportException("Invalid protocol buffer exception: " + e.getMessage());
            }
        } else if (code == CoreTransport.CODE_TRANSPORT_ERROR) {
            String errorMessage = new String(responseBytes);
            throw new TransportException(errorMessage);
        } else {
            throw new TransportException("Unknown error code: " + code);
        }
    }
    
    /**
     * Method: connectionNew
     */
    public DatabaseDriverV1.ConnectionNewResponse connectionNew(DatabaseDriverV1.ConnectionNewRequest request) throws ServiceException, TransportException {
        TransportResponse response = transport.handleMessage(
            "DatabaseDriver",
            "connection_new",
            request.toByteArray()
        );
        
        int code = response.getCode();
        byte[] responseBytes = response.getResponseBytes();
        
        if (code == CoreTransport.CODE_SUCCESS) {
            try {
                return DatabaseDriverV1.ConnectionNewResponse.parseFrom(responseBytes);
            } catch (InvalidProtocolBufferException e) {
                throw new TransportException("Invalid protocol buffer exception: " + e.getMessage());
            }
        } else if (code == CoreTransport.CODE_APPLICATION_ERROR) {
            try {
                DatabaseDriverV1.DriverException error = DatabaseDriverV1.DriverException.parseFrom(responseBytes);
                throw new ServiceException(error);
            } catch (InvalidProtocolBufferException e) {
                throw new TransportException("Invalid protocol buffer exception: " + e.getMessage());
            }
        } else if (code == CoreTransport.CODE_TRANSPORT_ERROR) {
            String errorMessage = new String(responseBytes);
            throw new TransportException(errorMessage);
        } else {
            throw new TransportException("Unknown error code: " + code);
        }
    }
    
    /**
     * Method: connectionSetOptionString
     */
    public DatabaseDriverV1.ConnectionSetOptionStringResponse connectionSetOptionString(DatabaseDriverV1.ConnectionSetOptionStringRequest request) throws ServiceException, TransportException {
        TransportResponse response = transport.handleMessage(
            "DatabaseDriver",
            "connection_set_option_string",
            request.toByteArray()
        );
        
        int code = response.getCode();
        byte[] responseBytes = response.getResponseBytes();
        
        if (code == CoreTransport.CODE_SUCCESS) {
            try {
                return DatabaseDriverV1.ConnectionSetOptionStringResponse.parseFrom(responseBytes);
            } catch (InvalidProtocolBufferException e) {
                throw new TransportException("Invalid protocol buffer exception: " + e.getMessage());
            }
        } else if (code == CoreTransport.CODE_APPLICATION_ERROR) {
            try {
                DatabaseDriverV1.DriverException error = DatabaseDriverV1.DriverException.parseFrom(responseBytes);
                throw new ServiceException(error);
            } catch (InvalidProtocolBufferException e) {
                throw new TransportException("Invalid protocol buffer exception: " + e.getMessage());
            }
        } else if (code == CoreTransport.CODE_TRANSPORT_ERROR) {
            String errorMessage = new String(responseBytes);
            throw new TransportException(errorMessage);
        } else {
            throw new TransportException("Unknown error code: " + code);
        }
    }
    
    /**
     * Method: connectionSetOptionBytes
     */
    public DatabaseDriverV1.ConnectionSetOptionBytesResponse connectionSetOptionBytes(DatabaseDriverV1.ConnectionSetOptionBytesRequest request) throws ServiceException, TransportException {
        TransportResponse response = transport.handleMessage(
            "DatabaseDriver",
            "connection_set_option_bytes",
            request.toByteArray()
        );
        
        int code = response.getCode();
        byte[] responseBytes = response.getResponseBytes();
        
        if (code == CoreTransport.CODE_SUCCESS) {
            try {
                return DatabaseDriverV1.ConnectionSetOptionBytesResponse.parseFrom(responseBytes);
            } catch (InvalidProtocolBufferException e) {
                throw new TransportException("Invalid protocol buffer exception: " + e.getMessage());
            }
        } else if (code == CoreTransport.CODE_APPLICATION_ERROR) {
            try {
                DatabaseDriverV1.DriverException error = DatabaseDriverV1.DriverException.parseFrom(responseBytes);
                throw new ServiceException(error);
            } catch (InvalidProtocolBufferException e) {
                throw new TransportException("Invalid protocol buffer exception: " + e.getMessage());
            }
        } else if (code == CoreTransport.CODE_TRANSPORT_ERROR) {
            String errorMessage = new String(responseBytes);
            throw new TransportException(errorMessage);
        } else {
            throw new TransportException("Unknown error code: " + code);
        }
    }
    
    /**
     * Method: connectionSetOptionInt
     */
    public DatabaseDriverV1.ConnectionSetOptionIntResponse connectionSetOptionInt(DatabaseDriverV1.ConnectionSetOptionIntRequest request) throws ServiceException, TransportException {
        TransportResponse response = transport.handleMessage(
            "DatabaseDriver",
            "connection_set_option_int",
            request.toByteArray()
        );
        
        int code = response.getCode();
        byte[] responseBytes = response.getResponseBytes();
        
        if (code == CoreTransport.CODE_SUCCESS) {
            try {
                return DatabaseDriverV1.ConnectionSetOptionIntResponse.parseFrom(responseBytes);
            } catch (InvalidProtocolBufferException e) {
                throw new TransportException("Invalid protocol buffer exception: " + e.getMessage());
            }
        } else if (code == CoreTransport.CODE_APPLICATION_ERROR) {
            try {
                DatabaseDriverV1.DriverException error = DatabaseDriverV1.DriverException.parseFrom(responseBytes);
                throw new ServiceException(error);
            } catch (InvalidProtocolBufferException e) {
                throw new TransportException("Invalid protocol buffer exception: " + e.getMessage());
            }
        } else if (code == CoreTransport.CODE_TRANSPORT_ERROR) {
            String errorMessage = new String(responseBytes);
            throw new TransportException(errorMessage);
        } else {
            throw new TransportException("Unknown error code: " + code);
        }
    }
    
    /**
     * Method: connectionSetOptionDouble
     */
    public DatabaseDriverV1.ConnectionSetOptionDoubleResponse connectionSetOptionDouble(DatabaseDriverV1.ConnectionSetOptionDoubleRequest request) throws ServiceException, TransportException {
        TransportResponse response = transport.handleMessage(
            "DatabaseDriver",
            "connection_set_option_double",
            request.toByteArray()
        );
        
        int code = response.getCode();
        byte[] responseBytes = response.getResponseBytes();
        
        if (code == CoreTransport.CODE_SUCCESS) {
            try {
                return DatabaseDriverV1.ConnectionSetOptionDoubleResponse.parseFrom(responseBytes);
            } catch (InvalidProtocolBufferException e) {
                throw new TransportException("Invalid protocol buffer exception: " + e.getMessage());
            }
        } else if (code == CoreTransport.CODE_APPLICATION_ERROR) {
            try {
                DatabaseDriverV1.DriverException error = DatabaseDriverV1.DriverException.parseFrom(responseBytes);
                throw new ServiceException(error);
            } catch (InvalidProtocolBufferException e) {
                throw new TransportException("Invalid protocol buffer exception: " + e.getMessage());
            }
        } else if (code == CoreTransport.CODE_TRANSPORT_ERROR) {
            String errorMessage = new String(responseBytes);
            throw new TransportException(errorMessage);
        } else {
            throw new TransportException("Unknown error code: " + code);
        }
    }
    
    /**
     * Method: connectionInit
     */
    public DatabaseDriverV1.ConnectionInitResponse connectionInit(DatabaseDriverV1.ConnectionInitRequest request) throws ServiceException, TransportException {
        TransportResponse response = transport.handleMessage(
            "DatabaseDriver",
            "connection_init",
            request.toByteArray()
        );
        
        int code = response.getCode();
        byte[] responseBytes = response.getResponseBytes();
        
        if (code == CoreTransport.CODE_SUCCESS) {
            try {
                return DatabaseDriverV1.ConnectionInitResponse.parseFrom(responseBytes);
            } catch (InvalidProtocolBufferException e) {
                throw new TransportException("Invalid protocol buffer exception: " + e.getMessage());
            }
        } else if (code == CoreTransport.CODE_APPLICATION_ERROR) {
            try {
                DatabaseDriverV1.DriverException error = DatabaseDriverV1.DriverException.parseFrom(responseBytes);
                throw new ServiceException(error);
            } catch (InvalidProtocolBufferException e) {
                throw new TransportException("Invalid protocol buffer exception: " + e.getMessage());
            }
        } else if (code == CoreTransport.CODE_TRANSPORT_ERROR) {
            String errorMessage = new String(responseBytes);
            throw new TransportException(errorMessage);
        } else {
            throw new TransportException("Unknown error code: " + code);
        }
    }
    
    /**
     * Method: connectionRelease
     */
    public DatabaseDriverV1.ConnectionReleaseResponse connectionRelease(DatabaseDriverV1.ConnectionReleaseRequest request) throws ServiceException, TransportException {
        TransportResponse response = transport.handleMessage(
            "DatabaseDriver",
            "connection_release",
            request.toByteArray()
        );
        
        int code = response.getCode();
        byte[] responseBytes = response.getResponseBytes();
        
        if (code == CoreTransport.CODE_SUCCESS) {
            try {
                return DatabaseDriverV1.ConnectionReleaseResponse.parseFrom(responseBytes);
            } catch (InvalidProtocolBufferException e) {
                throw new TransportException("Invalid protocol buffer exception: " + e.getMessage());
            }
        } else if (code == CoreTransport.CODE_APPLICATION_ERROR) {
            try {
                DatabaseDriverV1.DriverException error = DatabaseDriverV1.DriverException.parseFrom(responseBytes);
                throw new ServiceException(error);
            } catch (InvalidProtocolBufferException e) {
                throw new TransportException("Invalid protocol buffer exception: " + e.getMessage());
            }
        } else if (code == CoreTransport.CODE_TRANSPORT_ERROR) {
            String errorMessage = new String(responseBytes);
            throw new TransportException(errorMessage);
        } else {
            throw new TransportException("Unknown error code: " + code);
        }
    }
    
    /**
     * Method: connectionGetInfo
     */
    public DatabaseDriverV1.ConnectionGetInfoResponse connectionGetInfo(DatabaseDriverV1.ConnectionGetInfoRequest request) throws ServiceException, TransportException {
        TransportResponse response = transport.handleMessage(
            "DatabaseDriver",
            "connection_get_info",
            request.toByteArray()
        );
        
        int code = response.getCode();
        byte[] responseBytes = response.getResponseBytes();
        
        if (code == CoreTransport.CODE_SUCCESS) {
            try {
                return DatabaseDriverV1.ConnectionGetInfoResponse.parseFrom(responseBytes);
            } catch (InvalidProtocolBufferException e) {
                throw new TransportException("Invalid protocol buffer exception: " + e.getMessage());
            }
        } else if (code == CoreTransport.CODE_APPLICATION_ERROR) {
            try {
                DatabaseDriverV1.DriverException error = DatabaseDriverV1.DriverException.parseFrom(responseBytes);
                throw new ServiceException(error);
            } catch (InvalidProtocolBufferException e) {
                throw new TransportException("Invalid protocol buffer exception: " + e.getMessage());
            }
        } else if (code == CoreTransport.CODE_TRANSPORT_ERROR) {
            String errorMessage = new String(responseBytes);
            throw new TransportException(errorMessage);
        } else {
            throw new TransportException("Unknown error code: " + code);
        }
    }
    
    /**
     * Method: connectionGetObjects
     */
    public DatabaseDriverV1.ConnectionGetObjectsResponse connectionGetObjects(DatabaseDriverV1.ConnectionGetObjectsRequest request) throws ServiceException, TransportException {
        TransportResponse response = transport.handleMessage(
            "DatabaseDriver",
            "connection_get_objects",
            request.toByteArray()
        );
        
        int code = response.getCode();
        byte[] responseBytes = response.getResponseBytes();
        
        if (code == CoreTransport.CODE_SUCCESS) {
            try {
                return DatabaseDriverV1.ConnectionGetObjectsResponse.parseFrom(responseBytes);
            } catch (InvalidProtocolBufferException e) {
                throw new TransportException("Invalid protocol buffer exception: " + e.getMessage());
            }
        } else if (code == CoreTransport.CODE_APPLICATION_ERROR) {
            try {
                DatabaseDriverV1.DriverException error = DatabaseDriverV1.DriverException.parseFrom(responseBytes);
                throw new ServiceException(error);
            } catch (InvalidProtocolBufferException e) {
                throw new TransportException("Invalid protocol buffer exception: " + e.getMessage());
            }
        } else if (code == CoreTransport.CODE_TRANSPORT_ERROR) {
            String errorMessage = new String(responseBytes);
            throw new TransportException(errorMessage);
        } else {
            throw new TransportException("Unknown error code: " + code);
        }
    }
    
    /**
     * Method: connectionGetTableSchema
     */
    public DatabaseDriverV1.ConnectionGetTableSchemaResponse connectionGetTableSchema(DatabaseDriverV1.ConnectionGetTableSchemaRequest request) throws ServiceException, TransportException {
        TransportResponse response = transport.handleMessage(
            "DatabaseDriver",
            "connection_get_table_schema",
            request.toByteArray()
        );
        
        int code = response.getCode();
        byte[] responseBytes = response.getResponseBytes();
        
        if (code == CoreTransport.CODE_SUCCESS) {
            try {
                return DatabaseDriverV1.ConnectionGetTableSchemaResponse.parseFrom(responseBytes);
            } catch (InvalidProtocolBufferException e) {
                throw new TransportException("Invalid protocol buffer exception: " + e.getMessage());
            }
        } else if (code == CoreTransport.CODE_APPLICATION_ERROR) {
            try {
                DatabaseDriverV1.DriverException error = DatabaseDriverV1.DriverException.parseFrom(responseBytes);
                throw new ServiceException(error);
            } catch (InvalidProtocolBufferException e) {
                throw new TransportException("Invalid protocol buffer exception: " + e.getMessage());
            }
        } else if (code == CoreTransport.CODE_TRANSPORT_ERROR) {
            String errorMessage = new String(responseBytes);
            throw new TransportException(errorMessage);
        } else {
            throw new TransportException("Unknown error code: " + code);
        }
    }
    
    /**
     * Method: connectionGetTableTypes
     */
    public DatabaseDriverV1.ConnectionGetTableTypesResponse connectionGetTableTypes(DatabaseDriverV1.ConnectionGetTableTypesRequest request) throws ServiceException, TransportException {
        TransportResponse response = transport.handleMessage(
            "DatabaseDriver",
            "connection_get_table_types",
            request.toByteArray()
        );
        
        int code = response.getCode();
        byte[] responseBytes = response.getResponseBytes();
        
        if (code == CoreTransport.CODE_SUCCESS) {
            try {
                return DatabaseDriverV1.ConnectionGetTableTypesResponse.parseFrom(responseBytes);
            } catch (InvalidProtocolBufferException e) {
                throw new TransportException("Invalid protocol buffer exception: " + e.getMessage());
            }
        } else if (code == CoreTransport.CODE_APPLICATION_ERROR) {
            try {
                DatabaseDriverV1.DriverException error = DatabaseDriverV1.DriverException.parseFrom(responseBytes);
                throw new ServiceException(error);
            } catch (InvalidProtocolBufferException e) {
                throw new TransportException("Invalid protocol buffer exception: " + e.getMessage());
            }
        } else if (code == CoreTransport.CODE_TRANSPORT_ERROR) {
            String errorMessage = new String(responseBytes);
            throw new TransportException(errorMessage);
        } else {
            throw new TransportException("Unknown error code: " + code);
        }
    }
    
    /**
     * Method: connectionCommit
     */
    public DatabaseDriverV1.ConnectionCommitResponse connectionCommit(DatabaseDriverV1.ConnectionCommitRequest request) throws ServiceException, TransportException {
        TransportResponse response = transport.handleMessage(
            "DatabaseDriver",
            "connection_commit",
            request.toByteArray()
        );
        
        int code = response.getCode();
        byte[] responseBytes = response.getResponseBytes();
        
        if (code == CoreTransport.CODE_SUCCESS) {
            try {
                return DatabaseDriverV1.ConnectionCommitResponse.parseFrom(responseBytes);
            } catch (InvalidProtocolBufferException e) {
                throw new TransportException("Invalid protocol buffer exception: " + e.getMessage());
            }
        } else if (code == CoreTransport.CODE_APPLICATION_ERROR) {
            try {
                DatabaseDriverV1.DriverException error = DatabaseDriverV1.DriverException.parseFrom(responseBytes);
                throw new ServiceException(error);
            } catch (InvalidProtocolBufferException e) {
                throw new TransportException("Invalid protocol buffer exception: " + e.getMessage());
            }
        } else if (code == CoreTransport.CODE_TRANSPORT_ERROR) {
            String errorMessage = new String(responseBytes);
            throw new TransportException(errorMessage);
        } else {
            throw new TransportException("Unknown error code: " + code);
        }
    }
    
    /**
     * Method: connectionRollback
     */
    public DatabaseDriverV1.ConnectionRollbackResponse connectionRollback(DatabaseDriverV1.ConnectionRollbackRequest request) throws ServiceException, TransportException {
        TransportResponse response = transport.handleMessage(
            "DatabaseDriver",
            "connection_rollback",
            request.toByteArray()
        );
        
        int code = response.getCode();
        byte[] responseBytes = response.getResponseBytes();
        
        if (code == CoreTransport.CODE_SUCCESS) {
            try {
                return DatabaseDriverV1.ConnectionRollbackResponse.parseFrom(responseBytes);
            } catch (InvalidProtocolBufferException e) {
                throw new TransportException("Invalid protocol buffer exception: " + e.getMessage());
            }
        } else if (code == CoreTransport.CODE_APPLICATION_ERROR) {
            try {
                DatabaseDriverV1.DriverException error = DatabaseDriverV1.DriverException.parseFrom(responseBytes);
                throw new ServiceException(error);
            } catch (InvalidProtocolBufferException e) {
                throw new TransportException("Invalid protocol buffer exception: " + e.getMessage());
            }
        } else if (code == CoreTransport.CODE_TRANSPORT_ERROR) {
            String errorMessage = new String(responseBytes);
            throw new TransportException(errorMessage);
        } else {
            throw new TransportException("Unknown error code: " + code);
        }
    }
    
    /**
     * Method: statementNew
     */
    public DatabaseDriverV1.StatementNewResponse statementNew(DatabaseDriverV1.StatementNewRequest request) throws ServiceException, TransportException {
        TransportResponse response = transport.handleMessage(
            "DatabaseDriver",
            "statement_new",
            request.toByteArray()
        );
        
        int code = response.getCode();
        byte[] responseBytes = response.getResponseBytes();
        
        if (code == CoreTransport.CODE_SUCCESS) {
            try {
                return DatabaseDriverV1.StatementNewResponse.parseFrom(responseBytes);
            } catch (InvalidProtocolBufferException e) {
                throw new TransportException("Invalid protocol buffer exception: " + e.getMessage());
            }
        } else if (code == CoreTransport.CODE_APPLICATION_ERROR) {
            try {
                DatabaseDriverV1.DriverException error = DatabaseDriverV1.DriverException.parseFrom(responseBytes);
                throw new ServiceException(error);
            } catch (InvalidProtocolBufferException e) {
                throw new TransportException("Invalid protocol buffer exception: " + e.getMessage());
            }
        } else if (code == CoreTransport.CODE_TRANSPORT_ERROR) {
            String errorMessage = new String(responseBytes);
            throw new TransportException(errorMessage);
        } else {
            throw new TransportException("Unknown error code: " + code);
        }
    }
    
    /**
     * Method: statementRelease
     */
    public DatabaseDriverV1.StatementReleaseResponse statementRelease(DatabaseDriverV1.StatementReleaseRequest request) throws ServiceException, TransportException {
        TransportResponse response = transport.handleMessage(
            "DatabaseDriver",
            "statement_release",
            request.toByteArray()
        );
        
        int code = response.getCode();
        byte[] responseBytes = response.getResponseBytes();
        
        if (code == CoreTransport.CODE_SUCCESS) {
            try {
                return DatabaseDriverV1.StatementReleaseResponse.parseFrom(responseBytes);
            } catch (InvalidProtocolBufferException e) {
                throw new TransportException("Invalid protocol buffer exception: " + e.getMessage());
            }
        } else if (code == CoreTransport.CODE_APPLICATION_ERROR) {
            try {
                DatabaseDriverV1.DriverException error = DatabaseDriverV1.DriverException.parseFrom(responseBytes);
                throw new ServiceException(error);
            } catch (InvalidProtocolBufferException e) {
                throw new TransportException("Invalid protocol buffer exception: " + e.getMessage());
            }
        } else if (code == CoreTransport.CODE_TRANSPORT_ERROR) {
            String errorMessage = new String(responseBytes);
            throw new TransportException(errorMessage);
        } else {
            throw new TransportException("Unknown error code: " + code);
        }
    }
    
    /**
     * Method: statementSetSqlQuery
     */
    public DatabaseDriverV1.StatementSetSqlQueryResponse statementSetSqlQuery(DatabaseDriverV1.StatementSetSqlQueryRequest request) throws ServiceException, TransportException {
        TransportResponse response = transport.handleMessage(
            "DatabaseDriver",
            "statement_set_sql_query",
            request.toByteArray()
        );
        
        int code = response.getCode();
        byte[] responseBytes = response.getResponseBytes();
        
        if (code == CoreTransport.CODE_SUCCESS) {
            try {
                return DatabaseDriverV1.StatementSetSqlQueryResponse.parseFrom(responseBytes);
            } catch (InvalidProtocolBufferException e) {
                throw new TransportException("Invalid protocol buffer exception: " + e.getMessage());
            }
        } else if (code == CoreTransport.CODE_APPLICATION_ERROR) {
            try {
                DatabaseDriverV1.DriverException error = DatabaseDriverV1.DriverException.parseFrom(responseBytes);
                throw new ServiceException(error);
            } catch (InvalidProtocolBufferException e) {
                throw new TransportException("Invalid protocol buffer exception: " + e.getMessage());
            }
        } else if (code == CoreTransport.CODE_TRANSPORT_ERROR) {
            String errorMessage = new String(responseBytes);
            throw new TransportException(errorMessage);
        } else {
            throw new TransportException("Unknown error code: " + code);
        }
    }
    
    /**
     * Method: statementSetSubstraitPlan
     */
    public DatabaseDriverV1.StatementSetSubstraitPlanResponse statementSetSubstraitPlan(DatabaseDriverV1.StatementSetSubstraitPlanRequest request) throws ServiceException, TransportException {
        TransportResponse response = transport.handleMessage(
            "DatabaseDriver",
            "statement_set_substrait_plan",
            request.toByteArray()
        );
        
        int code = response.getCode();
        byte[] responseBytes = response.getResponseBytes();
        
        if (code == CoreTransport.CODE_SUCCESS) {
            try {
                return DatabaseDriverV1.StatementSetSubstraitPlanResponse.parseFrom(responseBytes);
            } catch (InvalidProtocolBufferException e) {
                throw new TransportException("Invalid protocol buffer exception: " + e.getMessage());
            }
        } else if (code == CoreTransport.CODE_APPLICATION_ERROR) {
            try {
                DatabaseDriverV1.DriverException error = DatabaseDriverV1.DriverException.parseFrom(responseBytes);
                throw new ServiceException(error);
            } catch (InvalidProtocolBufferException e) {
                throw new TransportException("Invalid protocol buffer exception: " + e.getMessage());
            }
        } else if (code == CoreTransport.CODE_TRANSPORT_ERROR) {
            String errorMessage = new String(responseBytes);
            throw new TransportException(errorMessage);
        } else {
            throw new TransportException("Unknown error code: " + code);
        }
    }
    
    /**
     * Method: statementPrepare
     */
    public DatabaseDriverV1.StatementPrepareResponse statementPrepare(DatabaseDriverV1.StatementPrepareRequest request) throws ServiceException, TransportException {
        TransportResponse response = transport.handleMessage(
            "DatabaseDriver",
            "statement_prepare",
            request.toByteArray()
        );
        
        int code = response.getCode();
        byte[] responseBytes = response.getResponseBytes();
        
        if (code == CoreTransport.CODE_SUCCESS) {
            try {
                return DatabaseDriverV1.StatementPrepareResponse.parseFrom(responseBytes);
            } catch (InvalidProtocolBufferException e) {
                throw new TransportException("Invalid protocol buffer exception: " + e.getMessage());
            }
        } else if (code == CoreTransport.CODE_APPLICATION_ERROR) {
            try {
                DatabaseDriverV1.DriverException error = DatabaseDriverV1.DriverException.parseFrom(responseBytes);
                throw new ServiceException(error);
            } catch (InvalidProtocolBufferException e) {
                throw new TransportException("Invalid protocol buffer exception: " + e.getMessage());
            }
        } else if (code == CoreTransport.CODE_TRANSPORT_ERROR) {
            String errorMessage = new String(responseBytes);
            throw new TransportException(errorMessage);
        } else {
            throw new TransportException("Unknown error code: " + code);
        }
    }
    
    /**
     * Method: statementSetOptionString
     */
    public DatabaseDriverV1.StatementSetOptionStringResponse statementSetOptionString(DatabaseDriverV1.StatementSetOptionStringRequest request) throws ServiceException, TransportException {
        TransportResponse response = transport.handleMessage(
            "DatabaseDriver",
            "statement_set_option_string",
            request.toByteArray()
        );
        
        int code = response.getCode();
        byte[] responseBytes = response.getResponseBytes();
        
        if (code == CoreTransport.CODE_SUCCESS) {
            try {
                return DatabaseDriverV1.StatementSetOptionStringResponse.parseFrom(responseBytes);
            } catch (InvalidProtocolBufferException e) {
                throw new TransportException("Invalid protocol buffer exception: " + e.getMessage());
            }
        } else if (code == CoreTransport.CODE_APPLICATION_ERROR) {
            try {
                DatabaseDriverV1.DriverException error = DatabaseDriverV1.DriverException.parseFrom(responseBytes);
                throw new ServiceException(error);
            } catch (InvalidProtocolBufferException e) {
                throw new TransportException("Invalid protocol buffer exception: " + e.getMessage());
            }
        } else if (code == CoreTransport.CODE_TRANSPORT_ERROR) {
            String errorMessage = new String(responseBytes);
            throw new TransportException(errorMessage);
        } else {
            throw new TransportException("Unknown error code: " + code);
        }
    }
    
    /**
     * Method: statementSetOptionBytes
     */
    public DatabaseDriverV1.StatementSetOptionBytesResponse statementSetOptionBytes(DatabaseDriverV1.StatementSetOptionBytesRequest request) throws ServiceException, TransportException {
        TransportResponse response = transport.handleMessage(
            "DatabaseDriver",
            "statement_set_option_bytes",
            request.toByteArray()
        );
        
        int code = response.getCode();
        byte[] responseBytes = response.getResponseBytes();
        
        if (code == CoreTransport.CODE_SUCCESS) {
            try {
                return DatabaseDriverV1.StatementSetOptionBytesResponse.parseFrom(responseBytes);
            } catch (InvalidProtocolBufferException e) {
                throw new TransportException("Invalid protocol buffer exception: " + e.getMessage());
            }
        } else if (code == CoreTransport.CODE_APPLICATION_ERROR) {
            try {
                DatabaseDriverV1.DriverException error = DatabaseDriverV1.DriverException.parseFrom(responseBytes);
                throw new ServiceException(error);
            } catch (InvalidProtocolBufferException e) {
                throw new TransportException("Invalid protocol buffer exception: " + e.getMessage());
            }
        } else if (code == CoreTransport.CODE_TRANSPORT_ERROR) {
            String errorMessage = new String(responseBytes);
            throw new TransportException(errorMessage);
        } else {
            throw new TransportException("Unknown error code: " + code);
        }
    }
    
    /**
     * Method: statementSetOptionInt
     */
    public DatabaseDriverV1.StatementSetOptionIntResponse statementSetOptionInt(DatabaseDriverV1.StatementSetOptionIntRequest request) throws ServiceException, TransportException {
        TransportResponse response = transport.handleMessage(
            "DatabaseDriver",
            "statement_set_option_int",
            request.toByteArray()
        );
        
        int code = response.getCode();
        byte[] responseBytes = response.getResponseBytes();
        
        if (code == CoreTransport.CODE_SUCCESS) {
            try {
                return DatabaseDriverV1.StatementSetOptionIntResponse.parseFrom(responseBytes);
            } catch (InvalidProtocolBufferException e) {
                throw new TransportException("Invalid protocol buffer exception: " + e.getMessage());
            }
        } else if (code == CoreTransport.CODE_APPLICATION_ERROR) {
            try {
                DatabaseDriverV1.DriverException error = DatabaseDriverV1.DriverException.parseFrom(responseBytes);
                throw new ServiceException(error);
            } catch (InvalidProtocolBufferException e) {
                throw new TransportException("Invalid protocol buffer exception: " + e.getMessage());
            }
        } else if (code == CoreTransport.CODE_TRANSPORT_ERROR) {
            String errorMessage = new String(responseBytes);
            throw new TransportException(errorMessage);
        } else {
            throw new TransportException("Unknown error code: " + code);
        }
    }
    
    /**
     * Method: statementSetOptionDouble
     */
    public DatabaseDriverV1.StatementSetOptionDoubleResponse statementSetOptionDouble(DatabaseDriverV1.StatementSetOptionDoubleRequest request) throws ServiceException, TransportException {
        TransportResponse response = transport.handleMessage(
            "DatabaseDriver",
            "statement_set_option_double",
            request.toByteArray()
        );
        
        int code = response.getCode();
        byte[] responseBytes = response.getResponseBytes();
        
        if (code == CoreTransport.CODE_SUCCESS) {
            try {
                return DatabaseDriverV1.StatementSetOptionDoubleResponse.parseFrom(responseBytes);
            } catch (InvalidProtocolBufferException e) {
                throw new TransportException("Invalid protocol buffer exception: " + e.getMessage());
            }
        } else if (code == CoreTransport.CODE_APPLICATION_ERROR) {
            try {
                DatabaseDriverV1.DriverException error = DatabaseDriverV1.DriverException.parseFrom(responseBytes);
                throw new ServiceException(error);
            } catch (InvalidProtocolBufferException e) {
                throw new TransportException("Invalid protocol buffer exception: " + e.getMessage());
            }
        } else if (code == CoreTransport.CODE_TRANSPORT_ERROR) {
            String errorMessage = new String(responseBytes);
            throw new TransportException(errorMessage);
        } else {
            throw new TransportException("Unknown error code: " + code);
        }
    }
    
    /**
     * Method: statementGetParameterSchema
     */
    public DatabaseDriverV1.StatementGetParameterSchemaResponse statementGetParameterSchema(DatabaseDriverV1.StatementGetParameterSchemaRequest request) throws ServiceException, TransportException {
        TransportResponse response = transport.handleMessage(
            "DatabaseDriver",
            "statement_get_parameter_schema",
            request.toByteArray()
        );
        
        int code = response.getCode();
        byte[] responseBytes = response.getResponseBytes();
        
        if (code == CoreTransport.CODE_SUCCESS) {
            try {
                return DatabaseDriverV1.StatementGetParameterSchemaResponse.parseFrom(responseBytes);
            } catch (InvalidProtocolBufferException e) {
                throw new TransportException("Invalid protocol buffer exception: " + e.getMessage());
            }
        } else if (code == CoreTransport.CODE_APPLICATION_ERROR) {
            try {
                DatabaseDriverV1.DriverException error = DatabaseDriverV1.DriverException.parseFrom(responseBytes);
                throw new ServiceException(error);
            } catch (InvalidProtocolBufferException e) {
                throw new TransportException("Invalid protocol buffer exception: " + e.getMessage());
            }
        } else if (code == CoreTransport.CODE_TRANSPORT_ERROR) {
            String errorMessage = new String(responseBytes);
            throw new TransportException(errorMessage);
        } else {
            throw new TransportException("Unknown error code: " + code);
        }
    }
    
    /**
     * Method: statementBind
     */
    public DatabaseDriverV1.StatementBindResponse statementBind(DatabaseDriverV1.StatementBindRequest request) throws ServiceException, TransportException {
        TransportResponse response = transport.handleMessage(
            "DatabaseDriver",
            "statement_bind",
            request.toByteArray()
        );
        
        int code = response.getCode();
        byte[] responseBytes = response.getResponseBytes();
        
        if (code == CoreTransport.CODE_SUCCESS) {
            try {
                return DatabaseDriverV1.StatementBindResponse.parseFrom(responseBytes);
            } catch (InvalidProtocolBufferException e) {
                throw new TransportException("Invalid protocol buffer exception: " + e.getMessage());
            }
        } else if (code == CoreTransport.CODE_APPLICATION_ERROR) {
            try {
                DatabaseDriverV1.DriverException error = DatabaseDriverV1.DriverException.parseFrom(responseBytes);
                throw new ServiceException(error);
            } catch (InvalidProtocolBufferException e) {
                throw new TransportException("Invalid protocol buffer exception: " + e.getMessage());
            }
        } else if (code == CoreTransport.CODE_TRANSPORT_ERROR) {
            String errorMessage = new String(responseBytes);
            throw new TransportException(errorMessage);
        } else {
            throw new TransportException("Unknown error code: " + code);
        }
    }
    
    /**
     * Method: statementBindStream
     */
    public DatabaseDriverV1.StatementBindStreamResponse statementBindStream(DatabaseDriverV1.StatementBindStreamRequest request) throws ServiceException, TransportException {
        TransportResponse response = transport.handleMessage(
            "DatabaseDriver",
            "statement_bind_stream",
            request.toByteArray()
        );
        
        int code = response.getCode();
        byte[] responseBytes = response.getResponseBytes();
        
        if (code == CoreTransport.CODE_SUCCESS) {
            try {
                return DatabaseDriverV1.StatementBindStreamResponse.parseFrom(responseBytes);
            } catch (InvalidProtocolBufferException e) {
                throw new TransportException("Invalid protocol buffer exception: " + e.getMessage());
            }
        } else if (code == CoreTransport.CODE_APPLICATION_ERROR) {
            try {
                DatabaseDriverV1.DriverException error = DatabaseDriverV1.DriverException.parseFrom(responseBytes);
                throw new ServiceException(error);
            } catch (InvalidProtocolBufferException e) {
                throw new TransportException("Invalid protocol buffer exception: " + e.getMessage());
            }
        } else if (code == CoreTransport.CODE_TRANSPORT_ERROR) {
            String errorMessage = new String(responseBytes);
            throw new TransportException(errorMessage);
        } else {
            throw new TransportException("Unknown error code: " + code);
        }
    }
    
    /**
     * Method: statementExecuteQuery
     */
    public DatabaseDriverV1.StatementExecuteQueryResponse statementExecuteQuery(DatabaseDriverV1.StatementExecuteQueryRequest request) throws ServiceException, TransportException {
        TransportResponse response = transport.handleMessage(
            "DatabaseDriver",
            "statement_execute_query",
            request.toByteArray()
        );
        
        int code = response.getCode();
        byte[] responseBytes = response.getResponseBytes();
        
        if (code == CoreTransport.CODE_SUCCESS) {
            try {
                return DatabaseDriverV1.StatementExecuteQueryResponse.parseFrom(responseBytes);
            } catch (InvalidProtocolBufferException e) {
                throw new TransportException("Invalid protocol buffer exception: " + e.getMessage());
            }
        } else if (code == CoreTransport.CODE_APPLICATION_ERROR) {
            try {
                DatabaseDriverV1.DriverException error = DatabaseDriverV1.DriverException.parseFrom(responseBytes);
                throw new ServiceException(error);
            } catch (InvalidProtocolBufferException e) {
                throw new TransportException("Invalid protocol buffer exception: " + e.getMessage());
            }
        } else if (code == CoreTransport.CODE_TRANSPORT_ERROR) {
            String errorMessage = new String(responseBytes);
            throw new TransportException(errorMessage);
        } else {
            throw new TransportException("Unknown error code: " + code);
        }
    }
    
    /**
     * Method: statementExecutePartitions
     */
    public DatabaseDriverV1.StatementExecutePartitionsResponse statementExecutePartitions(DatabaseDriverV1.StatementExecutePartitionsRequest request) throws ServiceException, TransportException {
        TransportResponse response = transport.handleMessage(
            "DatabaseDriver",
            "statement_execute_partitions",
            request.toByteArray()
        );
        
        int code = response.getCode();
        byte[] responseBytes = response.getResponseBytes();
        
        if (code == CoreTransport.CODE_SUCCESS) {
            try {
                return DatabaseDriverV1.StatementExecutePartitionsResponse.parseFrom(responseBytes);
            } catch (InvalidProtocolBufferException e) {
                throw new TransportException("Invalid protocol buffer exception: " + e.getMessage());
            }
        } else if (code == CoreTransport.CODE_APPLICATION_ERROR) {
            try {
                DatabaseDriverV1.DriverException error = DatabaseDriverV1.DriverException.parseFrom(responseBytes);
                throw new ServiceException(error);
            } catch (InvalidProtocolBufferException e) {
                throw new TransportException("Invalid protocol buffer exception: " + e.getMessage());
            }
        } else if (code == CoreTransport.CODE_TRANSPORT_ERROR) {
            String errorMessage = new String(responseBytes);
            throw new TransportException(errorMessage);
        } else {
            throw new TransportException("Unknown error code: " + code);
        }
    }
    
    /**
     * Method: statementReadPartition
     */
    public DatabaseDriverV1.StatementReadPartitionResponse statementReadPartition(DatabaseDriverV1.StatementReadPartitionRequest request) throws ServiceException, TransportException {
        TransportResponse response = transport.handleMessage(
            "DatabaseDriver",
            "statement_read_partition",
            request.toByteArray()
        );
        
        int code = response.getCode();
        byte[] responseBytes = response.getResponseBytes();
        
        if (code == CoreTransport.CODE_SUCCESS) {
            try {
                return DatabaseDriverV1.StatementReadPartitionResponse.parseFrom(responseBytes);
            } catch (InvalidProtocolBufferException e) {
                throw new TransportException("Invalid protocol buffer exception: " + e.getMessage());
            }
        } else if (code == CoreTransport.CODE_APPLICATION_ERROR) {
            try {
                DatabaseDriverV1.DriverException error = DatabaseDriverV1.DriverException.parseFrom(responseBytes);
                throw new ServiceException(error);
            } catch (InvalidProtocolBufferException e) {
                throw new TransportException("Invalid protocol buffer exception: " + e.getMessage());
            }
        } else if (code == CoreTransport.CODE_TRANSPORT_ERROR) {
            String errorMessage = new String(responseBytes);
            throw new TransportException(errorMessage);
        } else {
            throw new TransportException("Unknown error code: " + code);
        }
    }
    
}
