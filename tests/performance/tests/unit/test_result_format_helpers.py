"""Unit tests for JSON/Arrow result-format helpers in the perf harness."""
from pathlib import Path
from types import SimpleNamespace

import pytest

from runner.local_compare import compare_with_history
from runner.modes import wiremock_runner
from runner.result_format import (
    comparable_tags_for_upload,
    is_arrow_baseline_run,
    merge_arrow_baseline_runs,
    read_result_format_tag,
)
from runner.test_types import PerfTestType

from conftest import _prepare_setup_queries, _skip_json_unsupported

pytestmark = pytest.mark.supports_json


def test_read_result_format_tag_defaults_to_arrow_when_missing(tmp_path):
    assert read_result_format_tag(tmp_path) == "ARROW"


def test_read_result_format_tag_reads_run_config(tmp_path):
    (tmp_path / "run_config.json").write_text('{"result_format": "json"}')
    assert read_result_format_tag(tmp_path) == "JSON"


def test_read_result_format_tag_treats_invalid_json_as_arrow(tmp_path):
    (tmp_path / "run_config.json").write_text("{")
    assert read_result_format_tag(tmp_path) == "ARROW"


def test_comparable_tags_omit_result_format_on_arrow():
    tags = ["DRIVER=python", "RESULT_FORMAT=ARROW", "REGION=us-west-2"]
    assert comparable_tags_for_upload(tags, "ARROW") == [
        "DRIVER=python",
        "REGION=us-west-2",
    ]


def test_comparable_tags_keep_result_format_on_json():
    tags = ["DRIVER=python", "RESULT_FORMAT=JSON"]
    assert comparable_tags_for_upload(tags, "json") == tags


def test_prepare_setup_queries_sets_universal_driver_format():
    queries = _prepare_setup_queries(PerfTestType.SELECT, "{}", result_format="json")
    assert "alter session set universal_driver_query_result_format = 'JSON'" in queries
    assert "alter session set python_connector_query_result_format = 'JSON'" in queries
    assert not any("c_api_query_result_format" in q for q in queries)


def test_prepare_setup_queries_skips_format_for_put_get():
    params = '{"testconnection": {"database": "DB"}}'
    queries = _prepare_setup_queries(PerfTestType.PUT_GET, params, result_format="json")
    assert queries == ["USE DATABASE DB"]


class _FakeConfig:
    def __init__(self, result_format="json", driver="core"):
        self._opts = {
            "--result-format": result_format,
            "--driver": driver,
            "--driver-type": "universal",
        }

    def getoption(self, name):
        return self._opts.get(name)


class _FakeItem:
    def __init__(self, markers=(), fetch_mode=None, result_format="json", driver="core"):
        self.config = _FakeConfig(result_format, driver)
        self._markers = {name: object() for name in markers}
        self.name = "test_fake"
        self.callspec = SimpleNamespace(
            params={"fetch_mode": fetch_mode} if fetch_mode else {}
        )

    def get_closest_marker(self, name):
        return self._markers.get(name)


def test_skip_json_unsupported_skips_unmarked(monkeypatch):
    monkeypatch.delenv("PERF_RESULT_FORMAT", raising=False)
    monkeypatch.delenv("PERF_DRIVER", raising=False)
    with pytest.raises(pytest.skip.Exception, match="not marked supports_json"):
        _skip_json_unsupported(_FakeItem())


def test_skip_json_unsupported_skips_nodejs_even_when_marked(monkeypatch):
    monkeypatch.delenv("PERF_RESULT_FORMAT", raising=False)
    monkeypatch.delenv("PERF_DRIVER", raising=False)
    with pytest.raises(pytest.skip.Exception, match="nodejs not supported"):
        _skip_json_unsupported(
            _FakeItem(markers=("supports_json",), driver="nodejs")
        )


def test_skip_json_unsupported_allows_marked_core(monkeypatch):
    monkeypatch.delenv("PERF_RESULT_FORMAT", raising=False)
    monkeypatch.delenv("PERF_DRIVER", raising=False)
    _skip_json_unsupported(_FakeItem(markers=("supports_json",), driver="core"))


def test_is_arrow_baseline_run_treats_untagged_as_arrow():
    assert is_arrow_baseline_run(SimpleNamespace(tags=["DRIVER=python"]))
    assert is_arrow_baseline_run(SimpleNamespace(tags=["RESULT_FORMAT=ARROW"]))
    assert not is_arrow_baseline_run(SimpleNamespace(tags=["RESULT_FORMAT=JSON"]))


def test_merge_arrow_baseline_uses_tagged_when_enough():
    tagged = [SimpleNamespace(run_key=i, tags=["RESULT_FORMAT=ARROW"]) for i in range(3)]
    assert merge_arrow_baseline_runs(tagged, [], 3) == tagged


def test_merge_arrow_baseline_falls_back_to_untagged():
    tagged = [SimpleNamespace(run_key=1, tags=["RESULT_FORMAT=ARROW"])]
    unfiltered = [
        SimpleNamespace(run_key=1, tags=["RESULT_FORMAT=ARROW"]),
        SimpleNamespace(run_key=2, tags=[]),
        SimpleNamespace(run_key=3, tags=["RESULT_FORMAT=JSON"]),
        SimpleNamespace(run_key=4, tags=[]),
    ]
    runs = merge_arrow_baseline_runs(tagged, unfiltered, 3)
    assert [r.run_key for r in runs] == [1, 2, 4]


def test_get_mappings_dir_reuses_legacy_arrow_layout(tmp_path, monkeypatch):
    monkeypatch.setattr(wiremock_runner, "MAPPINGS_BASE_DIR", tmp_path)
    legacy = tmp_path / "run_1" / "select_string"
    legacy.mkdir(parents=True)
    mappings, skip = wiremock_runner._get_mappings_dir(
        "select_string",
        tmp_path / "results" / "run_1",
        reuse_mappings_dir="run_1",
        result_format="arrow",
    )
    assert skip is True
    assert mappings == legacy.resolve()


def test_get_mappings_dir_does_not_reuse_legacy_layout_for_json(tmp_path, monkeypatch):
    monkeypatch.setattr(wiremock_runner, "MAPPINGS_BASE_DIR", tmp_path)
    (tmp_path / "run_1" / "select_string").mkdir(parents=True)
    with pytest.raises(RuntimeError, match="Reuse mappings directory not found"):
        wiremock_runner._get_mappings_dir(
            "select_string",
            tmp_path / "results" / "run_1",
            reuse_mappings_dir="run_1",
            result_format="json",
        )


def _write_result_csv(path: Path, fetch_s: float):
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(f"timestamp_ms,query_s,fetch_s\n1,0.1,{fetch_s}\n")


def test_compare_with_history_ignores_other_result_format(tmp_path):
    results_base = tmp_path / "results"
    arrow_run = results_base / "run_20260101_000000"
    json_run = results_base / "run_20260102_000000"
    arrow_run.mkdir(parents=True)
    json_run.mkdir(parents=True)
    (arrow_run / "run_config.json").write_text('{"result_format": "arrow"}')
    (json_run / "run_config.json").write_text('{"result_format": "json"}')
    _write_result_csv(
        arrow_run / "universal" / "select_string_1_row" / "select_string_1_row_core_1.csv",
        1.0,
    )
    current = json_run / "universal" / "select_string_1_row" / "select_string_1_row_core_2.csv"
    _write_result_csv(current, 2.0)

    comp = compare_with_history(
        [current],
        json_run,
        "select_string_1_row",
        "core",
        None,
        result_format="json",
    )
    assert comp is not None
    assert comp["history"] == []
    assert comp["current_median"] == 2.0
