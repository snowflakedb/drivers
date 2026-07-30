import tempfile

from pathlib import Path

import pytest

from tests.e2e.put_get.put_get_helper import (
    as_file_uri,
    create_temporary_stage,
    is_aws_test_account,
    list_stage_contents,
)


_GIT_REPO_NAME = "testing_setup.public.ud_test_homebrew_git_repo"


# ``ci/account_setup.sql`` is provisioned on the AWS account only for now.
pytestmark = pytest.mark.skipif(
    not is_aws_test_account(),
    reason="Git repository e2e tests require ci/account_setup.sql on the AWS test account only",
)


@pytest.fixture(scope="module")
def git_repository(connection_factory):
    """Resolves the pre-existing git repository stage.

    The repository is provisioned once per account by ci/account_setup.sql.
    """
    with connection_factory() as conn:
        with conn.cursor() as cur:
            cur.execute(f"DESCRIBE GIT REPOSITORY {_GIT_REPO_NAME.upper()}")
            if not cur.fetchone():
                pytest.fail(f"Git repository {_GIT_REPO_NAME!r} not found; run ci/account_setup.sql to provision it")
    return _GIT_REPO_NAME


def test_get_single_file_from_git_stage(connection, git_repository):
    """GET a single file from a git stage tag should download it successfully."""
    with connection.cursor() as cursor:
        with tempfile.TemporaryDirectory() as tmp_dir:
            download_dir = Path(tmp_dir)

            # When a single file is downloaded from a git stage tag
            cursor.execute(f"GET '@{git_repository}/tags/v3.6.0/README.md' 'file://{as_file_uri(download_dir)}/'")
            result = cursor.fetchone()

            # Then the file is reported as downloaded and exists on disk
            assert result[2] == "DOWNLOADED", f"Expected DOWNLOADED, got: {result[2]}"
            assert (download_dir / "README.md").exists()


def test_get_directory_from_git_stage(connection, git_repository):
    """GET a directory from a git stage tag should download all files in it."""
    expected_files = {
        "snowflake-cli.rb",
        "snowflake-cli.tmpl.rb",
        "snowcli.rb",
        "snowcli.tmpl.rb",
    }
    with connection.cursor() as cursor:
        with tempfile.TemporaryDirectory() as tmp_dir:
            download_dir = Path(tmp_dir)

            # When a directory is downloaded from a git stage tag
            cursor.execute(f"GET '@{git_repository}/tags/v3.6.0/Casks/' 'file://{as_file_uri(download_dir)}/'")
            results = cursor.fetchall()

            # Then all expected files are present on disk
            # Git stage GET reports repo-internal paths (commit/Casks/filename);
            # compare basenames like other put/get wildcard tests.
            downloaded = {Path(row[0]).name for row in results}
            assert downloaded == expected_files, f"Expected {expected_files}, got {downloaded}"
            for filename in expected_files:
                assert (download_dir / filename).exists()


def test_copy_files_from_git_stage_to_regular_stage(connection, git_repository):
    """COPY FILES from a git branch into a regular stage should succeed."""
    with connection.cursor() as cursor:
        dest_stage = create_temporary_stage(cursor, "GIT_COPY_DEST")

        # When files are copied from a git stage branch into a regular stage
        cursor.execute(f"COPY FILES INTO @{dest_stage} FROM '@{git_repository}/branches/main/'")

        # Then the destination stage is non-empty
        files = list_stage_contents(cursor, dest_stage)
        assert len(files) > 0, "Expected at least one file copied from the git stage branch"
