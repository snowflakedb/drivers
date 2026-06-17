"""Integration tests for ``Cursor.is_file_transfer``.

The property reflects the *last executed* statement: it is True only after a
PUT or GET, and reverts to False after any other command. It is driven by the
server's response, so it must be exercised against a real connection.
"""

import tempfile

from pathlib import Path

from tests.integ.utils_put_get import as_file_uri, create_temporary_stage
from tests.utils import shared_test_data_dir


def test_is_file_transfer_false_before_execute(connection):
    with connection.cursor() as cursor:
        assert cursor.is_file_transfer is False


def test_is_file_transfer_true_after_put(connection):
    test_file_path = shared_test_data_dir() / "compression" / "test_data.csv"

    with connection.cursor() as cursor:
        stage_name = create_temporary_stage(cursor, "TEST_STAGE_IS_FILE_TRANSFER_PUT")
        cursor.execute(f"PUT 'file://{as_file_uri(test_file_path)}' @{stage_name}")
        assert cursor.is_file_transfer is True


def test_is_file_transfer_true_after_get(connection):
    test_file_path = shared_test_data_dir() / "compression" / "test_data.csv"

    with connection.cursor() as cursor:
        stage_name = create_temporary_stage(cursor, "TEST_STAGE_IS_FILE_TRANSFER_GET")
        cursor.execute(f"PUT 'file://{as_file_uri(test_file_path)}' @{stage_name}")
        with tempfile.TemporaryDirectory() as temp_dir:
            cursor.execute(f"GET @{stage_name}/{test_file_path.name} 'file://{as_file_uri(Path(temp_dir))}/'")
            assert cursor.is_file_transfer is True


def test_is_file_transfer_false_after_non_transfer_command(connection):
    """A PUT sets the flag; a subsequent LS / SELECT must clear it."""
    test_file_path = shared_test_data_dir() / "compression" / "test_data.csv"

    with connection.cursor() as cursor:
        stage_name = create_temporary_stage(cursor, "TEST_STAGE_IS_FILE_TRANSFER_RESET")
        cursor.execute(f"PUT 'file://{as_file_uri(test_file_path)}' @{stage_name}")
        assert cursor.is_file_transfer is True

        cursor.execute(f"LS @{stage_name}")
        assert cursor.is_file_transfer is False

        cursor.execute("SELECT 1")
        assert cursor.is_file_transfer is False
