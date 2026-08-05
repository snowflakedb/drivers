import tempfile

from pathlib import Path

from .config import get_test_parameters
from .utils import repo_root


# Keep TempPrivateKeyFile alive when a real path is required (old-driver default
# JWT setup and explicit private_key_file tests). New-driver paths prefer PEM via
# get_private_key_pem_from_parameters() so core accepts the string directly.
_temp_key_files_registry = []


class TempPrivateKeyFile:
    """
    File is automatically deleted when this object is garbage collected.
    """

    def __init__(self, private_key_contents: list[str]):
        """Create temporary private key file."""
        self._temp_dir = tempfile.TemporaryDirectory(prefix="snowflake_key_")
        self._file_path = Path(self._temp_dir.name) / "private_key.p8"
        self._file_path.write_text("\n".join(private_key_contents) + "\n")

    def path(self) -> str:
        return str(self._file_path)

    def __enter__(self):
        """Context manager entry."""
        return self

    def __exit__(self, exc_type, exc_val, exc_tb):
        """Context manager exit - cleanup."""
        self.cleanup()

    def __del__(self):
        """Destructor - cleanup when garbage collected."""
        self.cleanup()

    def cleanup(self):
        """Clean up the temporary directory and file."""
        if hasattr(self, "_temp_dir") and self._temp_dir is not None:
            try:
                self._temp_dir.cleanup()
                self._temp_dir = None
            except Exception:
                pass


def get_private_key_pem_from_parameters() -> str:
    """Return private-key PEM text from test parameters.

    Prefers ``SNOWFLAKE_TEST_PRIVATE_KEY_FILE`` when set; otherwise joins
    ``SNOWFLAKE_TEST_PRIVATE_KEY_CONTENTS``. Pass the result as ``private_key``
    to the new driver — no tempfile required.

    Raises:
        RuntimeError: If neither file nor contents are configured.
    """
    test_params = get_test_parameters()

    private_key_file = test_params.get("SNOWFLAKE_TEST_PRIVATE_KEY_FILE")
    if private_key_file:
        return Path(private_key_file).read_text()

    private_key_contents = test_params.get("SNOWFLAKE_TEST_PRIVATE_KEY_CONTENTS")
    if not private_key_contents:
        raise RuntimeError("SNOWFLAKE_TEST_PRIVATE_KEY_CONTENTS not found in test parameters.")

    if isinstance(private_key_contents, list):
        return "\n".join(private_key_contents) + "\n"
    return str(private_key_contents)


def get_private_key_from_parameters() -> str:
    """Get private key file path from test parameters.

    Used by old-driver JWT defaults and tests that specifically exercise
    ``private_key_file``. Prefer :func:`get_private_key_pem_from_parameters`
    for new-driver ``private_key`` string paths.

    Returns:
        Path to the private key file (configured path or a temp file for contents)

    Raises:
        RuntimeError: If SNOWFLAKE_TEST_PRIVATE_KEY_CONTENTS not found
    """
    test_params = get_test_parameters()

    private_key_file = test_params.get("SNOWFLAKE_TEST_PRIVATE_KEY_FILE")
    if private_key_file:
        return private_key_file

    private_key_contents = test_params.get("SNOWFLAKE_TEST_PRIVATE_KEY_CONTENTS")

    if not private_key_contents:
        raise RuntimeError("SNOWFLAKE_TEST_PRIVATE_KEY_CONTENTS not found in test parameters.")

    if not isinstance(private_key_contents, list):
        private_key_contents = str(private_key_contents).splitlines()

    temp_file = TempPrivateKeyFile(private_key_contents)
    # Keep object alive so file persists until program exit
    _temp_key_files_registry.append(temp_file)
    return temp_file.path()


def get_private_key_password() -> str | None:
    test_params = get_test_parameters()
    return test_params.get("SNOWFLAKE_TEST_PRIVATE_KEY_PASSWORD")


def get_test_private_key_path() -> str:
    """Get path to the shared test private key file.

    This key is used for invalid key tests and wiremock tests.

    Returns:
        Path to invalid_rsa_key.p8 in test_data
    """
    return str(repo_root() / "tests" / "test_data" / "invalid_rsa_key.p8")
