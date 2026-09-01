"""Verify that committed mirror configuration matches the generator.

Run with:
    python3 -m unittest ci/mirroring/scripts/test_generate_mirror_config.py
"""

from __future__ import annotations

import json
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parents[3]
GENERATOR = Path(__file__).with_name("generate_mirror_config.py")
GENERATOR_CONFIG = REPO_ROOT / "ci/mirroring/generator_config.json"
GENERATED_FILES_MANIFEST = Path("ci/mirroring/generated_files.json")


def generate_into(
    output_root: Path, *, check: bool = True
) -> subprocess.CompletedProcess[str]:
    return subprocess.run(
        [sys.executable, str(GENERATOR), "--config", str(GENERATOR_CONFIG)],
        cwd=output_root,
        check=check,
        capture_output=True,
        text=True,
    )


def snapshot_files(root: Path) -> dict[Path, bytes]:
    return {
        path.relative_to(root): path.read_bytes()
        for path in sorted(root.rglob("*"))
        if path.is_file()
    }


class GenerateMirrorConfigTests(unittest.TestCase):
    def test_committed_files_match_generator(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            output_root = Path(temp_dir)
            generate_into(output_root)

            generated_files = snapshot_files(output_root)
            self.assertTrue(generated_files, "Generator produced no files")
            for path, generated_content in generated_files.items():
                with self.subTest(path=path):
                    committed_path = REPO_ROOT / path
                    self.assertTrue(
                        committed_path.is_file(),
                        f"Generated file is not committed: {path}",
                    )
                    self.assertEqual(
                        committed_path.read_bytes(),
                        generated_content,
                        "Committed mirror configuration is stale. Regenerate it "
                        "with the command in ci/mirroring/mirroring.md.",
                    )

    def test_generator_is_idempotent(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            output_root = Path(temp_dir)
            generate_into(output_root)
            first_generation = snapshot_files(output_root)

            generate_into(output_root)

            self.assertEqual(first_generation, snapshot_files(output_root))

    def test_obsolete_generated_files_are_rejected(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            output_root = Path(temp_dir)
            generate_into(output_root)

            obsolete_path = Path("obsolete-generated-file")
            (output_root / obsolete_path).write_text("stale", encoding="utf-8")
            manifest_path = output_root / GENERATED_FILES_MANIFEST
            manifest = json.loads(manifest_path.read_text(encoding="utf-8"))
            manifest.append(obsolete_path.as_posix())
            manifest_path.write_text(json.dumps(manifest), encoding="utf-8")

            result = generate_into(output_root, check=False)

            self.assertNotEqual(result.returncode, 0)
            self.assertIn(obsolete_path.as_posix(), result.stderr)


if __name__ == "__main__":
    unittest.main()
