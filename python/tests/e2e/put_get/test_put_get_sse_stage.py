"""
PUT/GET operations on stages with server-side encryption (SNOWFLAKE_SSE).

Snowflake stages created with `encryption = (TYPE = 'SNOWFLAKE_SSE')` don't
return client-side `encryptionMaterial` in the PUT/GET response because the
server handles encryption itself.  The driver must tolerate a missing (null)
encryption material for these stages.

See: snowflake-cli  tests_integration/test_stage.py::test_stage_diff_json
"""

import tempfile
import uuid

from pathlib import Path

from tests.e2e.put_get.put_get_helper import as_file_uri, create_test_file


def _create_sse_stage(cursor, prefix: str) -> str:
    """Create a stage that uses server-side encryption only (no client-side key)."""
    stage_name = f"{prefix}_{uuid.uuid4().hex}".upper()
    cursor.execute(f"CREATE TEMPORARY STAGE IF NOT EXISTS {stage_name} ENCRYPTION = (TYPE = 'SNOWFLAKE_SSE')")
    return stage_name


def test_should_put_file_to_sse_stage(connection):
    with connection.cursor() as cursor:
        # Given Stage with server-side encryption (SNOWFLAKE_SSE)
        stage_name = _create_sse_stage(cursor, "TEST_SSE_PUT")

        with tempfile.TemporaryDirectory() as td:
            test_file = create_test_file(Path(td), "sse_test.txt", "hello sse\n")
            file_uri = as_file_uri(test_file)

            # When File is uploaded using PUT command
            cursor.execute(f"PUT 'file://{file_uri}' @{stage_name} AUTO_COMPRESS=FALSE OVERWRITE=TRUE")
            result = cursor.fetchone()

            # Then File should be uploaded successfully
            assert result is not None
            assert result[6] == "UPLOADED"


def test_should_get_file_from_sse_stage(connection):
    with connection.cursor() as cursor:
        # Given File is uploaded to stage with server-side encryption (SNOWFLAKE_SSE)
        stage_name = _create_sse_stage(cursor, "TEST_SSE_GET")

        with tempfile.TemporaryDirectory() as upload_dir:
            test_file = create_test_file(Path(upload_dir), "sse_get.txt", "get sse\n")
            file_uri = as_file_uri(test_file)

            cursor.execute(f"PUT 'file://{file_uri}' @{stage_name} AUTO_COMPRESS=FALSE OVERWRITE=TRUE")
            upload_result = cursor.fetchone()
            assert upload_result[6] == "UPLOADED"

        with tempfile.TemporaryDirectory() as download_dir:
            download_uri = as_file_uri(Path(download_dir))

            # When File is downloaded using GET command
            cursor.execute(f"GET @{stage_name}/sse_get.txt 'file://{download_uri}/'")
            get_result = cursor.fetchone()

            # Then File should be downloaded
            assert get_result is not None
            assert get_result[2] == "DOWNLOADED"

            # And Have correct content
            downloaded = Path(download_dir) / "sse_get.txt"
            assert downloaded.exists()
            assert downloaded.read_text().strip() == "get sse"


def test_should_put_file_to_sse_stage_with_directory_enabled(connection):
    with connection.cursor() as cursor:
        # Given Stage with server-side encryption and DIRECTORY enabled
        stage_name = f"TEST_SSE_DIR_{uuid.uuid4().hex}".upper()
        cursor.execute(
            f"CREATE TEMPORARY STAGE IF NOT EXISTS {stage_name} "
            f"ENCRYPTION = (TYPE = 'SNOWFLAKE_SSE') "
            f"DIRECTORY = (ENABLE = TRUE)"
        )

        with tempfile.TemporaryDirectory() as td:
            test_file = create_test_file(Path(td), "test.txt", "Initial contents\n")
            file_uri = as_file_uri(test_file)

            # When File is uploaded using PUT command
            cursor.execute(f"PUT 'file://{file_uri}' @{stage_name} AUTO_COMPRESS=FALSE OVERWRITE=TRUE")
            result = cursor.fetchone()

            # Then File should be uploaded successfully
            assert result is not None
            assert result[6] == "UPLOADED"
