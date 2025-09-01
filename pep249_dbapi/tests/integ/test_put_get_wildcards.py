import tempfile
from pathlib import Path

from .utils_put_get import (
    as_file_uri,
    write_text_file,
    create_temporary_stage,
)


def test_put_ls_wildcard_question_mark(cursor):
    stage_name = create_temporary_stage(cursor, "PYTEST_STAGE_PUT_WILDCARD_QMARK")
    base = "test_put_wildcard_question_mark"

    # Create a temporary directory and write some files to it
    with tempfile.TemporaryDirectory() as tmp:
        tmpdir = Path(tmp)
        # Create 5 matching files
        for i in range(1, 6):
            write_text_file(tmpdir, f"{base}_{i}.csv", "1,2,3\n")

        # Create 2 non-matching files
        write_text_file(tmpdir, f"{base}_10.csv", "1,2,3\n")
        write_text_file(tmpdir, f"{base}_abc.csv", "1,2,3\n")

        # Create a pattern to match the files
        pattern = f"{as_file_uri(tmpdir)}/{base}_?.csv"

        # Upload the files to the stage
        cursor.execute(
            f"PUT 'file://{pattern}' @{stage_name}"
        )

        # List the files in the stage
        cursor.execute(f"LS @{stage_name}")
        rows = cursor.fetchall()
        text = "\n".join(str(r) for r in rows)

        # Verify that the matching files were uploaded and the non-matching files were not uploaded
        for i in range(1, 6):
            assert f"{base}_{i}.csv.gz" in text

        # Verify that the non-matching files were not uploaded
        assert f"{base}_10.csv.gz" not in text
        assert f"{base}_abc.csv.gz" not in text


def test_put_ls_wildcard_star(cursor):
    stage_name = create_temporary_stage(cursor, "PYTEST_STAGE_PUT_WILDCARD_STAR")
    base = "test_put_wildcard_star"

    # Create a temporary directory and write some files to it
    with tempfile.TemporaryDirectory() as tmp:
        tmpdir = Path(tmp)

        # Create 5 matching files
        for i in range(1, 6):
            write_text_file(tmpdir, f"{base}_{i}{i}{i}.csv", "1,2,3\n")

        # Create 2 non-matching files
        write_text_file(tmpdir, f"{base}.csv", "1,2,3\n")
        write_text_file(tmpdir, f"{base}_test.txt", "1,2,3\n")

        # Create a pattern to match the files
        pattern = f"{as_file_uri(tmpdir)}/{base}_*.csv"

        # Upload the files to the stage
        cursor.execute(
            f"PUT 'file://{pattern}' @{stage_name}"
        )

        # List the files in the stage
        cursor.execute(f"LS @{stage_name}")
        rows = cursor.fetchall()
        text = "\n".join(str(r) for r in rows)

        # Verify that the matching files were uploaded and the non-matching files were not uploaded
        for i in range(1, 6):
            assert f"{base}_{i}{i}{i}.csv.gz" in text
        assert f"{base}.csv.gz" not in text
        assert f"{base}_test.txt.gz" not in text


def test_put_get_regexp(cursor):
    # Note: backend handles regexp; we don't test different regexp patterns here
    stage_name = create_temporary_stage(cursor, "PYTEST_STAGE_PUT_GET_REGEXP")
    base = "data"

    # Create a temporary directory and write some files to it
    with tempfile.TemporaryDirectory() as tmp:
        tmpdir = Path(tmp)

        # Create 5 matching files and upload them to the stage
        for i in range(1, 6):
            path = write_text_file(tmpdir, f"{base}_{i}.csv", "1,2,3\n")
            cursor.execute(
                f"PUT 'file://{as_file_uri(path)}' @{stage_name}"
            )

        # Create 2 non-matching files and upload them to the stage
        nm1 = write_text_file(tmpdir, f"{base}_10.csv", "1,2,3\n")
        nm2 = write_text_file(tmpdir, f"{base}_abc.csv", "1,2,3\n")
        cursor.execute(
            f"PUT 'file://{as_file_uri(nm1)}' @{stage_name}"
        )
        cursor.execute(
            f"PUT 'file://{as_file_uri(nm2)}' @{stage_name}"
        )

        download_dir = tmpdir / "download"
        download_dir.mkdir(parents=True, exist_ok=True)

        # The last two dots escaped to match literal .csv.gz
        get_pattern = r".*/data_.\.csv\.gz"

        # Download the files from the stage using a pattern
        cursor.execute(
            f"GET @{stage_name} 'file://{as_file_uri(download_dir)}/' PATTERN='{get_pattern}'"
        )

        # Verify that the matching files were downloaded and the non-matching files were not downloaded
        for i in range(1, 6):
            expected = download_dir / f"{base}_{i}.csv.gz"
            assert expected.exists(), f"Expected file: {expected}"

        assert not (download_dir / f"{base}_10.csv.gz").exists()
        assert not (download_dir / f"{base}_abc.csv.gz").exists()
