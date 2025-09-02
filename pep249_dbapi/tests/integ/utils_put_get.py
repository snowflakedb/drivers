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


def write_text_file(dir_path: Path, filename: str, content: str) -> Path:
    dir_path.mkdir(parents=True, exist_ok=True)
    file_path = dir_path / filename
    file_path.write_text(content)
    return file_path


def write_binary_file(dir_path: Path, filename: str, data: bytes) -> Path:
    dir_path.mkdir(parents=True, exist_ok=True)
    file_path = dir_path / filename
    file_path.write_bytes(data)
    return file_path


def decompress_gzip_file(path: Path) -> str:
    with gzip.open(path, "rt", encoding="utf-8") as f:
        return f.read()


def compress_bytes(data: bytes, comp: str) -> bytes:
    comp = comp.upper()
    if comp == "GZIP":
        buf = io.BytesIO()
        with gzip.GzipFile(fileobj=buf, mode="wb") as gz:
            gz.write(data)
        return buf.getvalue()
    if comp == "BZ2":
        return bz2.compress(data)
    if comp == "DEFLATE":
        return zlib.compress(data)
    if comp == "BROTLI":
        return brotli.compress(data)
    if comp == "ZSTD":
        return zstd.ZstdCompressor().compress(data)
    pytest.skip(f"Unsupported compression type in test: {comp}")
