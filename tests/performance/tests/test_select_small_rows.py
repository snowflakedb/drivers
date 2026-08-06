"""Fetch performance for small result sets (Python driver).

Row counts approximate p50/p90/p99 of typical customer query result sizes:
1 row (p50), 15 rows (p90), 400 rows (p99).
"""
import pytest

ITERATIONS = 30
WARMUP_ITERATIONS = 2

SQL_TEMPLATE = "SELECT L_COMMENT FROM SNOWFLAKE_SAMPLE_DATA.TPCH_SF100.LINEITEM LIMIT {n}"

ROW_COUNTS = [(1, "1_row"), (15, "15_rows"), (400, "400_rows")]
FETCH_MODES = ["fetchmany", "fetchall", "fetchone", "pandas"]


def _name(label: str, fetch_mode: str) -> str:
    return label if fetch_mode == "fetchmany" else f"{label}_{fetch_mode}"


CASES = [
    (row_count, _name(label, fetch_mode), fetch_mode)
    for row_count, label in ROW_COUNTS
    for fetch_mode in FETCH_MODES
]
IDS = [name for _, name, _ in CASES]


@pytest.mark.iterations(ITERATIONS)
@pytest.mark.warmup_iterations(WARMUP_ITERATIONS)
@pytest.mark.parametrize("row_count,name,fetch_mode", CASES, ids=IDS)
def test_select_string(perf_test, row_count, name, fetch_mode):
    perf_test(
        sql_command=SQL_TEMPLATE.format(n=row_count),
        fetch_mode=fetch_mode,
        test_name=f"select_string_{name}",
    )
