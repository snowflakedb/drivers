"""Performance test for 1M rows with WireMock - for stability testing.

Bind-mode matrix (ODBC):
  * no suffix / existing names — SQL_C_CHAR (to_string); historical BenchDash baselines
  * `_default` before `_recorded_http` — SQL_C_DEFAULT; separate charts

A single run of this file (or of tests/) executes the complete type × bind_mode matrix.
Ordered variants stay CHAR-only (historical baselines).

Test function names stay stable for Jenkins `-k` / node-id filters; SQL is shared via
select_1m_queries.
"""
import pytest
from runner.test_types import PerfTestType
from select_1m_queries import ORDERED_QUERIES, TYPE_QUERIES

ITERATIONS = 10
WARMUP_ITERATIONS = 2


def _make_recorded_test(sql: str, bind_mode: str = "char"):
    @pytest.mark.iterations(ITERATIONS)
    @pytest.mark.warmup_iterations(WARMUP_ITERATIONS)
    def test_fn(perf_test, _sql=sql, _bind_mode=bind_mode):
        kwargs = {
            "test_type": PerfTestType.SELECT_RECORDED_HTTP,
            "sql_command": _sql,
        }
        if _bind_mode != "char":
            kwargs["bind_mode"] = _bind_mode
        perf_test(**kwargs)

    if bind_mode == "default":
        test_fn = pytest.mark.supported_drivers("odbc")(test_fn)
    return test_fn


for type_key, sql in TYPE_QUERIES:
    char_name = f"test_select_{type_key}_1M_arrow_recorded_http"
    globals()[char_name] = _make_recorded_test(sql, "char")
    globals()[char_name].__name__ = char_name
    globals()[char_name].__qualname__ = char_name

    default_name = f"test_select_{type_key}_1M_arrow_default_recorded_http"
    globals()[default_name] = _make_recorded_test(sql, "default")
    globals()[default_name].__name__ = default_name
    globals()[default_name].__qualname__ = default_name

for type_key, sql in ORDERED_QUERIES:
    ordered_name = f"test_select_{type_key}_1M_ordered_arrow_recorded_http"
    globals()[ordered_name] = _make_recorded_test(sql, "char")
    globals()[ordered_name].__name__ = ordered_name
    globals()[ordered_name].__qualname__ = ordered_name
