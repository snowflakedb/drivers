import subprocess
from pathlib import Path


def repo_root() -> Path:
    result = subprocess.run(
        ["git", "rev-parse", "--show-toplevel"],
        capture_output=True,
        text=True,
    )
    if result.returncode == 0:
        root = result.stdout.strip()
        if root:
            return Path(root)
    raise RuntimeError("Failed to determine repository root")


def shared_test_data_dir() -> Path:
    return repo_root() / "tests" / "test_data" / "generated_test_data"
