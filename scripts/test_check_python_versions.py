"""Unit tests for scripts/check_python_versions.py.

Run with:
    python3 -m unittest scripts/test_check_python_versions.py -v
"""

from __future__ import annotations

import importlib.util
import sys
import tempfile
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
MODULE_PATH = ROOT / "scripts" / "check_python_versions.py"
VERSIONS = ["3.13", "3.14", "3.14t"]


def _load():
    spec = importlib.util.spec_from_file_location("check_python_versions", MODULE_PATH)
    mod = importlib.util.module_from_spec(spec)
    assert spec.loader is not None
    sys.modules[spec.name] = mod
    spec.loader.exec_module(mod)
    return mod


def _upload_step(version: str) -> str:
    minor = version[2:].rstrip("t")
    abi = f"cp3{minor}t" if version.endswith("t") else f"cp3{minor}"
    return (
        f"      - name: Upload py{version} wheel\n"
        f"        if: contains(fromJSON(matrix.python_versions_json), '{version}')\n"
        "        uses: actions/upload-artifact@v7\n"
        "        with:\n"
        f"          name: ${{{{ matrix.artifact_id }}}}_py{version}\n"
        f"          path: python/dist/*-cp3{minor}-{abi}-*.whl\n"
    )


def _files(versions: list[str]) -> dict[str, str]:
    quoted = ", ".join(f'"{v}"' for v in versions)
    minors = sorted({v.rstrip("t") for v in versions})
    classifiers = "".join(f'    "Programming Language :: Python :: {m}",\n' for m in minors)
    return {
        "ci/test_matrix/models/python.py": (
            f'PARAMS = {{"PyVersion": [{quoted}]}}\n'
            f"_CLOUD_FOR_PY = {{v: 'aws' for v in PARAMS['PyVersion']}}\n"
            f'PR_CELLS = [{{"PyVersion": "{versions[0]}"}}]\n'
            "MERGE_QUEUE_CELLS = []\n"
            'JSON_CELLS = {"pr": []}\n'
        ),
        "ci/test_matrix/mappings/python.py": (
            f'PYTHON_PLATFORM = {{("ubuntu", "x64"): {{"wheels": {{{quoted}}}}}}}\n'
            "SDIST_PY = set()\n"
        ),
        ".github/workflows/_build-python-wheels.yml": "jobs:\n  build:\n    steps:\n"
        + "\n".join(_upload_step(v) for v in versions),
        ".github/workflows/test-python.yml": f"continue-on-error: ${{{{ matrix.py == '{versions[-1]}' }}}}\n",
        "python/pyproject.toml": (
            "[project]\n"
            f'requires-python = ">={minors[0]}"\n'
            f"classifiers = [\n{classifiers}]\n"
            + "".join(
                f"[[tool.hatch.envs.{env}.matrix]]\npython = [{quoted}]\n"
                for env in ("dev", "test", "test-pandas")
            )
            + f'[[tool.hatch.envs.test-native-arrow.matrix]]\npython = ["{versions[0]}"]\n'
            + f'[[tool.hatch.envs.packaging.matrix]]\npython = ["{versions[0]}"]\n'
        ),
        "python/README.md": (
            f"Requires Python {minors[0]}.0 (or higher).\n\n- Python {minors[0]}+\n"
            f"- Supports Python matrix: {', '.join(versions)}\n"
        ),
    }


class CheckPythonVersionsTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        cls.mod = _load()

    def setUp(self) -> None:
        tmp = tempfile.TemporaryDirectory()
        self.addCleanup(tmp.cleanup)
        self.root = Path(tmp.name)
        for rel, text in _files(VERSIONS).items():
            path = self.root / rel
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_text(text, encoding="utf-8")

    def _replace(self, rel: str, old: str, new: str) -> None:
        path = self.root / rel
        text = path.read_text(encoding="utf-8")
        self.assertIn(old, text)
        path.write_text(text.replace(old, new, 1), encoding="utf-8")

    def _errors(self) -> list[str]:
        return self.mod.check(self.root)

    def test_consistent_fixture_passes(self) -> None:
        self.assertEqual(self._errors(), [])

    def test_repository_passes(self) -> None:
        self.assertEqual(self.mod.check(ROOT), [])

    def test_version_added_only_to_pyversion(self) -> None:
        self._replace("ci/test_matrix/models/python.py", '"3.14t"]', '"3.14t", "3.15"]')

        self.assertEqual(
            self._errors(),
            [
                "ci/test_matrix/mappings/python.py wheels plus SDIST_PY: missing 3.15",
                "python/pyproject.toml [tool.hatch.envs.dev.matrix]: missing 3.15",
                "python/pyproject.toml [tool.hatch.envs.test.matrix]: missing 3.15",
                "python/pyproject.toml [tool.hatch.envs.test-pandas.matrix]: missing 3.15",
                "python/pyproject.toml 'Programming Language :: Python :: 3.N' classifiers: missing 3.15",
                "python/README.md 'Supports Python matrix:' line: missing 3.15",
            ],
        )

    def test_cloud_rotation_must_cover_pyversion(self) -> None:
        self._replace(
            "ci/test_matrix/models/python.py",
            "{v: 'aws' for v in PARAMS['PyVersion']}",
            "{'3.13': 'aws', '3.14': 'gcp', '3.12': 'azure'}",
        )

        self.assertEqual(
            self._errors(),
            [
                "ci/test_matrix/models/python.py _CLOUD_FOR_PY: missing 3.14t; "
                "has 3.12, which PyVersion does not list"
            ],
        )

    def test_cells_must_use_supported_versions(self) -> None:
        self._replace("ci/test_matrix/models/python.py", 'JSON_CELLS = {"pr": []}', 'JSON_CELLS = {"pr": [{"PyVersion": "3.10"}]}')

        self.assertEqual(
            self._errors(),
            ["ci/test_matrix/models/python.py JSON_CELLS['pr']: has 3.10, which PyVersion does not list"],
        )

    def test_upload_step_glob_must_not_match_the_free_threaded_wheel(self) -> None:
        self._replace(".github/workflows/_build-python-wheels.yml", "*-cp314-cp314-*.whl", "*cp314*.whl")

        self.assertEqual(
            self._errors(),
            [
                ".github/workflows/_build-python-wheels.yml upload step for 3.14: "
                "*cp314*.whl also matches the 3.14t wheel"
            ],
        )

    def test_upload_step_glob_must_match_its_own_wheel(self) -> None:
        self._replace(".github/workflows/_build-python-wheels.yml", "*-cp314-cp314t-*.whl", "*-cp314-cp314-*.whl")

        self.assertEqual(
            self._errors(),
            [
                ".github/workflows/_build-python-wheels.yml upload step for 3.14t: *-cp314-cp314-*.whl "
                "does not match snowflake_connector_python-1.0.0-cp314-cp314t-manylinux_2_28_x86_64.whl",
                ".github/workflows/_build-python-wheels.yml upload step for 3.14t: *-cp314-cp314-*.whl "
                "also matches the 3.14 wheel",
            ],
        )

    def test_upload_step_artifact_must_name_its_version(self) -> None:
        self._replace(".github/workflows/_build-python-wheels.yml", "}}_py3.14t", "}}_py3.14")

        self.assertEqual(
            self._errors(),
            [
                ".github/workflows/_build-python-wheels.yml upload step for 3.14t: "
                "artifact name must be ${{ matrix.artifact_id }}_py3.14t"
            ],
        )

    def test_every_built_wheel_needs_an_upload_step(self) -> None:
        self._replace(".github/workflows/_build-python-wheels.yml", "'3.13'", "'3.99'")

        self.assertIn(
            ".github/workflows/_build-python-wheels.yml upload steps: missing 3.13; "
            "has 3.99, which PyVersion does not list",
            self._errors(),
        )

    def test_subset_matrices_may_not_list_unsupported_versions(self) -> None:
        self._replace("python/pyproject.toml", '[[tool.hatch.envs.packaging.matrix]]\npython = ["3.13"]', '[[tool.hatch.envs.packaging.matrix]]\npython = ["3.12"]')

        self.assertEqual(
            self._errors(),
            ["python/pyproject.toml [tool.hatch.envs.packaging.matrix]: has 3.12, which PyVersion does not list"],
        )

    def test_requires_python_and_readme_floors_follow_the_lowest_version(self) -> None:
        self._replace("python/pyproject.toml", ">=3.13", ">=3.12")
        self._replace("python/README.md", "Python 3.13+", "Python 3.12+")

        self.assertEqual(
            self._errors(),
            [
                "python/pyproject.toml requires-python: expected >=3.13",
                "python/README.md: 'Python 3.12+' does not name the minimum version 3.13",
            ],
        )

    def test_test_workflow_literals_must_be_supported(self) -> None:
        self._replace(".github/workflows/test-python.yml", "'3.14t'", "'3.12t'")

        self.assertEqual(
            self._errors(),
            [".github/workflows/test-python.yml matrix.py literals: has 3.12t, which PyVersion does not list"],
        )


if __name__ == "__main__":
    unittest.main()
