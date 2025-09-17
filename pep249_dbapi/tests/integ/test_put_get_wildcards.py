import tempfile
from pathlib import Path

from .utils_put_get import (
    as_file_uri,
    create_temporary_stage,
    shared_test_data_dir,
)


def test_put_ls_wildcard_question_mark(cursor):
    stage_name = create_temporary_stage(cursor, "PYTEST_STAGE_PUT_WILDCARD_QMARK")
    wildcard_dir = wildcard_tests_dir()

    # Upload using ? pattern: should match pattern_1.csv and pattern_2.csv, but not pattern_10.csv or patternabc.csv
    pattern = f"{as_file_uri(wildcard_dir)}/pattern_?.csv"
    cursor.execute(
        f"PUT 'file://{pattern}' @{stage_name}"
    )

    # List the files in the stage
    cursor.execute(f"LS @{stage_name}")
    rows = cursor.fetchall()
    text = "\n".join(str(r) for r in rows)

    for name in ["pattern_1.csv.gz", "pattern_2.csv.gz"]:
        assert name in text
    for name in ["pattern_10.csv.gz", "patternabc.csv.gz"]:
        assert name not in text


def test_put_ls_wildcard_star(cursor):
    stage_name = create_temporary_stage(cursor, "PYTEST_STAGE_PUT_WILDCARD_STAR")
    wildcard_dir = wildcard_tests_dir()

    # Upload using * pattern: should match pattern_1.csv, pattern_2.csv, pattern_10.csv
    pattern = f"{as_file_uri(wildcard_dir)}/pattern_*.csv"
    cursor.execute(
        f"PUT 'file://{pattern}' @{stage_name}"
    )

    # List the files in the stage
    cursor.execute(f"LS @{stage_name}")
    rows = cursor.fetchall()
    text = "\n".join(str(r) for r in rows)

    for name in ["pattern_1.csv.gz", "pattern_2.csv.gz", "pattern_10.csv.gz"]:
        assert name in text
    assert "patternabc.csv.gz" not in text


def test_put_get_regexp(cursor):
    # Note: backend handles regexp; we don't test different regexp patterns here
    stage_name = create_temporary_stage(cursor, "PYTEST_STAGE_PUT_GET_REGEXP")
    wildcard_dir = wildcard_tests_dir()

    # Upload all wildcard files
    for fname in ["pattern_1.csv", "pattern_2.csv", "pattern_10.csv", "patternabc.csv"]:
        cursor.execute(
            f"PUT 'file://{as_file_uri(wildcard_dir / fname)}' @{stage_name}"
        )

    with tempfile.TemporaryDirectory() as tmp:
        download_dir = Path(tmp) / "download"
        download_dir.mkdir(parents=True, exist_ok=True)

        # The last two dots escaped to match literal .csv.gz
        get_pattern = r".*/pattern_.\.csv\.gz"

        # Download the files from the stage using a pattern
        cursor.execute(
            f"GET @{stage_name} 'file://{as_file_uri(download_dir)}/' PATTERN='{get_pattern}'"
        )

        # Verify that the matching files were downloaded and the non-matching files were not downloaded
        for fname in ["pattern_1.csv.gz", "pattern_2.csv.gz"]:
            expected = download_dir / fname
            assert expected.exists(), f"Expected file: {expected}"

        for fname in ["pattern_10.csv.gz", "patternabc.csv.gz"]:
            assert not (download_dir / fname).exists()


def wildcard_tests_dir():
    return shared_test_data_dir() / "wildcard"
