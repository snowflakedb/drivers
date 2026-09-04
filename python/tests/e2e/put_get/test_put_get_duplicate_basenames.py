import logging
import tempfile
import time

from pathlib import Path

import pytest

from snowflake.connector.errors import OperationalError
from tests.e2e.put_get.put_get_helper import (
    as_file_uri,
    create_temporary_stage,
    create_test_file,
    list_stage_contents,
    upload_file_to_stage,
)


# TODO(SNOW-4053333): run_against_sync_and_async_connection would also cover the
# legacy async test, but the async Connection's core WARN logs are intermittently
# dropped for reasons unrelated to this warning (see the ticket), so this covers
# the sync Connection only until that's fixed.
def test_should_warn_when_get_downloads_files_that_collide_on_destination_basename(connection, caplog):
    with tempfile.TemporaryDirectory() as upload_temp_dir, connection.cursor() as cursor:
        # Given The same basename is uploaded under two different stage subdirectories
        test_file = create_test_file(Path(upload_temp_dir), "data.csv")
        stage_name = create_temporary_stage(cursor, "TEST_GET_DUPLICATE_BASENAMES")
        upload_file_to_stage(cursor, f"{stage_name}/data/1", test_file, auto_compress=True, overwrite=True)
        upload_file_to_stage(cursor, f"{stage_name}/data/2", test_file, auto_compress=True, overwrite=True)

        # And Both uploaded files are visible in a stage listing
        for _ in range(10):
            if len(list_stage_contents(cursor, stage_name)) >= 2:
                break
            time.sleep(1)
        else:
            pytest.fail(f"Files not visible in stage {stage_name} after 10 seconds")

        with tempfile.TemporaryDirectory() as download_dir:
            # When Both files are downloaded together, colliding on the destination basename
            download_uri = as_file_uri(Path(download_dir))
            with caplog.at_level(logging.WARNING):
                try:
                    cursor.execute(f"GET @{stage_name} 'file://{download_uri}/' PATTERN='.*data.csv.gz'")
                except OperationalError:
                    # Cloud storage listing can lag behind the LS check above; the
                    # warning is logged before the download loop runs, so it still
                    # fires even if the GET itself then fails transiently.
                    pass

            # Then A warning names the colliding basename
            assert any(
                "Downloading multiple files with the same name" in record.message and "data.csv.gz" in record.message
                for record in caplog.records
            ), f"Expected duplicate-basename WARNING not found in logs: {caplog.text}"
