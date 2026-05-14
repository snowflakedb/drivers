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
            params, constraints, _merge_valid, pr_cells, json_cells = gm.load_model(path)
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
            _params, _c, _mv, _pr, json_cells = gm.load_model(path)
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
            _, constraints, _mv, _, _ = gm.load_model(path)
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
            _params, _constraints, merge_valid, _pr, _json = gm.load_model(path)
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
            _params, _c, merge_valid, _pr, _json = gm.load_model(path)
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

    def test_no_duplicate_macos_rows_at_mq(self) -> None:
        # Regression test for the python.py MERGE_VALID/PR_CELLS sync invariant.
        # The merge_valid() predicate pins macos-arm to one combo and macos-x64
        # to another. The macOS entry in PR_CELLS MUST match the macos-arm pin
        # exactly; if they diverge, the PR cell ships at trigger_level=pr while
        # pairwise ships a *different* macos-arm row at trigger_level=merge,
        # giving two macOS-arm jobs at the merge queue (defeating the point of
        # MERGE_VALID). This test fails loudly when the sync drifts.
        mq_macos = [
            r for r in self.gha
            if r["os"] == "macos-latest" and r["trigger_level"] in ("pr", "merge")
        ]
        self.assertEqual(
            len(mq_macos), 1,
            f"expected exactly one macos-arm row at merge-queue scope; "
            f"got {[r['name'] for r in mq_macos]}. "
            f"This usually means the PR_CELLS macOS entry has drifted "
            f"away from the macos-arm pin in merge_valid(): both reach "
            f"the merge level and macOS runs twice per MQ build.",
        )
        mq_intel = [
            r for r in self.gha
            if r["os"] == "macos-15-intel" and r["trigger_level"] in ("pr", "merge")
        ]
        self.assertEqual(
            len(mq_intel), 1,
            f"expected exactly one macos-x64 row at merge-queue scope; "
            f"got {[r['name'] for r in mq_intel]}",
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
                params, constraints, _mv, pr_cells, _json_cells = gm.load_model(path)
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
    Locks the JSON cell count at the merge trigger level across odbc + python.

    Main has 1 ODBC json cell (every PR — appears at merge cumulatively) plus
    2 Python json cells (gated on merge). This test pins the combined merge-
    level JSON count at 3 so future model edits don't silently change the
    JSON-format coverage.
    """

    @classmethod
    def setUpClass(cls) -> None:
        cls.odbc_gha = gm.generate(ODBC_PATH, "odbc")
        cls.py_gha = gm.generate(PYTHON_PATH, "python")

    def test_combined_json_count_at_merge_level(self) -> None:
        # `merge` is cumulative — it includes pr cells plus rows whose
        # trigger_level is `pr` or `merge`.
        active_levels = {"pr", "merge"}
        json_at_merge = [
            r for r in (self.odbc_gha + self.py_gha)
            if r.get("result_format") == "json" and r["trigger_level"] in active_levels
        ]
        self.assertEqual(
            len(json_at_merge), 3,
            f"expected 3 json cells active at merge, got {len(json_at_merge)}: "
            f"{[r['name'] for r in json_at_merge]}",
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
# Trigger-level filtering
# ---------------------------------------------------------------------------

class FilterTests(unittest.TestCase):
    def test_level_for_event(self) -> None:
        self.assertEqual(gm.level_for_event("pull_request"), "pr")
        self.assertEqual(gm.level_for_event("push"), "merge")
        self.assertEqual(gm.level_for_event("merge_group"), "merge")
        self.assertEqual(gm.level_for_event("schedule"), "nightly")
        self.assertEqual(gm.level_for_event("unknown"), "pr")
        self.assertEqual(gm.level_for_event(None), "pr")

    def test_filter_active_cumulative(self) -> None:
        rows = [
            {"trigger_level": "pr"},
            {"trigger_level": "merge"},
            {"trigger_level": "nightly"},
        ]
        self.assertEqual(len(gm.filter_active(rows, "pr")), 1)
        self.assertEqual(len(gm.filter_active(rows, "merge")), 2)
        self.assertEqual(len(gm.filter_active(rows, "nightly")), 3)


if __name__ == "__main__":
    unittest.main()
