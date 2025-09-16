import gzip
from pathlib import Path
import uuid
import io
import pytest
import bz2
import zlib
import brotli
import zstandard as zstd

from pep249_dbapi.cursor import Cursor

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


# Shared test-data helpers
def repo_root() -> Path:
    # tests live at repo_root/tests/... in this project layout
    # Walk up until we find a Cargo.toml at the root directory
    p = Path(__file__).resolve()
    for _ in range(6):
        if (p.parent / "Cargo.toml").exists():
            return p.parent
        p = p.parent
    # Fallback to cwd
    return Path.cwd()


def shared_test_data_dir() -> Path:
    return repo_root() / "tests" / "test_data"


def ensure_test_data_generated(path: Path | None = None) -> Path:
    data_dir = path or shared_test_data_dir()
    if not data_dir.exists() or not any(data_dir.iterdir()):
        raise RuntimeError(
            f"Test data not found in {data_dir}. Please run generate_put_get_test_data.py in test/tes_data/utils to create it.")

    return data_dir
