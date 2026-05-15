import gzip
import tempfile

from pathlib import Path

from tests.e2e.put_get.put_get_helper import (
    create_temporary_stage_and_upload_file,
    get_file_from_stage,
)
from tests.utils import shared_test_data_dir


GZIP_FLG_FNAME = 0x08
GZIP_FLG_OFFSET = 3


def _read_gzip_fname(gz_bytes: bytes) -> str | None:
    """Return the original filename from the gzip header's FNAME field, or
    None when the FNAME bit on the FLG byte is not set. The driver never
    emits FEXTRA, so FNAME (when present) starts at the fixed 10-byte
    offset right after ID1/ID2/CM/FLG/MTIME[4]/XFL/OS."""
    assert len(gz_bytes) >= 10
    assert gz_bytes[0] == 0x1F and gz_bytes[1] == 0x8B
    if not gz_bytes[GZIP_FLG_OFFSET] & GZIP_FLG_FNAME:
        return None
    end = gz_bytes.index(0, 10)
    return gz_bytes[10:end].decode("latin-1")


def test_should_compress_the_file_before_uploading_to_stage_when_auto_compress_set_to_true(
    connection,
):
    uncompressed_file_path = shared_test_data_dir() / "compression" / "test_data.csv"
    uncompressed_filename = "test_data.csv"
    compressed_filename = "test_data.csv.gz"
    with connection.cursor() as cursor:
        # Given Snowflake client is logged in
        pass

        # When File is uploaded to stage with AUTO_COMPRESS set to true
        stage_name, _ = create_temporary_stage_and_upload_file(
            cursor,
            "TEST_PUT_GET_AUTO_COMPRESS_TRUE",
            uncompressed_file_path,
            auto_compress=True,
            overwrite=True,
        )

        with tempfile.TemporaryDirectory() as temp_dir:
            # Then Only compressed file should be downloaded
            download_dir = Path(temp_dir)

            get_result = get_file_from_stage(cursor, stage_name, uncompressed_filename, download_dir)

            assert get_result[2] == "DOWNLOADED"

            expected_file_path = download_dir / compressed_filename
            assert expected_file_path.exists()

            not_expected_file_path = download_dir / uncompressed_filename
            assert not not_expected_file_path.exists()

            # And Have correct content
            #
            # The gzip wire bytes faithfully reproduce the legacy Python
            # file-PUT shape (RFC 1952 §2.3.1.10, plus
            # compress_file_with_gzip + normalize_gzip_header):
            #   FLG = 0x08 (FNAME present)
            #   FNAME = `len(basename) + 2` 0x20 spaces, NUL-terminated
            #   MTIME = 0
            #   XFL = 2 (derived from Compression::best(), level 9)
            #   OS = 0xff (CPython gzip.py hardcodes b'\xff')
            # Both the legacy connector and the universal driver now emit
            # identical bytes — no OLD/NEW split is needed. The reference
            # .gz fixture (a 26-byte no-FNAME flate2-default blob) is not
            # the spec for this assertion.
            downloaded_content = expected_file_path.read_bytes()
            assert downloaded_content[GZIP_FLG_OFFSET] & GZIP_FLG_FNAME, (
                f"FLG byte 0x{downloaded_content[GZIP_FLG_OFFSET]:02x} should have FNAME (0x08) bit set"
            )
            assert _read_gzip_fname(downloaded_content) == " " * (len(uncompressed_filename) + 2)
            assert downloaded_content[4:8] == b"\x00\x00\x00\x00", "MTIME should be zeroed"
            assert downloaded_content[8] == 2, "XFL should be 2 (derived from level 9)"
            assert downloaded_content[9] == 0xFF, "OS should be 255 (CPython hardcodes b'\\xff')"

            # And the decompressed payload matches the original CSV.
            assert gzip.decompress(downloaded_content) == uncompressed_file_path.read_bytes()


def test_should_not_compress_the_file_before_uploading_to_stage_when_auto_compress_set_to_false(
    connection,
):
    uncompressed_file_path = shared_test_data_dir() / "compression" / "test_data.csv"
    uncompressed_filename = "test_data.csv"
    compressed_filename = "test_data.csv.gz"

    with connection.cursor() as cursor:
        # Given Snowflake client is logged in
        pass

        # When File is uploaded to stage with AUTO_COMPRESS set to false
        stage_name, _ = create_temporary_stage_and_upload_file(
            cursor,
            "TEST_PUT_GET_AUTO_COMPRESS_FALSE",
            uncompressed_file_path,
            auto_compress=False,
            overwrite=True,
        )

        with tempfile.TemporaryDirectory() as temp_dir:
            # Then Only uncompressed file should be downloaded
            download_dir = Path(temp_dir)
            get_result = get_file_from_stage(cursor, stage_name, uncompressed_filename, download_dir)

            assert get_result[2] == "DOWNLOADED"

            expected_file_path = download_dir / uncompressed_filename
            assert expected_file_path.exists()

            not_expected_file_path = download_dir / compressed_filename
            assert not not_expected_file_path.exists()

            # And Have correct content
            downloaded_content = expected_file_path.read_bytes()
            reference_content = uncompressed_file_path.read_bytes()
            assert downloaded_content == reference_content
