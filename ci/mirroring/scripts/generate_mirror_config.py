#!/usr/bin/env python3
# generate_mirror_config.py — Generate Copybara mirroring infrastructure for a repo.
#
# Produces files in the caller's CWD that implement bidirectional mirroring
# between snowflake-eng/<REPO_NAME> (internal) and snowflakedb/<MIRROR_REPO_NAME>
# (public mirror). The two names may differ.
#
# Required env:
#   REPO_NAME             Internal repository name at snowflake-eng/
#                         (e.g., snowflake-connector-python)
#
# Optional env:
#   MIRROR_REPO_NAME      Mirror repository name at snowflakedb/ if different
#                         from REPO_NAME. Defaults to REPO_NAME.
#   EXTRA_EXCLUDED_PATHS  Comma-separated additional denylist entries
#                         (e.g., "legacy/**,vendor/**")
#   EXTRA_EXCLUDED_PATH_OVERRIDES
#                         Comma-separated path overrides to re-include from
#                         excluded prefixes (e.g., ".github/workflows/pre-commit.yml")
#   EXTRA_IMPORT_EXCLUDED_PATHS
#                         Comma-separated additional exclusions for the inbound
#                         import script (prepare_import_paths.py). These are paths
#                         excluded during PR import but not necessarily from outbound.
#   EXTRA_IMPORT_OVERRIDES
#                         Comma-separated path overrides for the import script
#                         (e.g., ".github/actions/cargo-cache/action.yml")
#   MAIN_BRANCH           Name of the primary branch ("main" or "master").
#                         Defaults to "main".
#   IS_SOURCE_OF_TRUTH_REPO
#                         "true" or "false" (default). Set to "true" for the
#                         repo that owns the generator (snowflake-eng/drivers) —
#                         it doesn't need to check itself for staleness.
#   INTERNAL_TOKEN_NAME   Name of the GitHub secret for snowflake-eng access.
#                         Defaults to "DRIVER_MIRROR_TOKEN".
#   MIRROR_TOKEN_NAME     Name of the GitHub secret for snowflakedb access.
#                         Defaults to "DRIVER_MIRROR_TOKEN_SNOWFLAKEDB".
#   SLACK_CHANNEL_ID      Slack channel ID for failure notifications (e.g.,
#                         "C092X1UAAMB"). If empty, the notify-on-failure job
#                         is omitted from mirror.yml.
#   SLACK_ONCALL_SUBTEAM_ID
#                         Slack subteam ID for on-call mention (e.g.,
#                         "S077RA1UXAS"). Used in the failure alert message.
#
# Usage:
#   REPO_NAME=snowflake-connector-python \
#   python3 ci/mirroring/scripts/generate_mirror_config.py
#
# Or via the mini bootstrap script (see sync-mirror-config.sh.template).

from __future__ import annotations

import json
import os
import stat
import sys
from pathlib import Path

TEMPLATES_DIR = Path(__file__).resolve().parent / "templates"

# ─── Derived constants ────────────────────────────────────────────────────────

COPYBARA_RELEASE = "v20260504"
COPYBARA_JAR_SHA256 = "a87af86f628d2754135fc6e3e0b5ee3f22aa781de4fc7a1039e0a69180576c0e"

# ─── Path lists (single source of truth) ─────────────────────────────────────
# These define what gets excluded from mirroring and importing.

NOMIRROR_PATHS = [
    "NOMIRROR/**",
    "**/NOMIRROR/**",
]

# Outbound mirror denylist: paths excluded from the public mirror.
# NOMIRROR_PATHS are prepended automatically by copy.bara.sky.
OUTBOUND_EXCLUDED_PATHS = [
    ".ai/**",
    ".cursor/**",
    ".claude/**",
    "scripts/mirror/**",
    "ci/mirroring/**",
    ".github/workflows/mirror.yml",
    ".github/workflows/mirror-inbound.yml",
    ".github/CODEOWNERS",
    ".github/workflows/security-signoff.yml",
    ".github/workflows/security-label.yml",
    ".github/security-partners.yml",
    "tests/bugs_analysis/**",
]

# Inbound import exclusions: paths skipped when importing a mirror PR.
# EXTRA_IMPORT_EXCLUDED_PATHS env var is prepended to it.
# The outbound denylist (copy.bara.sky) already controls what reaches the mirror;
# these are additional filters for the import direction only.
IMPORT_EXCLUDED_PATHS = [
    ".ai/**",
    ".cursor/**",
    ".claude/**",
    ".buildkite/**",
    ".ci/**",
    ".github/**",
    "ci/**",
    "scripts/**",
    "shell.nix"
]

# Import overrides: paths re-included from inside an IMPORT_EXCLUDED_PATHS prefix.
# Empty by default — populated entirely from EXTRA_IMPORT_OVERRIDES env var.
IMPORT_EXCLUDED_PATH_OVERRIDES: list[str] = []


# ─── Helpers ──────────────────────────────────────────────────────────────────

def read_template(name: str) -> str:
    """Read a template file from the templates directory."""
    path = TEMPLATES_DIR / name
    if not path.exists():
        print(f"ERROR: Template not found: {path}", file=sys.stderr)
        sys.exit(1)
    return path.read_text(encoding="utf-8")


def substitute(content: str, replacements: dict[str, str]) -> str:
    """Apply __VAR__ placeholder replacements to content."""
    for placeholder, value in replacements.items():
        content = content.replace(placeholder, value)
    return content


def insert_fragment(content: str, marker: str, fragment: str) -> str:
    """Replace an insertion-point marker with fragment content."""
    return content.replace(marker, fragment)


def parse_csv(csv_string: str) -> list[str]:
    if not csv_string.strip():
        return []
    return [item.strip() for item in csv_string.split(",") if item.strip()]


# Format full path lists as indented Starlark list entries.
def format_entries(paths: list[str]) -> str:
    return "".join(f'    "{p}",\n' for p in paths)


# ─── Main ─────────────────────────────────────────────────────────────────────


def main() -> None:
    # ─── Input validation ─────────────────────────────────────────────────
    repo_name = os.environ.get("REPO_NAME")
    if not repo_name:
        print("ERROR: REPO_NAME must be set (e.g., snowflake-connector-python)", file=sys.stderr)
        sys.exit(1)

    mirror_repo_name = os.environ.get("MIRROR_REPO_NAME") or repo_name
    extra_excluded_paths = os.environ.get("EXTRA_EXCLUDED_PATHS", "")
    extra_excluded_path_overrides = os.environ.get("EXTRA_EXCLUDED_PATH_OVERRIDES", "")
    extra_import_excluded_paths = os.environ.get("EXTRA_IMPORT_EXCLUDED_PATHS", "")
    extra_import_overrides = os.environ.get("EXTRA_IMPORT_OVERRIDES", "")
    main_branch = os.environ.get("MAIN_BRANCH", "main")
    is_source_of_truth = os.environ.get("IS_SOURCE_OF_TRUTH_REPO", "false") == "true"
    internal_token_name = os.environ.get("INTERNAL_TOKEN_NAME", "DRIVER_MIRROR_TOKEN")
    mirror_token_name = os.environ.get("MIRROR_TOKEN_NAME", "DRIVER_MIRROR_TOKEN_SNOWFLAKEDB")
    slack_channel_id = os.environ.get("SLACK_CHANNEL_ID", "")
    slack_oncall_subteam_id = os.environ.get("SLACK_ONCALL_SUBTEAM_ID", "")

    # ─── Build full path lists (base + extras) ─────────────────────────────
    outbound_excluded = OUTBOUND_EXCLUDED_PATHS + parse_csv(extra_excluded_paths)
    outbound_overrides = parse_csv(extra_excluded_path_overrides)
    import_excluded = IMPORT_EXCLUDED_PATHS + parse_csv(extra_import_excluded_paths)
    import_overrides = IMPORT_EXCLUDED_PATH_OVERRIDES + parse_csv(extra_import_overrides)

    # ─── Create output directories ───────────────────────────────────────
    Path("ci/mirroring").mkdir(parents=True, exist_ok=True)
    Path("ci/mirroring/scripts").mkdir(parents=True, exist_ok=True)
    Path(".github/workflows").mkdir(parents=True, exist_ok=True)

    # ─── Base placeholder replacements ────────────────────────────────────
    # Applied to all templates after fragment insertion.
    base_replacements = {
        "__REPO_NAME__": repo_name,
        "__MIRROR_REPO_NAME__": mirror_repo_name,
        "__COPYBARA_RELEASE__": COPYBARA_RELEASE,
        "__COPYBARA_JAR_SHA256__": COPYBARA_JAR_SHA256,
        "__MAIN_BRANCH__": main_branch,
        "__INTERNAL_TOKEN_NAME__": internal_token_name,
        "__MIRROR_TOKEN_NAME__": mirror_token_name,
    }

    # ─── File: ci/mirroring/Dockerfile.copybara ───────────────────────────
    content = read_template("template.Dockerfile.copybara")
    content = substitute(content, base_replacements)
    Path("ci/mirroring/Dockerfile.copybara").write_text(content, encoding="utf-8")

    # ─── File: ci/mirroring/import_paths.bara.sky ─────────────────────────
    content = read_template("template.import_paths.bara.sky")
    Path("ci/mirroring/import_paths.bara.sky").write_text(content, encoding="utf-8")

    # ─── File: ci/mirroring/copy.bara.sky ─────────────────────────────────
    content = read_template("template.copy.bara.sky")
    content = substitute(content, base_replacements)

    excluded_entries = format_entries(outbound_excluded)
    content = content.replace("__EXCLUDED_PATHS_ENTRIES__", excluded_entries)

    if outbound_overrides:
        content = content.replace("__EXCLUDED_PATH_OVERRIDES_ENTRIES__", format_entries(outbound_overrides))
    else:
        content = content.replace(
            "EXCLUDED_PATH_OVERRIDES = [\n__EXCLUDED_PATH_OVERRIDES_ENTRIES__]",
            "EXCLUDED_PATH_OVERRIDES = []",
        )

    Path("ci/mirroring/copy.bara.sky").write_text(content, encoding="utf-8")

    # ─── File: .github/workflows/mirror.yml ───────────────────────────────
    content = read_template("template.mirror.yml")

    # Insert staleness job fragment. mirror-main always depends on it (even
    # when the job is a no-op for source-of-truth repos) so the DAG is uniform.
    if is_source_of_truth:
        staleness_fragment = read_template("template.mirror_staleness_noop_job.yml")
    else:
        staleness_fragment = read_template("template.mirror_staleness_job.yml")

    content = insert_fragment(content, "# __INSERT_STALENESS_JOB__\n", staleness_fragment)
    content = insert_fragment(content, "# __INSERT_STALENESS_NEEDS__\n", "    needs: check-generator-staleness\n")

    # Insert Slack notification job fragment (conditional)
    if slack_channel_id:
        slack_fragment = read_template("template.mirror_slack_job.yml")
        content = insert_fragment(content, "# __INSERT_SLACK_JOB__\n", slack_fragment)
    else:
        content = insert_fragment(content, "# __INSERT_SLACK_JOB__\n", "")

    content = substitute(content, base_replacements)

    # Slack-specific placeholders (only present if slack job was inserted)
    if slack_channel_id:
        content = content.replace("__SLACK_CHANNEL_ID__", slack_channel_id)
        content = content.replace("__SLACK_ONCALL_SUBTEAM_ID__", slack_oncall_subteam_id)

    Path(".github/workflows/mirror.yml").write_text(content, encoding="utf-8")

    # ─── File: .github/workflows/mirror-inbound.yml ───────────────────────
    content = read_template("template.mirror-inbound.yml")
    content = substitute(content, base_replacements)
    Path(".github/workflows/mirror-inbound.yml").write_text(content, encoding="utf-8")

    # ─── File: .github/workflows/close-imported-pr.yml ────────────────────
    content = read_template("template.close-imported-pr.yml")
    content = substitute(content, base_replacements)
    Path(".github/workflows/close-imported-pr.yml").write_text(content, encoding="utf-8")

    # ─── File: ci/mirroring/mirror_config.json ────────────────────────────
    # Runtime config for prepare_import_paths.py (static script).
    # Uses the same lists computed above from the module-level constants + extras.
    mirror_config = {
        "mirror_repo": f"snowflakedb/{mirror_repo_name}",
        "nomirror_paths": NOMIRROR_PATHS,
        "import_excluded_paths": import_excluded,
        "import_excluded_path_overrides": import_overrides,
    }

    Path("ci/mirroring/mirror_config.json").write_text(
        json.dumps(mirror_config, indent=2) + "\n",
        encoding="utf-8",
    )

    # ─── File: ci/mirroring/mirroring.md ──────────────────────────────────
    if is_source_of_truth:
        content = read_template("template.mirroring_source_of_truth.md")
        # The source-of-truth template has a regeneration command example
        # that includes the raw EXTRA_IMPORT_OVERRIDES value.
        content = content.replace("__EXTRA_IMPORT_OVERRIDES_RAW__", extra_import_overrides)
        content = content.replace("__SLACK_CHANNEL_ID__", slack_channel_id)
        content = content.replace("__SLACK_ONCALL_SUBTEAM_ID__", slack_oncall_subteam_id)
    else:
        content = read_template("template.mirroring_consumer.md")

    content = substitute(content, base_replacements)
    Path("ci/mirroring/mirroring.md").write_text(content, encoding="utf-8")

    # ─── File: ci/mirroring/scripts/prepare_import_paths.py ──────────────────
    # Copy the static version from alongside the generator.
    source = Path(__file__).resolve().parent / "prepare_import_paths.py"
    dest = Path("ci/mirroring/scripts/prepare_import_paths.py")
    dest.write_text(source.read_text(encoding="utf-8"), encoding="utf-8")
    # Make executable
    dest.chmod(dest.stat().st_mode | stat.S_IXUSR | stat.S_IXGRP | stat.S_IXOTH)

    # ─── Summary ─────────────────────────────────────────────────────────
    print()
    print(f"Generated mirror infrastructure for {repo_name}:")
    print("  ci/mirroring/copy.bara.sky")
    print("  ci/mirroring/import_paths.bara.sky")
    print("  ci/mirroring/Dockerfile.copybara")
    print("  ci/mirroring/mirror_config.json")
    print("  ci/mirroring/scripts/prepare_import_paths.py")
    print("  .github/workflows/mirror.yml")
    print("  .github/workflows/mirror-inbound.yml")
    print("  .github/workflows/close-imported-pr.yml")
    print("  ci/mirroring/mirroring.md")
    print()
    print("Next steps:")
    print(f"  1. Review and commit the generated files")
    print(f"  2. Provision secrets: {internal_token_name}, {mirror_token_name}")
    print("  3. Sync public and private repositories so their contents are the same. With the exception of repository specific values that should not be mirrored")
    print("  4. Run first outbound: workflow_dispatch mirror.yml with last_rev=<first-public-sha>")
    print("  5. Create test PR on the public repository to verify inbound mirroring")
    print("  6. Create 'ok-to-import' label on mirror repository and apply it to test PR")
    print("  7. Run mirror-inbound job on private repository with pr_number=<your-test-PR-number>")
    print("  8. Merge test PR and run mirroring job to verify it gets pushed properly to public repository")
    print("  9. Manually deploy close-imported-pr.yml on private repository and verify test PR got closed")


if __name__ == "__main__":
    main()
