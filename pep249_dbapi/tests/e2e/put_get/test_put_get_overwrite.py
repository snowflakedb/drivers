from tests.e2e.put_get.put_get_helper import (
    create_temporary_stage_and_upload_file,
    upload_file_to_stage,
)
from tests.utils import shared_test_data_dir


def test_should_overwrite_file_when_overwrite_is_set_to_true(connection):
    original_file_path = (
        shared_test_data_dir() / "overwrite" / "original" / "test_data.csv"
    )
    updated_file_path = (
        shared_test_data_dir() / "overwrite" / "updated" / "test_data.csv"
    )

    with connection.cursor() as cursor:

        # Given File is uploaded to stage
        assert cursor is not None
        stage_name, _ = create_temporary_stage_and_upload_file(
            cursor,
            "TEST_PUT_GET_OVERWRITE_TRUE",
            original_file_path,
            auto_compress=False,
            overwrite=True,
        )

        # When Updated file is uploaded with OVERWRITE set to true
        updated_upload_result = upload_file_to_stage(
            cursor, stage_name, updated_file_path, auto_compress=False, overwrite=True
        )

        # Then UPLOADED status is returned
        assert updated_upload_result[6] == "UPLOADED"

        # And File was overwritten
        cursor.execute(f"SELECT $1, $2, $3 FROM @{stage_name}")
        result = cursor.fetchone()
        assert result[0] == "updated"
        assert result[1] == "test"
        assert result[2] == "data"


def test_should_not_overwrite_file_when_overwrite_is_set_to_false(connection):
    original_file_path = (
        shared_test_data_dir() / "overwrite" / "original" / "test_data.csv"
    )
    updated_file_path = (
        shared_test_data_dir() / "overwrite" / "updated" / "test_data.csv"
    )

    with connection.cursor() as cursor:

        # Given File is uploaded to stage
        stage_name, _ = create_temporary_stage_and_upload_file(
            cursor,
            "TEST_PUT_GET_OVERWRITE_FALSE",
            original_file_path,
            auto_compress=False,
            overwrite=True,
        )

        # When Updated file is uploaded with OVERWRITE set to false
        updated_upload_result = upload_file_to_stage(
            cursor, stage_name, updated_file_path, auto_compress=False, overwrite=False
        )

        # Then SKIPPED status is returned
        assert updated_upload_result[6] == "SKIPPED"

        cursor.execute(f"SELECT $1, $2, $3 FROM @{stage_name}")

        # And File was not overwritten
        result = cursor.fetchone()
        assert result[0] == "original"
        assert result[1] == "test"
        assert result[2] == "data"
