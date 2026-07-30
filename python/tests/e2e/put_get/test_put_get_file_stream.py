"""E2E tests for chunked streaming PUT via ``cursor.execute(sql, file_stream=...)``.

``file_stream`` uploads an in-memory binary stream (e.g. ``io.BytesIO``) instead
of a file on disk, via the ConnectionUploadStream begin/chunk/finish RPCs.
Exercised here against a live Snowflake account.
"""

import gzip
import io
import tempfile

from pathlib import Path

import pytest

from snowflake.connector.errors import ProgrammingError
from tests.compatibility import NEW_DRIVER_ONLY, OLD_DRIVER_ONLY
from tests.e2e.put_get.put_get_helper import (
    create_temporary_stage,
    get_file_from_stage,
    list_stage_contents,
)


def _stage_filenames(cursor, stage_name: str) -> list[str]:
    """Return the basenames of files currently on *stage_name*."""
    # LS rows have the stage-relative path in column 0 (e.g. "stage/data.csv").
    return [row[0].split("/")[-1] for row in list_stage_contents(cursor, stage_name)]


def test_should_file_stream_basic_upload_and_ls(connection):
    """PUT with file_stream uploads the bytes and LS shows the destination file."""
    payload = b"hello,stream\n1,2\n"
    dest_filename = "data.csv"

    with connection.cursor() as cursor:
        # Given A temporary stage
        stage_name = create_temporary_stage(cursor, "TEST_FILE_STREAM_LS")

        # When PUT via file_stream. AUTO_COMPRESS=FALSE keeps the destination filename exact (no .gz).
        # PUT stage-path args don't support ? bindings; neither value here is user input, so interpolation is safe.
        cursor.execute(
            f"PUT file://{dest_filename} @{stage_name} AUTO_COMPRESS=FALSE OVERWRITE=TRUE",
            file_stream=io.BytesIO(payload),
        )
        put_row = cursor.fetchone()

        # Then UPLOADED status is returned and the file appears in stage LS output
        assert put_row is not None, "PUT returned no rows"
        # Column 6 is the status column for the Python PUT-result flavor.
        assert put_row[6] == "UPLOADED", f"expected UPLOADED, got {put_row[6]!r}: {put_row}"
        assert dest_filename in put_row[1], f"target {put_row[1]!r} lacks {dest_filename!r}"

        assert dest_filename in _stage_filenames(cursor, stage_name)


def test_should_file_stream_content_round_trip(connection):
    """Bytes uploaded via file_stream survive a GET unchanged."""
    payload = b"col1,col2\n" + b"x,y\n" * 5000  # a small multi-line payload
    dest_filename = "roundtrip.csv"

    with connection.cursor() as cursor:
        # Given A temporary stage
        stage_name = create_temporary_stage(cursor, "TEST_FILE_STREAM_RT")

        # When The payload is uploaded via file_stream
        # PUT stage-path args don't support ? bindings; neither value here is user input, so interpolation is safe.
        cursor.execute(
            f"PUT file://{dest_filename} @{stage_name} AUTO_COMPRESS=FALSE OVERWRITE=TRUE",
            file_stream=io.BytesIO(payload),
        )
        assert cursor.fetchone()[6] == "UPLOADED"

        # Then The same bytes are returned by a subsequent GET
        with tempfile.TemporaryDirectory() as tmp_dir:
            download_dir = Path(tmp_dir)
            get_row = get_file_from_stage(cursor, stage_name, dest_filename, download_dir)

            assert get_row is not None, "GET returned no rows"
            assert get_row[2] == "DOWNLOADED", f"GET status: {get_row[2]!r}"

            downloaded = download_dir / dest_filename
            assert downloaded.exists(), f"missing downloaded file: {downloaded}"
            assert downloaded.read_bytes() == payload, "round-trip content mismatch"


def test_should_file_stream_auto_compress(connection):
    """file_stream with AUTO_COMPRESS=TRUE lands a real gzip-compressed file on the stage."""
    payload = b"col1,col2\n1,2\n3,4\n"
    dest_filename = "compressed.csv"

    with connection.cursor() as cursor:
        # Given A temporary stage
        stage_name = create_temporary_stage(cursor, "TEST_FILE_STREAM_GZ")

        # When PUT is executed via file_stream with AUTO_COMPRESS set to true
        # PUT stage-path args don't support ? bindings; neither value here is user input, so interpolation is safe.
        cursor.execute(
            f"PUT file://{dest_filename} @{stage_name} AUTO_COMPRESS=TRUE OVERWRITE=TRUE",
            file_stream=io.BytesIO(payload),
        )
        assert cursor.fetchone()[6] == "UPLOADED"

        # Then A compressed (.gz) file lands on the stage
        names = _stage_filenames(cursor, stage_name)
        gz_names = [n for n in names if n.endswith(".gz")]
        assert gz_names, f"expected a .gz file on stage: {names}"

        # And Its content is actually gzip-compressed, not just named ".gz"
        with tempfile.TemporaryDirectory() as tmp_dir:
            download_dir = Path(tmp_dir)
            get_row = get_file_from_stage(cursor, stage_name, gz_names[0], download_dir)

            assert get_row is not None, "GET returned no rows"
            assert get_row[2] == "DOWNLOADED", f"GET status: {get_row[2]!r}"

            downloaded = download_dir / gz_names[0]
            assert downloaded.exists(), f"missing downloaded file: {downloaded}"
            assert gzip.decompress(downloaded.read_bytes()) == payload, (
                "decompressed content does not match the original payload"
            )


def test_should_file_stream_non_put_diverges_by_driver(connection):
    """file_stream on a non-PUT statement: UD raises ProgrammingError; the reference silently ignores it (BD#43)."""
    with connection.cursor() as cursor:
        # Given a file_stream supplied alongside a non-PUT statement
        stream = io.BytesIO(b"data")
        if NEW_DRIVER_ONLY("BD#43"):
            # When the non-PUT statement is executed with file_stream
            # Then the universal driver rejects it with ProgrammingError
            with pytest.raises(ProgrammingError):
                cursor.execute("SELECT 1", file_stream=stream)
        elif OLD_DRIVER_ONLY("BD#43"):
            # When the non-PUT statement is executed with file_stream
            cursor.execute("SELECT 1", file_stream=stream)
            # Then the reference driver silently ignores file_stream and runs the SQL normally
            assert cursor.fetchone() == (1,)


@pytest.mark.skipif(OLD_DRIVER_ONLY("chunked-download-stream"), reason="new driver only")
def test_should_download_stream_round_trip(connection):
    """Uploads via file_stream, downloads via download_stream, bytes match exactly."""
    payload = b"col1,col2\n" + b"a,b\n" * 5000  # multiple chunks
    dest_filename = "dl_roundtrip.csv"

    with connection.cursor() as cursor:
        # Given A multi-chunk payload uploaded to a stage via file_stream
        stage_name = create_temporary_stage(cursor, "TEST_DOWNLOAD_STREAM_RT")

        # PUT stage-path args don't support ? bindings; neither value here is user input, so interpolation is safe.
        cursor.execute(
            f"PUT file://{dest_filename} @{stage_name} AUTO_COMPRESS=FALSE OVERWRITE=TRUE",
            file_stream=io.BytesIO(payload),
        )
        assert cursor.fetchone()[6] == "UPLOADED"

        # When The file is read back through the chunked zero-disk download stream
        with cursor.download_stream(f"@{stage_name}/{dest_filename}") as stream:
            # Then The streamed bytes match the original payload exactly
            assert stream.read() == payload, "download_stream content mismatch"


@pytest.mark.skipif(OLD_DRIVER_ONLY("chunked-download-stream"), reason="new driver only")
def test_should_download_stream_decompress(connection):
    """download_stream(decompress=True) gunzips an AUTO_COMPRESS'd stage file."""
    payload = b"col1,col2\n1,2\n3,4\n"
    dest_filename = "dl_gz.csv"

    with connection.cursor() as cursor:
        # Given An AUTO_COMPRESS'd (.gz) file uploaded to a stage via file_stream
        stage_name = create_temporary_stage(cursor, "TEST_DOWNLOAD_STREAM_GZ")

        # PUT stage-path args don't support ? bindings; neither value here is user input, so interpolation is safe.
        cursor.execute(
            f"PUT file://{dest_filename} @{stage_name} AUTO_COMPRESS=TRUE OVERWRITE=TRUE",
            file_stream=io.BytesIO(payload),
        )
        assert cursor.fetchone()[6] == "UPLOADED"

        # When The .gz file is downloaded with decompress=True (core gunzips it)
        with cursor.download_stream(f"@{stage_name}/{dest_filename}.gz", decompress=True) as stream:
            # Then The decompressed bytes match the original payload
            assert stream.read() == payload, "decompressed content mismatch"
