"""Unit tests for S3 listing used by subset PUT/GET downloads."""
import subprocess
from pathlib import Path
from types import SimpleNamespace
from unittest.mock import patch

import pytest

from runner.s3_utils import (
    _download_s3_prefix_subset,
    _list_s3_object_keys,
    download_s3_files,
)

S3_URL = "s3://sfc-eng-data/ecosystem/1.2Gx10/"


def test_list_s3_object_keys_returns_top_level_objects_in_list_order():
    ls_output = (
        "\n"
        "                           PRE nested/\n"
        "2024-01-01 00:00:00 1288490188 file_a.bin\n"
        "2024-01-01 00:00:00 1288490188 file with spaces.bin\n"
        "2024-01-01 00:00:00 1288490188 file_b.bin\n"
    )
    with patch("runner.s3_utils.subprocess.run") as mock_run:
        mock_run.return_value = SimpleNamespace(stdout=ls_output)
        keys = _list_s3_object_keys(S3_URL)

    assert keys == ["file_a.bin", "file with spaces.bin", "file_b.bin"]
    mock_run.assert_called_once()
    assert mock_run.call_args.args[0] == ["aws", "s3", "ls", S3_URL]


def test_download_s3_prefix_subset_copies_first_n_listed_objects(tmp_path):
    ls_output = (
        "2024-01-01 00:00:00 1288490188 file_a.bin\n"
        "2024-01-01 00:00:00 1288490188 file_b.bin\n"
        "2024-01-01 00:00:00 1288490188 file_c.bin\n"
    )
    copied = []

    def fake_run(cmd, **_kwargs):
        if cmd[:3] == ["aws", "s3", "ls"]:
            return SimpleNamespace(stdout=ls_output)
        copied.append(cmd[4])
        return SimpleNamespace(stdout="")

    with patch("runner.s3_utils.subprocess.run", side_effect=fake_run):
        _download_s3_prefix_subset(S3_URL, tmp_path, max_files=1)

    assert copied == [S3_URL.rstrip("/") + "/file_a.bin"]


def test_download_s3_files_skips_when_cached_count_matches_max_files(tmp_path):
    dest = tmp_path / "cache"
    dest.mkdir()
    (dest / "file_a.bin").write_bytes(b"x")

    with patch("runner.s3_utils.subprocess.run") as mock_run:
        count = download_s3_files(S3_URL, dest, max_files=1)

    assert count == 1
    mock_run.assert_not_called()
    assert (dest / "file_a.bin").read_bytes() == b"x"


def test_download_s3_files_redownloads_when_cached_count_mismatches_max_files(tmp_path):
    dest = tmp_path / "cache"
    dest.mkdir()
    (dest / "file_a.bin").write_bytes(b"old-a")
    (dest / "file_b.bin").write_bytes(b"old-b")
    ls_output = "2024-01-01 00:00:00 1288490188 file_a.bin\n"

    def fake_run(cmd, **_kwargs):
        if cmd[:3] == ["aws", "s3", "ls"]:
            return SimpleNamespace(stdout=ls_output)
        dest.mkdir(parents=True, exist_ok=True)
        Path(cmd[5]).write_bytes(b"new-a")
        return SimpleNamespace(stdout="")

    with patch("runner.s3_utils.subprocess.run", side_effect=fake_run):
        count = download_s3_files(S3_URL, dest, max_files=1)

    assert count == 1
    assert (dest / "file_a.bin").read_bytes() == b"new-a"
    assert not (dest / "file_b.bin").exists()


def test_download_s3_files_wipes_dir_when_subset_download_fails(tmp_path):
    dest = tmp_path / "cache"
    ls_output = (
        "2024-01-01 00:00:00 1288490188 file_a.bin\n"
        "2024-01-01 00:00:00 1288490188 file_b.bin\n"
    )

    def fake_run(cmd, **_kwargs):
        if cmd[:3] == ["aws", "s3", "ls"]:
            return SimpleNamespace(stdout=ls_output)
        dest.mkdir(parents=True, exist_ok=True)
        if cmd[4].endswith("file_a.bin"):
            Path(cmd[5]).write_bytes(b"partial")
            return SimpleNamespace(stdout="")
        raise subprocess.CalledProcessError(1, cmd, stderr="boom")

    with (
        patch("runner.s3_utils.subprocess.run", side_effect=fake_run),
        pytest.raises(RuntimeError, match="S3 download failed"),
    ):
        download_s3_files(S3_URL, dest, max_files=2)

    assert not dest.exists()
