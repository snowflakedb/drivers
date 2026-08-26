import os
import shutil
import sys
import tempfile

from pathlib import Path

import pytest

from tests.compatibility import NEW_DRIVER_ONLY, OLD_DRIVER_ONLY
from tests.e2e.put_get.put_get_helper import (
    as_file_uri,
    create_temporary_stage,
    create_test_file,
    upload_file_to_stage,
)


def test_should_upload_file_when_source_path_contains_dotdot_segments(connection):
    with tempfile.TemporaryDirectory() as temp_dir, connection.cursor() as cursor:
        temp_path = Path(temp_dir)

        # Given A source file exists in a temporary directory
        sub_dir = temp_path / "sub"
        sub_dir.mkdir()
        create_test_file(temp_path, "dotdot_data.csv", "a,b,c\n")

        # When PUT command is executed with a source path containing dotdot segments
        dotdot_path = sub_dir / ".." / "dotdot_data.csv"  # absolute but un-normalized
        stage_name = create_temporary_stage(cursor, "TEST_PUT_DOTDOT")
        result = upload_file_to_stage(cursor, stage_name, dotdot_path, auto_compress=False, overwrite=True)

        # Then File is uploaded successfully with correct target name
        assert result[6] == "UPLOADED"
        assert result[1] == "dotdot_data.csv"


def test_should_upload_file_when_source_path_is_relative_to_working_directory(connection):
    # Create under CWD so os.path.relpath stays same-drive on Windows (temp can be
    # on another mount, which raises ValueError from relpath).
    cwd = Path.cwd()
    work_dir = Path(tempfile.mkdtemp(prefix="put_relative_py_", dir=str(cwd)))
    try:
        with connection.cursor() as cursor:
            # Given A source file exists in a temporary directory
            source_file = create_test_file(work_dir, "relative_data.csv", "a,b,c\n")

            # When PUT command is executed with a path relative to the process working directory
            relative_path = as_file_uri(Path(os.path.relpath(source_file, cwd)))
            stage_name = create_temporary_stage(cursor, "TEST_PUT_RELATIVE")
            # PUT syntax does not support ? binding for file URIs or @stage references;
            # stage_name is connector-internally generated.
            put_command = f"PUT 'file://{relative_path}' @{stage_name} AUTO_COMPRESS=FALSE OVERWRITE=TRUE"
            cursor.execute(put_command)
            result = cursor.fetchone()

            # Then File is uploaded successfully with correct target name
            assert result[6] == "UPLOADED"
            assert result[1] == "relative_data.csv"
    finally:
        shutil.rmtree(work_dir, ignore_errors=True)


@pytest.mark.skipif(sys.platform == "win32", reason="Symlinks require Unix")
def test_should_upload_file_at_symlinked_source_path(connection):
    with tempfile.TemporaryDirectory() as temp_dir, connection.cursor() as cursor:
        temp_path = Path(temp_dir)

        # Given A source file and a symlink pointing to it exist in a temporary directory
        real_file = create_test_file(temp_path, "real.csv", "a,b,c\n")
        link_path = temp_path / "link.csv"
        link_path.symlink_to(real_file)

        # When PUT command is executed with the symlink as source path
        stage_name = create_temporary_stage(cursor, "TEST_PUT_SYMLINK")
        result = upload_file_to_stage(cursor, stage_name, link_path, auto_compress=False, overwrite=True)

        # Then File is uploaded successfully
        assert result[6] == "UPLOADED"

        # New driver resolves the symlink to the target's basename (BD#81, JDBC parity).
        # Old driver uses os.path.abspath which is purely lexical and preserves the symlink name.
        if NEW_DRIVER_ONLY("BD#81"):
            assert result[1] == "real.csv"
        if OLD_DRIVER_ONLY("BD#81"):
            assert result[1] == "link.csv"


def test_should_upload_file_when_source_path_starts_with_tilde(connection):
    # Given A source file exists in a subdirectory under the home directory
    home_dir = Path.home()
    sub_dir = Path(tempfile.mkdtemp(prefix="tilde_py_", dir=str(home_dir)))
    try:
        create_test_file(sub_dir, "tilde_data.csv", "a,b,c\n")

        with connection.cursor() as cursor:
            # When PUT command is executed with a leading ~ in the source path
            stage_name = create_temporary_stage(cursor, "TEST_PUT_TILDE")
            # PUT syntax does not support ? binding for file URIs or @stage references;
            # stage_name is connector-internally generated.
            put_command = (
                f"PUT 'file://~/{sub_dir.name}/tilde_data.csv' @{stage_name} AUTO_COMPRESS=FALSE OVERWRITE=TRUE"
            )
            cursor.execute(put_command)
            result = cursor.fetchone()

        # Then File is uploaded successfully
        assert result[6] == "UPLOADED"
    finally:
        shutil.rmtree(sub_dir, ignore_errors=True)
