"""
E2E tests for PUT/GET error handling.

These tests verify that the driver surfaces proper exceptions for PUT/GET
failure scenarios.
"""

import tempfile
import uuid

import pytest

from snowflake.connector.errors import OperationalError, ProgrammingError
from tests.e2e.put_get.put_get_helper import (
    create_temporary_stage,
)


def test_should_return_error_when_putting_nonexistent_local_file(connection):
    with connection.cursor() as cursor:
        # Given A stage is created
        stage_name = create_temporary_stage(cursor, "TEST_STAGE_PUT_ERR")

        # When PUT is executed with a path to a nonexistent local file
        nonexistent_path = f"/tmp/nonexistent_file_{uuid.uuid4().hex}.csv"
        put_command = f"PUT 'file://{nonexistent_path}' @{stage_name}"

        # Then An error is raised indicating the local file does not exist
        with pytest.raises(ProgrammingError) as excinfo:
            cursor.execute(put_command)
        error = excinfo.value
        assert error.errno == 253006


def test_should_return_error_when_getting_nonexistent_file_from_stage(connection):
    with connection.cursor() as cursor:
        # Given An empty stage is created
        stage_name = create_temporary_stage(cursor, "TEST_STAGE_GET_ERR")

        with tempfile.TemporaryDirectory() as temp_dir:
            # When GET is executed for a file that does not exist in stage
            nonexistent_file = f"nonexistent_file_{uuid.uuid4().hex}.csv"
            get_command = f"GET @{stage_name}/{nonexistent_file} 'file://{temp_dir}/'"

            # Then An error is raised indicating the remote file does not exist
            with pytest.raises(OperationalError) as excinfo:
                cursor.execute(get_command)
            error = excinfo.value
            assert "the file does not exist" in error.msg.lower()
            assert error.errno == 253006
