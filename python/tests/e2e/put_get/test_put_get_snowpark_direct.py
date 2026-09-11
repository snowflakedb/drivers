"""E2E smoke tests for the direct file-transfer cursor methods Snowpark calls.

``session.file.put``/``get``/``put_stream``/``get_stream`` bypass SQL and call
``cursor._upload``/``_download``/``_upload_stream``/``_download_stream`` with
paths plus an options dict. Snowpark only takes that branch inside a stored
procedure, so these tests drive the cursor methods directly against a live
account. Argument handling (path/stage normalization, option rendering) is
covered at the unit level; these just confirm each method round-trips for real.

The published connector only implements ``_upload_stream`` outside a stored
procedure. ``_upload``, ``_download``, and ``_download_stream`` are unfinished
there and fail, so those three tests skip on the reference driver.
``_upload_stream`` still runs on both.
"""

import io
import sys
import tempfile

from pathlib import Path

import pytest

from tests.e2e.put_get.put_get_helper import create_temporary_stage, list_stage_contents


PAYLOAD = b"col1,col2\nhello,world\n"

# Column offsets in the Python PUT/GET result flavors.
PUT_TARGET, PUT_STATUS = 1, 6
GET_FILE, GET_STATUS = 0, 2

# These methods synthesize SQL from an unquoted local path, matching legacy's
# own parse_file_operation. That only works because production callers are a
# stored procedure running in a Linux sandbox; a real Windows path (backslashes,
# 8.3 short names) breaks the unquoted SQL token these tests would generate.
pytestmark = pytest.mark.skipif(
    sys.platform == "win32", reason="_upload/_download only run inside a Linux stored-procedure sandbox in production"
)


def _stage_filenames(cursor, stage_name: str) -> list[str]:
    """Return the basenames of the files currently on *stage_name*."""
    return [row[0].split("/")[-1] for row in list_stage_contents(cursor, stage_name)]


@pytest.fixture
def local_file(tmp_path: Path) -> Path:
    path = tmp_path / "data.csv"
    path.write_bytes(PAYLOAD)
    return path


_SKIP_REFERENCE_UNIMPLEMENTED = (
    "The published connector only implements _upload_stream outside a stored "
    "procedure; _upload, _download, and _download_stream are unfinished."
)


@pytest.mark.skip_reference(reason=_SKIP_REFERENCE_UNIMPLEMENTED)
def test_should_upload_a_local_file_and_report_it_uploaded(connection, local_file):
    with connection.cursor() as cursor:
        stage_name = create_temporary_stage(cursor, "TEST_SNOWPARK_UPLOAD")

        # When the file is uploaded through the direct path
        cursor._upload(local_file.as_posix(), f"@{stage_name}", {"auto_compress": False, "overwrite": True})
        put_row = cursor.fetchone()

        # Then the cursor reports UPLOADED and the file is on the stage
        assert put_row[PUT_STATUS] == "UPLOADED", f"expected UPLOADED, got {put_row[PUT_STATUS]!r}: {put_row}"
        assert local_file.name in _stage_filenames(cursor, stage_name)


def test_should_upload_a_stream_to_the_named_stage_file(connection):
    with connection.cursor() as cursor:
        stage_name = create_temporary_stage(cursor, "TEST_SNOWPARK_UPLOAD_STREAM")

        # When the payload is streamed to an explicit stage file
        cursor._upload_stream(
            io.BytesIO(PAYLOAD), f"@{stage_name}/streamed.csv", {"auto_compress": False, "overwrite": True}
        )

        # Then that exact filename lands on the stage
        assert cursor.fetchone()[PUT_STATUS] == "UPLOADED"
        assert "streamed.csv" in _stage_filenames(cursor, stage_name)


@pytest.mark.skip_reference(reason=_SKIP_REFERENCE_UNIMPLEMENTED)
def test_should_download_a_stage_file_into_the_target_directory(connection, local_file):
    with connection.cursor() as cursor:
        stage_name = create_temporary_stage(cursor, "TEST_SNOWPARK_DOWNLOAD")
        cursor._upload(local_file.as_posix(), f"@{stage_name}", {"auto_compress": False, "overwrite": True})

        # When the staged file is downloaded through the direct path
        with tempfile.TemporaryDirectory() as tmp_dir:
            cursor._download(f"@{stage_name}/{local_file.name}", tmp_dir, {"parallel": 1})
            get_row = cursor.fetchone()

            # Then the bytes on disk match what was uploaded
            assert get_row[GET_STATUS] == "DOWNLOADED", f"expected DOWNLOADED, got {get_row[GET_STATUS]!r}: {get_row}"
            assert (Path(tmp_dir) / local_file.name).read_bytes() == PAYLOAD


@pytest.mark.skip_reference(reason=_SKIP_REFERENCE_UNIMPLEMENTED)
def test_should_read_a_stage_file_through_a_stream(connection, local_file):
    with connection.cursor() as cursor:
        stage_name = create_temporary_stage(cursor, "TEST_SNOWPARK_DOWNLOAD_STREAM")
        cursor._upload(local_file.as_posix(), f"@{stage_name}", {"auto_compress": False, "overwrite": True})

        # When the stage file is read back through a stream
        with cursor._download_stream(f"@{stage_name}/{local_file.name}") as stream:
            # Then the bytes match what was uploaded
            assert stream.read() == PAYLOAD
