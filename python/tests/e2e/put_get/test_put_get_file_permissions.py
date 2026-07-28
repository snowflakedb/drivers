"""
End-to-end coverage for GET-downloaded file permissions.

With the default settings (`unsafe_file_write=False`), both the universal
driver and the reference connector must produce downloaded files with mode
0o600 (owner read/write only) on Unix.

With `unsafe_file_write=True`, both drivers must fall back to the process
umask instead of forcing 0o600 — mirroring the Python connector's behaviour
introduced in SNOW-1944208.
"""

import stat
import sys
import tempfile

from pathlib import Path

import pytest

from tests.e2e.put_get.put_get_helper import (
    create_temporary_stage_and_upload_file,
    get_file_from_stage,
)
from tests.utils import shared_test_data_dir


pytestmark = pytest.mark.skipif(
    sys.platform == "win32",
    reason="File-mode checks are Unix-only; GET file permissions are not enforced on Windows",
)


def test_get_downloaded_file_has_owner_only_permissions(connection):
    test_file_path = shared_test_data_dir() / "compression" / "test_data.csv"
    filename = test_file_path.name

    with connection.cursor() as cursor:
        stage_name, _ = create_temporary_stage_and_upload_file(
            cursor,
            "TEST_FILE_PERMS_DEFAULT",
            test_file_path,
            auto_compress=False,
            overwrite=True,
        )
        with tempfile.TemporaryDirectory() as temp_dir:
            download_dir = Path(temp_dir)
            # When a file is downloaded from a stage with default settings
            get_result = get_file_from_stage(cursor, stage_name, filename, download_dir)

            assert get_result[2] == "DOWNLOADED"
            downloaded_file = download_dir / filename
            assert downloaded_file.exists()

            # Then the downloaded file has owner-only (0600) permissions
            mode = stat.S_IMODE(downloaded_file.stat().st_mode)
            assert mode == 0o600, (
                f"Expected 0o600 (owner-only), got {oct(mode)}. "
                "GET downloads must use restrictive permissions by default."
            )


def test_get_downloaded_file_uses_umask_when_unsafe_file_write_is_true(connection_factory):
    test_file_path = shared_test_data_dir() / "compression" / "test_data.csv"
    filename = test_file_path.name

    with tempfile.TemporaryDirectory() as baseline_dir:
        baseline = Path(baseline_dir) / "baseline"
        baseline.write_text("")
        expected_mode = stat.S_IMODE(baseline.stat().st_mode)

    with connection_factory(unsafe_file_write=True) as conn:
        with conn.cursor() as cursor:
            stage_name, _ = create_temporary_stage_and_upload_file(
                cursor,
                "TEST_FILE_PERMS_UNSAFE",
                test_file_path,
                auto_compress=False,
                overwrite=True,
            )
            with tempfile.TemporaryDirectory() as temp_dir:
                download_dir = Path(temp_dir)
                # When a file is downloaded with unsafe_file_write=True
                get_result = get_file_from_stage(cursor, stage_name, filename, download_dir)

                assert get_result[2] == "DOWNLOADED"
                downloaded_file = download_dir / filename
                assert downloaded_file.exists()

                # Then the downloaded file has umask-derived permissions
                mode = stat.S_IMODE(downloaded_file.stat().st_mode)
                assert mode == expected_mode, (
                    f"Expected umask-derived {oct(expected_mode)}, got {oct(mode)} with unsafe_file_write=True"
                )
