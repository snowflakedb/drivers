from google.protobuf import descriptor_pb2 as _descriptor_pb2
from google.protobuf.internal import containers as _containers
from google.protobuf.internal import enum_type_wrapper as _enum_type_wrapper
from google.protobuf import descriptor as _descriptor
from google.protobuf import message as _message
from collections.abc import Iterable as _Iterable, Mapping as _Mapping
from typing import ClassVar as _ClassVar, Optional as _Optional, Union as _Union

DESCRIPTOR: _descriptor.FileDescriptor

class StatusCode(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    STATUS_CODE_UNSPECIFIED: _ClassVar[StatusCode]
    STATUS_CODE_OK: _ClassVar[StatusCode]
    STATUS_CODE_AUTHENTICATION_ERROR: _ClassVar[StatusCode]
    STATUS_CODE_NOT_IMPLEMENTED: _ClassVar[StatusCode]
    STATUS_CODE_NOT_FOUND: _ClassVar[StatusCode]
    STATUS_CODE_ALREADY_EXISTS: _ClassVar[StatusCode]
    STATUS_CODE_INVALID_ARGUMENT: _ClassVar[StatusCode]
    STATUS_CODE_INVALID_STATE: _ClassVar[StatusCode]
    STATUS_CODE_INVALID_DATA: _ClassVar[StatusCode]
    STATUS_CODE_IO: _ClassVar[StatusCode]
    STATUS_CODE_CANCELLED: _ClassVar[StatusCode]
    STATUS_CODE_UNAUTHENTICATED: _ClassVar[StatusCode]
    STATUS_CODE_UNAUTHORIZED: _ClassVar[StatusCode]
    STATUS_CODE_GENERIC_ERROR: _ClassVar[StatusCode]
    STATUS_CODE_INTERNAL_ERROR: _ClassVar[StatusCode]
    STATUS_CODE_MISSING_PARAMETER: _ClassVar[StatusCode]
    STATUS_CODE_INVALID_PARAMETER_VALUE: _ClassVar[StatusCode]
    STATUS_CODE_LOGIN_ERROR: _ClassVar[StatusCode]

class InfoCode(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    INFO_CODE_UNSPECIFIED: _ClassVar[InfoCode]
    INFO_CODE_VENDOR_NAME: _ClassVar[InfoCode]
    INFO_CODE_VENDOR_VERSION: _ClassVar[InfoCode]
    INFO_CODE_VENDOR_ARROW_VERSION: _ClassVar[InfoCode]
    INFO_CODE_VENDOR_SQL: _ClassVar[InfoCode]
    INFO_CODE_VENDOR_SUBSTRAIT: _ClassVar[InfoCode]
    INFO_CODE_VENDOR_SUBSTRAIT_MIN_VERSION: _ClassVar[InfoCode]
    INFO_CODE_VENDOR_SUBSTRAIT_MAX_VERSION: _ClassVar[InfoCode]
    INFO_CODE_DRIVER_NAME: _ClassVar[InfoCode]
    INFO_CODE_DRIVER_VERSION: _ClassVar[InfoCode]
    INFO_CODE_DRIVER_ARROW_VERSION: _ClassVar[InfoCode]
    INFO_CODE_DRIVER_ADBC_VERSION: _ClassVar[InfoCode]
STATUS_CODE_UNSPECIFIED: StatusCode
STATUS_CODE_OK: StatusCode
STATUS_CODE_AUTHENTICATION_ERROR: StatusCode
STATUS_CODE_NOT_IMPLEMENTED: StatusCode
STATUS_CODE_NOT_FOUND: StatusCode
STATUS_CODE_ALREADY_EXISTS: StatusCode
STATUS_CODE_INVALID_ARGUMENT: StatusCode
STATUS_CODE_INVALID_STATE: StatusCode
STATUS_CODE_INVALID_DATA: StatusCode
STATUS_CODE_IO: StatusCode
STATUS_CODE_CANCELLED: StatusCode
STATUS_CODE_UNAUTHENTICATED: StatusCode
STATUS_CODE_UNAUTHORIZED: StatusCode
STATUS_CODE_GENERIC_ERROR: StatusCode
STATUS_CODE_INTERNAL_ERROR: StatusCode
STATUS_CODE_MISSING_PARAMETER: StatusCode
STATUS_CODE_INVALID_PARAMETER_VALUE: StatusCode
STATUS_CODE_LOGIN_ERROR: StatusCode
INFO_CODE_UNSPECIFIED: InfoCode
INFO_CODE_VENDOR_NAME: InfoCode
INFO_CODE_VENDOR_VERSION: InfoCode
INFO_CODE_VENDOR_ARROW_VERSION: InfoCode
INFO_CODE_VENDOR_SQL: InfoCode
INFO_CODE_VENDOR_SUBSTRAIT: InfoCode
INFO_CODE_VENDOR_SUBSTRAIT_MIN_VERSION: InfoCode
INFO_CODE_VENDOR_SUBSTRAIT_MAX_VERSION: InfoCode
INFO_CODE_DRIVER_NAME: InfoCode
INFO_CODE_DRIVER_VERSION: InfoCode
INFO_CODE_DRIVER_ARROW_VERSION: InfoCode
INFO_CODE_DRIVER_ADBC_VERSION: InfoCode
SERVICE_ERROR_FIELD_NUMBER: _ClassVar[int]
service_error: _descriptor.FieldDescriptor
METHOD_ERROR_FIELD_NUMBER: _ClassVar[int]
method_error: _descriptor.FieldDescriptor

class ErrorDetail(_message.Message):
    __slots__ = ("key", "value")
    KEY_FIELD_NUMBER: _ClassVar[int]
    VALUE_FIELD_NUMBER: _ClassVar[int]
    key: str
    value: str
    def __init__(self, key: _Optional[str] = ..., value: _Optional[str] = ...) -> None: ...

class AuthenticationError(_message.Message):
    __slots__ = ("detail",)
    DETAIL_FIELD_NUMBER: _ClassVar[int]
    detail: str
    def __init__(self, detail: _Optional[str] = ...) -> None: ...

class GenericError(_message.Message):
    __slots__ = ()
    def __init__(self) -> None: ...

class InternalError(_message.Message):
    __slots__ = ()
    def __init__(self) -> None: ...

class LoginError(_message.Message):
    __slots__ = ("message", "code")
    MESSAGE_FIELD_NUMBER: _ClassVar[int]
    CODE_FIELD_NUMBER: _ClassVar[int]
    message: str
    code: int
    def __init__(self, message: _Optional[str] = ..., code: _Optional[int] = ...) -> None: ...

class MissingParameter(_message.Message):
    __slots__ = ("parameter",)
    PARAMETER_FIELD_NUMBER: _ClassVar[int]
    parameter: str
    def __init__(self, parameter: _Optional[str] = ...) -> None: ...

class InvalidParameterValue(_message.Message):
    __slots__ = ("parameter", "value", "explanation")
    PARAMETER_FIELD_NUMBER: _ClassVar[int]
    VALUE_FIELD_NUMBER: _ClassVar[int]
    EXPLANATION_FIELD_NUMBER: _ClassVar[int]
    parameter: str
    value: str
    explanation: str
    def __init__(self, parameter: _Optional[str] = ..., value: _Optional[str] = ..., explanation: _Optional[str] = ...) -> None: ...

class DriverError(_message.Message):
    __slots__ = ("auth_error", "generic_error", "internal_error", "missing_parameter", "invalid_parameter_value", "login_error")
    AUTH_ERROR_FIELD_NUMBER: _ClassVar[int]
    GENERIC_ERROR_FIELD_NUMBER: _ClassVar[int]
    INTERNAL_ERROR_FIELD_NUMBER: _ClassVar[int]
    MISSING_PARAMETER_FIELD_NUMBER: _ClassVar[int]
    INVALID_PARAMETER_VALUE_FIELD_NUMBER: _ClassVar[int]
    LOGIN_ERROR_FIELD_NUMBER: _ClassVar[int]
    auth_error: AuthenticationError
    generic_error: GenericError
    internal_error: InternalError
    missing_parameter: MissingParameter
    invalid_parameter_value: InvalidParameterValue
    login_error: LoginError
    def __init__(self, auth_error: _Optional[_Union[AuthenticationError, _Mapping]] = ..., generic_error: _Optional[_Union[GenericError, _Mapping]] = ..., internal_error: _Optional[_Union[InternalError, _Mapping]] = ..., missing_parameter: _Optional[_Union[MissingParameter, _Mapping]] = ..., invalid_parameter_value: _Optional[_Union[InvalidParameterValue, _Mapping]] = ..., login_error: _Optional[_Union[LoginError, _Mapping]] = ...) -> None: ...

class DriverException(_message.Message):
    __slots__ = ("message", "status_code", "error", "report")
    MESSAGE_FIELD_NUMBER: _ClassVar[int]
    STATUS_CODE_FIELD_NUMBER: _ClassVar[int]
    ERROR_FIELD_NUMBER: _ClassVar[int]
    REPORT_FIELD_NUMBER: _ClassVar[int]
    message: str
    status_code: StatusCode
    error: DriverError
    report: str
    def __init__(self, message: _Optional[str] = ..., status_code: _Optional[_Union[StatusCode, str]] = ..., error: _Optional[_Union[DriverError, _Mapping]] = ..., report: _Optional[str] = ...) -> None: ...

class ColumnMetadata(_message.Message):
    __slots__ = ("name", "type", "precision", "scale", "length", "byte_length", "nullable")
    NAME_FIELD_NUMBER: _ClassVar[int]
    TYPE_FIELD_NUMBER: _ClassVar[int]
    PRECISION_FIELD_NUMBER: _ClassVar[int]
    SCALE_FIELD_NUMBER: _ClassVar[int]
    LENGTH_FIELD_NUMBER: _ClassVar[int]
    BYTE_LENGTH_FIELD_NUMBER: _ClassVar[int]
    NULLABLE_FIELD_NUMBER: _ClassVar[int]
    name: str
    type: str
    precision: int
    scale: int
    length: int
    byte_length: int
    nullable: bool
    def __init__(self, name: _Optional[str] = ..., type: _Optional[str] = ..., precision: _Optional[int] = ..., scale: _Optional[int] = ..., length: _Optional[int] = ..., byte_length: _Optional[int] = ..., nullable: _Optional[bool] = ...) -> None: ...

class ExecuteResult(_message.Message):
    __slots__ = ("stream", "rows_affected", "query_id", "columns")
    STREAM_FIELD_NUMBER: _ClassVar[int]
    ROWS_AFFECTED_FIELD_NUMBER: _ClassVar[int]
    QUERY_ID_FIELD_NUMBER: _ClassVar[int]
    COLUMNS_FIELD_NUMBER: _ClassVar[int]
    stream: ArrowArrayStreamPtr
    rows_affected: int
    query_id: str
    columns: _containers.RepeatedCompositeFieldContainer[ColumnMetadata]
    def __init__(self, stream: _Optional[_Union[ArrowArrayStreamPtr, _Mapping]] = ..., rows_affected: _Optional[int] = ..., query_id: _Optional[str] = ..., columns: _Optional[_Iterable[_Union[ColumnMetadata, _Mapping]]] = ...) -> None: ...

class PartitionedResult(_message.Message):
    __slots__ = ("schema", "partitions", "rows_affected")
    SCHEMA_FIELD_NUMBER: _ClassVar[int]
    PARTITIONS_FIELD_NUMBER: _ClassVar[int]
    ROWS_AFFECTED_FIELD_NUMBER: _ClassVar[int]
    schema: int
    partitions: _containers.RepeatedScalarFieldContainer[bytes]
    rows_affected: int
    def __init__(self, schema: _Optional[int] = ..., partitions: _Optional[_Iterable[bytes]] = ..., rows_affected: _Optional[int] = ...) -> None: ...

class DatabaseHandle(_message.Message):
    __slots__ = ("id", "magic")
    ID_FIELD_NUMBER: _ClassVar[int]
    MAGIC_FIELD_NUMBER: _ClassVar[int]
    id: int
    magic: int
    def __init__(self, id: _Optional[int] = ..., magic: _Optional[int] = ...) -> None: ...

class ConnectionHandle(_message.Message):
    __slots__ = ("id", "magic")
    ID_FIELD_NUMBER: _ClassVar[int]
    MAGIC_FIELD_NUMBER: _ClassVar[int]
    id: int
    magic: int
    def __init__(self, id: _Optional[int] = ..., magic: _Optional[int] = ...) -> None: ...

class StatementHandle(_message.Message):
    __slots__ = ("id", "magic")
    ID_FIELD_NUMBER: _ClassVar[int]
    MAGIC_FIELD_NUMBER: _ClassVar[int]
    id: int
    magic: int
    def __init__(self, id: _Optional[int] = ..., magic: _Optional[int] = ...) -> None: ...

class ArrowArrayStreamPtr(_message.Message):
    __slots__ = ("value",)
    VALUE_FIELD_NUMBER: _ClassVar[int]
    value: bytes
    def __init__(self, value: _Optional[bytes] = ...) -> None: ...

class ArrowSchemaPtr(_message.Message):
    __slots__ = ("value",)
    VALUE_FIELD_NUMBER: _ClassVar[int]
    value: bytes
    def __init__(self, value: _Optional[bytes] = ...) -> None: ...

class ArrowArrayPtr(_message.Message):
    __slots__ = ("value",)
    VALUE_FIELD_NUMBER: _ClassVar[int]
    value: bytes
    def __init__(self, value: _Optional[bytes] = ...) -> None: ...

class DatabaseNewRequest(_message.Message):
    __slots__ = ()
    def __init__(self) -> None: ...

class DatabaseNewResponse(_message.Message):
    __slots__ = ("db_handle",)
    DB_HANDLE_FIELD_NUMBER: _ClassVar[int]
    db_handle: DatabaseHandle
    def __init__(self, db_handle: _Optional[_Union[DatabaseHandle, _Mapping]] = ...) -> None: ...

class DatabaseSetOptionStringRequest(_message.Message):
    __slots__ = ("db_handle", "key", "value")
    DB_HANDLE_FIELD_NUMBER: _ClassVar[int]
    KEY_FIELD_NUMBER: _ClassVar[int]
    VALUE_FIELD_NUMBER: _ClassVar[int]
    db_handle: DatabaseHandle
    key: str
    value: str
    def __init__(self, db_handle: _Optional[_Union[DatabaseHandle, _Mapping]] = ..., key: _Optional[str] = ..., value: _Optional[str] = ...) -> None: ...

class DatabaseSetOptionStringResponse(_message.Message):
    __slots__ = ()
    def __init__(self) -> None: ...

class DatabaseSetOptionBytesRequest(_message.Message):
    __slots__ = ("db_handle", "key", "value")
    DB_HANDLE_FIELD_NUMBER: _ClassVar[int]
    KEY_FIELD_NUMBER: _ClassVar[int]
    VALUE_FIELD_NUMBER: _ClassVar[int]
    db_handle: DatabaseHandle
    key: str
    value: bytes
    def __init__(self, db_handle: _Optional[_Union[DatabaseHandle, _Mapping]] = ..., key: _Optional[str] = ..., value: _Optional[bytes] = ...) -> None: ...

class DatabaseSetOptionBytesResponse(_message.Message):
    __slots__ = ()
    def __init__(self) -> None: ...

class DatabaseSetOptionIntRequest(_message.Message):
    __slots__ = ("db_handle", "key", "value")
    DB_HANDLE_FIELD_NUMBER: _ClassVar[int]
    KEY_FIELD_NUMBER: _ClassVar[int]
    VALUE_FIELD_NUMBER: _ClassVar[int]
    db_handle: DatabaseHandle
    key: str
    value: int
    def __init__(self, db_handle: _Optional[_Union[DatabaseHandle, _Mapping]] = ..., key: _Optional[str] = ..., value: _Optional[int] = ...) -> None: ...

class DatabaseSetOptionIntResponse(_message.Message):
    __slots__ = ()
    def __init__(self) -> None: ...

class DatabaseSetOptionDoubleRequest(_message.Message):
    __slots__ = ("db_handle", "key", "value")
    DB_HANDLE_FIELD_NUMBER: _ClassVar[int]
    KEY_FIELD_NUMBER: _ClassVar[int]
    VALUE_FIELD_NUMBER: _ClassVar[int]
    db_handle: DatabaseHandle
    key: str
    value: float
    def __init__(self, db_handle: _Optional[_Union[DatabaseHandle, _Mapping]] = ..., key: _Optional[str] = ..., value: _Optional[float] = ...) -> None: ...

class DatabaseSetOptionDoubleResponse(_message.Message):
    __slots__ = ()
    def __init__(self) -> None: ...

class DatabaseInitRequest(_message.Message):
    __slots__ = ("db_handle",)
    DB_HANDLE_FIELD_NUMBER: _ClassVar[int]
    db_handle: DatabaseHandle
    def __init__(self, db_handle: _Optional[_Union[DatabaseHandle, _Mapping]] = ...) -> None: ...

class DatabaseInitResponse(_message.Message):
    __slots__ = ()
    def __init__(self) -> None: ...

class DatabaseReleaseRequest(_message.Message):
    __slots__ = ("db_handle",)
    DB_HANDLE_FIELD_NUMBER: _ClassVar[int]
    db_handle: DatabaseHandle
    def __init__(self, db_handle: _Optional[_Union[DatabaseHandle, _Mapping]] = ...) -> None: ...

class DatabaseReleaseResponse(_message.Message):
    __slots__ = ()
    def __init__(self) -> None: ...

class ConnectionNewRequest(_message.Message):
    __slots__ = ()
    def __init__(self) -> None: ...

class ConnectionNewResponse(_message.Message):
    __slots__ = ("conn_handle",)
    CONN_HANDLE_FIELD_NUMBER: _ClassVar[int]
    conn_handle: ConnectionHandle
    def __init__(self, conn_handle: _Optional[_Union[ConnectionHandle, _Mapping]] = ...) -> None: ...

class ConnectionSetOptionStringRequest(_message.Message):
    __slots__ = ("conn_handle", "key", "value")
    CONN_HANDLE_FIELD_NUMBER: _ClassVar[int]
    KEY_FIELD_NUMBER: _ClassVar[int]
    VALUE_FIELD_NUMBER: _ClassVar[int]
    conn_handle: ConnectionHandle
    key: str
    value: str
    def __init__(self, conn_handle: _Optional[_Union[ConnectionHandle, _Mapping]] = ..., key: _Optional[str] = ..., value: _Optional[str] = ...) -> None: ...

class ConnectionSetOptionStringResponse(_message.Message):
    __slots__ = ()
    def __init__(self) -> None: ...

class ConnectionSetOptionBytesRequest(_message.Message):
    __slots__ = ("conn_handle", "key", "value")
    CONN_HANDLE_FIELD_NUMBER: _ClassVar[int]
    KEY_FIELD_NUMBER: _ClassVar[int]
    VALUE_FIELD_NUMBER: _ClassVar[int]
    conn_handle: ConnectionHandle
    key: str
    value: bytes
    def __init__(self, conn_handle: _Optional[_Union[ConnectionHandle, _Mapping]] = ..., key: _Optional[str] = ..., value: _Optional[bytes] = ...) -> None: ...

class ConnectionSetOptionBytesResponse(_message.Message):
    __slots__ = ()
    def __init__(self) -> None: ...

class ConnectionSetOptionIntRequest(_message.Message):
    __slots__ = ("conn_handle", "key", "value")
    CONN_HANDLE_FIELD_NUMBER: _ClassVar[int]
    KEY_FIELD_NUMBER: _ClassVar[int]
    VALUE_FIELD_NUMBER: _ClassVar[int]
    conn_handle: ConnectionHandle
    key: str
    value: int
    def __init__(self, conn_handle: _Optional[_Union[ConnectionHandle, _Mapping]] = ..., key: _Optional[str] = ..., value: _Optional[int] = ...) -> None: ...

class ConnectionSetOptionIntResponse(_message.Message):
    __slots__ = ()
    def __init__(self) -> None: ...

class ConnectionSetOptionDoubleRequest(_message.Message):
    __slots__ = ("conn_handle", "key", "value")
    CONN_HANDLE_FIELD_NUMBER: _ClassVar[int]
    KEY_FIELD_NUMBER: _ClassVar[int]
    VALUE_FIELD_NUMBER: _ClassVar[int]
    conn_handle: ConnectionHandle
    key: str
    value: float
    def __init__(self, conn_handle: _Optional[_Union[ConnectionHandle, _Mapping]] = ..., key: _Optional[str] = ..., value: _Optional[float] = ...) -> None: ...

class ConnectionSetOptionDoubleResponse(_message.Message):
    __slots__ = ()
    def __init__(self) -> None: ...

class ConnectionInitRequest(_message.Message):
    __slots__ = ("conn_handle", "db_handle")
    CONN_HANDLE_FIELD_NUMBER: _ClassVar[int]
    DB_HANDLE_FIELD_NUMBER: _ClassVar[int]
    conn_handle: ConnectionHandle
    db_handle: DatabaseHandle
    def __init__(self, conn_handle: _Optional[_Union[ConnectionHandle, _Mapping]] = ..., db_handle: _Optional[_Union[DatabaseHandle, _Mapping]] = ...) -> None: ...

class ConnectionInitResponse(_message.Message):
    __slots__ = ()
    def __init__(self) -> None: ...

class ConnectionReleaseRequest(_message.Message):
    __slots__ = ("conn_handle",)
    CONN_HANDLE_FIELD_NUMBER: _ClassVar[int]
    conn_handle: ConnectionHandle
    def __init__(self, conn_handle: _Optional[_Union[ConnectionHandle, _Mapping]] = ...) -> None: ...

class ConnectionReleaseResponse(_message.Message):
    __slots__ = ()
    def __init__(self) -> None: ...

class ConnectionGetInfoRequest(_message.Message):
    __slots__ = ("conn_handle", "info_codes")
    CONN_HANDLE_FIELD_NUMBER: _ClassVar[int]
    INFO_CODES_FIELD_NUMBER: _ClassVar[int]
    conn_handle: ConnectionHandle
    info_codes: _containers.RepeatedScalarFieldContainer[InfoCode]
    def __init__(self, conn_handle: _Optional[_Union[ConnectionHandle, _Mapping]] = ..., info_codes: _Optional[_Iterable[_Union[InfoCode, str]]] = ...) -> None: ...

class ConnectionGetInfoResponse(_message.Message):
    __slots__ = ("info_data",)
    INFO_DATA_FIELD_NUMBER: _ClassVar[int]
    info_data: bytes
    def __init__(self, info_data: _Optional[bytes] = ...) -> None: ...

class ConnectionGetObjectsRequest(_message.Message):
    __slots__ = ("conn_handle", "depth", "catalog", "db_schema", "table_name", "table_type", "column_name")
    CONN_HANDLE_FIELD_NUMBER: _ClassVar[int]
    DEPTH_FIELD_NUMBER: _ClassVar[int]
    CATALOG_FIELD_NUMBER: _ClassVar[int]
    DB_SCHEMA_FIELD_NUMBER: _ClassVar[int]
    TABLE_NAME_FIELD_NUMBER: _ClassVar[int]
    TABLE_TYPE_FIELD_NUMBER: _ClassVar[int]
    COLUMN_NAME_FIELD_NUMBER: _ClassVar[int]
    conn_handle: ConnectionHandle
    depth: int
    catalog: str
    db_schema: str
    table_name: str
    table_type: _containers.RepeatedScalarFieldContainer[str]
    column_name: str
    def __init__(self, conn_handle: _Optional[_Union[ConnectionHandle, _Mapping]] = ..., depth: _Optional[int] = ..., catalog: _Optional[str] = ..., db_schema: _Optional[str] = ..., table_name: _Optional[str] = ..., table_type: _Optional[_Iterable[str]] = ..., column_name: _Optional[str] = ...) -> None: ...

class ConnectionGetObjectsResponse(_message.Message):
    __slots__ = ("objects_data",)
    OBJECTS_DATA_FIELD_NUMBER: _ClassVar[int]
    objects_data: bytes
    def __init__(self, objects_data: _Optional[bytes] = ...) -> None: ...

class ConnectionGetTableSchemaRequest(_message.Message):
    __slots__ = ("conn_handle", "catalog", "db_schema", "table_name")
    CONN_HANDLE_FIELD_NUMBER: _ClassVar[int]
    CATALOG_FIELD_NUMBER: _ClassVar[int]
    DB_SCHEMA_FIELD_NUMBER: _ClassVar[int]
    TABLE_NAME_FIELD_NUMBER: _ClassVar[int]
    conn_handle: ConnectionHandle
    catalog: str
    db_schema: str
    table_name: str
    def __init__(self, conn_handle: _Optional[_Union[ConnectionHandle, _Mapping]] = ..., catalog: _Optional[str] = ..., db_schema: _Optional[str] = ..., table_name: _Optional[str] = ...) -> None: ...

class ConnectionGetTableSchemaResponse(_message.Message):
    __slots__ = ("schema_data",)
    SCHEMA_DATA_FIELD_NUMBER: _ClassVar[int]
    schema_data: bytes
    def __init__(self, schema_data: _Optional[bytes] = ...) -> None: ...

class ConnectionGetTableTypesRequest(_message.Message):
    __slots__ = ("conn_handle",)
    CONN_HANDLE_FIELD_NUMBER: _ClassVar[int]
    conn_handle: ConnectionHandle
    def __init__(self, conn_handle: _Optional[_Union[ConnectionHandle, _Mapping]] = ...) -> None: ...

class ConnectionGetTableTypesResponse(_message.Message):
    __slots__ = ("table_types_data",)
    TABLE_TYPES_DATA_FIELD_NUMBER: _ClassVar[int]
    table_types_data: bytes
    def __init__(self, table_types_data: _Optional[bytes] = ...) -> None: ...

class ConnectionCommitRequest(_message.Message):
    __slots__ = ("conn_handle",)
    CONN_HANDLE_FIELD_NUMBER: _ClassVar[int]
    conn_handle: ConnectionHandle
    def __init__(self, conn_handle: _Optional[_Union[ConnectionHandle, _Mapping]] = ...) -> None: ...

class ConnectionCommitResponse(_message.Message):
    __slots__ = ()
    def __init__(self) -> None: ...

class ConnectionRollbackRequest(_message.Message):
    __slots__ = ("conn_handle",)
    CONN_HANDLE_FIELD_NUMBER: _ClassVar[int]
    conn_handle: ConnectionHandle
    def __init__(self, conn_handle: _Optional[_Union[ConnectionHandle, _Mapping]] = ...) -> None: ...

class ConnectionRollbackResponse(_message.Message):
    __slots__ = ()
    def __init__(self) -> None: ...

class ConnectionSetSessionParametersRequest(_message.Message):
    __slots__ = ("conn_handle", "parameters")
    class ParametersEntry(_message.Message):
        __slots__ = ("key", "value")
        KEY_FIELD_NUMBER: _ClassVar[int]
        VALUE_FIELD_NUMBER: _ClassVar[int]
        key: str
        value: str
        def __init__(self, key: _Optional[str] = ..., value: _Optional[str] = ...) -> None: ...
    CONN_HANDLE_FIELD_NUMBER: _ClassVar[int]
    PARAMETERS_FIELD_NUMBER: _ClassVar[int]
    conn_handle: ConnectionHandle
    parameters: _containers.ScalarMap[str, str]
    def __init__(self, conn_handle: _Optional[_Union[ConnectionHandle, _Mapping]] = ..., parameters: _Optional[_Mapping[str, str]] = ...) -> None: ...

class ConnectionSetSessionParametersResponse(_message.Message):
    __slots__ = ()
    def __init__(self) -> None: ...

class ConnectionGetParameterRequest(_message.Message):
    __slots__ = ("conn_handle", "key")
    CONN_HANDLE_FIELD_NUMBER: _ClassVar[int]
    KEY_FIELD_NUMBER: _ClassVar[int]
    conn_handle: ConnectionHandle
    key: str
    def __init__(self, conn_handle: _Optional[_Union[ConnectionHandle, _Mapping]] = ..., key: _Optional[str] = ...) -> None: ...

class ConnectionGetParameterResponse(_message.Message):
    __slots__ = ("value",)
    VALUE_FIELD_NUMBER: _ClassVar[int]
    value: str
    def __init__(self, value: _Optional[str] = ...) -> None: ...

class StatementNewRequest(_message.Message):
    __slots__ = ("conn_handle",)
    CONN_HANDLE_FIELD_NUMBER: _ClassVar[int]
    conn_handle: ConnectionHandle
    def __init__(self, conn_handle: _Optional[_Union[ConnectionHandle, _Mapping]] = ...) -> None: ...

class StatementNewResponse(_message.Message):
    __slots__ = ("stmt_handle",)
    STMT_HANDLE_FIELD_NUMBER: _ClassVar[int]
    stmt_handle: StatementHandle
    def __init__(self, stmt_handle: _Optional[_Union[StatementHandle, _Mapping]] = ...) -> None: ...

class StatementReleaseRequest(_message.Message):
    __slots__ = ("stmt_handle",)
    STMT_HANDLE_FIELD_NUMBER: _ClassVar[int]
    stmt_handle: StatementHandle
    def __init__(self, stmt_handle: _Optional[_Union[StatementHandle, _Mapping]] = ...) -> None: ...

class StatementReleaseResponse(_message.Message):
    __slots__ = ()
    def __init__(self) -> None: ...

class StatementSetSqlQueryRequest(_message.Message):
    __slots__ = ("stmt_handle", "query")
    STMT_HANDLE_FIELD_NUMBER: _ClassVar[int]
    QUERY_FIELD_NUMBER: _ClassVar[int]
    stmt_handle: StatementHandle
    query: str
    def __init__(self, stmt_handle: _Optional[_Union[StatementHandle, _Mapping]] = ..., query: _Optional[str] = ...) -> None: ...

class StatementSetSqlQueryResponse(_message.Message):
    __slots__ = ()
    def __init__(self) -> None: ...

class StatementSetSubstraitPlanRequest(_message.Message):
    __slots__ = ("stmt_handle", "plan")
    STMT_HANDLE_FIELD_NUMBER: _ClassVar[int]
    PLAN_FIELD_NUMBER: _ClassVar[int]
    stmt_handle: StatementHandle
    plan: bytes
    def __init__(self, stmt_handle: _Optional[_Union[StatementHandle, _Mapping]] = ..., plan: _Optional[bytes] = ...) -> None: ...

class StatementSetSubstraitPlanResponse(_message.Message):
    __slots__ = ()
    def __init__(self) -> None: ...

class StatementPrepareRequest(_message.Message):
    __slots__ = ("stmt_handle",)
    STMT_HANDLE_FIELD_NUMBER: _ClassVar[int]
    stmt_handle: StatementHandle
    def __init__(self, stmt_handle: _Optional[_Union[StatementHandle, _Mapping]] = ...) -> None: ...

class StatementPrepareResponse(_message.Message):
    __slots__ = ()
    def __init__(self) -> None: ...

class StatementSetOptionStringRequest(_message.Message):
    __slots__ = ("stmt_handle", "key", "value")
    STMT_HANDLE_FIELD_NUMBER: _ClassVar[int]
    KEY_FIELD_NUMBER: _ClassVar[int]
    VALUE_FIELD_NUMBER: _ClassVar[int]
    stmt_handle: StatementHandle
    key: str
    value: str
    def __init__(self, stmt_handle: _Optional[_Union[StatementHandle, _Mapping]] = ..., key: _Optional[str] = ..., value: _Optional[str] = ...) -> None: ...

class StatementSetOptionStringResponse(_message.Message):
    __slots__ = ()
    def __init__(self) -> None: ...

class StatementSetOptionBytesRequest(_message.Message):
    __slots__ = ("stmt_handle", "key", "value")
    STMT_HANDLE_FIELD_NUMBER: _ClassVar[int]
    KEY_FIELD_NUMBER: _ClassVar[int]
    VALUE_FIELD_NUMBER: _ClassVar[int]
    stmt_handle: StatementHandle
    key: str
    value: bytes
    def __init__(self, stmt_handle: _Optional[_Union[StatementHandle, _Mapping]] = ..., key: _Optional[str] = ..., value: _Optional[bytes] = ...) -> None: ...

class StatementSetOptionBytesResponse(_message.Message):
    __slots__ = ()
    def __init__(self) -> None: ...

class StatementSetOptionIntRequest(_message.Message):
    __slots__ = ("stmt_handle", "key", "value")
    STMT_HANDLE_FIELD_NUMBER: _ClassVar[int]
    KEY_FIELD_NUMBER: _ClassVar[int]
    VALUE_FIELD_NUMBER: _ClassVar[int]
    stmt_handle: StatementHandle
    key: str
    value: int
    def __init__(self, stmt_handle: _Optional[_Union[StatementHandle, _Mapping]] = ..., key: _Optional[str] = ..., value: _Optional[int] = ...) -> None: ...

class StatementSetOptionIntResponse(_message.Message):
    __slots__ = ()
    def __init__(self) -> None: ...

class StatementSetOptionDoubleRequest(_message.Message):
    __slots__ = ("stmt_handle", "key", "value")
    STMT_HANDLE_FIELD_NUMBER: _ClassVar[int]
    KEY_FIELD_NUMBER: _ClassVar[int]
    VALUE_FIELD_NUMBER: _ClassVar[int]
    stmt_handle: StatementHandle
    key: str
    value: float
    def __init__(self, stmt_handle: _Optional[_Union[StatementHandle, _Mapping]] = ..., key: _Optional[str] = ..., value: _Optional[float] = ...) -> None: ...

class StatementSetOptionDoubleResponse(_message.Message):
    __slots__ = ()
    def __init__(self) -> None: ...

class StatementGetParameterSchemaRequest(_message.Message):
    __slots__ = ("stmt_handle",)
    STMT_HANDLE_FIELD_NUMBER: _ClassVar[int]
    stmt_handle: StatementHandle
    def __init__(self, stmt_handle: _Optional[_Union[StatementHandle, _Mapping]] = ...) -> None: ...

class StatementGetParameterSchemaResponse(_message.Message):
    __slots__ = ("schema",)
    SCHEMA_FIELD_NUMBER: _ClassVar[int]
    schema: ArrowSchemaPtr
    def __init__(self, schema: _Optional[_Union[ArrowSchemaPtr, _Mapping]] = ...) -> None: ...

class StatementBindRequest(_message.Message):
    __slots__ = ("stmt_handle", "schema", "array")
    STMT_HANDLE_FIELD_NUMBER: _ClassVar[int]
    SCHEMA_FIELD_NUMBER: _ClassVar[int]
    ARRAY_FIELD_NUMBER: _ClassVar[int]
    stmt_handle: StatementHandle
    schema: ArrowSchemaPtr
    array: ArrowArrayPtr
    def __init__(self, stmt_handle: _Optional[_Union[StatementHandle, _Mapping]] = ..., schema: _Optional[_Union[ArrowSchemaPtr, _Mapping]] = ..., array: _Optional[_Union[ArrowArrayPtr, _Mapping]] = ...) -> None: ...

class StatementBindResponse(_message.Message):
    __slots__ = ()
    def __init__(self) -> None: ...

class StatementBindStreamRequest(_message.Message):
    __slots__ = ("stmt_handle", "stream")
    STMT_HANDLE_FIELD_NUMBER: _ClassVar[int]
    STREAM_FIELD_NUMBER: _ClassVar[int]
    stmt_handle: StatementHandle
    stream: bytes
    def __init__(self, stmt_handle: _Optional[_Union[StatementHandle, _Mapping]] = ..., stream: _Optional[bytes] = ...) -> None: ...

class StatementBindStreamResponse(_message.Message):
    __slots__ = ()
    def __init__(self) -> None: ...

class StatementExecuteQueryRequest(_message.Message):
    __slots__ = ("stmt_handle",)
    STMT_HANDLE_FIELD_NUMBER: _ClassVar[int]
    stmt_handle: StatementHandle
    def __init__(self, stmt_handle: _Optional[_Union[StatementHandle, _Mapping]] = ...) -> None: ...

class StatementExecuteQueryResponse(_message.Message):
    __slots__ = ("result",)
    RESULT_FIELD_NUMBER: _ClassVar[int]
    result: ExecuteResult
    def __init__(self, result: _Optional[_Union[ExecuteResult, _Mapping]] = ...) -> None: ...

class StatementExecutePartitionsRequest(_message.Message):
    __slots__ = ("stmt_handle",)
    STMT_HANDLE_FIELD_NUMBER: _ClassVar[int]
    stmt_handle: StatementHandle
    def __init__(self, stmt_handle: _Optional[_Union[StatementHandle, _Mapping]] = ...) -> None: ...

class StatementExecutePartitionsResponse(_message.Message):
    __slots__ = ("result",)
    RESULT_FIELD_NUMBER: _ClassVar[int]
    result: PartitionedResult
    def __init__(self, result: _Optional[_Union[PartitionedResult, _Mapping]] = ...) -> None: ...

class StatementReadPartitionRequest(_message.Message):
    __slots__ = ("stmt_handle", "partition_descriptor")
    STMT_HANDLE_FIELD_NUMBER: _ClassVar[int]
    PARTITION_DESCRIPTOR_FIELD_NUMBER: _ClassVar[int]
    stmt_handle: StatementHandle
    partition_descriptor: bytes
    def __init__(self, stmt_handle: _Optional[_Union[StatementHandle, _Mapping]] = ..., partition_descriptor: _Optional[bytes] = ...) -> None: ...

class StatementReadPartitionResponse(_message.Message):
    __slots__ = ("partition_stream",)
    PARTITION_STREAM_FIELD_NUMBER: _ClassVar[int]
    partition_stream: int
    def __init__(self, partition_stream: _Optional[int] = ...) -> None: ...
