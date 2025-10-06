import tempfile
from pathlib import Path

from tests.e2e.put_get.put_get_helper import (
    get_files_with_wildcard,
    list_stage_contents,
    create_temporary_stage_and_upload_multiple_files,
    create_temporary_stage,
    upload_file_to_stage,
    create_matching_files,
    create_test_files,
)


def test_should_upload_files_that_match_wildcard_question_mark_pattern(connection):
    base_file_name = "test_put_wildcard_question_mark"

    with tempfile.TemporaryDirectory() as temp_dir:
        temp_path = Path(temp_dir)

        with connection.cursor() as cursor:

            # Given Files matching wildcard pattern
            matching_files = create_matching_files(temp_path, base_file_name)

            # And Files not matching wildcard pattern
            non_matching_files = [
                f"{base_file_name}_10.csv",
                f"{base_file_name}_abc.csv",
            ]
            create_test_files(temp_path, non_matching_files)

            # When Files are uploaded using command with question mark wildcard
            wildcard_pattern = (temp_path / f"{base_file_name}_?.csv").as_posix()
            stage_name, upload_results = (
                create_temporary_stage_and_upload_multiple_files(
                    cursor,
                    "TEST_PUT_WILDCARD_QUESTION_MARK",
                    wildcard_pattern,
                    auto_compress=False,
                    overwrite=True,
                )
            )

            # Then Files matching wildcard pattern are uploaded
            assert len(upload_results) == 5

            stage_contents = list_stage_contents(cursor, stage_name)
            uploaded_filenames = [Path(item[0]).name for item in stage_contents]

            for filename in matching_files:
                assert filename in uploaded_filenames

            # And Files not matching wildcard pattern are not uploaded
            for filename in non_matching_files:
                assert filename not in uploaded_filenames


def test_should_upload_files_that_match_wildcard_star_pattern(connection):
    base_file_name = "test_put_wildcard_star"

    with tempfile.TemporaryDirectory() as temp_dir:
        temp_path = Path(temp_dir)

        with connection.cursor() as cursor:

            # Given Files matching wildcard pattern
            matching_files = create_matching_files(temp_path, base_file_name)

            # And Files not matching wildcard pattern
            non_matching_files = [
                f"{base_file_name}.csv",
                f"{base_file_name}_test.txt",
            ]
            create_test_files(temp_path, non_matching_files)

            # When Files are uploaded using command with star wildcard
            wildcard_pattern = (temp_path / f"{base_file_name}_*.csv").as_posix()
            stage_name, upload_results = (
                create_temporary_stage_and_upload_multiple_files(
                    cursor,
                    "TEST_PUT_WILDCARD_STAR",
                    wildcard_pattern,
                    auto_compress=False,
                    overwrite=True,
                )
            )

            # Then Files matching wildcard pattern are uploaded
            assert len(upload_results) == 5

            stage_contents = list_stage_contents(cursor, stage_name)
            uploaded_filenames = [Path(item[0]).name for item in stage_contents]

            for filename in matching_files:
                assert filename in uploaded_filenames

            # And Files not matching wildcard pattern are not uploaded
            for filename in non_matching_files:
                assert filename not in uploaded_filenames


def test_should_download_files_that_are_matching_wildcard_pattern(connection):
    base_file_name = "test_get"

    with tempfile.TemporaryDirectory() as temp_dir:
        temp_path = Path(temp_dir)

        with connection.cursor() as cursor:

            # Given Files matching wildcard pattern are uploaded
            matching_files = create_matching_files(temp_path, base_file_name)
            stage_name = create_temporary_stage(cursor, "TEST_GET_WILDCARD")
            for filename in matching_files:
                file_path = temp_path / filename
                upload_file_to_stage(
                    cursor, stage_name, file_path, auto_compress=True, overwrite=True
                )

            # And Files not matching wildcard pattern are uploaded
            non_matching_files = [
                f"{base_file_name}_10.csv",
                f"{base_file_name}_abc.csv",
            ]
            create_test_files(temp_path, non_matching_files)
            for filename in non_matching_files:
                file_path = temp_path / filename
                upload_file_to_stage(
                    cursor, stage_name, file_path, auto_compress=True, overwrite=True
                )

            with tempfile.TemporaryDirectory() as download_temp_dir:
                download_dir = Path(download_temp_dir)

                # When Files are downloaded using command with wildcard
                pattern = rf".*/{base_file_name}_.\.csv\.gz"
                get_files_with_wildcard(cursor, stage_name, pattern, download_dir)

                # Then Files matching wildcard pattern are downloaded
                downloaded_files = list(download_dir.iterdir())
                assert len(downloaded_files) == 5
                downloaded_filenames = [f.name for f in downloaded_files]

                matching_files_gz = [f"{f}.gz" for f in matching_files]
                for filename in matching_files_gz:
                    assert filename in downloaded_filenames

                # And Files not matching wildcard pattern are not downloaded
                non_matching_files_gz = [f"{f}.gz" for f in non_matching_files]
                for filename in non_matching_files_gz:
                    assert filename not in downloaded_filenames
