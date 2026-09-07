"""Run-level result-format tag helpers shared by upload, compare, and pytest."""
import json
from pathlib import Path
from typing import Optional


def read_result_format_tag(results_dir: Path) -> str:
    """RESULT_FORMAT tag for this run (ARROW default for untagged/legacy dirs)."""
    config_file = results_dir / "run_config.json"
    if config_file.exists():
        try:
            fmt = json.loads(config_file.read_text()).get("result_format", "arrow")
        except (json.JSONDecodeError, OSError):
            fmt = "arrow"
        if isinstance(fmt, str) and fmt.lower() in ("arrow", "json"):
            return fmt.upper()
    return "ARROW"


def comparable_tags_for_upload(tags: list[str], result_format: str) -> list[str]:
    """Arrow continues the untagged series; JSON is a new comparable identity."""
    if result_format.upper() == "JSON":
        return list(tags)
    return [t for t in tags if not t.startswith("RESULT_FORMAT=")]


def result_format_from_run_tags(tags) -> Optional[str]:
    for tag in tags:
        if tag.startswith("RESULT_FORMAT="):
            return tag.split("=", 1)[1].upper()
    return None


def is_arrow_baseline_run(run_info) -> bool:
    fmt = result_format_from_run_tags(run_info.tags)
    return fmt is None or fmt == "ARROW"


def merge_arrow_baseline_runs(tagged_runs, unfiltered_runs, num_runs: int):
    """Prefer RESULT_FORMAT=ARROW hits; fill from untagged/Arrow unfiltered runs."""
    tagged = list(tagged_runs)
    if len(tagged) >= num_runs:
        return tagged[:num_runs]
    seen = {run.run_key for run in tagged}
    fallback = [
        run
        for run in unfiltered_runs
        if run.run_key not in seen and is_arrow_baseline_run(run)
    ]
    return (tagged + fallback)[:num_runs]
