from __future__ import annotations

import gzip
from pathlib import Path
import uuid
import io
import pytest

from pep249_dbapi.cursor import Cursor


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
        import bz2

        return bz2.compress(data)
    if comp == "DEFLATE":
        import zlib

        return zlib.compress(data)
    if comp == "BROTLI":
        try:
            import brotli  # type: ignore
        except Exception:
            pytest.skip("brotli package not available")
        return brotli.compress(data)
    if comp == "ZSTD":
        try:
            import zstandard as zstd  # type: ignore
        except Exception:
            pytest.skip("zstandard package not available")
        c = zstd.ZstdCompressor()
        return c.compress(data)
    pytest.skip(f"Unsupported compression type in test: {comp}")