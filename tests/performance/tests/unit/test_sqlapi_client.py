"""Unit tests for SQL API partition/format helpers (no network, no pyarrow)."""
import sys
from pathlib import Path

import pytest

_ROOT = Path(__file__).resolve().parents[2] / "drivers"
_SHARED = _ROOT / "_shared"
_APP = _ROOT / "sqlapi" / "app"
sys.path.insert(0, str(_SHARED))
sys.path.insert(0, str(_APP))

from client import (  # noqa: E402
    ARROW_FORMAT,
    JSON_FORMAT,
    TOKEN_TYPE_JWT,
    TOKEN_TYPE_PAT,
    api_base,
    assert_row_count,
    auth_headers,
    json_row_count,
    metadata_num_rows,
    partition_count,
    partition_headers,
    partition_url,
    partitions_to_fetch,
    sql_object_name,
    sqlapi_format,
    statement_body,
)


def test_sqlapi_format_maps_session_names():
    assert sqlapi_format("arrow") == ARROW_FORMAT
    assert sqlapi_format("json") == JSON_FORMAT
    assert sqlapi_format("arrowv1") == ARROW_FORMAT
    with pytest.raises(ValueError):
        sqlapi_format("csv")


def test_partition_url_replaces_existing_partition_param():
    url = partition_url(
        "xy12345.snowflakecomputing.com",
        "/api/v2/statements/abc?requestId=rid&partition=0",
        3,
    )
    assert url == (
        "https://xy12345.snowflakecomputing.com/api/v2/statements/abc"
        "?requestId=rid&partition=3"
    )


def test_partition_url_does_not_append_duplicate_ampersand_partition():
    url = partition_url(
        "https://xy12345.snowflakecomputing.com",
        "/api/v2/statements/abc?partition=0",
        1,
    )
    assert url == (
        "https://xy12345.snowflakecomputing.com/api/v2/statements/abc?partition=1"
    )


def test_absolute_statement_status_url():
    url = partition_url(
        "ignored.example.com",
        "https://acct.snowflakecomputing.com/api/v2/statements/h?partition=0",
        2,
    )
    assert url == (
        "https://acct.snowflakecomputing.com/api/v2/statements/h?partition=2"
    )


def test_partitions_to_fetch_skip_zero():
    assert list(partitions_to_fetch(1)) == []
    assert list(partitions_to_fetch(3)) == [1, 2]
    assert list(partitions_to_fetch(0)) == []


def test_json_row_count_and_num_rows():
    payload = {
        "data": [["a"], ["b"]],
        "resultSetMetaData": {
            "numRows": 2,
            "partitionInfo": [{}, {}],
        },
    }
    assert json_row_count(payload) == 2
    assert metadata_num_rows(payload) == 2
    assert partition_count(payload) == 2
    assert_row_count(2, 2, "test")
    with pytest.raises(RuntimeError, match="mismatch"):
        assert_row_count(1, 2, "test")
    with pytest.raises(RuntimeError, match="is 0"):
        assert_row_count(0, 0, "test")


def test_auth_headers_set_token_type():
    headers = auth_headers("tok", TOKEN_TYPE_PAT)
    assert headers["Authorization"] == "Bearer tok"
    assert headers["X-Snowflake-Authorization-Token-Type"] == TOKEN_TYPE_PAT
    jwt_headers = auth_headers("jwt", TOKEN_TYPE_JWT)
    assert jwt_headers["X-Snowflake-Authorization-Token-Type"] == TOKEN_TYPE_JWT


def test_arrow_partition_accept_header():
    base = auth_headers("tok", TOKEN_TYPE_PAT)
    arrow = partition_headers(base, "arrow")
    json_h = partition_headers(base, "json")
    assert arrow["Accept"] == "application/vnd.apache.arrow.stream"
    assert json_h["Accept"] == "application/json"
    assert "Content-Type" not in arrow
    assert "Content-Type" not in json_h
    assert base["Content-Type"] == "application/json"


def test_assert_result_format_rejects_jsonv2_when_arrow_requested():
    from client import assert_result_format

    payload = {"resultSetMetaData": {"format": "jsonv2"}}
    with pytest.raises(RuntimeError, match="ENABLE_SQL_API_ARROW_V1"):
        assert_result_format(payload, "arrow")
    assert_result_format({"resultSetMetaData": {"format": "arrowv1"}}, "arrow")


def test_statement_body_sets_format():
    body = statement_body(
        "SELECT 1",
        database="DB",
        schema="SC",
        warehouse="WH",
        role="ROLE",
        result_format="arrow",
    )
    assert body["resultSetMetaData"]["format"] == ARROW_FORMAT
    assert body["database"] == "DB"


def test_sql_object_name_uppercases_unquoted_identifiers():
    assert sql_object_name("testrole_universal_perf") == "TESTROLE_UNIVERSAL_PERF"
    assert sql_object_name("  warehouse_perf  ") == "WAREHOUSE_PERF"
    assert sql_object_name('"MixedCaseRole"') == "MixedCaseRole"
    assert sql_object_name("") is None
    assert sql_object_name(None) is None


def test_statement_body_uppercases_session_objects_like_connectors():
    body = statement_body(
        "SELECT 1",
        database="db1",
        schema="public",
        warehouse="wh_perf",
        role="testrole_universal_perf",
        result_format="arrow",
    )
    assert body["database"] == "DB1"
    assert body["schema"] == "PUBLIC"
    assert body["warehouse"] == "WH_PERF"
    assert body["role"] == "TESTROLE_UNIVERSAL_PERF"


def test_api_base_strips_scheme():
    assert api_base("https://acct.snowflakecomputing.com/foo") == (
        "https://acct.snowflakecomputing.com"
    )


def test_single_impl_drivers():
    from runner.utils import is_single_impl_driver

    assert is_single_impl_driver("sqlapi")
    assert is_single_impl_driver("adbc")
    assert is_single_impl_driver("core")
    assert not is_single_impl_driver("python")


def test_sqlapi_1m_cases_share_fetchmany_test_names():
    sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
    from test_select_1M import CASES, SQLAPI_CASES

    python_names = {p.values[2] for p in CASES if p.values[3] == "fetchmany"}
    sqlapi_names = {p.values[2] for p in SQLAPI_CASES}
    assert sqlapi_names <= python_names
    assert "string_1M_arrow" in sqlapi_names
    assert all(p.id.endswith("_sqlapi") for p in SQLAPI_CASES)
    assert all(any(m.name == "supports_json" for m in p.marks) for p in SQLAPI_CASES)


def test_adbc_1m_cases_share_arrow_batches_test_names():
    sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
    from test_select_1M import ADBC_CASES, CASES

    python_names = {p.values[2] for p in CASES if p.values[3] == "arrow_batches"}
    adbc_names = {p.values[2] for p in ADBC_CASES}
    assert adbc_names <= python_names
    assert "string_1M_arrow_arrow_batches" in adbc_names
    assert all(p.id.endswith("_adbc") for p in ADBC_CASES)
