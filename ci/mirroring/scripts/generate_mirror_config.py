#!/usr/bin/env python3
# generate_mirror_config.py — Generate Copybara mirroring infrastructure for a repo.
#
# Produces files in the caller's CWD that implement bidirectional mirroring
# between snowflake-eng/<REPO_NAME> (internal) and snowflakedb/<MIRROR_REPO_NAME>
# (public mirror). The two names may differ.
#
# Required setting (environment variable or --config JSON key):
#   REPO_NAME             Internal repository name at snowflake-eng/
#                         (e.g., snowflake-connector-python)
#
# Optional settings:
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
# Settings can instead be supplied as string values in a JSON file:
#   python3 ci/mirroring/scripts/generate_mirror_config.py --config path/to/config.json
#
# Or via the mini bootstrap script (see sync-mirror-config.sh.template).

from __future__ import annotations

import argparse
import json
import os
import stat
import sys
from pathlib import Path

TEMPLATES_DIR = Path(__file__).resolve().parent / "templates"
GENERATED_FILES_MANIFEST = Path("ci/mirroring/generated_files.json")

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
    # Security-context bundle: architecture, trust boundaries, threat model,
    # and invariant status. Internal review tooling — not for the public mirror.
    ".security/**",
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


def load_config() -> dict[str, str]:
    parser = argparse.ArgumentParser()
    parser.add_argument(
        "--config",
        type=Path,
        help="JSON file whose values override generator environment variables",
    )
    args = parser.parse_args()
    if args.config is None:
        return {}

    try:
        config = json.loads(args.config.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as error:
        parser.error(f"cannot read generator config {args.config}: {error}")

    if not isinstance(config, dict) or not all(
        isinstance(key, str) and isinstance(value, str) for key, value in config.items()
    ):
        parser.error("generator config must be a JSON object with string keys and values")
    return config


def validate_generated_path(path: Path) -> Path:
    if path.is_absolute() or ".." in path.parts or path == Path("."):
        raise ValueError(f"generated path must be a safe relative path: {path}")
    return path


class GeneratedFiles:
    def __init__(self) -> None:
        self.paths: set[Path] = set()
        self.previous_paths = self._read_manifest()

    @staticmethod
    def _read_manifest() -> set[Path]:
        if not GENERATED_FILES_MANIFEST.exists():
            return set()
        try:
            manifest = json.loads(GENERATED_FILES_MANIFEST.read_text(encoding="utf-8"))
        except (OSError, json.JSONDecodeError) as error:
            print(f"ERROR: Cannot read {GENERATED_FILES_MANIFEST}: {error}", file=sys.stderr)
            sys.exit(1)
        if not isinstance(manifest, list) or not all(isinstance(path, str) for path in manifest):
            print(
                f"ERROR: {GENERATED_FILES_MANIFEST} must contain a JSON list of paths",
                file=sys.stderr,
            )
            sys.exit(1)
        try:
            return {validate_generated_path(Path(path)) for path in manifest}
        except ValueError as error:
            print(f"ERROR: Invalid generated-files manifest: {error}", file=sys.stderr)
            sys.exit(1)

    def write_text(self, path: str | Path, content: str, *, executable: bool = False) -> None:
        output_path = validate_generated_path(Path(path))
        output_path.parent.mkdir(parents=True, exist_ok=True)
        output_path.write_text(content, encoding="utf-8")
        if executable:
            output_path.chmod(
                output_path.stat().st_mode | stat.S_IXUSR | stat.S_IXGRP | stat.S_IXOTH
            )
        self.paths.add(output_path)

    def finalize(self) -> tuple[Path, ...]:
        self.paths.add(GENERATED_FILES_MANIFEST)
        obsolete_paths = sorted(
            path for path in self.previous_paths - self.paths if path.exists() or path.is_symlink()
        )
        if obsolete_paths:
            print(
                "ERROR: Files listed in the previous manifest are no longer generated:",
                file=sys.stderr,
            )
            for path in obsolete_paths:
                print(f"  {path}", file=sys.stderr)
            print("Delete these obsolete files and regenerate.", file=sys.stderr)
            sys.exit(1)

        paths = tuple(sorted(self.paths))
        GENERATED_FILES_MANIFEST.write_text(
            json.dumps([path.as_posix() for path in paths], indent=2) + "\n",
            encoding="utf-8",
        )
        return paths


# ─── Main ─────────────────────────────────────────────────────────────────────


def main() -> None:
    config = load_config()

    def setting(name: str, default: str = "") -> str:
        return config.pop(name, os.environ.get(name, default))

    # ─── Input validation ─────────────────────────────────────────────────
    repo_name = setting("REPO_NAME")
    if not repo_name:
        print("ERROR: REPO_NAME must be set (e.g., snowflake-connector-python)", file=sys.stderr)
        sys.exit(1)

    mirror_repo_name = setting("MIRROR_REPO_NAME") or repo_name
    extra_excluded_paths = setting("EXTRA_EXCLUDED_PATHS")
    extra_excluded_path_overrides = setting("EXTRA_EXCLUDED_PATH_OVERRIDES")
    extra_import_excluded_paths = setting("EXTRA_IMPORT_EXCLUDED_PATHS")
    extra_import_overrides = setting("EXTRA_IMPORT_OVERRIDES")
    main_branch = setting("MAIN_BRANCH", "main")
    is_source_of_truth = setting("IS_SOURCE_OF_TRUTH_REPO", "false") == "true"
    internal_token_name = setting("INTERNAL_TOKEN_NAME", "DRIVER_MIRROR_TOKEN")
    slack_channel_id = setting("SLACK_CHANNEL_ID")
    slack_oncall_subteam_id = setting("SLACK_ONCALL_SUBTEAM_ID")
    if config:
        print(
            f"ERROR: Unknown generator config settings: {', '.join(sorted(config))}",
            file=sys.stderr,
        )
        sys.exit(1)
    generated_files = GeneratedFiles()

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
    }

    # ─── File: ci/mirroring/Dockerfile.copybara ───────────────────────────
    content = read_template("template.Dockerfile.copybara")
    content = substitute(content, base_replacements)
    generated_files.write_text("ci/mirroring/Dockerfile.copybara", content)

    # ─── File: ci/mirroring/import_paths.bara.sky ─────────────────────────
    content = read_template("template.import_paths.bara.sky")
    generated_files.write_text("ci/mirroring/import_paths.bara.sky", content)

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

    generated_files.write_text("ci/mirroring/copy.bara.sky", content)

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

    generated_files.write_text(".github/workflows/mirror.yml", content)

    # ─── File: .github/workflows/mirror-inbound.yml ───────────────────────
    content = read_template("template.mirror-inbound.yml")
    content = substitute(content, base_replacements)
    generated_files.write_text(".github/workflows/mirror-inbound.yml", content)

    # ─── File: .github/workflows/close-imported-pr.yml ────────────────────
    content = read_template("template.close-imported-pr.yml")
    content = substitute(content, base_replacements)
    generated_files.write_text(".github/workflows/close-imported-pr.yml", content)

    # ─── File: ci/mirroring/mirror_config.json ────────────────────────────
    # Runtime config for prepare_import_paths.py (static script).
    # Uses the same lists computed above from the module-level constants + extras.
    mirror_config = {
        "mirror_repo": f"snowflakedb/{mirror_repo_name}",
        "nomirror_paths": NOMIRROR_PATHS,
        "import_excluded_paths": import_excluded,
        "import_excluded_path_overrides": import_overrides,
    }

    generated_files.write_text(
        "ci/mirroring/mirror_config.json",
        json.dumps(mirror_config, indent=2) + "\n",
    )

    # ─── File: ci/mirroring/mirroring.md ──────────────────────────────────
    if is_source_of_truth:
        content = read_template("template.mirroring_source_of_truth.md")
    else:
        content = read_template("template.mirroring_consumer.md")

    content = substitute(content, base_replacements)
    generated_files.write_text("ci/mirroring/mirroring.md", content)

    # ─── File: ci/mirroring/scripts/prepare_import_paths.py ──────────────────
    # Copy the static version from alongside the generator.
    source = Path(__file__).resolve().parent / "prepare_import_paths.py"
    generated_files.write_text(
        "ci/mirroring/scripts/prepare_import_paths.py",
        source.read_text(encoding="utf-8"),
        executable=True,
    )

    # ─── Summary ─────────────────────────────────────────────────────────
    output_paths = generated_files.finalize()
    print()
    print(f"Generated mirror infrastructure for {repo_name}:")
    for path in output_paths:
        print(f"  {path}")
    print()
    print("Next steps:")
    print(f"  1. Review and commit the generated files")
    print(f"  2. Provision secrets: {internal_token_name} (internal PAT), MIRRORING_APP_ID, MIRRORING_APP_PRIVATE_KEY (GitHub App credentials)")
    print("  3. Sync public and private repositories so their contents are the same. With the exception of repository specific values that should not be mirrored")
    print("  4. Run first outbound: workflow_dispatch mirror.yml with last_rev=<first-public-sha>")
    print("  5. Create test PR on the public repository to verify inbound mirroring")
    print("  6. Create 'ok-to-import' label on mirror repository and apply it to test PR")
    print("  7. Run mirror-inbound job on private repository with pr_number=<your-test-PR-number>")
    print("  8. Merge test PR and run mirroring job to verify it gets pushed properly to public repository")
    print("  9. Manually deploy close-imported-pr.yml on private repository and verify test PR got closed")


if __name__ == "__main__":
    main()
