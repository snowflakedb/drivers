"""
E2E regression test for LIST/LS result truncation on large stages.

`LIST @stage` (aka `ls`) returns results in the JSON result format. When the
result is large enough to spill past the inline first batch into external
result chunks, the driver must download and concatenate those chunks. A
regression caused only the inline batch to be returned, silently dropping the
rest — which broke snowflake-cli `git execute` (its temp-stage listing missed
files and reported "No files matched pattern").

Observed before the fix (local reg): a stage with 1000 files returns only
~677 rows from `ls`; 500 files (which fit in the inline batch) returns all 500.

The files are generated server-side via ``COPY INTO @stage ... PARTITION BY``
(one file per partition) so the test does not depend on thousands of slow
client-side PUT uploads.
"""

# Enough files that the LS result spills past the inline first batch into
# external result chunks (the inline cutoff is ~670 rows).
NUM_FILES = 1000


def test_ls_returns_all_files_for_large_stage(cursor):
    # Given a stage holding many files (more than one LS result batch)
    cursor.execute("CREATE OR REPLACE TEMPORARY STAGE test_list_large_stage")

    # When NUM_FILES files are generated server-side (one partition => one file)
    cursor.execute(
        "COPY INTO @test_list_large_stage FROM "
        f"(SELECT seq4() AS i FROM TABLE(GENERATOR(ROWCOUNT => {NUM_FILES}))) "
        "PARTITION BY (TO_VARCHAR(i)) FILE_FORMAT = (TYPE = CSV) DETAILED_OUTPUT = TRUE"
    )
    files_written = len(cursor.fetchall())
    assert files_written == NUM_FILES, f"COPY wrote {files_written} files, expected {NUM_FILES}"

    # Then LS returns every file, not just the inline first batch
    cursor.execute("LS @test_list_large_stage")
    batches = cursor.get_result_batches()
    assert batches is not None and len(batches) > 1, (
        f"LS result fit in a single batch ({len(batches) if batches else 0}); "
        "increase NUM_FILES so the result spills into external chunks"
    )
    listed = cursor.fetchall()
    assert len(listed) == files_written, (
        f"LS returned {len(listed)} files, expected {files_written} (large LIST result truncated to inline batch)"
    )
