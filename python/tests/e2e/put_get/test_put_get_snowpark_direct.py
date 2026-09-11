"""E2E tests for the direct file-transfer cursor methods Snowpark calls.

``session.file.put``/``get``/``put_stream``/``get_stream`` bypass SQL and call
``cursor._upload``/``_download``/``_upload_stream``/``_download_stream`` with
paths plus an options dict. Snowpark only takes that branch inside a stored
procedure, so these tests drive the cursor methods directly against a live
account — the same approach the legacy connector's own
``test_direct_file_operation_utils.py`` takes.

Every test runs against both drivers, so the comparison report reflects reality.
``_download`` and ``_download_stream`` fail on the reference connector, which
leaves both unimplemented outside the stored-procedure build; that puts them in
the report's "reference failed / universal passed" column instead of hiding them
behind a skip.
"""

import gzip
import io
import tempfile

from pathlib import Path

import pytest

from tests.e2e.put_get.put_get_helper import create_temporary_stage, list_stage_contents


PAYLOAD = b"col1,col2\nhello,world\n"

# Column offsets in the Python PUT/GET result flavors.
PUT_TARGET, PUT_STATUS = 1, 6
GET_FILE, GET_STATUS = 0, 2


def _stage_filenames(cursor, stage_name: str) -> list[str]:
    """Return the basenames of the files currently on *stage_name*."""
    return [row[0].split("/")[-1] for row in list_stage_contents(cursor, stage_name)]


@pytest.fixture
def local_file(tmp_path: Path) -> Path:
    path = tmp_path / "data.csv"
    path.write_bytes(PAYLOAD)
    return path


def test_should_upload_a_local_file_and_report_it_uploaded(connection, local_file):
    """_upload stages the file and leaves a PUT result row on the cursor."""
    with connection.cursor() as cursor:
        # Given a temporary stage
        stage_name = create_temporary_stage(cursor, "TEST_SNOWPARK_UPLOAD")

        # When the file is uploaded through the direct path
        cursor._upload(
            local_file.as_posix(),
            f"@{stage_name}",
            {"parallel": 4, "source_compression": "AUTO_DETECT", "auto_compress": False, "overwrite": True},
        )
        put_row = cursor.fetchone()

        # Then the cursor reports UPLOADED and the file appears on the stage
        assert put_row is not None, "PUT returned no rows"
        assert put_row[PUT_STATUS] == "UPLOADED", f"expected UPLOADED, got {put_row[PUT_STATUS]!r}: {put_row}"
        assert local_file.name in put_row[PUT_TARGET]
        assert local_file.name in _stage_filenames(cursor, stage_name)


def test_should_accept_a_pre_quoted_local_path(connection, local_file):
    """write_pandas-style callers pass an already-quoted 'file://…' literal."""
    with connection.cursor() as cursor:
        stage_name = create_temporary_stage(cursor, "TEST_SNOWPARK_UPLOAD_QUOTED")

        cursor._upload(
            f"'file://{local_file.as_posix()}'",
            f"@{stage_name}",
            {"auto_compress": False, "overwrite": True},
        )

        assert cursor.fetchone()[PUT_STATUS] == "UPLOADED"
        assert local_file.name in _stage_filenames(cursor, stage_name)


def test_should_upload_a_stream_to_the_named_stage_file(connection):
    """_upload_stream takes the destination filename from the stage location."""
    with connection.cursor() as cursor:
        stage_name = create_temporary_stage(cursor, "TEST_SNOWPARK_UPLOAD_STREAM")

        # When the payload is streamed to an explicit stage file
        cursor._upload_stream(
            io.BytesIO(PAYLOAD),
            f"@{stage_name}/streamed.csv",
            {"parallel": 4, "auto_compress": False, "overwrite": True},
        )

        # Then that exact filename lands on the stage
        assert cursor.fetchone()[PUT_STATUS] == "UPLOADED"
        assert "streamed.csv" in _stage_filenames(cursor, stage_name)


def test_should_upload_a_stream_left_at_a_non_zero_position(connection):
    """Snowpark hands over streams it has already read to the end (SNOW-4072349)."""
    stream = io.BytesIO(PAYLOAD)
    stream.seek(0, io.SEEK_END)

    with connection.cursor() as cursor:
        stage_name = create_temporary_stage(cursor, "TEST_SNOWPARK_UPLOAD_STREAM_POS")

        cursor._upload_stream(stream, f"@{stage_name}/rewound.csv", {"auto_compress": False, "overwrite": True})

        assert cursor.fetchone()[PUT_STATUS] == "UPLOADED"
        with tempfile.TemporaryDirectory() as download_dir:
            cursor._download(f"@{stage_name}/rewound.csv", download_dir, {"parallel": 1})
            assert (Path(download_dir) / "rewound.csv").read_bytes() == PAYLOAD


def test_should_download_a_stage_file_into_the_target_directory(connection, local_file):
    """_download writes the file to disk and leaves a GET result row on the cursor."""
    with connection.cursor() as cursor:
        # Given a file already on a stage
        stage_name = create_temporary_stage(cursor, "TEST_SNOWPARK_DOWNLOAD")
        cursor._upload(local_file.as_posix(), f"@{stage_name}", {"auto_compress": False, "overwrite": True})

        # When it is downloaded through the direct path
        with tempfile.TemporaryDirectory() as tmp_dir:
            target = Path(tmp_dir) / "nested" / "target"
            cursor._download(f"@{stage_name}/{local_file.name}", target.as_posix(), {"parallel": 1})
            get_row = cursor.fetchone()

            # Then the bytes match and the missing target directory was created
            assert get_row is not None, "GET returned no rows"
            assert get_row[GET_STATUS] == "DOWNLOADED", f"expected DOWNLOADED, got {get_row[GET_STATUS]!r}: {get_row}"
            assert local_file.name in get_row[GET_FILE]
            assert (target / local_file.name).read_bytes() == PAYLOAD


def test_should_read_a_stage_file_through_a_stream(connection, local_file):
    """_download_stream returns a reader over the stage file's bytes."""
    with connection.cursor() as cursor:
        stage_name = create_temporary_stage(cursor, "TEST_SNOWPARK_DOWNLOAD_STREAM")
        cursor._upload(local_file.as_posix(), f"@{stage_name}", {"auto_compress": False, "overwrite": True})

        with cursor._download_stream(f"@{stage_name}/{local_file.name}") as stream:
            assert stream.read() == PAYLOAD


def test_should_decompress_a_gzipped_stage_file_on_request(connection, tmp_path):
    """decompress=True gunzips the stage file, as session.file.get_stream documents."""
    source = tmp_path / "compressed.csv"
    source.write_bytes(PAYLOAD)

    with connection.cursor() as cursor:
        stage_name = create_temporary_stage(cursor, "TEST_SNOWPARK_DOWNLOAD_STREAM_GZ")
        # AUTO_COMPRESS gzips on upload, so the stage file gains a .gz suffix.
        cursor._upload(source.as_posix(), f"@{stage_name}", {"auto_compress": True, "overwrite": True})

        stage_path = f"@{stage_name}/{source.name}.gz"
        with cursor._download_stream(stage_path, decompress=True) as stream:
            assert stream.read() == PAYLOAD

        with cursor._download_stream(stage_path) as stream:
            assert gzip.decompress(stream.read()) == PAYLOAD

