from abc import ABC, abstractmethod
from typing import Optional
from google.protobuf import message
from .database_driver_v1_pb2 import *
from .proto_exception import *

class ProtoError(Exception):
    def __init__(self, error_type: str, details: str):
        self.error_type = error_type
        self.details = details
        super().__init__(f"{error_type}: {details}")

class DatabaseDriver(ABC):
    @abstractmethod
    def database_new(self, request: DatabaseNewRequest) -> DatabaseNewResponse:
        pass

    @abstractmethod
    def database_set_option_string(self, request: DatabaseSetOptionStringRequest) -> DatabaseSetOptionStringResponse:
        pass

    @abstractmethod
    def database_set_option_bytes(self, request: DatabaseSetOptionBytesRequest) -> DatabaseSetOptionBytesResponse:
        pass

    @abstractmethod
    def database_set_option_int(self, request: DatabaseSetOptionIntRequest) -> DatabaseSetOptionIntResponse:
        pass

    @abstractmethod
    def database_set_option_double(self, request: DatabaseSetOptionDoubleRequest) -> DatabaseSetOptionDoubleResponse:
        pass

    @abstractmethod
    def database_init(self, request: DatabaseInitRequest) -> DatabaseInitResponse:
        pass

    @abstractmethod
    def database_release(self, request: DatabaseReleaseRequest) -> DatabaseReleaseResponse:
        pass

    @abstractmethod
    def connection_new(self, request: ConnectionNewRequest) -> ConnectionNewResponse:
        pass

    @abstractmethod
    def connection_set_option_string(self, request: ConnectionSetOptionStringRequest) -> ConnectionSetOptionStringResponse:
        pass

    @abstractmethod
    def connection_set_option_bytes(self, request: ConnectionSetOptionBytesRequest) -> ConnectionSetOptionBytesResponse:
        pass

    @abstractmethod
    def connection_set_option_int(self, request: ConnectionSetOptionIntRequest) -> ConnectionSetOptionIntResponse:
        pass

    @abstractmethod
    def connection_set_option_double(self, request: ConnectionSetOptionDoubleRequest) -> ConnectionSetOptionDoubleResponse:
        pass

    @abstractmethod
    def connection_init(self, request: ConnectionInitRequest) -> ConnectionInitResponse:
        pass

    @abstractmethod
    def connection_release(self, request: ConnectionReleaseRequest) -> ConnectionReleaseResponse:
        pass

    @abstractmethod
    def connection_get_info(self, request: ConnectionGetInfoRequest) -> ConnectionGetInfoResponse:
        pass

    @abstractmethod
    def connection_get_objects(self, request: ConnectionGetObjectsRequest) -> ConnectionGetObjectsResponse:
        pass

    @abstractmethod
    def connection_get_table_schema(self, request: ConnectionGetTableSchemaRequest) -> ConnectionGetTableSchemaResponse:
        pass

    @abstractmethod
    def connection_get_table_types(self, request: ConnectionGetTableTypesRequest) -> ConnectionGetTableTypesResponse:
        pass

    @abstractmethod
    def connection_commit(self, request: ConnectionCommitRequest) -> ConnectionCommitResponse:
        pass

    @abstractmethod
    def connection_rollback(self, request: ConnectionRollbackRequest) -> ConnectionRollbackResponse:
        pass

    @abstractmethod
    def statement_new(self, request: StatementNewRequest) -> StatementNewResponse:
        pass

    @abstractmethod
    def statement_release(self, request: StatementReleaseRequest) -> StatementReleaseResponse:
        pass

    @abstractmethod
    def statement_set_sql_query(self, request: StatementSetSqlQueryRequest) -> StatementSetSqlQueryResponse:
        pass

    @abstractmethod
    def statement_set_substrait_plan(self, request: StatementSetSubstraitPlanRequest) -> StatementSetSubstraitPlanResponse:
        pass

    @abstractmethod
    def statement_prepare(self, request: StatementPrepareRequest) -> StatementPrepareResponse:
        pass

    @abstractmethod
    def statement_set_option_string(self, request: StatementSetOptionStringRequest) -> StatementSetOptionStringResponse:
        pass

    @abstractmethod
    def statement_set_option_bytes(self, request: StatementSetOptionBytesRequest) -> StatementSetOptionBytesResponse:
        pass

    @abstractmethod
    def statement_set_option_int(self, request: StatementSetOptionIntRequest) -> StatementSetOptionIntResponse:
        pass

    @abstractmethod
    def statement_set_option_double(self, request: StatementSetOptionDoubleRequest) -> StatementSetOptionDoubleResponse:
        pass

    @abstractmethod
    def statement_get_parameter_schema(self, request: StatementGetParameterSchemaRequest) -> StatementGetParameterSchemaResponse:
        pass

    @abstractmethod
    def statement_bind(self, request: StatementBindRequest) -> StatementBindResponse:
        pass

    @abstractmethod
    def statement_bind_stream(self, request: StatementBindStreamRequest) -> StatementBindStreamResponse:
        pass

    @abstractmethod
    def statement_execute_query(self, request: StatementExecuteQueryRequest) -> StatementExecuteQueryResponse:
        pass

    @abstractmethod
    def statement_execute_partitions(self, request: StatementExecutePartitionsRequest) -> StatementExecutePartitionsResponse:
        pass

    @abstractmethod
    def statement_read_partition(self, request: StatementReadPartitionRequest) -> StatementReadPartitionResponse:
        pass



class DatabaseDriverServer(DatabaseDriver):
    def handle_message(self, method: str, message_bytes: bytes) -> bytes:
        try:
            # Dispatch to appropriate method
            method_map = {
                'database_new': (self.database_new, DatabaseNewRequest),
                'database_set_option_string': (self.database_set_option_string, DatabaseSetOptionStringRequest),
                'database_set_option_bytes': (self.database_set_option_bytes, DatabaseSetOptionBytesRequest),
                'database_set_option_int': (self.database_set_option_int, DatabaseSetOptionIntRequest),
                'database_set_option_double': (self.database_set_option_double, DatabaseSetOptionDoubleRequest),
                'database_init': (self.database_init, DatabaseInitRequest),
                'database_release': (self.database_release, DatabaseReleaseRequest),
                'connection_new': (self.connection_new, ConnectionNewRequest),
                'connection_set_option_string': (self.connection_set_option_string, ConnectionSetOptionStringRequest),
                'connection_set_option_bytes': (self.connection_set_option_bytes, ConnectionSetOptionBytesRequest),
                'connection_set_option_int': (self.connection_set_option_int, ConnectionSetOptionIntRequest),
                'connection_set_option_double': (self.connection_set_option_double, ConnectionSetOptionDoubleRequest),
                'connection_init': (self.connection_init, ConnectionInitRequest),
                'connection_release': (self.connection_release, ConnectionReleaseRequest),
                'connection_get_info': (self.connection_get_info, ConnectionGetInfoRequest),
                'connection_get_objects': (self.connection_get_objects, ConnectionGetObjectsRequest),
                'connection_get_table_schema': (self.connection_get_table_schema, ConnectionGetTableSchemaRequest),
                'connection_get_table_types': (self.connection_get_table_types, ConnectionGetTableTypesRequest),
                'connection_commit': (self.connection_commit, ConnectionCommitRequest),
                'connection_rollback': (self.connection_rollback, ConnectionRollbackRequest),
                'statement_new': (self.statement_new, StatementNewRequest),
                'statement_release': (self.statement_release, StatementReleaseRequest),
                'statement_set_sql_query': (self.statement_set_sql_query, StatementSetSqlQueryRequest),
                'statement_set_substrait_plan': (self.statement_set_substrait_plan, StatementSetSubstraitPlanRequest),
                'statement_prepare': (self.statement_prepare, StatementPrepareRequest),
                'statement_set_option_string': (self.statement_set_option_string, StatementSetOptionStringRequest),
                'statement_set_option_bytes': (self.statement_set_option_bytes, StatementSetOptionBytesRequest),
                'statement_set_option_int': (self.statement_set_option_int, StatementSetOptionIntRequest),
                'statement_set_option_double': (self.statement_set_option_double, StatementSetOptionDoubleRequest),
                'statement_get_parameter_schema': (self.statement_get_parameter_schema, StatementGetParameterSchemaRequest),
                'statement_bind': (self.statement_bind, StatementBindRequest),
                'statement_bind_stream': (self.statement_bind_stream, StatementBindStreamRequest),
                'statement_execute_query': (self.statement_execute_query, StatementExecuteQueryRequest),
                'statement_execute_partitions': (self.statement_execute_partitions, StatementExecutePartitionsRequest),
                'statement_read_partition': (self.statement_read_partition, StatementReadPartitionRequest)
            }
            
            if method not in method_map:
                raise ProtoError('Transport', f'Unknown method: {method}')
                
            handler, request_class = method_map[method]
            request = request_class()
            request.ParseFromString(message_bytes)
            response = handler(request)
            return response.SerializeToString()
            
        except Exception as e:
            raise ProtoError('Transport', str(e))

class DatabaseDriverClient:
    def __init__(self, transport):
        self._transport = transport

    def database_new(self, request: DatabaseNewRequest) -> DatabaseNewResponse:
        (code, response_bytes) = self._transport.handle_message('DatabaseDriver', 'database_new', request.SerializeToString())
        if code == 0:
            response = DatabaseNewResponse()
            response.ParseFromString(response_bytes)
            return response
        elif code == 1:
            error = DriverException()
            error.ParseFromString(response_bytes)
            raise ProtoApplicationException(error)
        elif code == 2:
            error = str(response_bytes)
            raise ProtoTransportException(response_bytes)
        else:
            raise ProtoTransportException(f"Unknown error code: %s", code)

        response.ParseFromString(self._transport.handle_message('DatabaseDriver', 'database_new', request.SerializeToString()))
        return response

    def database_set_option_string(self, request: DatabaseSetOptionStringRequest) -> DatabaseSetOptionStringResponse:
        (code, response_bytes) = self._transport.handle_message('DatabaseDriver', 'database_set_option_string', request.SerializeToString())
        if code == 0:
            response = DatabaseSetOptionStringResponse()
            response.ParseFromString(response_bytes)
            return response
        elif code == 1:
            error = DriverException()
            error.ParseFromString(response_bytes)
            raise ProtoApplicationException(error)
        elif code == 2:
            error = str(response_bytes)
            raise ProtoTransportException(response_bytes)
        else:
            raise ProtoTransportException(f"Unknown error code: %s", code)

        response.ParseFromString(self._transport.handle_message('DatabaseDriver', 'database_set_option_string', request.SerializeToString()))
        return response

    def database_set_option_bytes(self, request: DatabaseSetOptionBytesRequest) -> DatabaseSetOptionBytesResponse:
        (code, response_bytes) = self._transport.handle_message('DatabaseDriver', 'database_set_option_bytes', request.SerializeToString())
        if code == 0:
            response = DatabaseSetOptionBytesResponse()
            response.ParseFromString(response_bytes)
            return response
        elif code == 1:
            error = DriverException()
            error.ParseFromString(response_bytes)
            raise ProtoApplicationException(error)
        elif code == 2:
            error = str(response_bytes)
            raise ProtoTransportException(response_bytes)
        else:
            raise ProtoTransportException(f"Unknown error code: %s", code)

        response.ParseFromString(self._transport.handle_message('DatabaseDriver', 'database_set_option_bytes', request.SerializeToString()))
        return response

    def database_set_option_int(self, request: DatabaseSetOptionIntRequest) -> DatabaseSetOptionIntResponse:
        (code, response_bytes) = self._transport.handle_message('DatabaseDriver', 'database_set_option_int', request.SerializeToString())
        if code == 0:
            response = DatabaseSetOptionIntResponse()
            response.ParseFromString(response_bytes)
            return response
        elif code == 1:
            error = DriverException()
            error.ParseFromString(response_bytes)
            raise ProtoApplicationException(error)
        elif code == 2:
            error = str(response_bytes)
            raise ProtoTransportException(response_bytes)
        else:
            raise ProtoTransportException(f"Unknown error code: %s", code)

        response.ParseFromString(self._transport.handle_message('DatabaseDriver', 'database_set_option_int', request.SerializeToString()))
        return response

    def database_set_option_double(self, request: DatabaseSetOptionDoubleRequest) -> DatabaseSetOptionDoubleResponse:
        (code, response_bytes) = self._transport.handle_message('DatabaseDriver', 'database_set_option_double', request.SerializeToString())
        if code == 0:
            response = DatabaseSetOptionDoubleResponse()
            response.ParseFromString(response_bytes)
            return response
        elif code == 1:
            error = DriverException()
            error.ParseFromString(response_bytes)
            raise ProtoApplicationException(error)
        elif code == 2:
            error = str(response_bytes)
            raise ProtoTransportException(response_bytes)
        else:
            raise ProtoTransportException(f"Unknown error code: %s", code)

        response.ParseFromString(self._transport.handle_message('DatabaseDriver', 'database_set_option_double', request.SerializeToString()))
        return response

    def database_init(self, request: DatabaseInitRequest) -> DatabaseInitResponse:
        (code, response_bytes) = self._transport.handle_message('DatabaseDriver', 'database_init', request.SerializeToString())
        if code == 0:
            response = DatabaseInitResponse()
            response.ParseFromString(response_bytes)
            return response
        elif code == 1:
            error = DriverException()
            error.ParseFromString(response_bytes)
            raise ProtoApplicationException(error)
        elif code == 2:
            error = str(response_bytes)
            raise ProtoTransportException(response_bytes)
        else:
            raise ProtoTransportException(f"Unknown error code: %s", code)

        response.ParseFromString(self._transport.handle_message('DatabaseDriver', 'database_init', request.SerializeToString()))
        return response

    def database_release(self, request: DatabaseReleaseRequest) -> DatabaseReleaseResponse:
        (code, response_bytes) = self._transport.handle_message('DatabaseDriver', 'database_release', request.SerializeToString())
        if code == 0:
            response = DatabaseReleaseResponse()
            response.ParseFromString(response_bytes)
            return response
        elif code == 1:
            error = DriverException()
            error.ParseFromString(response_bytes)
            raise ProtoApplicationException(error)
        elif code == 2:
            error = str(response_bytes)
            raise ProtoTransportException(response_bytes)
        else:
            raise ProtoTransportException(f"Unknown error code: %s", code)

        response.ParseFromString(self._transport.handle_message('DatabaseDriver', 'database_release', request.SerializeToString()))
        return response

    def connection_new(self, request: ConnectionNewRequest) -> ConnectionNewResponse:
        (code, response_bytes) = self._transport.handle_message('DatabaseDriver', 'connection_new', request.SerializeToString())
        if code == 0:
            response = ConnectionNewResponse()
            response.ParseFromString(response_bytes)
            return response
        elif code == 1:
            error = DriverException()
            error.ParseFromString(response_bytes)
            raise ProtoApplicationException(error)
        elif code == 2:
            error = str(response_bytes)
            raise ProtoTransportException(response_bytes)
        else:
            raise ProtoTransportException(f"Unknown error code: %s", code)

        response.ParseFromString(self._transport.handle_message('DatabaseDriver', 'connection_new', request.SerializeToString()))
        return response

    def connection_set_option_string(self, request: ConnectionSetOptionStringRequest) -> ConnectionSetOptionStringResponse:
        (code, response_bytes) = self._transport.handle_message('DatabaseDriver', 'connection_set_option_string', request.SerializeToString())
        if code == 0:
            response = ConnectionSetOptionStringResponse()
            response.ParseFromString(response_bytes)
            return response
        elif code == 1:
            error = DriverException()
            error.ParseFromString(response_bytes)
            raise ProtoApplicationException(error)
        elif code == 2:
            error = str(response_bytes)
            raise ProtoTransportException(response_bytes)
        else:
            raise ProtoTransportException(f"Unknown error code: %s", code)

        response.ParseFromString(self._transport.handle_message('DatabaseDriver', 'connection_set_option_string', request.SerializeToString()))
        return response

    def connection_set_option_bytes(self, request: ConnectionSetOptionBytesRequest) -> ConnectionSetOptionBytesResponse:
        (code, response_bytes) = self._transport.handle_message('DatabaseDriver', 'connection_set_option_bytes', request.SerializeToString())
        if code == 0:
            response = ConnectionSetOptionBytesResponse()
            response.ParseFromString(response_bytes)
            return response
        elif code == 1:
            error = DriverException()
            error.ParseFromString(response_bytes)
            raise ProtoApplicationException(error)
        elif code == 2:
            error = str(response_bytes)
            raise ProtoTransportException(response_bytes)
        else:
            raise ProtoTransportException(f"Unknown error code: %s", code)

        response.ParseFromString(self._transport.handle_message('DatabaseDriver', 'connection_set_option_bytes', request.SerializeToString()))
        return response

    def connection_set_option_int(self, request: ConnectionSetOptionIntRequest) -> ConnectionSetOptionIntResponse:
        (code, response_bytes) = self._transport.handle_message('DatabaseDriver', 'connection_set_option_int', request.SerializeToString())
        if code == 0:
            response = ConnectionSetOptionIntResponse()
            response.ParseFromString(response_bytes)
            return response
        elif code == 1:
            error = DriverException()
            error.ParseFromString(response_bytes)
            raise ProtoApplicationException(error)
        elif code == 2:
            error = str(response_bytes)
            raise ProtoTransportException(response_bytes)
        else:
            raise ProtoTransportException(f"Unknown error code: %s", code)

        response.ParseFromString(self._transport.handle_message('DatabaseDriver', 'connection_set_option_int', request.SerializeToString()))
        return response

    def connection_set_option_double(self, request: ConnectionSetOptionDoubleRequest) -> ConnectionSetOptionDoubleResponse:
        (code, response_bytes) = self._transport.handle_message('DatabaseDriver', 'connection_set_option_double', request.SerializeToString())
        if code == 0:
            response = ConnectionSetOptionDoubleResponse()
            response.ParseFromString(response_bytes)
            return response
        elif code == 1:
            error = DriverException()
            error.ParseFromString(response_bytes)
            raise ProtoApplicationException(error)
        elif code == 2:
            error = str(response_bytes)
            raise ProtoTransportException(response_bytes)
        else:
            raise ProtoTransportException(f"Unknown error code: %s", code)

        response.ParseFromString(self._transport.handle_message('DatabaseDriver', 'connection_set_option_double', request.SerializeToString()))
        return response

    def connection_init(self, request: ConnectionInitRequest) -> ConnectionInitResponse:
        (code, response_bytes) = self._transport.handle_message('DatabaseDriver', 'connection_init', request.SerializeToString())
        if code == 0:
            response = ConnectionInitResponse()
            response.ParseFromString(response_bytes)
            return response
        elif code == 1:
            error = DriverException()
            error.ParseFromString(response_bytes)
            raise ProtoApplicationException(error)
        elif code == 2:
            error = str(response_bytes)
            raise ProtoTransportException(response_bytes)
        else:
            raise ProtoTransportException(f"Unknown error code: %s", code)

        response.ParseFromString(self._transport.handle_message('DatabaseDriver', 'connection_init', request.SerializeToString()))
        return response

    def connection_release(self, request: ConnectionReleaseRequest) -> ConnectionReleaseResponse:
        (code, response_bytes) = self._transport.handle_message('DatabaseDriver', 'connection_release', request.SerializeToString())
        if code == 0:
            response = ConnectionReleaseResponse()
            response.ParseFromString(response_bytes)
            return response
        elif code == 1:
            error = DriverException()
            error.ParseFromString(response_bytes)
            raise ProtoApplicationException(error)
        elif code == 2:
            error = str(response_bytes)
            raise ProtoTransportException(response_bytes)
        else:
            raise ProtoTransportException(f"Unknown error code: %s", code)

        response.ParseFromString(self._transport.handle_message('DatabaseDriver', 'connection_release', request.SerializeToString()))
        return response

    def connection_get_info(self, request: ConnectionGetInfoRequest) -> ConnectionGetInfoResponse:
        (code, response_bytes) = self._transport.handle_message('DatabaseDriver', 'connection_get_info', request.SerializeToString())
        if code == 0:
            response = ConnectionGetInfoResponse()
            response.ParseFromString(response_bytes)
            return response
        elif code == 1:
            error = DriverException()
            error.ParseFromString(response_bytes)
            raise ProtoApplicationException(error)
        elif code == 2:
            error = str(response_bytes)
            raise ProtoTransportException(response_bytes)
        else:
            raise ProtoTransportException(f"Unknown error code: %s", code)

        response.ParseFromString(self._transport.handle_message('DatabaseDriver', 'connection_get_info', request.SerializeToString()))
        return response

    def connection_get_objects(self, request: ConnectionGetObjectsRequest) -> ConnectionGetObjectsResponse:
        (code, response_bytes) = self._transport.handle_message('DatabaseDriver', 'connection_get_objects', request.SerializeToString())
        if code == 0:
            response = ConnectionGetObjectsResponse()
            response.ParseFromString(response_bytes)
            return response
        elif code == 1:
            error = DriverException()
            error.ParseFromString(response_bytes)
            raise ProtoApplicationException(error)
        elif code == 2:
            error = str(response_bytes)
            raise ProtoTransportException(response_bytes)
        else:
            raise ProtoTransportException(f"Unknown error code: %s", code)

        response.ParseFromString(self._transport.handle_message('DatabaseDriver', 'connection_get_objects', request.SerializeToString()))
        return response

    def connection_get_table_schema(self, request: ConnectionGetTableSchemaRequest) -> ConnectionGetTableSchemaResponse:
        (code, response_bytes) = self._transport.handle_message('DatabaseDriver', 'connection_get_table_schema', request.SerializeToString())
        if code == 0:
            response = ConnectionGetTableSchemaResponse()
            response.ParseFromString(response_bytes)
            return response
        elif code == 1:
            error = DriverException()
            error.ParseFromString(response_bytes)
            raise ProtoApplicationException(error)
        elif code == 2:
            error = str(response_bytes)
            raise ProtoTransportException(response_bytes)
        else:
            raise ProtoTransportException(f"Unknown error code: %s", code)

        response.ParseFromString(self._transport.handle_message('DatabaseDriver', 'connection_get_table_schema', request.SerializeToString()))
        return response

    def connection_get_table_types(self, request: ConnectionGetTableTypesRequest) -> ConnectionGetTableTypesResponse:
        (code, response_bytes) = self._transport.handle_message('DatabaseDriver', 'connection_get_table_types', request.SerializeToString())
        if code == 0:
            response = ConnectionGetTableTypesResponse()
            response.ParseFromString(response_bytes)
            return response
        elif code == 1:
            error = DriverException()
            error.ParseFromString(response_bytes)
            raise ProtoApplicationException(error)
        elif code == 2:
            error = str(response_bytes)
            raise ProtoTransportException(response_bytes)
        else:
            raise ProtoTransportException(f"Unknown error code: %s", code)

        response.ParseFromString(self._transport.handle_message('DatabaseDriver', 'connection_get_table_types', request.SerializeToString()))
        return response

    def connection_commit(self, request: ConnectionCommitRequest) -> ConnectionCommitResponse:
        (code, response_bytes) = self._transport.handle_message('DatabaseDriver', 'connection_commit', request.SerializeToString())
        if code == 0:
            response = ConnectionCommitResponse()
            response.ParseFromString(response_bytes)
            return response
        elif code == 1:
            error = DriverException()
            error.ParseFromString(response_bytes)
            raise ProtoApplicationException(error)
        elif code == 2:
            error = str(response_bytes)
            raise ProtoTransportException(response_bytes)
        else:
            raise ProtoTransportException(f"Unknown error code: %s", code)

        response.ParseFromString(self._transport.handle_message('DatabaseDriver', 'connection_commit', request.SerializeToString()))
        return response

    def connection_rollback(self, request: ConnectionRollbackRequest) -> ConnectionRollbackResponse:
        (code, response_bytes) = self._transport.handle_message('DatabaseDriver', 'connection_rollback', request.SerializeToString())
        if code == 0:
            response = ConnectionRollbackResponse()
            response.ParseFromString(response_bytes)
            return response
        elif code == 1:
            error = DriverException()
            error.ParseFromString(response_bytes)
            raise ProtoApplicationException(error)
        elif code == 2:
            error = str(response_bytes)
            raise ProtoTransportException(response_bytes)
        else:
            raise ProtoTransportException(f"Unknown error code: %s", code)

        response.ParseFromString(self._transport.handle_message('DatabaseDriver', 'connection_rollback', request.SerializeToString()))
        return response

    def statement_new(self, request: StatementNewRequest) -> StatementNewResponse:
        (code, response_bytes) = self._transport.handle_message('DatabaseDriver', 'statement_new', request.SerializeToString())
        if code == 0:
            response = StatementNewResponse()
            response.ParseFromString(response_bytes)
            return response
        elif code == 1:
            error = DriverException()
            error.ParseFromString(response_bytes)
            raise ProtoApplicationException(error)
        elif code == 2:
            error = str(response_bytes)
            raise ProtoTransportException(response_bytes)
        else:
            raise ProtoTransportException(f"Unknown error code: %s", code)

        response.ParseFromString(self._transport.handle_message('DatabaseDriver', 'statement_new', request.SerializeToString()))
        return response

    def statement_release(self, request: StatementReleaseRequest) -> StatementReleaseResponse:
        (code, response_bytes) = self._transport.handle_message('DatabaseDriver', 'statement_release', request.SerializeToString())
        if code == 0:
            response = StatementReleaseResponse()
            response.ParseFromString(response_bytes)
            return response
        elif code == 1:
            error = DriverException()
            error.ParseFromString(response_bytes)
            raise ProtoApplicationException(error)
        elif code == 2:
            error = str(response_bytes)
            raise ProtoTransportException(response_bytes)
        else:
            raise ProtoTransportException(f"Unknown error code: %s", code)

        response.ParseFromString(self._transport.handle_message('DatabaseDriver', 'statement_release', request.SerializeToString()))
        return response

    def statement_set_sql_query(self, request: StatementSetSqlQueryRequest) -> StatementSetSqlQueryResponse:
        (code, response_bytes) = self._transport.handle_message('DatabaseDriver', 'statement_set_sql_query', request.SerializeToString())
        if code == 0:
            response = StatementSetSqlQueryResponse()
            response.ParseFromString(response_bytes)
            return response
        elif code == 1:
            error = DriverException()
            error.ParseFromString(response_bytes)
            raise ProtoApplicationException(error)
        elif code == 2:
            error = str(response_bytes)
            raise ProtoTransportException(response_bytes)
        else:
            raise ProtoTransportException(f"Unknown error code: %s", code)

        response.ParseFromString(self._transport.handle_message('DatabaseDriver', 'statement_set_sql_query', request.SerializeToString()))
        return response

    def statement_set_substrait_plan(self, request: StatementSetSubstraitPlanRequest) -> StatementSetSubstraitPlanResponse:
        (code, response_bytes) = self._transport.handle_message('DatabaseDriver', 'statement_set_substrait_plan', request.SerializeToString())
        if code == 0:
            response = StatementSetSubstraitPlanResponse()
            response.ParseFromString(response_bytes)
            return response
        elif code == 1:
            error = DriverException()
            error.ParseFromString(response_bytes)
            raise ProtoApplicationException(error)
        elif code == 2:
            error = str(response_bytes)
            raise ProtoTransportException(response_bytes)
        else:
            raise ProtoTransportException(f"Unknown error code: %s", code)

        response.ParseFromString(self._transport.handle_message('DatabaseDriver', 'statement_set_substrait_plan', request.SerializeToString()))
        return response

    def statement_prepare(self, request: StatementPrepareRequest) -> StatementPrepareResponse:
        (code, response_bytes) = self._transport.handle_message('DatabaseDriver', 'statement_prepare', request.SerializeToString())
        if code == 0:
            response = StatementPrepareResponse()
            response.ParseFromString(response_bytes)
            return response
        elif code == 1:
            error = DriverException()
            error.ParseFromString(response_bytes)
            raise ProtoApplicationException(error)
        elif code == 2:
            error = str(response_bytes)
            raise ProtoTransportException(response_bytes)
        else:
            raise ProtoTransportException(f"Unknown error code: %s", code)

        response.ParseFromString(self._transport.handle_message('DatabaseDriver', 'statement_prepare', request.SerializeToString()))
        return response

    def statement_set_option_string(self, request: StatementSetOptionStringRequest) -> StatementSetOptionStringResponse:
        (code, response_bytes) = self._transport.handle_message('DatabaseDriver', 'statement_set_option_string', request.SerializeToString())
        if code == 0:
            response = StatementSetOptionStringResponse()
            response.ParseFromString(response_bytes)
            return response
        elif code == 1:
            error = DriverException()
            error.ParseFromString(response_bytes)
            raise ProtoApplicationException(error)
        elif code == 2:
            error = str(response_bytes)
            raise ProtoTransportException(response_bytes)
        else:
            raise ProtoTransportException(f"Unknown error code: %s", code)

        response.ParseFromString(self._transport.handle_message('DatabaseDriver', 'statement_set_option_string', request.SerializeToString()))
        return response

    def statement_set_option_bytes(self, request: StatementSetOptionBytesRequest) -> StatementSetOptionBytesResponse:
        (code, response_bytes) = self._transport.handle_message('DatabaseDriver', 'statement_set_option_bytes', request.SerializeToString())
        if code == 0:
            response = StatementSetOptionBytesResponse()
            response.ParseFromString(response_bytes)
            return response
        elif code == 1:
            error = DriverException()
            error.ParseFromString(response_bytes)
            raise ProtoApplicationException(error)
        elif code == 2:
            error = str(response_bytes)
            raise ProtoTransportException(response_bytes)
        else:
            raise ProtoTransportException(f"Unknown error code: %s", code)

        response.ParseFromString(self._transport.handle_message('DatabaseDriver', 'statement_set_option_bytes', request.SerializeToString()))
        return response

    def statement_set_option_int(self, request: StatementSetOptionIntRequest) -> StatementSetOptionIntResponse:
        (code, response_bytes) = self._transport.handle_message('DatabaseDriver', 'statement_set_option_int', request.SerializeToString())
        if code == 0:
            response = StatementSetOptionIntResponse()
            response.ParseFromString(response_bytes)
            return response
        elif code == 1:
            error = DriverException()
            error.ParseFromString(response_bytes)
            raise ProtoApplicationException(error)
        elif code == 2:
            error = str(response_bytes)
            raise ProtoTransportException(response_bytes)
        else:
            raise ProtoTransportException(f"Unknown error code: %s", code)

        response.ParseFromString(self._transport.handle_message('DatabaseDriver', 'statement_set_option_int', request.SerializeToString()))
        return response

    def statement_set_option_double(self, request: StatementSetOptionDoubleRequest) -> StatementSetOptionDoubleResponse:
        (code, response_bytes) = self._transport.handle_message('DatabaseDriver', 'statement_set_option_double', request.SerializeToString())
        if code == 0:
            response = StatementSetOptionDoubleResponse()
            response.ParseFromString(response_bytes)
            return response
        elif code == 1:
            error = DriverException()
            error.ParseFromString(response_bytes)
            raise ProtoApplicationException(error)
        elif code == 2:
            error = str(response_bytes)
            raise ProtoTransportException(response_bytes)
        else:
            raise ProtoTransportException(f"Unknown error code: %s", code)

        response.ParseFromString(self._transport.handle_message('DatabaseDriver', 'statement_set_option_double', request.SerializeToString()))
        return response

    def statement_get_parameter_schema(self, request: StatementGetParameterSchemaRequest) -> StatementGetParameterSchemaResponse:
        (code, response_bytes) = self._transport.handle_message('DatabaseDriver', 'statement_get_parameter_schema', request.SerializeToString())
        if code == 0:
            response = StatementGetParameterSchemaResponse()
            response.ParseFromString(response_bytes)
            return response
        elif code == 1:
            error = DriverException()
            error.ParseFromString(response_bytes)
            raise ProtoApplicationException(error)
        elif code == 2:
            error = str(response_bytes)
            raise ProtoTransportException(response_bytes)
        else:
            raise ProtoTransportException(f"Unknown error code: %s", code)

        response.ParseFromString(self._transport.handle_message('DatabaseDriver', 'statement_get_parameter_schema', request.SerializeToString()))
        return response

    def statement_bind(self, request: StatementBindRequest) -> StatementBindResponse:
        (code, response_bytes) = self._transport.handle_message('DatabaseDriver', 'statement_bind', request.SerializeToString())
        if code == 0:
            response = StatementBindResponse()
            response.ParseFromString(response_bytes)
            return response
        elif code == 1:
            error = DriverException()
            error.ParseFromString(response_bytes)
            raise ProtoApplicationException(error)
        elif code == 2:
            error = str(response_bytes)
            raise ProtoTransportException(response_bytes)
        else:
            raise ProtoTransportException(f"Unknown error code: %s", code)

        response.ParseFromString(self._transport.handle_message('DatabaseDriver', 'statement_bind', request.SerializeToString()))
        return response

    def statement_bind_stream(self, request: StatementBindStreamRequest) -> StatementBindStreamResponse:
        (code, response_bytes) = self._transport.handle_message('DatabaseDriver', 'statement_bind_stream', request.SerializeToString())
        if code == 0:
            response = StatementBindStreamResponse()
            response.ParseFromString(response_bytes)
            return response
        elif code == 1:
            error = DriverException()
            error.ParseFromString(response_bytes)
            raise ProtoApplicationException(error)
        elif code == 2:
            error = str(response_bytes)
            raise ProtoTransportException(response_bytes)
        else:
            raise ProtoTransportException(f"Unknown error code: %s", code)

        response.ParseFromString(self._transport.handle_message('DatabaseDriver', 'statement_bind_stream', request.SerializeToString()))
        return response

    def statement_execute_query(self, request: StatementExecuteQueryRequest) -> StatementExecuteQueryResponse:
        (code, response_bytes) = self._transport.handle_message('DatabaseDriver', 'statement_execute_query', request.SerializeToString())
        if code == 0:
            response = StatementExecuteQueryResponse()
            response.ParseFromString(response_bytes)
            return response
        elif code == 1:
            error = DriverException()
            error.ParseFromString(response_bytes)
            raise ProtoApplicationException(error)
        elif code == 2:
            error = str(response_bytes)
            raise ProtoTransportException(response_bytes)
        else:
            raise ProtoTransportException(f"Unknown error code: %s", code)

        response.ParseFromString(self._transport.handle_message('DatabaseDriver', 'statement_execute_query', request.SerializeToString()))
        return response

    def statement_execute_partitions(self, request: StatementExecutePartitionsRequest) -> StatementExecutePartitionsResponse:
        (code, response_bytes) = self._transport.handle_message('DatabaseDriver', 'statement_execute_partitions', request.SerializeToString())
        if code == 0:
            response = StatementExecutePartitionsResponse()
            response.ParseFromString(response_bytes)
            return response
        elif code == 1:
            error = DriverException()
            error.ParseFromString(response_bytes)
            raise ProtoApplicationException(error)
        elif code == 2:
            error = str(response_bytes)
            raise ProtoTransportException(response_bytes)
        else:
            raise ProtoTransportException(f"Unknown error code: %s", code)

        response.ParseFromString(self._transport.handle_message('DatabaseDriver', 'statement_execute_partitions', request.SerializeToString()))
        return response

    def statement_read_partition(self, request: StatementReadPartitionRequest) -> StatementReadPartitionResponse:
        (code, response_bytes) = self._transport.handle_message('DatabaseDriver', 'statement_read_partition', request.SerializeToString())
        if code == 0:
            response = StatementReadPartitionResponse()
            response.ParseFromString(response_bytes)
            return response
        elif code == 1:
            error = DriverException()
            error.ParseFromString(response_bytes)
            raise ProtoApplicationException(error)
        elif code == 2:
            error = str(response_bytes)
            raise ProtoTransportException(response_bytes)
        else:
            raise ProtoTransportException(f"Unknown error code: %s", code)

        response.ParseFromString(self._transport.handle_message('DatabaseDriver', 'statement_read_partition', request.SerializeToString()))
        return response

