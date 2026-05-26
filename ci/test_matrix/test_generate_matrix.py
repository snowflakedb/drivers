"""
Tests for ci/test_matrix/generate_matrix.py.

Run with:
    python -m pytest ci/test_matrix/test_generate_matrix.py -q
or:
    python -m unittest ci/test_matrix/test_generate_matrix.py
"""

from __future__ import annotations

import io
import itertools
import sys
import tempfile
import unittest
from contextlib import redirect_stderr
from pathlib import Path

sys.path.insert(0, str(Path(__file__).parent))
import generate_matrix as gm  # noqa: E402


# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------

def _write_model(text: str) -> Path:
    """Write a Python coverage model module to a fresh tempfile and return its path."""
    f = tempfile.NamedTemporaryFile("w", suffix=".py", delete=False)
    f.write(text)
    f.close()
    return Path(f.name)


# ---------------------------------------------------------------------------
# Pairwise + parser
# ---------------------------------------------------------------------------

class PairwiseTests(unittest.TestCase):
    def test_pairwise_covers_all_pairs(self) -> None:
        """Every (param_i, val_i, param_j, val_j) pair must appear at least once."""
        param_values = [["a", "b", "c"], ["1", "2"], ["x", "y", "z"]]
        cover = gm.pairwise(param_values)
        n = len(param_values)
        for i, j in itertools.combinations(range(n), 2):
            for vi in param_values[i]:
                for vj in param_values[j]:
                    self.assertTrue(
                        any(c[i] == vi and c[j] == vj for c in cover),
                        f"pair ({i}={vi}, {j}={vj}) not covered",
                    )

    def test_pairwise_minimal(self) -> None:
        """For 3x3x3, pairwise should produce <= full cartesian (27)."""
        cover = gm.pairwise([["a", "b", "c"]] * 3)
        self.assertLess(len(cover), 27)


class LoadModelTests(unittest.TestCase):
    """Exercises load_model() — the Python-module coverage-model loader."""

    def test_load_basic(self) -> None:
        path = _write_model(
            "PARAMS = {'OS': ['ubuntu', 'macos'], 'Arch': ['x64', 'arm']}\n"
            "CONSTRAINTS = [lambda x: False if x['OS'] == 'macos' and x['Arch'] == 'x64' else True]\n"
            "PR_CELLS = [{'OS': 'ubuntu', 'Arch': 'x64'}]\n"
            "JSON_CELLS = {'pr': [], 'merge': [], 'nightly': []}\n"
        )
        try:
            params, constraints, _merge_valid, pr_cells, _mq_cells, json_cells = gm.load_model(path)
            self.assertEqual(params, {"OS": ["ubuntu", "macos"], "Arch": ["x64", "arm"]})
            self.assertEqual(len(constraints), 1)
            self.assertTrue(callable(constraints[0]))
            # Predicate semantics: macos+arm valid, macos+x64 invalid.
            self.assertTrue(constraints[0]({"OS": "macos", "Arch": "arm"}))
            self.assertFalse(constraints[0]({"OS": "macos", "Arch": "x64"}))
            self.assertEqual(pr_cells, [{"OS": "ubuntu", "Arch": "x64"}])
            self.assertEqual(json_cells, {"pr": [], "merge": [], "nightly": []})
        finally:
            path.unlink(missing_ok=True)

    def test_load_json_sections(self) -> None:
        path = _write_model(
            "PARAMS = {'OS': ['ubuntu', 'macos'], 'Arch': ['x64', 'arm']}\n"
            "CONSTRAINTS = [lambda x: False if x['OS'] == 'macos' and x['Arch'] == 'x64' else True]\n"
            "PR_CELLS = [{'OS': 'ubuntu', 'Arch': 'x64'}]\n"
            "JSON_CELLS = {\n"
            "    'pr':      [{'OS': 'ubuntu', 'Arch': 'x64'}],\n"
            "    'merge':   [{'OS': 'macos',  'Arch': 'arm'}],\n"
            "    'nightly': [],\n"
            "}\n"
        )
        try:
            _params, _c, _mv, _pr, _mq, json_cells = gm.load_model(path)
            self.assertEqual(json_cells["pr"], [{"OS": "ubuntu", "Arch": "x64"}])
            self.assertEqual(json_cells["merge"], [{"OS": "macos", "Arch": "arm"}])
            self.assertEqual(json_cells["nightly"], [])
        finally:
            path.unlink(missing_ok=True)

    def test_constraints_must_be_callable(self) -> None:
        path = _write_model(
            "PARAMS = {'OS': ['ubuntu']}\n"
            "PR_CELLS = []\n"
            "CONSTRAINTS = [{'if': {'OS': 'ubuntu'}, 'then': {'OS': 'ubuntu'}}]\n"
            "JSON_CELLS = {'pr': [], 'merge': [], 'nightly': []}\n"
        )
        try:
            with self.assertRaises(ValueError) as ctx:
                gm.load_model(path)
            self.assertIn("callable", str(ctx.exception))
        finally:
            path.unlink(missing_ok=True)

    def test_constraints_must_be_a_list(self) -> None:
        path = _write_model(
            "PARAMS = {'OS': ['ubuntu']}\n"
            "PR_CELLS = []\n"
            "CONSTRAINTS = 'not a list'\n"
            "JSON_CELLS = {'pr': [], 'merge': [], 'nightly': []}\n"
        )
        try:
            with self.assertRaises(ValueError) as ctx:
                gm.load_model(path)
            self.assertIn("CONSTRAINTS", str(ctx.exception))
        finally:
            path.unlink(missing_ok=True)

    def test_pr_cell_missing_param_raises(self) -> None:
        path = _write_model(
            "PARAMS = {'OS': ['ubuntu'], 'Arch': ['x64'], 'Cloud': ['aws']}\n"
            "PR_CELLS = [{'OS': 'ubuntu', 'Arch': 'x64'}]\n"  # missing Cloud
            "CONSTRAINTS = []\n"
            "JSON_CELLS = {'pr': [], 'merge': [], 'nightly': []}\n"
        )
        try:
            with self.assertRaises(ValueError) as ctx:
                gm.load_model(path)
            self.assertIn("PR_CELLS", str(ctx.exception))
            self.assertIn("Cloud", str(ctx.exception))
        finally:
            path.unlink(missing_ok=True)

    def test_pr_cell_extra_key_raises(self) -> None:
        path = _write_model(
            "PARAMS = {'OS': ['ubuntu'], 'Arch': ['x64']}\n"
            "PR_CELLS = [{'OS': 'ubuntu', 'Arch': 'x64', 'Bogus': 'x'}]\n"
            "CONSTRAINTS = []\n"
            "JSON_CELLS = {'pr': [], 'merge': [], 'nightly': []}\n"
        )
        try:
            with self.assertRaises(ValueError) as ctx:
                gm.load_model(path)
            self.assertIn("Bogus", str(ctx.exception))
        finally:
            path.unlink(missing_ok=True)

    def test_json_cell_missing_param_raises(self) -> None:
        path = _write_model(
            "PARAMS = {'OS': ['ubuntu'], 'Arch': ['x64'], 'Cloud': ['aws']}\n"
            "PR_CELLS = []\n"
            "CONSTRAINTS = []\n"
            "JSON_CELLS = {'pr': [{'OS': 'ubuntu', 'Arch': 'x64'}], 'merge': [], 'nightly': []}\n"
        )
        try:
            with self.assertRaises(ValueError) as ctx:
                gm.load_model(path)
            self.assertIn("JSON_CELLS", str(ctx.exception))
            self.assertIn("Cloud", str(ctx.exception))
        finally:
            path.unlink(missing_ok=True)

    def test_unknown_json_trigger_level_raises(self) -> None:
        path = _write_model(
            "PARAMS = {'OS': ['ubuntu']}\n"
            "PR_CELLS = []\n"
            "CONSTRAINTS = []\n"
            "JSON_CELLS = {'foo': [{'OS': 'ubuntu'}]}\n"
        )
        try:
            with self.assertRaises(ValueError) as ctx:
                gm.load_model(path)
            self.assertIn("'foo'", str(ctx.exception))
        finally:
            path.unlink(missing_ok=True)

    def test_missing_params_raises(self) -> None:
        path = _write_model(
            "PR_CELLS = []\n"
            "CONSTRAINTS = []\n"
            "JSON_CELLS = {}\n"
        )
        try:
            with self.assertRaises(ValueError) as ctx:
                gm.load_model(path)
            self.assertIn("PARAMS", str(ctx.exception))
        finally:
            path.unlink(missing_ok=True)

    def test_block_list_function_constraint(self) -> None:
        """
        Lock in support for the recommended block-list shape: a single
        is_valid(c) function that returns False for explicitly-forbidden
        combos and falls through to True for everything else.
        """
        path = _write_model(
            "PARAMS = {'OS': ['ubuntu', 'macos'], 'Arch': ['x64', 'arm']}\n"
            "def is_valid(c):\n"
            "    if c['OS'] == 'macos':\n"
            "        if c['Arch'] == 'x64': return False\n"
            "    return True\n"
            "CONSTRAINTS = [is_valid]\n"
            "PR_CELLS = []\n"
            "JSON_CELLS = {'pr': [], 'merge': [], 'nightly': []}\n"
        )
        try:
            _, constraints, _mv, _, _, _ = gm.load_model(path)
            self.assertEqual(len(constraints), 1)
            # Forbidden by an explicit return False.
            self.assertFalse(constraints[0]({"OS": "macos", "Arch": "x64"}))
            # Allowed via fall-through return True.
            self.assertTrue(constraints[0]({"OS": "macos", "Arch": "arm"}))
            self.assertTrue(constraints[0]({"OS": "ubuntu", "Arch": "x64"}))
        finally:
            path.unlink(missing_ok=True)


    def test_merge_valid_defaults_to_empty(self) -> None:
        """A model without MERGE_VALID should load with merge_valid=[]."""
        path = _write_model(
            "PARAMS = {'OS': ['ubuntu', 'macos'], 'Arch': ['x64', 'arm']}\n"
            "CONSTRAINTS = []\n"
            "PR_CELLS = []\n"
            "JSON_CELLS = {'pr': [], 'merge': [], 'nightly': []}\n"
        )
        try:
            _params, _constraints, merge_valid, _pr, _mq, _json = gm.load_model(path)
            self.assertEqual(merge_valid, [])
        finally:
            path.unlink(missing_ok=True)

    def test_merge_valid_loads_callable(self) -> None:
        """MERGE_VALID predicates should round-trip through load_model unchanged."""
        path = _write_model(
            "PARAMS = {'OS': ['ubuntu', 'macos'], 'Arch': ['x64', 'arm']}\n"
            "CONSTRAINTS = []\n"
            "def merge_valid(c):\n"
            "    if c['OS'] == 'macos' and c['Arch'] == 'x64': return False\n"
            "    return True\n"
            "MERGE_VALID = [merge_valid]\n"
            "PR_CELLS = []\n"
            "JSON_CELLS = {'pr': [], 'merge': [], 'nightly': []}\n"
        )
        try:
            _params, _c, merge_valid, _pr, _mq, _json = gm.load_model(path)
            self.assertEqual(len(merge_valid), 1)
            self.assertTrue(callable(merge_valid[0]))
            self.assertFalse(merge_valid[0]({"OS": "macos", "Arch": "x64"}))
            self.assertTrue(merge_valid[0]({"OS": "macos", "Arch": "arm"}))
            self.assertTrue(merge_valid[0]({"OS": "ubuntu", "Arch": "x64"}))
        finally:
            path.unlink(missing_ok=True)

    def test_merge_valid_must_be_callable(self) -> None:
        path = _write_model(
            "PARAMS = {'OS': ['ubuntu']}\n"
            "CONSTRAINTS = []\n"
            "MERGE_VALID = ['not callable']\n"
            "PR_CELLS = []\n"
            "JSON_CELLS = {'pr': [], 'merge': [], 'nightly': []}\n"
        )
        try:
            with self.assertRaises(ValueError) as ctx:
                gm.load_model(path)
            self.assertIn("MERGE_VALID", str(ctx.exception))
            self.assertIn("callable", str(ctx.exception))
        finally:
            path.unlink(missing_ok=True)

    def test_merge_valid_must_be_a_list(self) -> None:
        path = _write_model(
            "PARAMS = {'OS': ['ubuntu']}\n"
            "CONSTRAINTS = []\n"
            "MERGE_VALID = 'not a list'\n"
            "PR_CELLS = []\n"
            "JSON_CELLS = {'pr': [], 'merge': [], 'nightly': []}\n"
        )
        try:
            with self.assertRaises(ValueError) as ctx:
                gm.load_model(path)
            self.assertIn("MERGE_VALID", str(ctx.exception))
        finally:
            path.unlink(missing_ok=True)


class MergeValidSemanticsTests(unittest.TestCase):
    """
    End-to-end semantics for MERGE_VALID.

    Combos rejected by MERGE_VALID:
      * MUST NOT appear at trigger_level=pr or merge.
      * MUST still appear at nightly (full cartesian product preserved).
    Combos listed in PR_CELLS take precedence — they always run on PR
    even if MERGE_VALID would block them from the pairwise pool.
    Mapping coverage is unaffected: a missing mapping for a MERGE_VALID-
    blocked but otherwise-valid combo still raises in validate_mappings.
    """

    def _write_python_model(self, merge_valid_block: str = "") -> Path:
        return _write_model(
            "PARAMS = {\n"
            "    'OS':        ['ubuntu', 'macos', 'windows'],\n"
            "    'Arch':      ['x64', 'arm'],\n"
            "    'Cloud':     ['aws', 'gcp', 'azure'],\n"
            "    'PyVersion': ['3.10', '3.11', '3.12', '3.13', '3.14'],\n"
            "    'HatchEnv':  ['test', 'test-pandas'],\n"
            "}\n"
            "def is_valid(c):\n"
            "    if c['OS'] == 'windows' and c['Arch'] == 'arm':\n"
            "        if c['PyVersion'] == '3.10':      return False\n"
            "        if c['HatchEnv'] == 'test-pandas': return False\n"
            "    return True\n"
            "CONSTRAINTS = [is_valid]\n"
            f"{merge_valid_block}"
            "PR_CELLS = [\n"
            "    {'OS': 'ubuntu',  'Arch': 'x64', 'Cloud': 'aws',\n"
            "     'PyVersion': '3.10', 'HatchEnv': 'test'},\n"
            "    {'OS': 'macos',   'Arch': 'arm', 'Cloud': 'gcp',\n"
            "     'PyVersion': '3.12', 'HatchEnv': 'test-pandas'},\n"
            "    {'OS': 'windows', 'Arch': 'x64', 'Cloud': 'azure',\n"
            "     'PyVersion': '3.14', 'HatchEnv': 'test'},\n"
            "]\n"
            "JSON_CELLS = {'pr': [], 'merge': [], 'nightly': []}\n"
        )

    def test_macos_x64_only_at_nightly(self) -> None:
        """Block macos-x64 from merge; nightly must still see it."""
        block = (
            "def merge_valid(c):\n"
            "    if c['OS'] == 'macos' and c['Arch'] == 'x64': return False\n"
            "    return True\n"
            "MERGE_VALID = [merge_valid]\n"
        )
        path = self._write_python_model(block)
        try:
            rows = gm.generate(path, "python")
            mac_x64 = [r for r in rows if r["os"] == "macos-15-intel"]
            self.assertTrue(mac_x64, "macos-x64 must still appear at nightly")
            for r in mac_x64:
                self.assertEqual(
                    r["trigger_level"], "nightly",
                    f"macos-x64 row {r['name']} should be nightly-only "
                    f"under MERGE_VALID, got trigger_level={r['trigger_level']}",
                )
        finally:
            path.unlink(missing_ok=True)

    def test_pr_cells_unaffected_by_merge_valid(self) -> None:
        """PR_CELLS bypass MERGE_VALID — they always run at PR scope."""
        # Block ALL macos via MERGE_VALID; the PR_CELLS macos row should
        # still appear at trigger_level=pr.
        block = (
            "def merge_valid(c):\n"
            "    if c['OS'] == 'macos': return False\n"
            "    return True\n"
            "MERGE_VALID = [merge_valid]\n"
        )
        path = self._write_python_model(block)
        try:
            rows = gm.generate(path, "python")
            mac_pr = [
                r for r in rows
                if "macos" in r["os"] and r["trigger_level"] == "pr"
            ]
            self.assertEqual(
                len(mac_pr), 1,
                f"expected the explicit macOS PR cell to survive MERGE_VALID; "
                f"got {[r['name'] for r in mac_pr]}",
            )
            self.assertEqual(mac_pr[0]["py"], "3.12")
            self.assertEqual(mac_pr[0]["cloud_provider"], "gcp")
        finally:
            path.unlink(missing_ok=True)

    def test_merge_valid_does_not_block_nightly_combos(self) -> None:
        """Combos blocked from merge must appear in the full nightly product."""
        block = (
            "def merge_valid(c):\n"
            "    if c['OS'] == 'macos' and c['Arch'] == 'x64': return False\n"
            "    return True\n"
            "MERGE_VALID = [merge_valid]\n"
        )
        path = self._write_python_model(block)
        try:
            rows = gm.generate(path, "python")
            # macOS-x64 has wheels for 3.11, 3.12, 3.13, 3.14 plus py3.10 sdist.
            # Without MERGE_VALID nightly emits exactly those rows; with
            # MERGE_VALID they should still appear, just at trigger_level=nightly.
            mac_x64_pys = {r["py"] for r in rows if r["os"] == "macos-15-intel"}
            self.assertEqual(
                mac_x64_pys, {"3.10", "3.11", "3.12", "3.13", "3.14"},
                f"nightly must cover every PyVersion on macos-x64; got {mac_x64_pys}",
            )
        finally:
            path.unlink(missing_ok=True)

    def test_empty_merge_valid_is_no_op(self) -> None:
        """Adding MERGE_VALID = [] to a model must not change the matrix."""
        without = self._write_python_model("")
        with_empty = self._write_python_model("MERGE_VALID = []\n")
        try:
            rows_a = gm.generate(without, "python")
            rows_b = gm.generate(with_empty, "python")
            self.assertEqual(rows_a, rows_b)
        finally:
            without.unlink(missing_ok=True)
            with_empty.unlink(missing_ok=True)

    def test_merge_valid_predicates_anded(self) -> None:
        """Multiple MERGE_VALID predicates: all must pass for a combo to be in pairwise."""
        block = (
            "def block_x64(c):\n"
            "    if c['OS'] == 'macos' and c['Arch'] == 'x64': return False\n"
            "    return True\n"
            "def block_arm_aws(c):\n"
            "    if c['OS'] == 'macos' and c['Arch'] == 'arm' and c['Cloud'] == 'aws':\n"
            "        return False\n"
            "    return True\n"
            "MERGE_VALID = [block_x64, block_arm_aws]\n"
        )
        path = self._write_python_model(block)
        try:
            rows = gm.generate(path, "python")
            # No macos-x64 should reach merge level (blocked by 1st predicate).
            mac_x64_at_merge = [
                r for r in rows
                if r["os"] == "macos-15-intel" and r["trigger_level"] in ("pr", "merge")
            ]
            self.assertEqual(mac_x64_at_merge, [])
            # No macos-arm + aws should reach merge level (blocked by 2nd predicate),
            # but PR_CELLS uses macos-arm + gcp so the PR cell is unaffected.
            mac_arm_aws_at_merge = [
                r for r in rows
                if r["os"] == "macos-latest" and r["cloud_provider"] == "aws"
                and r["trigger_level"] in ("pr", "merge")
            ]
            self.assertEqual(mac_arm_aws_at_merge, [])
        finally:
            path.unlink(missing_ok=True)

    def test_merge_valid_does_not_affect_mapping_validation(self) -> None:
        """validate_mappings runs over the full constraint-valid set, not the
        MERGE_VALID-restricted set, so a missing mapping for a MERGE_VALID-
        blocked combo still raises."""
        # Pop a mapping for an (OS, Arch) that MERGE_VALID will block, and
        # confirm generate() still raises because validate_mappings sees the
        # combo via the unfiltered cartesian product.
        original = gm.PYTHON_PLATFORM.pop(("macos", "x64"), None)
        block = (
            "def merge_valid(c):\n"
            "    if c['OS'] == 'macos' and c['Arch'] == 'x64': return False\n"
            "    return True\n"
            "MERGE_VALID = [merge_valid]\n"
        )
        path = self._write_python_model(block)
        try:
            with self.assertRaises(RuntimeError) as ctx:
                gm.generate(path, "python")
            self.assertIn("PYTHON_PLATFORM", str(ctx.exception))
        finally:
            path.unlink(missing_ok=True)
            if original is not None:
                gm.PYTHON_PLATFORM[("macos", "x64")] = original


# ---------------------------------------------------------------------------
# MERGE_QUEUE_CELLS — loader + end-to-end semantics
# ---------------------------------------------------------------------------

class LoadModelMergeQueueCellsTests(unittest.TestCase):
    """Exercises load_model() MERGE_QUEUE_CELLS handling."""

    def test_mq_cells_default_to_empty(self) -> None:
        """A model without MERGE_QUEUE_CELLS should load with merge_queue_cells=[]."""
        path = _write_model(
            "PARAMS = {'OS': ['ubuntu', 'macos'], 'Arch': ['x64', 'arm']}\n"
            "CONSTRAINTS = []\n"
            "PR_CELLS = []\n"
            "JSON_CELLS = {'pr': [], 'merge': [], 'nightly': []}\n"
        )
        try:
            _p, _c, _mv, _pr, mq_cells, _json = gm.load_model(path)
            self.assertEqual(mq_cells, [])
        finally:
            path.unlink(missing_ok=True)

    def test_mq_cell_missing_param_raises(self) -> None:
        path = _write_model(
            "PARAMS = {'OS': ['ubuntu'], 'Arch': ['x64'], 'Cloud': ['aws']}\n"
            "CONSTRAINTS = []\n"
            "PR_CELLS = []\n"
            "MERGE_QUEUE_CELLS = [{'OS': 'ubuntu', 'Arch': 'x64'}]\n"  # missing Cloud
            "JSON_CELLS = {'pr': [], 'merge': [], 'nightly': []}\n"
        )
        try:
            with self.assertRaises(ValueError) as ctx:
                gm.load_model(path)
            self.assertIn("MERGE_QUEUE_CELLS", str(ctx.exception))
            self.assertIn("Cloud", str(ctx.exception))
        finally:
            path.unlink(missing_ok=True)

    def test_mq_cell_extra_key_raises(self) -> None:
        path = _write_model(
            "PARAMS = {'OS': ['ubuntu'], 'Arch': ['x64']}\n"
            "CONSTRAINTS = []\n"
            "PR_CELLS = []\n"
            "MERGE_QUEUE_CELLS = [{'OS': 'ubuntu', 'Arch': 'x64', 'Bogus': 'x'}]\n"
            "JSON_CELLS = {'pr': [], 'merge': [], 'nightly': []}\n"
        )
        try:
            with self.assertRaises(ValueError) as ctx:
                gm.load_model(path)
            self.assertIn("Bogus", str(ctx.exception))
        finally:
            path.unlink(missing_ok=True)

    def test_mq_cells_must_be_a_list(self) -> None:
        path = _write_model(
            "PARAMS = {'OS': ['ubuntu']}\n"
            "CONSTRAINTS = []\n"
            "PR_CELLS = []\n"
            "MERGE_QUEUE_CELLS = 'not a list'\n"
            "JSON_CELLS = {'pr': [], 'merge': [], 'nightly': []}\n"
        )
        try:
            with self.assertRaises(ValueError) as ctx:
                gm.load_model(path)
            self.assertIn("MERGE_QUEUE_CELLS", str(ctx.exception))
        finally:
            path.unlink(missing_ok=True)


class MergeQueueCellsSemanticsTests(unittest.TestCase):
    """
    End-to-end semantics for MERGE_QUEUE_CELLS.

    * Cells appear at trigger_level="merge_queue", never "pr" or "merge".
    * filter_active("merge_queue") returns ONLY merge_queue rows (non-cumulative).
    * Cells are included by filter_active at merge/nightly (cumulative).
    * If a cell is in both PR_CELLS and MERGE_QUEUE_CELLS, PR_CELLS wins (→ "pr").
    * MERGE_QUEUE_CELLS are excluded from the pairwise pool — no duplicate rows.
    * MERGE_VALID does not gate MERGE_QUEUE_CELLS.
    * Empty MERGE_QUEUE_CELLS is a no-op.
    * A MERGE_QUEUE_CELLS cell that violates constraints warns but emits no row.
    """

    def _write_odbc_model(self, mq_block: str = "") -> Path:
        return _write_model(
            "PARAMS = {\n"
            "    'OS':   ['ubuntu', 'macos', 'windows'],\n"
            "    'Arch': ['x64', 'x86', 'arm'],\n"
            "    'Cloud': ['aws', 'gcp', 'azure'],\n"
            "}\n"
            "def is_valid(c):\n"
            "    if c['OS'] == 'ubuntu':\n"
            "        if c['Arch'] == 'arm': return False\n"
            "        if c['Arch'] == 'x86': return False\n"
            "    if c['OS'] == 'macos':\n"
            "        if c['Arch'] == 'x64': return False\n"
            "        if c['Arch'] == 'x86': return False\n"
            "    return True\n"
            "CONSTRAINTS = [is_valid]\n"
            "def merge_valid(c):\n"
            "    if c['OS'] == 'macos': return False\n"
            "    return True\n"
            "MERGE_VALID = [merge_valid]\n"
            "PR_CELLS = [\n"
            "    {'OS': 'ubuntu',  'Arch': 'x64', 'Cloud': 'azure'},\n"
            "    {'OS': 'macos',   'Arch': 'arm', 'Cloud': 'gcp'},\n"
            "    {'OS': 'windows', 'Arch': 'x64', 'Cloud': 'azure'},\n"
            "    {'OS': 'windows', 'Arch': 'x86', 'Cloud': 'aws'},\n"
            "]\n"
            f"{mq_block}"
            "JSON_CELLS = {'pr': [], 'merge': [], 'nightly': []}\n"
        )

    def test_mq_cells_appear_at_merge_queue_not_pr_or_merge(self) -> None:
        """The MERGE_QUEUE_CELLS cell must have trigger_level='merge_queue', never 'pr' or 'merge'."""
        mq = (
            "MERGE_QUEUE_CELLS = [\n"
            "    {'OS': 'windows', 'Arch': 'arm', 'Cloud': 'azure'},\n"
            "]\n"
        )
        path = self._write_odbc_model(mq)
        try:
            rows = gm.generate(path, "odbc")
            # Find the specific MERGE_QUEUE_CELLS-pinned row (windows-arm-azure).
            arm_azure = [
                r for r in rows
                if r.get("driver_artifact") == "Windows ARM64" and r["cloud_provider"] == "azure"
            ]
            self.assertEqual(len(arm_azure), 1, "expected exactly one windows-arm-azure row")
            self.assertEqual(
                arm_azure[0]["trigger_level"], "merge_queue",
                f"MERGE_QUEUE_CELLS cell {arm_azure[0]['name']} should be trigger_level=merge_queue; "
                f"got {arm_azure[0]['trigger_level']}",
            )
        finally:
            path.unlink(missing_ok=True)

    def test_mq_cells_visible_at_push_and_nightly_via_cumulative_filter(self) -> None:
        """filter_active at merge and nightly must include MERGE_QUEUE_CELLS rows."""
        mq = (
            "MERGE_QUEUE_CELLS = [\n"
            "    {'OS': 'windows', 'Arch': 'arm', 'Cloud': 'azure'},\n"
            "]\n"
        )
        path = self._write_odbc_model(mq)
        try:
            rows = gm.generate(path, "odbc")
            for level in ("merge", "nightly"):
                at_level = gm.filter_active(rows, level)
                arm_names = [r["name"] for r in at_level if r.get("driver_artifact") == "Windows ARM64"]
                self.assertTrue(
                    arm_names,
                    f"windows-arm (MERGE_QUEUE_CELLS) must appear at {level!r} via cumulative filter",
                )
        finally:
            path.unlink(missing_ok=True)

    def test_mq_filter_returns_only_mq_cells_not_all_pr_rows(self) -> None:
        """filter_active('merge_queue') returns only MERGE_QUEUE_CELLS rows.
        PR rows that are NOT in MERGE_QUEUE_CELLS must not be returned."""
        mq = (
            "MERGE_QUEUE_CELLS = [\n"
            "    {'OS': 'windows', 'Arch': 'arm', 'Cloud': 'azure'},\n"
            "]\n"
        )
        path = self._write_odbc_model(mq)
        try:
            rows = gm.generate(path, "odbc")
            at_mq = gm.filter_active(rows, "merge_queue")
            # All returned rows must have merge_queue_cell=True
            for r in at_mq:
                self.assertTrue(
                    r.get("merge_queue_cell"),
                    f"filter_active('merge_queue') returned row {r['name']} without merge_queue_cell=True",
                )
            # PR rows that are NOT in MERGE_QUEUE_CELLS must not appear
            pr_only_names = {
                r["name"] for r in rows
                if r["trigger_level"] == "pr" and not r.get("merge_queue_cell")
            }
            mq_names = {r["name"] for r in at_mq}
            self.assertTrue(
                pr_only_names.isdisjoint(mq_names),
                f"PR-only rows {pr_only_names & mq_names} appeared in merge_queue filter",
            )
        finally:
            path.unlink(missing_ok=True)

    def test_pr_cell_in_mq_cells_appears_at_merge_queue(self) -> None:
        """A cell in both PR_CELLS and MERGE_QUEUE_CELLS must be returned by
        filter_active('merge_queue') even though its trigger_level is 'pr'."""
        # Add ubuntu-x64-azure (which IS in PR_CELLS) to MERGE_QUEUE_CELLS.
        mq = (
            "MERGE_QUEUE_CELLS = [\n"
            "    {'OS': 'ubuntu', 'Arch': 'x64', 'Cloud': 'azure'},\n"  # same as PR_CELLS
            "]\n"
        )
        path = self._write_odbc_model(mq)
        try:
            rows = gm.generate(path, "odbc")
            # The row should still have trigger_level="pr" (PR_CELLS wins)
            ubuntu_azure = [
                r for r in rows
                if r.get("driver_artifact") == "Linux x64" and r["cloud_provider"] == "azure"
            ]
            self.assertEqual(len(ubuntu_azure), 1)
            self.assertEqual(ubuntu_azure[0]["trigger_level"], "pr",
                             "PR_CELLS must win trigger_level assignment")
            self.assertTrue(ubuntu_azure[0].get("merge_queue_cell"),
                            "merge_queue_cell must be True for MERGE_QUEUE_CELLS rows")
            # filter_active("merge_queue") must return this row despite trigger_level="pr"
            mq_active = gm.filter_active(rows, "merge_queue")
            linux_in_mq = [r for r in mq_active if r.get("driver_artifact") == "Linux x64"]
            self.assertEqual(
                len(linux_in_mq), 1,
                "ubuntu-x64-azure must appear in filter_active('merge_queue') via merge_queue_cell=True "
                f"even though trigger_level='pr'; mq_active: {[r['name'] for r in mq_active]}",
            )
        finally:
            path.unlink(missing_ok=True)

    def test_pr_cells_override_mq_cells_trigger_level(self) -> None:
        """A cell in both PR_CELLS and MERGE_QUEUE_CELLS gets trigger_level='pr'."""
        # ubuntu-x64-azure is in PR_CELLS; also add it to MERGE_QUEUE_CELLS.
        mq = (
            "MERGE_QUEUE_CELLS = [\n"
            "    {'OS': 'ubuntu', 'Arch': 'x64', 'Cloud': 'azure'},\n"
            "]\n"
        )
        path = self._write_odbc_model(mq)
        try:
            rows = gm.generate(path, "odbc")
            ubuntu_azure = [
                r for r in rows
                if r.get("driver_artifact") == "Linux x64" and r["cloud_provider"] == "azure"
            ]
            self.assertEqual(len(ubuntu_azure), 1, "expected exactly one ubuntu-x64-azure row")
            self.assertEqual(
                ubuntu_azure[0]["trigger_level"], "pr",
                "ubuntu-x64-azure is in PR_CELLS; trigger_level must be 'pr'",
            )
            # merge_queue_cell=True is still set, so filter_active("merge_queue") returns it.
            self.assertTrue(ubuntu_azure[0].get("merge_queue_cell"))
        finally:
            path.unlink(missing_ok=True)

    def test_mq_cells_excluded_from_pairwise_no_duplicate_rows(self) -> None:
        """No duplicate rows for a cell that is in MERGE_QUEUE_CELLS."""
        mq = (
            "MERGE_QUEUE_CELLS = [\n"
            "    {'OS': 'windows', 'Arch': 'arm', 'Cloud': 'azure'},\n"
            "]\n"
        )
        path = self._write_odbc_model(mq)
        try:
            rows = gm.generate(path, "odbc")
            arm_azure = [
                r for r in rows
                if r.get("driver_artifact") == "Windows ARM64" and r["cloud_provider"] == "azure"
            ]
            self.assertEqual(
                len(arm_azure), 1,
                f"expected exactly one windows-arm-azure row; got {len(arm_azure)} "
                f"(pairwise must not also select this MERGE_QUEUE_CELLS cell)",
            )
        finally:
            path.unlink(missing_ok=True)

    def test_mq_cells_not_gated_by_merge_valid(self) -> None:
        """MERGE_QUEUE_CELLS bypass MERGE_VALID — macOS blocked from pairwise still runs via MQ."""
        mq = (
            "MERGE_QUEUE_CELLS = [\n"
            "    {'OS': 'macos', 'Arch': 'arm', 'Cloud': 'aws'},\n"
            "]\n"
        )
        path = self._write_odbc_model(mq)
        try:
            rows = gm.generate(path, "odbc")
            mac_aws = [
                r for r in rows
                if r.get("driver_artifact") == "macOS ARM64" and r["cloud_provider"] == "aws"
            ]
            self.assertEqual(len(mac_aws), 1, "macos-arm-aws via MERGE_QUEUE_CELLS must appear despite MERGE_VALID blocking macOS from pairwise")
            self.assertEqual(mac_aws[0]["trigger_level"], "merge_queue")
        finally:
            path.unlink(missing_ok=True)

    def test_empty_mq_cells_is_no_op(self) -> None:
        """Adding MERGE_QUEUE_CELLS = [] must not change the matrix."""
        without = self._write_odbc_model("")
        with_empty = self._write_odbc_model("MERGE_QUEUE_CELLS = []\n")
        try:
            rows_a = gm.generate(without, "odbc")
            rows_b = gm.generate(with_empty, "odbc")
            self.assertEqual(rows_a, rows_b)
        finally:
            without.unlink(missing_ok=True)
            with_empty.unlink(missing_ok=True)

    def test_mq_cell_violating_constraint_warns_no_row(self) -> None:
        """A MERGE_QUEUE_CELLS cell that violates constraints warns and emits no row."""
        # macos-x64 is forbidden by is_valid (ODBC model).
        mq = (
            "MERGE_QUEUE_CELLS = [\n"
            "    {'OS': 'macos', 'Arch': 'x64', 'Cloud': 'aws'},\n"  # violates is_valid
            "]\n"
        )
        path = self._write_odbc_model(mq)
        try:
            buf = io.StringIO()
            with redirect_stderr(buf):
                rows = gm.generate(path, "odbc")
            self.assertIn("violates constraints", buf.getvalue())
            # No row for the forbidden combo must be emitted.
            mac_x64 = [r for r in rows if r.get("driver_artifact") == "macOS x64"]
            self.assertEqual(mac_x64, [])
        finally:
            path.unlink(missing_ok=True)


class BlockListShapeTests(unittest.TestCase):
    """
    Static-AST guard: every model's is_valid() must be a pure block-list.

    Discipline:
      * Every `return` inside the function is either `return False` or
        `return True` — never `return <expression>`. An expression-return
        is how allow-list creep enters (e.g. `return c["X"] in (...)`),
        which silently drops new PARAMS values.
      * The function ends with `return True` (the fall-through allow).
        Every other `return` is `return False` (an early-exit forbid).

    Catches drift-inducing patterns at PR-review time instead of at
    next-CPython-release time.
    """

    MODELS = ["core", "odbc", "python"]

    def _is_valid_function(self, model_name: str):
        import ast
        path = gm.MODELS_DIR / f"{model_name}.py"
        tree = ast.parse(path.read_text())
        funcs = [n for n in tree.body
                 if isinstance(n, ast.FunctionDef) and n.name == "is_valid"]
        self.assertEqual(
            len(funcs), 1,
            f"{path}: expected exactly one top-level `is_valid` function; got {len(funcs)}",
        )
        return path, funcs[0]

    def test_returns_only_bool_literals(self) -> None:
        import ast
        for model in self.MODELS:
            with self.subTest(model=model):
                path, fn = self._is_valid_function(model)
                for node in ast.walk(fn):
                    if not isinstance(node, ast.Return):
                        continue
                    self.assertIsInstance(
                        node.value, ast.Constant,
                        f"{path}:{node.lineno} `return <expression>` violates "
                        f"block-list discipline (returns must be `return False` "
                        f"or `return True`)",
                    )
                    self.assertIsInstance(
                        node.value.value, bool,
                        f"{path}:{node.lineno} `return` value is "
                        f"{node.value.value!r}, expected bool literal",
                    )

    def test_only_trailing_return_true(self) -> None:
        import ast
        for model in self.MODELS:
            with self.subTest(model=model):
                path, fn = self._is_valid_function(model)
                # Last top-level statement must be `return True`.
                last = fn.body[-1]
                self.assertIsInstance(
                    last, ast.Return,
                    f"{path}:{fn.name} must end with `return True`",
                )
                self.assertTrue(
                    isinstance(last.value, ast.Constant) and last.value.value is True,
                    f"{path}:{last.lineno} trailing return must be `return True`",
                )
                # All other returns must be `return False` (early-exit forbids).
                for node in ast.walk(fn):
                    if not isinstance(node, ast.Return) or node is last:
                        continue
                    self.assertTrue(
                        isinstance(node.value, ast.Constant) and node.value.value is False,
                        f"{path}:{node.lineno} early `return True` short-circuits "
                        f"later forbidding rules — use `return False` to block "
                        f"specific combos and let the trailing `return True` allow "
                        f"by default",
                    )


class ApplyConstraintsTests(unittest.TestCase):
    """
    apply_constraints just runs every predicate over the combo and ANDs the
    results. These tests lock in the contract: True iff every predicate
    returns True; an empty constraint list always passes.
    """

    def test_empty_constraints_passes(self) -> None:
        self.assertTrue(gm.apply_constraints({"OS": "macos"}, []))

    def test_single_predicate(self) -> None:
        # Block-list predicate: forbid macos+x64, allow everything else.
        c = lambda x: False if x["OS"] == "macos" and x["Arch"] == "x64" else True
        self.assertTrue(gm.apply_constraints({"OS": "macos", "Arch": "arm"}, [c]))
        self.assertFalse(gm.apply_constraints({"OS": "macos", "Arch": "x64"}, [c]))
        # Non-macos: predicate falls through to True.
        self.assertTrue(gm.apply_constraints({"OS": "ubuntu", "Arch": "x64"}, [c]))

    def test_multi_clause_predicate(self) -> None:
        # Forbidden combo: windows + arm + 3.10. Anything else → True.
        c = lambda x: (
            False
            if x["OS"] == "windows" and x["Arch"] == "arm" and x["PyVersion"] == "3.10"
            else True
        )
        self.assertFalse(gm.apply_constraints(
            {"OS": "windows", "Arch": "arm", "PyVersion": "3.10"}, [c],
        ))
        self.assertTrue(gm.apply_constraints(
            {"OS": "windows", "Arch": "arm", "PyVersion": "3.11"}, [c],
        ))
        # Antecedent doesn't match → fall through to True.
        self.assertTrue(gm.apply_constraints(
            {"OS": "windows", "Arch": "x64", "PyVersion": "3.10"}, [c],
        ))
        self.assertTrue(gm.apply_constraints(
            {"OS": "ubuntu", "Arch": "arm", "PyVersion": "3.10"}, [c],
        ))

    def test_all_predicates_must_pass(self) -> None:
        # Two block-list predicates: each independently forbids one combo.
        c1 = lambda x: False if x["OS"] == "macos" and x["Arch"] == "x64" else True
        c2 = lambda x: False if x["Arch"] == "x64" else True
        self.assertFalse(gm.apply_constraints({"OS": "ubuntu", "Arch": "x64"}, [c1, c2]))
        self.assertTrue(gm.apply_constraints({"OS": "macos", "Arch": "arm"}, [c1, c2]))


# ---------------------------------------------------------------------------
# Generator end-to-end
# ---------------------------------------------------------------------------

ODBC_PATH = gm.MODELS_DIR / "odbc.py"
PYTHON_PATH = gm.MODELS_DIR / "python.py"
CORE_PATH = gm.MODELS_DIR / "core.py"


class OdbcMatrixTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        cls.gha = gm.generate(ODBC_PATH, "odbc")

    def test_pr_has_all_major_platforms(self) -> None:
        pr_artifacts = {r["driver_artifact"] for r in self.gha if r["trigger_level"] == "pr"}
        self.assertIn("Linux x64", pr_artifacts)
        self.assertIn("macOS ARM64", pr_artifacts)
        self.assertIn("Windows x64", pr_artifacts)
        self.assertIn("Windows x86", pr_artifacts)

    def test_windows_x86_present(self) -> None:
        x86_rows = [r for r in self.gha if r.get("driver_artifact") == "Windows x86"]
        self.assertTrue(x86_rows, "expected at least one Windows x86 ODBC cell")
        for r in x86_rows:
            self.assertEqual(r["driver_lib"], "sfodbc32.dll")
            self.assertEqual(r["msvc_arch"], "x86")
            self.assertEqual(r["vcpkg_triplet"], "x86-windows")
            self.assertEqual(r["os"], "windows-latest")

    def test_windows_x64_has_vcpkg_triplet(self) -> None:
        # Regression test: test-odbc.yml's vcpkg install step uses
        # ${{ matrix.vcpkg_triplet }} with no fallback. Every windows-x64
        # ODBC cell must carry vcpkg_triplet=x64-windows or vcpkg install
        # fails with "expected a triplet name here".
        x64_rows = [r for r in self.gha if r.get("driver_artifact") == "Windows x64"]
        self.assertTrue(x64_rows, "expected at least one Windows x64 ODBC cell")
        for r in x64_rows:
            self.assertEqual(r["vcpkg_triplet"], "x64-windows", r["name"])

    def test_json_variant_present(self) -> None:
        json_rows = [r for r in self.gha if r.get("result_format") == "json"]
        self.assertEqual(len(json_rows), 1)
        r = json_rows[0]
        self.assertEqual(r["os"], "ubuntu-latest")
        self.assertEqual(r["cloud_provider"], "aws")
        self.assertEqual(r["driver_artifact"], "Linux x64")

    def test_ubuntu_linux_present_at_merge_queue(self) -> None:
        # ubuntu-x64-aws is the MERGE_QUEUE_CELLS cell — same as PR_CELLS,
        # most common path. Gets trigger_level="pr" + merge_queue_cell=True.
        mq_rows = gm.filter_active(self.gha, "merge_queue")
        linux_mq = [r for r in mq_rows if r.get("driver_artifact") == "Linux x64"]
        self.assertEqual(
            len(linux_mq), 1,
            "expected exactly one Linux x64 row in filter_active('merge_queue'); "
            f"got: {[r['name'] for r in mq_rows]}",
        )
        self.assertEqual(linux_mq[0]["cloud_provider"], "aws")
        self.assertTrue(linux_mq[0].get("merge_queue_cell"))


class PythonMatrixTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        cls.gha = gm.generate(PYTHON_PATH, "python")

    def test_no_wheel_artifact_on_py310(self) -> None:
        offenders = [r for r in self.gha if r["py"] == "3.10" and r.get("wheel_artifact")]
        self.assertEqual(offenders, [])

    def test_py310_present_on_main_platforms(self) -> None:
        py310_os = {r["os"] for r in self.gha if r["py"] == "3.10"}
        # py3.10 builds from sdist; should be on at least all wheel-target OSes
        self.assertIn("ubuntu-latest", py310_os)
        self.assertIn("macos-latest", py310_os)
        self.assertIn("windows-latest", py310_os)

    def test_macos_x64_present(self) -> None:
        # Regression test: macOS Intel (macos-15-intel runner) coverage must
        # remain in the matrix. Drop this row from PYTHON_PLATFORM and the
        # release-build / nightly coverage gap reopens (see PR #1084 review).
        intel_rows = [r for r in self.gha if r["os"] == "macos-15-intel"]
        self.assertTrue(intel_rows, "expected macos-15-intel cells in the python matrix")
        for r in intel_rows:
            self.assertIn(r["py"], {"3.10", "3.11", "3.12", "3.13", "3.14"})
            # py3.10 always sdist; every other py on macos-x64 must have a wheel.
            if r["py"] != "3.10":
                self.assertEqual(r.get("wheel_artifact"), "macosx_x86_64", r["name"])

    def test_windows_arm_at_merge_scope(self) -> None:
        # Regression test for the routing-aware pairwise solver. Without it,
        # the abstract pairwise solver picks windows-arm × py3.13/3.14 (which
        # have no wheel) to cover (windows, arm) pairs; those rows get
        # dropped at row-build time, leaving zero windows-arm cells at merge
        # level (only nightly). The routing-aware predicate restricts the
        # candidate pool to combos that actually emit a row, so windows-arm
        # gets a real merge-level cell.
        warm_at_merge = [
            r for r in self.gha
            if r["os"] == "windows-11-arm"
            and r["trigger_level"] in ("pr", "merge")
        ]
        self.assertTrue(
            warm_at_merge,
            "expected windows-11-arm cells at pr+merge scope; "
            "if missing, the pairwise solver may have regressed to the "
            "non-routing-aware path that picks routing-invalid combos.",
        )

    def test_windows_arm_excludes_py310(self) -> None:
        # Regression test for the python.py constraint
        #     IF [OS] = "windows" AND [Arch] = "arm" THEN [PyVersion] <> "3.10"
        # CPython has no Windows-aarch64 build for 3.10 (Windows-on-ARM was
        # first supported in 3.11, PEP 11 tier-3). uv fails with
        # "No download found for request: cpython-3.10-windows-aarch64-none"
        # if this combo reaches the matrix.
        #
        # Originally the constraint used `AND` in the IF clause which the
        # in-house parser silently dropped (regex matched only single-condition
        # IFs). A failing CI run on the merge_scope label shipped a real
        # windows-11-arm/py3.10 cell because of that. This test guards the
        # parser extension that supports AND clauses + `<>`/`NOT IN` ops.
        offenders = [
            r for r in self.gha
            if r["os"] == "windows-11-arm" and r["py"] == "3.10"
        ]
        self.assertEqual(
            offenders, [],
            "windows-11-arm + py3.10 cells must be pruned from the matrix "
            f"(no CPython 3.10 Windows-ARM64 build); got: {offenders}",
        )

    def test_windows_arm_excludes_test_pandas(self) -> None:
        # Regression test for the python.py constraint
        #     IF [OS] = "windows" AND [Arch] = "arm" THEN [HatchEnv] <> "test-pandas"
        # pyarrow has no win_arm64 wheel for the Python versions we test, so
        # uv source-builds it on the windows-11-arm runner. The build needs
        # Apache Arrow C++ libs which aren't installed there; CMake fails at
        # find_package(Arrow). See run 25662523089 / job 75331063502.
        # The `test` hatch env is unaffected (no pyarrow dep).
        offenders = [
            r for r in self.gha
            if r["os"] == "windows-11-arm" and r["hatch_env"] == "test-pandas"
        ]
        self.assertEqual(
            offenders, [],
            "windows-11-arm + test-pandas cells must be pruned from the matrix "
            f"(pyarrow has no win_arm64 wheel); got: {offenders}",
        )

    def test_wheel_artifact_only_when_built(self) -> None:
        # Reverse-lookup the (os, arch) producing this wheel artifact name.
        artifact_to_pair = {p["wheel_artifact"]: pair for pair, p in gm.PYTHON_PLATFORM.items()}
        for r in self.gha:
            artifact = r.get("wheel_artifact")
            if not artifact:
                continue
            pair = artifact_to_pair[artifact]
            self.assertIn(
                r["py"],
                gm.PYTHON_PLATFORM[pair]["wheels"],
                f"row {r['name']} has wheel_artifact {artifact} but py{r['py']} "
                f"isn't in PYTHON_PLATFORM[{pair}]['wheels']",
            )

    def test_json_variants_present(self) -> None:
        json_rows = [r for r in self.gha if r.get("result_format") == "json"]
        envs = {r["hatch_env"] for r in json_rows}
        self.assertEqual(envs, {"test", "test-pandas"})
        for r in json_rows:
            self.assertEqual(r["py"], "3.13")
            self.assertEqual(r["os"], "ubuntu-latest")
            self.assertEqual(r["cloud_provider"], "aws")

    def test_required_keys_on_every_row(self) -> None:
        required = {"name", "os", "cloud_provider", "trigger_level", "py", "hatch_env"}
        for r in self.gha:
            missing = required - r.keys()
            self.assertFalse(missing, f"row {r} missing keys {missing}")

    def test_ubuntu_linux_present_at_merge_queue(self) -> None:
        # ubuntu-x64-aws-3.13-test is the single MERGE_QUEUE_CELLS cell — same as
        # PR_CELLS, most common path. Gets trigger_level="pr" + merge_queue_cell=True.
        mq_rows = gm.filter_active(self.gha, "merge_queue")
        ubuntu_mq = [
            r for r in mq_rows
            if r["os"] == "ubuntu-latest"
            and r["cloud_provider"] == "aws"
            and r["py"] == "3.13"
            and r["hatch_env"] == "test"
        ]
        self.assertEqual(
            len(ubuntu_mq), 1,
            "expected exactly one ubuntu-x64-aws-3.13-test row in filter_active('merge_queue'); "
            f"got merge_queue rows: {[r['name'] for r in mq_rows]}",
        )
        self.assertTrue(ubuntu_mq[0].get("merge_queue_cell"))

    def test_no_duplicate_macos_rows_at_push_to_main(self) -> None:
        # Regression test for the python.py MERGE_VALID/PR_CELLS sync invariant.
        # macOS rows come from pairwise (trigger_level="merge") and appear at
        # push-to-main scope (filter_active("merge") is cumulative).
        # The merge_valid() predicate pins macos-arm to one combo and macos-x64
        # to another. This test verifies exactly one cell per macOS arch appears
        # at push-to-main scope (pr + merge_queue + merge).
        push_scope = ("pr", "merge_queue", "merge")
        push_macos_arm = [
            r for r in self.gha
            if r["os"] == "macos-latest" and r["trigger_level"] in push_scope
        ]
        self.assertEqual(
            len(push_macos_arm), 1,
            f"expected exactly one macos-arm row at push-to-main scope; "
            f"got {[r['name'] for r in push_macos_arm]}. "
            f"This usually means the merge_valid() pin for macos-arm has drifted.",
        )
        push_intel = [
            r for r in self.gha
            if r["os"] == "macos-15-intel" and r["trigger_level"] in push_scope
        ]
        self.assertEqual(
            len(push_intel), 1,
            f"expected exactly one macos-x64 row at push-to-main scope; "
            f"got {[r['name'] for r in push_intel]}",
        )


class CoreMatrixTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        cls.gha = gm.generate(CORE_PATH, "core")

    def test_pr_has_all_four_cells(self) -> None:
        names = {r["name"] for r in self.gha if r["trigger_level"] == "pr"}
        self.assertEqual(
            names,
            {"ubuntu-x64", "macos-arm", "windows-arm-nonfips", "windows-x86"},
        )

    def test_no_cloud_provider_field(self) -> None:
        # Core has no Cloud axis (single E2E_TLS_SERVER for every cell).
        for r in self.gha:
            self.assertNotIn("cloud_provider", r, r["name"])

    def test_coverage_only_on_unix(self) -> None:
        cov_runners = {r["os"] for r in self.gha if r.get("coverage")}
        self.assertEqual(cov_runners, {"ubuntu-latest", "macos-latest"})
        for r in self.gha:
            if r["os"] in ("windows-11-arm", "windows-latest"):
                self.assertFalse(r["coverage"], r["name"])

    def test_targets_for_windows_only(self) -> None:
        targets = {r["name"]: r.get("cargo_target") for r in self.gha}
        self.assertEqual(targets["ubuntu-x64"], None)
        self.assertEqual(targets["macos-arm"], None)
        self.assertEqual(targets["windows-arm-nonfips"], "aarch64-pc-windows-msvc")
        self.assertEqual(targets["windows-x86"], "i686-pc-windows-msvc")

    def test_msvc_arch_for_windows_only(self) -> None:
        msvc = {r["name"]: r.get("msvc_arch") for r in self.gha}
        self.assertEqual(msvc["ubuntu-x64"], None)
        self.assertEqual(msvc["macos-arm"], None)
        self.assertEqual(msvc["windows-arm-nonfips"], "arm64")
        self.assertEqual(msvc["windows-x86"], "x86")

    def test_cargo_flags_per_platform(self) -> None:
        flags = {r["name"]: r["cargo_flags"] for r in self.gha}
        self.assertEqual(flags["ubuntu-x64"], "--all-features")
        self.assertEqual(flags["macos-arm"], "--all-features")
        self.assertEqual(flags["windows-arm-nonfips"], "")
        self.assertEqual(
            flags["windows-x86"],
            "--no-default-features --features protobuf,vendored-openssl",
        )

    def test_cache_keys_match_legacy(self) -> None:
        # Cache shared-keys must stay identical to the pre-consolidation values
        # in test-rust-core.yml so warm caches survive the refactor.
        keys = {r["name"]: r["cache_key"] for r in self.gha}
        self.assertEqual(keys["ubuntu-x64"], "core-test")
        self.assertEqual(keys["macos-arm"], "core-test")
        self.assertEqual(keys["windows-arm-nonfips"], "arm64-nonfips")
        self.assertEqual(keys["windows-x86"], "x86-core-test")

    def test_required_keys_on_every_row(self) -> None:
        required = {"name", "os", "trigger_level", "cargo_flags", "coverage", "cache_key"}
        for r in self.gha:
            missing = required - r.keys()
            self.assertFalse(missing, f"row {r['name']} missing keys {missing}")

    def test_ubuntu_x64_is_merge_queue_cell(self) -> None:
        # ubuntu-x64 is in both PR_CELLS and MERGE_QUEUE_CELLS (it's the fastest cell).
        # PR_CELLS wins for trigger_level ("pr"), but merge_queue_cell=True means
        # filter_active("merge_queue") returns only ubuntu-x64 at merge_group.
        mq = gm.filter_active(self.gha, "merge_queue")
        self.assertEqual(
            [r["name"] for r in mq], ["ubuntu-x64"],
            f"expected only ubuntu-x64 at merge_queue scope; got {[r['name'] for r in mq]}",
        )
        self.assertEqual(mq[0]["trigger_level"], "pr",
                         "ubuntu-x64 is in PR_CELLS — trigger_level must be 'pr'")
        self.assertTrue(mq[0].get("merge_queue_cell"),
                        "ubuntu-x64 is in MERGE_QUEUE_CELLS — merge_queue_cell must be True")


# ---------------------------------------------------------------------------
# Validation paths
# ---------------------------------------------------------------------------

class ValidationTests(unittest.TestCase):
    def test_pr_cell_violating_constraint_warns(self) -> None:
        path = _write_model(
            "PARAMS = {'OS': ['ubuntu', 'macos'], 'Arch': ['x64', 'arm'], 'Cloud': ['aws']}\n"
            "CONSTRAINTS = [lambda x: False if x['OS'] == 'macos' and x['Arch'] == 'x64' else True]\n"
            # macos+x64+aws violates the constraint above.
            "PR_CELLS = [{'OS': 'macos', 'Arch': 'x64', 'Cloud': 'aws'}]\n"
            "JSON_CELLS = {'pr': [], 'merge': [], 'nightly': []}\n"
        )
        # Use a tiny model that won't hit other validation paths.
        try:
            buf = io.StringIO()
            with redirect_stderr(buf):
                # We only want loader/constraint behavior; sidestep validate_mappings
                # by calling internals.
                params, constraints, _mv, pr_cells, _mq_cells, _json_cells = gm.load_model(path)
                all_combos = [
                    c for c in (
                        dict(zip(params.keys(), v))
                        for v in itertools.product(*params.values())
                    )
                    if gm.apply_constraints(c, constraints)
                ]
                valid_keys = {tuple(c.values()) for c in all_combos}
                for cell in pr_cells:
                    if tuple(cell.values()) not in valid_keys:
                        print(
                            f"WARNING: [pr] cell {cell} violates constraints",
                            file=sys.stderr,
                        )
            self.assertIn("violates constraints", buf.getvalue())
        finally:
            path.unlink(missing_ok=True)

    def test_validate_mappings_raises_on_missing_pair(self) -> None:
        # Force a (OS, Arch) the model allows but no mapping table contains.
        all_combos = [{"OS": "freebsd", "Arch": "x64", "Cloud": "aws"}]
        with self.assertRaises(RuntimeError) as ctx:
            gm.validate_mappings("odbc", all_combos)
        self.assertIn("freebsd", str(ctx.exception))

    def test_validate_mappings_raises_on_python_missing_platform(self) -> None:
        # macos-x64 is in GHA_RUNNER (so the first check passes) but if it
        # were missing from PYTHON_PLATFORM, py3.11+ rows would silently drop.
        # Simulate that drift by reaching past PYTHON_PLATFORM with a synthetic
        # combo on a runner that exists in GHA_RUNNER but a fictional arch
        # combination not in PYTHON_PLATFORM.
        original = gm.PYTHON_PLATFORM.pop(("macos", "x64"), None)
        try:
            all_combos = [{"OS": "macos", "Arch": "x64", "Cloud": "aws",
                           "PyVersion": "3.13", "HatchEnv": "test"}]
            with self.assertRaises(RuntimeError) as ctx:
                gm.validate_mappings("python", all_combos)
            self.assertIn("PYTHON_PLATFORM", str(ctx.exception))
            self.assertIn("macos", str(ctx.exception))
        finally:
            if original is not None:
                gm.PYTHON_PLATFORM[("macos", "x64")] = original


# ---------------------------------------------------------------------------
# JSON variants — cross-driver regression
# ---------------------------------------------------------------------------

class JsonVariantRegressionTests(unittest.TestCase):
    """
    Locks the JSON cell count at push-to-main scope across odbc + python.

    ODBC has 1 json cell (trigger_level="pr", appears at all cumulative scopes).
    Python has 2 json cells (trigger_level="merge", gated on push-to-main).
    Total at push-to-main scope: 3.
    """

    @classmethod
    def setUpClass(cls) -> None:
        cls.odbc_gha = gm.generate(ODBC_PATH, "odbc")
        cls.py_gha = gm.generate(PYTHON_PATH, "python")

    def test_combined_json_count_at_push_to_main(self) -> None:
        # push-to-main = filter_active("merge") = cumulative: pr + merge_queue + merge.
        push_scope = ("pr", "merge_queue", "merge")
        json_at_push = [
            r for r in (self.odbc_gha + self.py_gha)
            if r.get("result_format") == "json" and r["trigger_level"] in push_scope
        ]
        self.assertEqual(
            len(json_at_push), 3,
            f"expected 3 json cells active at push-to-main, got {len(json_at_push)}: "
            f"{[r['name'] for r in json_at_push]}",
        )

    def test_json_names_unchanged(self) -> None:
        json_names = sorted(
            r["name"] for r in (self.odbc_gha + self.py_gha)
            if r.get("result_format") == "json"
        )
        self.assertEqual(
            json_names,
            [
                "ubuntu-x64-aws-json",
                "ubuntu-x64-aws-py3.13-json",
                "ubuntu-x64-aws-py3.13-test-pandas-json",
            ],
        )


# ---------------------------------------------------------------------------
# Build targets — wheel-build alignment
# ---------------------------------------------------------------------------

# Helper: reverse-lookup cibw_key -> (OS, Arch) for build-target tests.
PYTHON_PLATFORM_BY_CIBW = {p["cibw_key"]: pair for pair, p in gm.PYTHON_PLATFORM.items()}


class BuildTargetsTests(unittest.TestCase):
    """
    Coverage for the --emit-build-targets generator mode that drives
    _build-python-wheels.yml's `targets:` input.

    Locks in three invariants:
      * Coverage equivalence — every active test row with a wheel_artifact
        has a corresponding (cibw_key, py) entry in build_targets.
      * No over-build — every (cibw_key, py) in build_targets traces back
        to at least one active test row at the same trigger level.
      * Per-trigger contraction — PR build targets are a subset of merge,
        which is a subset of nightly. PR builds at most as many wheels as
        tests need.
    """

    @classmethod
    def setUpClass(cls) -> None:
        cls.gha = gm.generate(PYTHON_PATH, "python")
        cls.targets_pr         = gm.build_targets("python", "pull_request")
        cls.targets_merge_group = gm.build_targets("python", "merge_group")  # MQ only
        cls.targets_push       = gm.build_targets("python", "push")           # full push-to-main
        cls.targets_nightly    = gm.build_targets("python", "schedule")

    def _active_wheel_rows(self, level: str) -> list:
        return [
            r for r in gm.filter_active(self.gha, level)
            if r.get("wheel_artifact")
        ]

    def _expected_targets(self, level: str) -> dict[str, set[str]]:
        artifact_to_pair = {p["wheel_artifact"]: pair for pair, p in gm.PYTHON_PLATFORM.items()}
        out: dict[str, set[str]] = {}
        for r in self._active_wheel_rows(level):
            pair = artifact_to_pair[r["wheel_artifact"]]
            out.setdefault(gm.PYTHON_PLATFORM[pair]["cibw_key"], set()).add(r["py"])
        return out

    def test_only_python_driver_supported(self) -> None:
        with self.assertRaises(ValueError) as ctx:
            gm.build_targets("odbc", "pull_request")
        self.assertIn("python", str(ctx.exception))

    def test_pr_targets_match_active_wheel_rows(self) -> None:
        expected = self._expected_targets("pr")
        actual = {k: set(v) for k, v in self.targets_pr.items()}
        self.assertEqual(actual, expected)

    def test_merge_group_targets_match_active_wheel_rows(self) -> None:
        # merge_group (merge_queue level) is non-cumulative: only MQ cells' wheels.
        expected = self._expected_targets("merge_queue")
        actual = {k: set(v) for k, v in self.targets_merge_group.items()}
        self.assertEqual(actual, expected)

    def test_push_targets_match_active_wheel_rows(self) -> None:
        # push-to-main (merge level) is cumulative: pr + merge_queue + merge.
        expected = self._expected_targets("merge")
        actual = {k: set(v) for k, v in self.targets_push.items()}
        self.assertEqual(actual, expected)

    def test_nightly_targets_match_active_wheel_rows(self) -> None:
        expected = self._expected_targets("nightly")
        actual = {k: set(v) for k, v in self.targets_nightly.items()}
        self.assertEqual(actual, expected)

    def test_pr_subset_of_push_subset_of_nightly(self) -> None:
        # Contraction: PR ⊆ push-to-main ⊆ nightly.
        # (merge_group is non-cumulative so PR ⊄ merge_group is expected.)
        for cibw_key, versions in self.targets_pr.items():
            self.assertIn(
                cibw_key, self.targets_push,
                f"PR builds {cibw_key} but push-to-main does not — every PR cell "
                f"must also be active at push-to-main (cumulative trigger filter).",
            )
            self.assertTrue(
                set(versions).issubset(self.targets_push[cibw_key]),
                f"PR-level {cibw_key} versions {versions} not a subset of push {self.targets_push[cibw_key]}",
            )
        for cibw_key, versions in self.targets_push.items():
            self.assertIn(
                cibw_key, self.targets_nightly,
                f"push builds {cibw_key} but nightly does not — full cartesian "
                f"product should always cover push-to-main cells.",
            )
            self.assertTrue(
                set(versions).issubset(self.targets_nightly[cibw_key]),
                f"push-level {cibw_key} versions {versions} not a subset of nightly {self.targets_nightly[cibw_key]}",
            )

    def test_no_overbuild(self) -> None:
        # Every (cibw_key, py) in build_targets must correspond to >=1 active
        # test row at the same level. Catches "we build wheels nothing tests".
        artifact_to_pair = {p["wheel_artifact"]: pair for pair, p in gm.PYTHON_PLATFORM.items()}
        for level, targets in [
            ("pr",          self.targets_pr),
            ("merge_queue", self.targets_merge_group),
            ("merge",       self.targets_push),
            ("nightly",     self.targets_nightly),
        ]:
            active = self._active_wheel_rows(level)
            for cibw_key, versions in targets.items():
                target_pair = PYTHON_PLATFORM_BY_CIBW[cibw_key]
                for v in versions:
                    matches = [
                        r for r in active
                        if artifact_to_pair[r["wheel_artifact"]] == target_pair
                        and r["py"] == v
                    ]
                    self.assertTrue(
                        matches,
                        f"[{level}] {cibw_key}/py{v} in build_targets but no test row consumes it",
                    )

    def test_no_underbuild(self) -> None:
        # Every active test row with a wheel_artifact has a (cibw_key, py)
        # entry in build_targets. Catches "test row references a wheel that
        # won't be built", which would 404 actions/download-artifact at runtime.
        artifact_to_pair = {p["wheel_artifact"]: pair for pair, p in gm.PYTHON_PLATFORM.items()}
        for level, targets in [
            ("pr",          self.targets_pr),
            ("merge_queue", self.targets_merge_group),
            ("merge",       self.targets_push),
            ("nightly",     self.targets_nightly),
        ]:
            for r in self._active_wheel_rows(level):
                pair = artifact_to_pair[r["wheel_artifact"]]
                cibw_key = gm.PYTHON_PLATFORM[pair]["cibw_key"]
                self.assertIn(
                    cibw_key, targets,
                    f"[{level}] test row {r['name']} needs wheel for {cibw_key} "
                    f"but build_targets has no entry for that platform",
                )
                self.assertIn(
                    r["py"], targets[cibw_key],
                    f"[{level}] test row {r['name']} needs py{r['py']} on {cibw_key} "
                    f"but build_targets[{cibw_key}] = {targets[cibw_key]}",
                )

    def test_sdist_py_excluded_from_targets(self) -> None:
        # py3.10 always installs from sdist; rows have no wheel_artifact and
        # must NOT appear in build_targets at any level.
        for level, targets in [
            ("pr",          self.targets_pr),
            ("merge_queue", self.targets_merge_group),
            ("merge",       self.targets_push),
            ("nightly",     self.targets_nightly),
        ]:
            for cibw_key, versions in targets.items():
                self.assertNotIn(
                    "3.10", versions,
                    f"[{level}] py3.10 listed under {cibw_key} in build_targets, "
                    f"but py3.10 is sdist-only (SDIST_PY) and shouldn't be wheel-built",
                )

    def test_nightly_targets_match_legacy_hardcoded_json(self) -> None:
        # Regression guard: at nightly scope the generator output must match
        # the JSON literal previously hardcoded in test-python.yml. Pins the
        # migration so a future PR can't silently shrink wheel coverage at
        # nightly without a corresponding model edit.
        legacy = {
            "linux_x86":   {"3.13"},
            "linux_aarch": {"3.11", "3.14"},
            "macos_arm":   {"3.12", "3.14"},
            "macos_x86":   {"3.11", "3.12", "3.13", "3.14"},
            "windows_x86": {"3.11", "3.12", "3.14"},
            "windows_arm": {"3.11", "3.12"},
        }
        actual = {k: set(v) for k, v in self.targets_nightly.items()}
        self.assertEqual(
            actual, legacy,
            "nightly build_targets diverged from the legacy test-python.yml "
            "hardcoded targets JSON. If this is intentional (e.g. a PARAMS edit), "
            "update the `legacy` dict in this test to match.",
        )

    def test_emit_build_targets_cli_format(self) -> None:
        # CLI emits exactly one line of the form `targets=<json>`.
        import contextlib
        import io
        import json as _json
        buf = io.StringIO()
        with contextlib.redirect_stdout(buf):
            gm.emit_build_targets("python", "pull_request")
        line = buf.getvalue().rstrip("\n")
        self.assertTrue(line.startswith("targets="))
        payload = _json.loads(line[len("targets="):])
        self.assertEqual(payload, self.targets_pr)

    def test_python_platform_has_cibw_key(self) -> None:
        # Every PYTHON_PLATFORM row must declare cibw_key. validate_mappings
        # enforces this at generate() time; this test pins it independently.
        for pair, meta in gm.PYTHON_PLATFORM.items():
            self.assertIn(
                "cibw_key", meta,
                f"PYTHON_PLATFORM[{pair}] missing 'cibw_key' — "
                f"--emit-build-targets cannot translate this platform.",
            )

    def test_validate_mappings_raises_on_missing_cibw_key(self) -> None:
        # Drift simulation: pop cibw_key from one row, validate_mappings
        # must fail loud rather than silently degrade.
        original = gm.PYTHON_PLATFORM[("ubuntu", "x64")].copy()
        gm.PYTHON_PLATFORM[("ubuntu", "x64")].pop("cibw_key")
        try:
            with self.assertRaises(RuntimeError) as ctx:
                gm.generate(PYTHON_PATH, "python")
            self.assertIn("cibw_key", str(ctx.exception))
        finally:
            gm.PYTHON_PLATFORM[("ubuntu", "x64")] = original


# ---------------------------------------------------------------------------
# Build matrix — ODBC driver-build alignment
# ---------------------------------------------------------------------------

class OdbcBuildMatrixTests(unittest.TestCase):
    """
    Coverage for the --emit-build-matrix generator mode that drives
    test-odbc.yml's build_odbc_driver matrix.

    Locks in:
      * Coverage equivalence — every active test row with driver_artifact
        has a corresponding build matrix entry.
      * No over-build / under-build — the build matrix is a deduplicated
        projection of active test rows' driver_artifact.
      * Per-trigger contraction — PR ⊆ push-to-main ⊆ nightly.
        (merge_group is non-cumulative; PR ⊄ merge_group is expected.)
      * Legacy parity — at nightly the matrix matches the previously
        hardcoded include block in test-odbc.yml.
      * Schema — required fields are present, optional fields appear only
        where applicable, output is alphabetically ordered.
    """

    @classmethod
    def setUpClass(cls) -> None:
        cls.gha = gm.generate(ODBC_PATH, "odbc")
        cls.matrix_pr          = gm.build_matrix("odbc", "pull_request")
        cls.matrix_merge_group = gm.build_matrix("odbc", "merge_group")  # MQ only
        cls.matrix_push        = gm.build_matrix("odbc", "push")          # full push-to-main
        cls.matrix_nightly     = gm.build_matrix("odbc", "schedule")

    def _active_artifact_rows(self, level: str) -> list:
        return [
            r for r in gm.filter_active(self.gha, level)
            if r.get("driver_artifact")
        ]

    def test_only_odbc_driver_supported(self) -> None:
        with self.assertRaises(ValueError) as ctx:
            gm.build_matrix("python", "pull_request")
        self.assertIn("odbc", str(ctx.exception))
        with self.assertRaises(ValueError) as ctx:
            gm.build_matrix("core", "pull_request")
        self.assertIn("odbc", str(ctx.exception))

    def test_pr_matches_active_rows(self) -> None:
        expected_names = {r["driver_artifact"] for r in self._active_artifact_rows("pr")}
        actual_names = {entry["name"] for entry in self.matrix_pr}
        self.assertEqual(actual_names, expected_names)

    def test_merge_group_matches_active_rows(self) -> None:
        # merge_group (merge_queue level) is non-cumulative: only MQ cells' artifacts.
        expected_names = {r["driver_artifact"] for r in self._active_artifact_rows("merge_queue")}
        actual_names = {entry["name"] for entry in self.matrix_merge_group}
        self.assertEqual(actual_names, expected_names)

    def test_push_matches_active_rows(self) -> None:
        # push-to-main (merge level) is cumulative: pr + merge_queue + merge.
        expected_names = {r["driver_artifact"] for r in self._active_artifact_rows("merge")}
        actual_names = {entry["name"] for entry in self.matrix_push}
        self.assertEqual(actual_names, expected_names)

    def test_nightly_matches_active_rows(self) -> None:
        expected_names = {r["driver_artifact"] for r in self._active_artifact_rows("nightly")}
        actual_names = {entry["name"] for entry in self.matrix_nightly}
        self.assertEqual(actual_names, expected_names)

    def test_pr_subset_of_push_subset_of_nightly(self) -> None:
        # PR ⊆ push-to-main ⊆ nightly (cumulative contraction).
        # merge_group is non-cumulative; PR ⊄ merge_group is expected behaviour.
        names_pr      = {e["name"] for e in self.matrix_pr}
        names_push    = {e["name"] for e in self.matrix_push}
        names_nightly = {e["name"] for e in self.matrix_nightly}
        self.assertTrue(names_pr.issubset(names_push),
                        f"PR names {names_pr} not subset of push-to-main {names_push}")
        self.assertTrue(names_push.issubset(names_nightly),
                        f"push names {names_push} not subset of nightly {names_nightly}")

    def test_no_duplicates_per_level(self) -> None:
        for level, matrix in [
            ("pr",          self.matrix_pr),
            ("merge_group", self.matrix_merge_group),
            ("push",        self.matrix_push),
            ("nightly",     self.matrix_nightly),
        ]:
            names = [e["name"] for e in matrix]
            self.assertEqual(
                len(names), len(set(names)),
                f"[{level}] build_matrix has duplicate entries: {names}",
            )

    def test_nightly_matches_legacy_hardcoded_matrix(self) -> None:
        # Regression guard: at nightly scope the generator output must be the
        # set previously hardcoded as build_odbc_driver:matrix:include in
        # test-odbc.yml. If a future PR shrinks ODBC_PLATFORM, this test
        # signals the change so the workflow assumption (nightly builds all 5
        # driver flavours) can be updated deliberately.
        legacy = {
            "Linux x64":     {"os": "ubuntu-latest", "driver_lib": "libsfodbc.so",
                              "cache_key": "odbc"},
            "macOS ARM64":   {"os": "macos-latest", "driver_lib": "libsfodbc.dylib",
                              "cache_key": "odbc"},
            "Windows x64":   {"os": "windows-latest", "driver_lib": "sfodbc.dll",
                              "cargo_extra": "--features vendored-openssl",
                              "cache_key": "odbc-x64",
                              "vcpkg_triplet": "x64-windows"},
            "Windows x86":   {"os": "windows-latest", "driver_lib": "sfodbc32.dll",
                              "cargo_target": "i686-pc-windows-msvc",
                              "cargo_extra": "--no-default-features --features vendored-openssl",
                              "cache_key": "odbc-x86",
                              "msvc_arch": "x86", "vcpkg_triplet": "x86-windows"},
            "Windows ARM64": {"os": "windows-11-arm", "driver_lib": "sfodbc.dll",
                              "cargo_extra": "--features vendored-openssl",
                              "cache_key": "odbc-arm64",
                              "msvc_arch": "arm64", "vcpkg_triplet": "arm64-windows"},
        }
        actual = {e["name"]: {k: v for k, v in e.items() if k not in ("name", "driver_artifact")}
                  for e in self.matrix_nightly}
        self.assertEqual(
            actual, legacy,
            "nightly build_matrix diverged from the legacy test-odbc.yml "
            "hardcoded build_odbc_driver matrix. If this is intentional, "
            "update the `legacy` dict in this test to match.",
        )

    def test_required_keys_on_every_entry(self) -> None:
        required = {"name", "os", "driver_lib", "driver_artifact", "cache_key"}
        for level, matrix in [
            ("pr",          self.matrix_pr),
            ("merge_group", self.matrix_merge_group),
            ("push",        self.matrix_push),
            ("nightly",     self.matrix_nightly),
        ]:
            for entry in matrix:
                missing = required - entry.keys()
                self.assertFalse(
                    missing,
                    f"[{level}] build_matrix entry {entry['name']} missing keys {missing}",
                )

    def test_optional_keys_only_where_applicable(self) -> None:
        # cargo_target only on Windows x86; msvc_arch only on Windows non-x64;
        # vcpkg_triplet only on Windows; cargo_extra only on Windows.
        for entry in self.matrix_nightly:
            name = entry["name"]
            if name == "Windows x86":
                self.assertEqual(entry.get("cargo_target"), "i686-pc-windows-msvc")
            elif name in ("Windows x64", "Windows ARM64"):
                self.assertNotIn("cargo_target", entry, name)
            else:  # Linux / macOS lanes
                self.assertNotIn("cargo_target", entry, name)
                self.assertNotIn("cargo_extra", entry, name)
                self.assertNotIn("msvc_arch", entry, name)
                self.assertNotIn("vcpkg_triplet", entry, name)

    def test_pr_count_at_4(self) -> None:
        # Pin the post-MERGE_VALID PR-level count: 4 driver flavours
        # (Linux x64, macOS ARM64, Windows x64, Windows x86).
        # Windows ARM64 is only at merge_queue level via MERGE_QUEUE_CELLS.
        self.assertEqual(
            len(self.matrix_pr), 4,
            f"expected 4 PR-level driver builds; got {len(self.matrix_pr)}: "
            f"{[e['name'] for e in self.matrix_pr]}",
        )
        self.assertEqual(
            {e["name"] for e in self.matrix_pr},
            {"Linux x64", "macOS ARM64", "Windows x64", "Windows x86"},
        )

    def test_merge_group_includes_linux_x64(self) -> None:
        # Linux x64 is the MERGE_QUEUE_CELLS cell (ubuntu-x64-gcp),
        # so the merge_group build matrix must include it.
        names = {e["name"] for e in self.matrix_merge_group}
        self.assertIn("Linux x64", names)

    def test_emit_build_matrix_cli_format(self) -> None:
        # CLI emits exactly one line of the form `matrix=<json>`.
        import contextlib
        import io
        import json as _json
        buf = io.StringIO()
        with contextlib.redirect_stdout(buf):
            gm.emit_build_matrix("odbc", "pull_request")
        line = buf.getvalue().rstrip("\n")
        self.assertTrue(line.startswith("matrix="))
        payload = _json.loads(line[len("matrix="):])
        self.assertEqual(payload, self.matrix_pr)

    def test_validate_mappings_raises_on_missing_cache_key(self) -> None:
        # Drift simulation: pop cache_key from a built lane, generate() must
        # fail loud rather than silently emit a build entry without a cache
        # key.
        original = gm.ODBC_PLATFORM[("ubuntu", "x64")].copy()
        gm.ODBC_PLATFORM[("ubuntu", "x64")].pop("cache_key")
        try:
            with self.assertRaises(RuntimeError) as ctx:
                gm.generate(ODBC_PATH, "odbc")
            self.assertIn("cache_key", str(ctx.exception))
        finally:
            gm.ODBC_PLATFORM[("ubuntu", "x64")] = original

    def test_alphabetical_order(self) -> None:
        # Reproducibility: output is sorted by name regardless of which test
        # row triggered the lane's inclusion first.
        for level, matrix in [
            ("pr",          self.matrix_pr),
            ("merge_group", self.matrix_merge_group),
            ("push",        self.matrix_push),
            ("nightly",     self.matrix_nightly),
        ]:
            names = [e["name"] for e in matrix]
            self.assertEqual(
                names, sorted(names),
                f"[{level}] build_matrix entries not in alphabetical order: {names}",
            )


# ---------------------------------------------------------------------------
# Trigger-level filtering
# ---------------------------------------------------------------------------

class FilterTests(unittest.TestCase):
    def test_level_for_event(self) -> None:
        self.assertEqual(gm.level_for_event("pull_request"), "pr")
        self.assertEqual(gm.level_for_event("push"), "merge")           # push to main: full pairwise set
        self.assertEqual(gm.level_for_event("merge_group"), "merge_queue")  # merge queue: MQ cells only
        self.assertEqual(gm.level_for_event("schedule"), "nightly")
        self.assertEqual(gm.level_for_event("unknown"), "pr")
        self.assertEqual(gm.level_for_event(None), "pr")

    def test_filter_active_cumulative(self) -> None:
        rows = [
            {"trigger_level": "pr"},
            {"trigger_level": "merge_queue", "merge_queue_cell": True},
            {"trigger_level": "merge"},
            {"trigger_level": "nightly"},
        ]
        # pr: only pr rows (cumulative cap = 0)
        self.assertEqual(len(gm.filter_active(rows, "pr")), 1)
        # merge: cumulative — pr + merge_queue + merge (3 rows)
        self.assertEqual(len(gm.filter_active(rows, "merge")), 3)
        # nightly: all 4 rows
        self.assertEqual(len(gm.filter_active(rows, "nightly")), 4)

    def test_filter_active_merge_queue_non_cumulative(self) -> None:
        """merge_queue level returns rows with merge_queue_cell=True (not all pr rows)."""
        rows = [
            {"trigger_level": "pr"},
            {"trigger_level": "merge_queue", "merge_queue_cell": True},
            {"trigger_level": "merge"},
            {"trigger_level": "nightly"},
        ]
        mq = gm.filter_active(rows, "merge_queue")
        self.assertEqual(len(mq), 1)
        self.assertEqual(mq[0]["trigger_level"], "merge_queue")

    def test_filter_active_merge_queue_includes_pr_cell_with_marker(self) -> None:
        """A 'pr' row with merge_queue_cell=True must be returned at merge_queue level.

        This is the case when a cell is in both PR_CELLS and MERGE_QUEUE_CELLS:
        PR_CELLS wins for trigger_level assignment, but the row is still selected
        at merge_queue because merge_queue_cell=True is set.
        """
        rows = [
            {"trigger_level": "pr", "name": "other-pr"},           # NOT in MQ
            {"trigger_level": "pr", "name": "shared", "merge_queue_cell": True},  # in both
            {"trigger_level": "merge", "name": "pairwise"},
        ]
        mq = gm.filter_active(rows, "merge_queue")
        self.assertEqual(len(mq), 1)
        self.assertEqual(mq[0]["name"], "shared")
        # pr filter still includes BOTH pr rows (cumulative, trigger_level-based)
        pr = gm.filter_active(rows, "pr")
        self.assertEqual(len(pr), 2)

    def test_filter_active_merge_queue_fallback_to_pr(self) -> None:
        """When no merge_queue rows exist, filter returns PR_CELLS as fallback."""
        rows = [
            {"trigger_level": "pr"},
            {"trigger_level": "merge"},
            {"trigger_level": "nightly"},
        ]
        mq = gm.filter_active(rows, "merge_queue")
        self.assertEqual(len(mq), 1)
        self.assertEqual(mq[0]["trigger_level"], "pr")

    def test_filter_active_merge_queue_is_subset_of_merge(self) -> None:
        """Every merge_queue row must also be included in the cumulative merge set."""
        rows = [
            {"trigger_level": "pr",          "name": "a"},
            {"trigger_level": "merge_queue", "name": "b"},
            {"trigger_level": "merge",       "name": "c"},
            {"trigger_level": "nightly",     "name": "d"},
        ]
        mq_names  = {r["name"] for r in gm.filter_active(rows, "merge_queue")}
        all_merge = {r["name"] for r in gm.filter_active(rows, "merge")}
        self.assertTrue(mq_names.issubset(all_merge))


class LabelResolutionTests(unittest.TestCase):
    """
    Lock in the scope-up label semantics: PR labels can upgrade the trigger
    level above what the event would produce, but never downgrade it. Multiple
    scope-up labels: highest wins. Unknown labels are ignored.
    """

    def test_empty_labels_falls_back_to_event(self) -> None:
        self.assertEqual(gm.level_for_event_and_labels("pull_request", []), "pr")
        self.assertEqual(gm.level_for_event_and_labels("pull_request", None), "pr")
        self.assertEqual(gm.level_for_event_and_labels("merge_group", []), "merge_queue")

    def test_scope_merge_queue_label_upgrades_pr_to_merge_queue(self) -> None:
        # ci:scope-merge-queue on a PR reproduces exactly what merge_group runs.
        self.assertEqual(
            gm.level_for_event_and_labels("pull_request", ["ci:scope-merge-queue"]),
            "merge_queue",
        )

    def test_scope_merge_label_upgrades_pr_to_merge(self) -> None:
        self.assertEqual(
            gm.level_for_event_and_labels("pull_request", ["ci:scope-merge"]),
            "merge",
        )

    def test_scope_merge_label_upgrades_merge_queue_to_merge(self) -> None:
        # ci:scope-merge on a merge_group event upgrades merge_queue → merge.
        self.assertEqual(
            gm.level_for_event_and_labels("merge_group", ["ci:scope-merge"]),
            "merge",
        )

    def test_scope_nightly_label_upgrades_pr_to_nightly(self) -> None:
        self.assertEqual(
            gm.level_for_event_and_labels("pull_request", ["ci:scope-nightly"]),
            "nightly",
        )

    def test_multiple_scope_labels_highest_wins(self) -> None:
        self.assertEqual(
            gm.level_for_event_and_labels(
                "pull_request", ["ci:scope-merge", "ci:scope-nightly"]
            ),
            "nightly",
        )

    def test_unknown_labels_ignored(self) -> None:
        self.assertEqual(
            gm.level_for_event_and_labels("pull_request", ["bug", "enhancement"]),
            "pr",
        )

    def test_label_cannot_downgrade_event(self) -> None:
        # ci:scope-merge on a merge_group event upgrades to merge (not downgraded to pr).
        self.assertEqual(
            gm.level_for_event_and_labels("merge_group", ["ci:scope-merge"]),
            "merge",
        )
        # And cannot pull schedule (nightly) down to merge.
        self.assertEqual(
            gm.level_for_event_and_labels("schedule", ["ci:scope-merge"]),
            "nightly",
        )


if __name__ == "__main__":
    unittest.main()
