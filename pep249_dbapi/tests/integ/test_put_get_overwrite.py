from .utils_put_get import (
    as_file_uri,
    create_temporary_stage,
    PUT_ROW_SOURCE_IDX,
    PUT_ROW_STATUS_IDX,
)
from ..utils import shared_test_data_dir


def test_put_overwrite_true(cursor):
    stage_name = create_temporary_stage(cursor, "PYTEST_STAGE_OVERWRITE_TRUE")
    filename, original = original_test_file()
    _, updated = updated_test_file()

    # Upload the file to the stage
    cursor.execute(
        f"PUT 'file://{as_file_uri(original)}' @{stage_name}"
    )

    # Verify that the file was uploaded
    row = cursor.fetchone()
    assert row[PUT_ROW_SOURCE_IDX] == filename
    assert row[PUT_ROW_STATUS_IDX] == "UPLOADED"

    # Upload again with changed content and OVERWRITE=TRUE
    cursor.execute(
        f"PUT 'file://{as_file_uri(updated)}' @{stage_name} OVERWRITE=TRUE"
    )

    # Verify that the file was uploaded
    row = cursor.fetchone()
    assert row[PUT_ROW_SOURCE_IDX] == filename
    assert row[PUT_ROW_STATUS_IDX] == "UPLOADED"

    # Verify that the content was updated
    cursor.execute(f"SELECT $1, $2, $3 FROM @{stage_name}")
    row = cursor.fetchone()
    assert row == ("updated", "test", "data")


def test_put_overwrite_false(cursor):
    stage_name = create_temporary_stage(cursor, "PYTEST_STAGE_OVERWRITE_FALSE")
    filename, original = original_test_file()
    _, updated = updated_test_file()

    # Upload the file to the stage
    cursor.execute(
        f"PUT 'file://{as_file_uri(original)}' @{stage_name}"
    )

    # Verify that the file was uploaded
    row = cursor.fetchone()
    assert row[PUT_ROW_SOURCE_IDX] == filename
    assert row[PUT_ROW_STATUS_IDX] == "UPLOADED"

    # Try to upload changed content with OVERWRITE=FALSE
    cursor.execute(
        f"PUT 'file://{as_file_uri(updated)}' @{stage_name} OVERWRITE=FALSE"
    )

    # Verify that the file was not uploaded
    row = cursor.fetchone()
    assert row[PUT_ROW_SOURCE_IDX] == filename
    assert row[PUT_ROW_STATUS_IDX] == "SKIPPED"

    # Verify original content remains
    cursor.execute(f"SELECT $1, $2, $3 FROM @{stage_name}")
    row = cursor.fetchone()
    assert row == ("original", "test", "data")


def original_test_file():
    return (
        "test_data.csv",
        shared_test_data_dir() / "overwrite" / "original" / "test_data.csv",
    )


def updated_test_file():
    return (
        "test_data.csv",
        shared_test_data_dir() / "overwrite" / "updated" / "test_data.csv",
    )
