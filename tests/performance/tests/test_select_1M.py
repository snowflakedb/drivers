"""Live e2e SELECT 1M performance tests (direct Snowflake connection).

Bind-mode matrix (ODBC):
  * no suffix / existing names — SQL_C_CHAR (to_string); historical BenchDash baselines
  * `_default` suffix           — SQL_C_DEFAULT (driver-chosen C type); separate charts

A single run of this file (or of tests/) executes the complete type × bind_mode matrix.
See test_select_1M_recorded_http.py for the WireMock (CPU-only) counterparts.

Test function names stay stable (`test_select_string_1M_arrow`, …) for Jenkins smoke /
regression node-id filters; SQL is shared via catalog.
"""
import pytest
from catalog import TYPE_KEYS, get_sql

ITERATIONS = 10
WARMUP_ITERATIONS = 2


def _make_select_test(sql: str, bind_mode: str = "char"):
    @pytest.mark.iterations(ITERATIONS)
    @pytest.mark.warmup_iterations(WARMUP_ITERATIONS)
    def test_fn(perf_test, _sql=sql, _bind_mode=bind_mode):
        kwargs = {"sql_command": _sql}
        if _bind_mode != "char":
            kwargs["bind_mode"] = _bind_mode
        perf_test(**kwargs)

    if bind_mode == "default":
        test_fn = pytest.mark.supported_drivers("odbc")(test_fn)
    return test_fn


for type_key in TYPE_KEYS:
    sql = get_sql(type_key, 1_000_000)
    char_name = f"test_select_{type_key}_1M_arrow"
    globals()[char_name] = _make_select_test(sql, "char")
    globals()[char_name].__name__ = char_name
    globals()[char_name].__qualname__ = char_name

    default_name = f"test_select_{type_key}_1M_arrow_default"
    globals()[default_name] = _make_select_test(sql, "default")
    globals()[default_name].__name__ = default_name
    globals()[default_name].__qualname__ = default_name
