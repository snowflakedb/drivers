"""Backward compatibility constants module.

Provides type-mapping constants and enumerations that the old
snowflake-connector-python exposed publicly.
"""

from __future__ import annotations

from collections import defaultdict
from enum import Enum, auto, unique
from typing import Any, DefaultDict, NamedTuple

from .config_manager import CONNECTIONS_FILE, CONFIG_FILE  # noqa: F401

# DBAPI type codes
DBAPI_TYPE_STRING = 0
DBAPI_TYPE_BINARY = 1
DBAPI_TYPE_NUMBER = 2
DBAPI_TYPE_TIMESTAMP = 3


class FieldType(NamedTuple):
    name: str
    dbapi_type: list[int]


FIELD_TYPES: tuple[FieldType, ...] = (
    FieldType(name="FIXED", dbapi_type=[DBAPI_TYPE_NUMBER]),
    FieldType(name="REAL", dbapi_type=[DBAPI_TYPE_NUMBER]),
    FieldType(name="TEXT", dbapi_type=[DBAPI_TYPE_STRING]),
    FieldType(name="DATE", dbapi_type=[DBAPI_TYPE_TIMESTAMP]),
    FieldType(name="TIMESTAMP", dbapi_type=[DBAPI_TYPE_TIMESTAMP]),
    FieldType(name="VARIANT", dbapi_type=[DBAPI_TYPE_BINARY]),
    FieldType(name="TIMESTAMP_LTZ", dbapi_type=[DBAPI_TYPE_TIMESTAMP]),
    FieldType(name="TIMESTAMP_TZ", dbapi_type=[DBAPI_TYPE_TIMESTAMP]),
    FieldType(name="TIMESTAMP_NTZ", dbapi_type=[DBAPI_TYPE_TIMESTAMP]),
    FieldType(name="OBJECT", dbapi_type=[DBAPI_TYPE_BINARY]),
    FieldType(name="ARRAY", dbapi_type=[DBAPI_TYPE_BINARY]),
    FieldType(name="BINARY", dbapi_type=[DBAPI_TYPE_BINARY]),
    FieldType(name="TIME", dbapi_type=[DBAPI_TYPE_TIMESTAMP]),
    FieldType(name="BOOLEAN", dbapi_type=[]),
    FieldType(name="GEOGRAPHY", dbapi_type=[DBAPI_TYPE_STRING]),
    FieldType(name="GEOMETRY", dbapi_type=[DBAPI_TYPE_STRING]),
    FieldType(name="VECTOR", dbapi_type=[DBAPI_TYPE_BINARY]),
    FieldType(name="MAP", dbapi_type=[DBAPI_TYPE_BINARY]),
    FieldType(name="FILE", dbapi_type=[DBAPI_TYPE_STRING]),
    FieldType(name="INTERVAL_YEAR_MONTH", dbapi_type=[DBAPI_TYPE_NUMBER]),
    FieldType(name="INTERVAL_DAY_TIME", dbapi_type=[DBAPI_TYPE_NUMBER]),
)

FIELD_NAME_TO_ID: DefaultDict[Any, int] = defaultdict(int)
FIELD_ID_TO_NAME: DefaultDict[int, str] = defaultdict(str)

for _idx, _field_type in enumerate(FIELD_TYPES):
    FIELD_ID_TO_NAME[_idx] = _field_type.name
    FIELD_NAME_TO_ID[_field_type.name] = _idx


@unique
class ResultStatus(Enum):
    ERROR = "ERROR"
    SUCCEEDED = "SUCCEEDED"
    UPLOADED = "UPLOADED"
    DOWNLOADED = "DOWNLOADED"
    COLLISION = "COLLISION"
    SKIPPED = "SKIPPED"
    RENEW_TOKEN = "RENEW_TOKEN"
    RENEW_PRESIGNED_URL = "RENEW_PRESIGNED_URL"
    NOT_FOUND_FILE = "NOT_FOUND_FILE"
    NEED_RETRY = "NEED_RETRY"
    NEED_RETRY_WITH_LOWER_CONCURRENCY = "NEED_RETRY_WITH_LOWER_CONCURRENCY"


@unique
class QueryStatus(Enum):
    RUNNING = 0
    ABORTING = 1
    SUCCESS = 2
    FAILED_WITH_ERROR = 3
    ABORTED = 4
    QUEUED = 5
    FAILED_WITH_INCIDENT = 6
    DISCONNECTED = 7
    RESUMING_WAREHOUSE = 8
    QUEUED_REPARING_WAREHOUSE = 9
    RESTARTED = 10
    BLOCKED = 11
    NO_DATA = 12


@unique
class OCSPMode(Enum):
    FAIL_CLOSED = "FAIL_CLOSED"
    FAIL_OPEN = "FAIL_OPEN"
    INSECURE = "INSECURE"
    DISABLE_OCSP_CHECKS = "DISABLE_OCSP_CHECKS"


@unique
class FileTransferType(Enum):
    PUT = auto()
    GET = auto()


@unique
class IterUnit(Enum):
    ROW_UNIT = "row"
    TABLE_UNIT = "table"


class SnowflakeS3FileEncryptionMaterial(NamedTuple):
    query_id: str
    query_stage_master_key: str
    smk_id: int


class MaterialDescriptor(NamedTuple):
    smk_id: int
    query_id: str
    key_size: int


class EncryptionMetadata(NamedTuple):
    key: str
    iv: str
    matdesc: str


class FileHeader(NamedTuple):
    digest: str | None
    content_length: int | None
    encryption_metadata: EncryptionMetadata | None


# HTTP header constants
HTTP_HEADER_CONTENT_TYPE = "Content-Type"
HTTP_HEADER_CONTENT_ENCODING = "Content-Encoding"
HTTP_HEADER_ACCEPT_ENCODING = "Accept-Encoding"
HTTP_HEADER_ACCEPT = "accept"
HTTP_HEADER_USER_AGENT = "User-Agent"
HTTP_HEADER_SERVICE_NAME = "X-Snowflake-Service"
HTTP_HEADER_VALUE_OCTET_STREAM = "application/octet-stream"

# Parameter constants
PARAMETER_AUTOCOMMIT = "AUTOCOMMIT"
PARAMETER_CLIENT_SESSION_KEEP_ALIVE_HEARTBEAT_FREQUENCY = (
    "CLIENT_SESSION_KEEP_ALIVE_HEARTBEAT_FREQUENCY"
)
PARAMETER_CLIENT_SESSION_KEEP_ALIVE = "CLIENT_SESSION_KEEP_ALIVE"
PARAMETER_CLIENT_PREFETCH_THREADS = "CLIENT_PREFETCH_THREADS"
PARAMETER_CLIENT_TELEMETRY_ENABLED = "CLIENT_TELEMETRY_ENABLED"
PARAMETER_CLIENT_TELEMETRY_OOB_ENABLED = "CLIENT_OUT_OF_BAND_TELEMETRY_ENABLED"
PARAMETER_CLIENT_STORE_TEMPORARY_CREDENTIAL = "CLIENT_STORE_TEMPORARY_CREDENTIAL"
PARAMETER_CLIENT_REQUEST_MFA_TOKEN = "CLIENT_REQUEST_MFA_TOKEN"
PARAMETER_QUERY_CONTEXT_CACHE_SIZE = "QUERY_CONTEXT_CACHE_SIZE"
PARAMETER_TIMEZONE = "TIMEZONE"
PARAMETER_SERVICE_NAME = "SERVICE_NAME"
PARAMETER_CLIENT_VALIDATE_DEFAULT_PARAMETERS = "CLIENT_VALIDATE_DEFAULT_PARAMETERS"
PARAMETER_PYTHON_CONNECTOR_QUERY_RESULT_FORMAT = "PYTHON_CONNECTOR_QUERY_RESULT_FORMAT"
PARAMETER_MULTI_STATEMENT_COUNT = "MULTI_STATEMENT_COUNT"

# PUT/GET related
S3_FS = "S3"
AZURE_FS = "AZURE"
GCS_FS = "GCS"
LOCAL_FS = "LOCAL_FS"
CMD_TYPE_UPLOAD = "UPLOAD"
CMD_TYPE_DOWNLOAD = "DOWNLOAD"
FILE_PROTOCOL = "file://"

# String literals
UTF8 = "utf-8"
SHA256_DIGEST = "sha256_digest"

# Size constants
kilobyte = 1024
megabyte = kilobyte * 1024
gigabyte = megabyte * 1024

# Log format
LOG_FORMAT = (
    "%(asctime)s - %(filename)s:%(lineno)d - "
    "%(funcName)s() - %(levelname)s - %(message)s"
)

DAY_IN_SECONDS = 60 * 60 * 24

ENV_VAR_PARTNER = "SF_PARTNER"
ENV_VAR_TEST_MODE = "SNOWFLAKE_TEST_MODE"
