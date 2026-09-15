"""Unit tests for PR-check percent threshold (no Benchstore).

The 10-run main median is the production stabilizer for the date-test
window shift (#1991 / #1872). These tests only cover the percent rule.
"""
from runner.pr_smoke_reg_detection.regression_check import (
    check_regression,
    exceeds_regression_threshold,
)


def test_percent_over_threshold_is_a_regression():
    # Same numbers as the #1991 date flake vs a 3-run baseline.
    # 10-run median is what should absorb that in Benchstore; the % rule still flags it.
    assert exceeds_regression_threshold(0.120, 0.102, 10.0)
    assert exceeds_regression_threshold(1.04, 0.94, 10.0)
    # number_1M ~30ms: +50% must still flag (a 25ms floor would have hidden this).
    assert exceeds_regression_threshold(0.045, 0.030, 10.0)


def test_percent_under_threshold_is_not_a_regression():
    assert not exceeds_regression_threshold(2.1, 2.0, 10.0)
    assert not exceeds_regression_threshold(0.110, 0.102, 10.0)


def test_zero_or_missing_baseline_is_not_a_regression():
    assert not exceeds_regression_threshold(1.0, 0.0, 10.0)
    results = check_regression({"select_date_1M_arrow_recorded_http": 0.120}, {}, 10.0)
    assert results == []


def test_check_regression_uses_percent_only():
    results = check_regression(
        {
            "select_date_1M_arrow_recorded_http": 0.120,
            "select_string_1M_arrow_recorded_http": 0.106,
        },
        {
            "select_date_1M_arrow_recorded_http": 0.102,
            "select_string_1M_arrow_recorded_http": 0.111,
        },
        threshold_pct=10.0,
    )
    by_name = {r.test_name: r for r in results}
    assert by_name["select_date_1M_arrow_recorded_http"].is_regressed
    assert not by_name["select_string_1M_arrow_recorded_http"].is_regressed
