"""Fetch performance for small result sets.

Row counts approximate p50/p90/p99 of typical customer query result sizes:
1 row (p50), 15 rows (p90), 400 rows (p99).

fetchmany is the default path and runs for all drivers. fetchall / fetchone /
pandas require FETCH_MODE and are Python-only.
"""
import pytest

ITERATIONS = 30
WARMUP_ITERATIONS = 2

SQL_TEMPLATE = "SELECT L_COMMENT FROM SNOWFLAKE_SAMPLE_DATA.TPCH_SF100.LINEITEM LIMIT {n}"

ROW_COUNTS = [(1, "1_row"), (15, "15_rows"), (400, "400_rows")]
FETCH_MODES = ["fetchmany", "fetchall", "fetchone", "pandas"]


def _name(label: str, fetch_mode: str) -> str:
    return label if fetch_mode == "fetchmany" else f"{label}_{fetch_mode}"


def _case(row_count: int, label: str, fetch_mode: str):
    name = _name(label, fetch_mode)
    marks = (
        [pytest.mark.supported_drivers("python")]
        if fetch_mode != "fetchmany"
        else []
    )
    return pytest.param(row_count, name, fetch_mode, id=name, marks=marks)


CASES = [
    _case(row_count, label, fetch_mode)
    for row_count, label in ROW_COUNTS
    for fetch_mode in FETCH_MODES
]


@pytest.mark.iterations(ITERATIONS)
@pytest.mark.warmup_iterations(WARMUP_ITERATIONS)
@pytest.mark.parametrize("row_count,name,fetch_mode", CASES)
def test_select_string(perf_test, row_count, name, fetch_mode):
    perf_test(
        sql_command=SQL_TEMPLATE.format(n=row_count),
        fetch_mode=fetch_mode,
        test_name=f"select_string_{name}",
    )
