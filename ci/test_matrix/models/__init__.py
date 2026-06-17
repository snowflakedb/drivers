"""
Coverage models for ci/test_matrix.

Each driver has a module here (`<driver>.py`) declaring four top-level names.
The generator imports the module, validates the shape, and feeds the rest
of the pipeline.

Schema
------

PARAMS: dict[str, list[str]]
    Parameter name -> allowed values. Order is preserved by the loader and
    affects which specific cells land in the merge-level pairwise cover.
    Nightly coverage is stable regardless of order (it's the full
    constraint-valid cartesian product). Pick an order and keep it stable.

CONSTRAINTS: list[Callable[[dict[str, str]], bool]]
    Each constraint is a Python predicate that takes a candidate combo
    (dict of param name -> value) and returns True if the combo is valid.
    A combo is kept iff every predicate returns True.

    The generator iterates over the full cartesian product of PARAMS and
    feeds every candidate through CONSTRAINTS — so the recommended shape
    is a single `is_valid(c)` block-list that names what to *forbid* with
    `return False`, and lets everything else fall through to `return True`:

        def is_valid(c):
            # Forbid: Windows-on-ARM has no CPython 3.10 build.
            if c["OS"] == "windows" and c["Arch"] == "arm" and c["PyVersion"] == "3.10":
                return False
            # Forbid: no pyarrow win_arm64 wheel.
            if c["OS"] == "windows" and c["Arch"] == "arm" and c["HatchEnv"] == "test-pandas":
                return False
            return True

        CONSTRAINTS = [is_valid]

    Why block-list (forbid-only) rather than allow-list (enumerate-allowed):
    a block-list is *drift-resistant* — adding a new value to PARAMS (e.g.
    a new PyVersion) is automatically accepted, since no rule names it. An
    allow-list would silently drop the new value until the rule is updated.
    Encode each rejection with a positive `==` check on the forbidden value
    so the rule reads as "this specific combo is forbidden because <reason>".

PR_CELLS: list[dict[str, str]]
    Explicit cells that always run on every PR. Each cell must list a value
    for every declared parameter (loader rejects missing or extra keys).

MERGE_QUEUE_CELLS: list[dict[str, str]]   (optional, default [])
    Explicit cells that run ONLY on the merge queue (merge_group event,
    trigger_level="merge_queue"). They do NOT run on PRs (unless also in
    PR_CELLS) and are NOT included in the pairwise set that runs at push-to-main.

    Semantics differ from PR_CELLS in one critical way: filter_active at
    "merge_queue" level is NON-CUMULATIVE — it returns ONLY these cells, not
    PR_CELLS. This makes the merge queue a focused, fast validation gate while
    push-to-main runs the full matrix (PR_CELLS + MERGE_QUEUE_CELLS + pairwise).

    If MERGE_QUEUE_CELLS is empty or absent, the merge queue falls back to
    running PR_CELLS (preserving backward compatibility for models that have
    not yet defined an explicit merge-queue configuration).

    If a cell appears in both PR_CELLS and MERGE_QUEUE_CELLS, PR_CELLS wins
    and the cell runs at trigger_level="pr" (cumulative at all scopes).

    MERGE_VALID does not gate these — they bypass the pairwise block-list and
    run unconditionally at merge_queue. Cells listed here are also excluded
    from the pairwise candidate pool so the greedy solver does not redundantly
    select them and emit duplicate rows at the push-to-main ("merge") level.

    Use MERGE_VALID to prevent pairwise from generating similar lanes that
    would duplicate coverage already provided by MERGE_QUEUE_CELLS:

        MERGE_QUEUE_CELLS = [
            # windows-arm: fast gate on azure only; other clouds at push-to-main.
            {"OS": "windows", "Arch": "arm", "Cloud": "azure"},
        ]
        # Also add to merge_valid() to block arm-aws and arm-gcp from pairwise.

MERGE_VALID: list[Callable[[dict[str, str]], bool]]   (optional, default [])
    Pairwise-only block-list. Same shape and semantics as CONSTRAINTS
    (return True = keep, return False = forbid). Predicates here gate
    *only* the merge-level pairwise cover — combos rejected by MERGE_VALID
    still appear at nightly via the unfiltered cartesian product, and PR
    cells listed in PR_CELLS still run regardless. Use for lanes whose
    runner cost or availability makes them unsuitable for every-MQ-run
    execution but where nightly should still catch interaction bugs:

        def merge_valid(c):
            # Limited macOS runner availability — keep Intel mac off the MQ.
            if c["OS"] == "macos" and c["Arch"] == "x64": return False
            return True

        MERGE_VALID = [merge_valid]

    Mapping coverage is unaffected: validate_mappings runs over the full
    constraint-valid combo set, so a MERGE_VALID-blocked combo still needs
    its (OS, Arch) row in the mappings tables.

JSON_CELLS: dict[str, list[dict[str, str]]]
    Maps trigger level ("pr" / "merge" / "nightly") to cells that get
    duplicated with result_format="json" at that level. Same per-cell
    completeness rule as PR_CELLS.
"""
