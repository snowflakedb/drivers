"""GET and download streams inside a stored procedure, served by the XP host."""

from __future__ import annotations

import gzip
import os

from pathlib import Path

import pytest

from snowflake.connector.errors import NotSupportedError, OperationalError
from tests.e2e.put_get.put_get_helper import (
    as_file_uri,
    create_temporary_stage,
    create_test_file,
    upload_file_to_stage,
)


GET_STATUS = 2
FILE_NOT_EXISTS_ERRNO = 253006
CHUNKED_PAYLOAD_BYTES = 6 * 1024 * 1024


def _get(cursor, stage_location: str, target_dir: Path, options: str = ""):
    cursor.execute(f"GET @{stage_location} 'file://{as_file_uri(target_dir)}/' {options}")
    return cursor.fetchall()


def test_should_get_single_stage_file(cursor, tmp_path):
    # Given A file on a stage
    source_dir, target_dir = tmp_path / "source", tmp_path / "target"
    stage = create_temporary_stage(cursor, "TEST_SPROC_GET_SINGLE")
    source = create_test_file(source_dir, "single.csv", "a,b\n1,2\n")
    upload_file_to_stage(cursor, stage, source, auto_compress=False)

    # When The file is fetched by its full stage path
    rows = _get(cursor, f"{stage}/single.csv", target_dir)

    # Then One row reports DOWNLOADED and the bytes match
    assert [row[GET_STATUS] for row in rows] == ["DOWNLOADED"]
    assert (target_dir / "single.csv").read_bytes() == b"a,b\n1,2\n"


def test_should_get_every_file_under_a_stage_prefix(cursor, tmp_path):
    # Given Two files under one stage directory and one file outside it
    source_dir, target_dir = tmp_path / "source", tmp_path / "target"
    stage = create_temporary_stage(cursor, "TEST_SPROC_GET_PREFIX")
    for name, content in (("a.csv", "a\n"), ("b.csv", "b\n")):
        upload_file_to_stage(cursor, f"{stage}/dir", create_test_file(source_dir, name, content), auto_compress=False)
    upload_file_to_stage(
        cursor, f"{stage}/other", create_test_file(source_dir, "outside.csv", "x\n"), auto_compress=False
    )

    # When The directory prefix is fetched
    rows = _get(cursor, f"{stage}/dir", target_dir)

    # Then Both files are downloaded and the file outside the prefix is not
    assert sorted(row[GET_STATUS] for row in rows) == ["DOWNLOADED", "DOWNLOADED"]
    assert sorted(os.listdir(target_dir)) == ["a.csv", "b.csv"]
    assert (target_dir / "a.csv").read_bytes() == b"a\n"
    assert (target_dir / "b.csv").read_bytes() == b"b\n"


def test_should_create_a_missing_target_directory(cursor, tmp_path):
    # Given A file on a stage and a target directory that does not exist yet
    stage = create_temporary_stage(cursor, "TEST_SPROC_GET_MKDIR")
    source = create_test_file(tmp_path / "source", "mk.csv", "m\n")
    upload_file_to_stage(cursor, stage, source, auto_compress=False)
    target_dir = tmp_path / "nested" / "target"

    # When The file is fetched into it
    _get(cursor, f"{stage}/mk.csv", target_dir)

    # Then The directory is created and holds the file
    assert (target_dir / "mk.csv").read_bytes() == b"m\n"


def test_should_fail_get_when_no_stage_file_matches(cursor, tmp_path):
    # Given A stage with no files
    stage = create_temporary_stage(cursor, "TEST_SPROC_GET_MISSING")

    # When A path on it is fetched
    with pytest.raises(OperationalError) as excinfo:
        _get(cursor, f"{stage}/nothing_here.csv", tmp_path / "target")

    # Then The GET fails as it does against GS and nothing is written
    assert excinfo.value.errno == FILE_NOT_EXISTS_ERRNO
    assert not (tmp_path / "target").exists() or not os.listdir(tmp_path / "target")


def test_should_reject_get_with_pattern(cursor, tmp_path):
    # Given A stage with a file
    stage = create_temporary_stage(cursor, "TEST_SPROC_GET_PATTERN")
    source = create_test_file(tmp_path / "source", "p.csv", "p\n")
    upload_file_to_stage(cursor, stage, source, auto_compress=False)
    target_dir = tmp_path / "target"

    # When GET carries a PATTERN
    with pytest.raises(NotSupportedError):
        _get(cursor, stage, target_dir, "PATTERN='.*[.]csv'")

    # Then It is rejected and nothing is written
    assert not target_dir.exists() or not os.listdir(target_dir)


def test_should_stream_a_multi_chunk_file(cursor, tmp_path):
    # Given A file larger than one download chunk
    payload = os.urandom(CHUNKED_PAYLOAD_BYTES)
    stage = create_temporary_stage(cursor, "TEST_SPROC_STREAM_CHUNKED")
    source = tmp_path / "big.bin"
    source.write_bytes(payload)
    upload_file_to_stage(cursor, stage, source, auto_compress=False)

    # When It is read through download_stream in small reads
    with cursor.download_stream(f"@{stage}/big.bin") as stream:
        received = bytearray()
        while chunk := stream.read(1024 * 1024):
            received += chunk

    # Then The bytes match exactly
    assert bytes(received) == payload


def test_should_stream_a_gzip_file_decompressed(cursor, tmp_path):
    # Given A gzip file on a stage
    payload = b"col1,col2\n1,2\n3,4\n"
    stage = create_temporary_stage(cursor, "TEST_SPROC_STREAM_GZ")
    source = tmp_path / "zipped.csv.gz"
    source.write_bytes(gzip.compress(payload))
    upload_file_to_stage(cursor, stage, source, auto_compress=False)

    # When It is streamed with decompress=True
    with cursor.download_stream(f"@{stage}/zipped.csv.gz", decompress=True) as stream:
        # Then The decompressed bytes are returned
        assert stream.read() == payload


@pytest.mark.parametrize("quote", [False, True], ids=["bare", "quoted"])
def test_should_stream_a_file_named_without_the_at_prefix(cursor, tmp_path, quote):
    # Given A file on a stage
    stage = create_temporary_stage(cursor, "TEST_SPROC_STREAM_REF")
    source = create_test_file(tmp_path / "source", "ref.csv", "r\n")
    upload_file_to_stage(cursor, stage, source, auto_compress=False)
    location = f"{stage}/ref.csv"
    location = f"'@{location}'" if quote else location

    # When It is streamed with a stage reference that GS accepts without the host's help
    with cursor.download_stream(location) as stream:
        # Then The host resolves the same file as for the `@stage/file` form
        assert stream.read() == b"r\n"
