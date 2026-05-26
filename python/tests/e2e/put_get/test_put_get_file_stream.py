"""E2E tests for cursor.execute() with the ``file_stream`` keyword argument.

The ``file_stream`` kwarg allows callers to supply an in-memory binary stream
(e.g. ``io.BytesIO``) as the source for a PUT statement, bypassing the need
for a real file on disk.  These tests verify:

* The stream bytes reach the stage intact.
* The PUT result row has the expected shape (dest filename + status).
* A non-PUT SQL with ``file_stream`` raises ``ProgrammingError``.

The tests require a live Snowflake connection (e2e) and are skipped via the
normal ``connection`` fixture when credentials are absent.
"""

from __future__ import annotations

import io
import tempfile

from pathlib import Path

import pytest

from snowflake.connector.errors import ProgrammingError
from tests.compatibility import NEW_DRIVER_ONLY
from tests.e2e.put_get.put_get_helper import (
    create_temporary_stage,
    get_file_from_stage,
    list_stage_contents,
)


# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------


def _stage_file_names(rows: list) -> list[str]:
    """Extract just the filename component from LS stage rows."""
    # LS rows: (name, size, md5, last_modified)
    return [Path(row[0]).name for row in rows]


# ---------------------------------------------------------------------------
# Tests
# ---------------------------------------------------------------------------


@pytest.mark.skipif(NEW_DRIVER_ONLY("gap-4-file-stream"), reason="new driver only")
def test_file_stream_basic_upload_and_ls(connection):
    """PUT with file_stream uploads bytes; LS sees the destination filename.

    Steps:
      1. Create a temporary stage.
      2. Execute a PUT using ``file_stream=io.BytesIO(b"hello")``.
      3. Assert the PUT result row reports UPLOADED and the expected filename.
      4. Assert LS @stage shows the filename.
    """
    payload = b"hello"
    dest_filename = "data.csv"

    with connection.cursor() as cursor:
        stage_name = create_temporary_stage(cursor, "TEST_FILE_STREAM_LS")

        # When PUT is executed with an in-memory stream instead of a real file
        cursor.execute(
            f"PUT file://{dest_filename} @{stage_name} AUTO_COMPRESS=FALSE OVERWRITE=TRUE",
            file_stream=io.BytesIO(payload),
        )

        put_row = cursor.fetchone()

        # Then the PUT result row reports UPLOADED with the expected destination filename
        assert put_row is not None, "PUT returned no rows"

        # Column 6 is the status column for the Python flavor.
        status = put_row[6]
        assert status == "UPLOADED", f"Expected UPLOADED, got {status!r}; full row: {put_row}"

        # The target column (index 1) should contain the destination filename.
        target = put_row[1]
        assert dest_filename in target, (
            f"Target column {target!r} does not contain {dest_filename!r}; full row: {put_row}"
        )

        # Then the LS output also shows the uploaded filename
        ls_rows = list_stage_contents(cursor, stage_name)
        names = _stage_file_names(ls_rows)
        assert any(dest_filename in n for n in names), f"{dest_filename!r} not found in stage listing: {names}"


@pytest.mark.skipif(NEW_DRIVER_ONLY("gap-4-file-stream"), reason="new driver only")
def test_file_stream_content_round_trip(connection):
    """Bytes uploaded via file_stream survive a GET and match the original payload.

    Uploads b"hello" without compression, downloads the file and asserts the
    content is identical.
    """
    payload = b"hello"
    dest_filename = "roundtrip.csv"

    with connection.cursor() as cursor:
        stage_name = create_temporary_stage(cursor, "TEST_FILE_STREAM_RT")

        # When file_stream bytes are PUT to the stage without compression
        cursor.execute(
            f"PUT file://{dest_filename} @{stage_name} AUTO_COMPRESS=FALSE OVERWRITE=TRUE",
            file_stream=io.BytesIO(payload),
        )
        put_row = cursor.fetchone()
        assert put_row is not None and put_row[6] == "UPLOADED"

        with tempfile.TemporaryDirectory() as tmp_dir:
            download_dir = Path(tmp_dir)

            # When the file is downloaded with GET
            get_row = get_file_from_stage(cursor, stage_name, dest_filename, download_dir)

            # Then the downloaded bytes match the original payload exactly
            assert get_row is not None, "GET returned no rows"
            assert get_row[2] == "DOWNLOADED", f"GET status: {get_row[2]!r}"

            downloaded = download_dir / dest_filename
            assert downloaded.exists(), f"Downloaded file not found: {downloaded}"
            assert downloaded.read_bytes() == payload, (
                f"Content mismatch: expected {payload!r}, got {downloaded.read_bytes()!r}"
            )


@pytest.mark.skipif(NEW_DRIVER_ONLY("gap-4-file-stream"), reason="new driver only")
def test_file_stream_auto_compress(connection):
    """file_stream with AUTO_COMPRESS=TRUE stores a .gz file on the stage."""
    payload = b"col1,col2\n1,2\n3,4\n"
    dest_filename = "compressed.csv"

    with connection.cursor() as cursor:
        stage_name = create_temporary_stage(cursor, "TEST_FILE_STREAM_GZ")

        # When PUT is executed with AUTO_COMPRESS=TRUE and a file_stream
        cursor.execute(
            f"PUT file://{dest_filename} @{stage_name} AUTO_COMPRESS=TRUE OVERWRITE=TRUE",
            file_stream=io.BytesIO(payload),
        )
        put_row = cursor.fetchone()
        assert put_row is not None and put_row[6] == "UPLOADED"

        # Then the stage listing shows the file with a .gz extension
        ls_rows = list_stage_contents(cursor, stage_name)
        names = _stage_file_names(ls_rows)
        # With AUTO_COMPRESS=TRUE, the stage file should have a .gz extension.
        assert any(n.endswith(".gz") for n in names), f"Expected a .gz file in stage listing: {names}"


@pytest.mark.skipif(NEW_DRIVER_ONLY("gap-4-file-stream"), reason="new driver only")
def test_file_stream_non_put_raises(connection):
    """Supplying file_stream for a non-PUT SQL raises ProgrammingError.

    Behavioral difference from reference connector: the reference connector
    silently ignores file_stream on non-PUT SQL.  This driver raises
    ProgrammingError to help users catch misuse early.  See the behavioral
    changes log in the PR description for rationale.
    """
    with connection.cursor() as cursor:
        # When file_stream is passed to a non-PUT SQL statement
        # Then ProgrammingError is raised mentioning file_stream
        with pytest.raises(ProgrammingError, match="(?i)file_stream"):
            cursor.execute("SELECT 1", file_stream=io.BytesIO(b"data"))
