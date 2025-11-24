import brotli
import pytest
import uuid
import zstandard as zstd
from pathlib import Path
from python.cursor import Cursor

GET_ROW_FILE_IDX = 0
GET_ROW_SIZE_IDX = 1
GET_ROW_STATUS_IDX = 2
GET_ROW_MESSAGE_IDX = 3

PUT_ROW_SOURCE_IDX = 0
PUT_ROW_TARGET_IDX = 1
PUT_ROW_SOURCE_SIZE_IDX = 2
PUT_ROW_TARGET_SIZE_IDX = 3
PUT_ROW_SOURCE_COMPRESSION_IDX = 4
PUT_ROW_TARGET_COMPRESSION_IDX = 5
PUT_ROW_STATUS_IDX = 6
PUT_ROW_MESSAGE_IDX = 7

LS_ROW_NAME_IDX = 0
LS_ROW_SIZE_IDX = 1
LS_ROW_MD5_IDX = 2
LS_ROW_LAST_MODIFIED_IDX = 3


def as_file_uri(p: Path) -> str:
    return p.as_posix().replace("\\", "/")


def create_temporary_stage(cursor, prefix: str) -> str:
    stage_name = f"{prefix}_{uuid.uuid4().hex}".upper()
    cursor.execute(f"CREATE TEMPORARY STAGE {stage_name}")
    return stage_name
