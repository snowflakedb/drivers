import tempfile
from pathlib import Path

from .utils_put_get import (
    as_file_uri,
    write_text_file,
    create_temporary_stage,
)


def test_put_overwrite_true(cursor):
    stage_name = create_temporary_stage(cursor, "PYTEST_STAGE_OVERWRITE_TRUE")
    filename = "test_overwrite_true.csv"

    # Create temporary CSV file
    with tempfile.TemporaryDirectory() as tmp:
        tmpdir = Path(tmp)
        original = write_text_file(tmpdir, filename, "original,data,1\n")

        # Upload the file to the stage
        cursor.execute(
            f"PUT 'file://{as_file_uri(original)}' @{stage_name}"
        )

        # Verify that the file was uploaded
        row = cursor.fetchone()
        assert row[0] == filename
        assert row[6] == "UPLOADED"

        # Upload again with changed content and OVERWRITE=TRUE
        updated = write_text_file(tmpdir, filename, "updated,data,2\n")
        cursor.execute(
            f"PUT 'file://{as_file_uri(updated)}' @{stage_name} OVERWRITE=TRUE"
        )

        # Verify that the file was uploaded
        row = cursor.fetchone()
        assert row[0] == filename
        assert row[6] == "UPLOADED"

        # Verify that the content was updated
        cursor.execute(f"SELECT $1, $2, $3 FROM @{stage_name}")
        row = cursor.fetchone()
        assert row == ("updated", "data", "2")


def test_put_overwrite_false(cursor):
    stage_name = create_temporary_stage(cursor, "PYTEST_STAGE_OVERWRITE_FALSE")
    filename = "test_overwrite_false.csv"

    # Create temporary CSV file
    with tempfile.TemporaryDirectory() as tmp:
        tmpdir = Path(tmp)
        original = write_text_file(tmpdir, filename, "original,data,1\n")

        # Upload the file to the stage
        cursor.execute(
            f"PUT 'file://{as_file_uri(original)}' @{stage_name}"
        )

        # Verify that the file was uploaded
        row = cursor.fetchone()
        assert row[0] == filename
        assert row[6] == "UPLOADED"

        # Try to upload changed content with OVERWRITE=FALSE
        updated = write_text_file(tmpdir, filename, "updated,data,2\n")
        cursor.execute(
            f"PUT 'file://{as_file_uri(updated)}' @{stage_name} OVERWRITE=FALSE"
        )

        # Verify that the file was not uploaded
        row = cursor.fetchone()
        assert row[0] == filename
        assert row[6] == "SKIPPED"

        # Verify original content remains
        cursor.execute(f"SELECT $1, $2, $3 FROM @{stage_name}")
        row = cursor.fetchone()
        assert row == ("original", "data", "1")


def test_put_overwrite_false_multiple_files_mixed_status(cursor):
    stage_name = create_temporary_stage(cursor, "PYTEST_STAGE_OVERWRITE_MIXED")
    base = "test_overwrite_mixed"

    # Create temporary CSV files
    with tempfile.TemporaryDirectory() as tmp:
        tmpdir = Path(tmp)
        f1 = f"{base}_1.csv"
        f2 = f"{base}_2.csv"
        f3 = f"{base}_3.csv"

        write_text_file(tmpdir, f1, "file1,content,1\n")
        p2 = write_text_file(tmpdir, f2, "file2,content,2\n")
        write_text_file(tmpdir, f3, "file3,content,3\n")

        # Upload file2 first so later pattern upload will skip it
        cursor.execute(
            f"PUT 'file://{as_file_uri(p2)}' @{stage_name}"
        )

        # Verify that the file was uploaded
        row = cursor.fetchone()
        assert row[0] == f2
        assert row[6] == "UPLOADED"

        # Update the second file with new content
        write_text_file(tmpdir, f2, "file2,new_content,2\n")

        # Upload wildcard with OVERWRITE=FALSE
        pattern = f"{as_file_uri(tmpdir)}/{base}_*.csv"
        cursor.execute(
            f"PUT 'file://{pattern}' @{stage_name} OVERWRITE=FALSE"
        )

        # Verify that only the second file was skipped
        rows = cursor.fetchall()
        assert len(rows) == 3

        # Sort rows by filename for easier testing
        rows.sort(key=lambda r: r[0])
        assert rows[0][0] == f1
        assert rows[0][6] == "UPLOADED"
        assert rows[1][0] == f2
        assert rows[1][6] == "SKIPPED"
        assert rows[2][0] == f3
        assert rows[2][6] == "UPLOADED"

        # Verify that all files are present in the stage and no content was changed
        cursor.execute(f"SELECT $1, $2, $3 FROM @{stage_name} ORDER BY $1")
        data = cursor.fetchall()
        assert len(data) == 3
        assert data[0] == ("file1", "content", "1")
        assert data[1] == ("file2", "content", "2")
        assert data[2] == ("file3", "content", "3")
