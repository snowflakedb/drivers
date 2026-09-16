"""SQL API v2 helpers used by the perf driver (stdlib only, unit-testable).

Arrow (arrowv1): POST partition 0 is empty; rows start at GET partition 1.
JSON (jsonv2): consume POST ``data`` as partition 0, then GET 1..N-1.
"""

from __future__ import annotations

from urllib.parse import parse_qs, urlencode, urljoin, urlparse, urlunparse

from conn_params import sql_object_name

ARROW_FORMAT = "arrowv1"
JSON_FORMAT = "jsonv2"
ARROW_ACCEPT = "application/vnd.apache.arrow.stream"
JSON_ACCEPT = "application/json"
TOKEN_TYPE_PAT = "PROGRAMMATIC_ACCESS_TOKEN"
TOKEN_TYPE_JWT = "KEYPAIR_JWT"
USER_AGENT = "snowflake-eng-drivers-perf-sqlapi"

__all__ = [
    "ARROW_FORMAT",
    "JSON_FORMAT",
    "ARROW_ACCEPT",
    "JSON_ACCEPT",
    "TOKEN_TYPE_PAT",
    "TOKEN_TYPE_JWT",
    "USER_AGENT",
    "sql_object_name",
    "sqlapi_format",
    "is_arrow_format",
    "api_base",
    "statements_url",
    "statement_url",
    "partition_url",
    "partitions_to_fetch",
    "auth_headers",
    "partition_headers",
    "statement_body",
    "result_format_of",
    "assert_result_format",
    "metadata_num_rows",
    "partition_count",
    "json_row_count",
    "assert_row_count",
]


def sqlapi_format(result_format: str) -> str:
    raw = (result_format or "arrow").strip().lower()
    if raw in ("json", "jsonv2"):
        return JSON_FORMAT
    if raw in ("arrow", "arrowv1"):
        return ARROW_FORMAT
    raise ValueError(f"Unsupported SQL API result format: {result_format!r}")


def is_arrow_format(fmt: str) -> bool:
    return sqlapi_format(fmt) == ARROW_FORMAT


def api_base(host: str) -> str:
    """Return https://host with no trailing slash."""
    raw = (host or "").strip()
    if raw.lower().startswith(("https://", "http://")):
        raw = raw.split("://", 1)[1]
    raw = raw.split("/", 1)[0].rstrip("/")
    if not raw:
        raise ValueError("SQL API host is empty")
    return f"https://{raw}"


def statements_url(host: str) -> str:
    return f"{api_base(host)}/api/v2/statements"


def statement_url(host: str, statement_handle: str) -> str:
    return f"{api_base(host)}/api/v2/statements/{statement_handle}"


def partition_url(host: str, statement_status_url: str, partition: int) -> str:
    """Build a GET URL for ``partition``, replacing any existing partition query param."""
    if not statement_status_url:
        raise ValueError("statementStatusUrl is empty")
    if statement_status_url.startswith("http://") or statement_status_url.startswith(
        "https://"
    ):
        parsed = urlparse(statement_status_url)
    else:
        parsed = urlparse(urljoin(api_base(host) + "/", statement_status_url.lstrip("/")))
    query = parse_qs(parsed.query, keep_blank_values=True)
    query["partition"] = [str(partition)]
    return urlunparse(parsed._replace(query=urlencode(query, doseq=True)))


def partitions_to_fetch(partition_count: int) -> range:
    """HTTP GET partitions after the POST body has been consumed.

    Arrow and JSON both GET 1..N-1. Partition 0 is empty on Arrow POST and
    already consumed as JSON ``data``.
    """
    n = int(partition_count)
    if n < 1:
        return range(1, 1)
    return range(1, n)


def auth_headers(token: str, token_type: str) -> dict[str, str]:
    if not token:
        raise ValueError("SQL API token is empty")
    if token_type not in (TOKEN_TYPE_PAT, TOKEN_TYPE_JWT):
        raise ValueError(f"Unknown SQL API token type: {token_type!r}")
    return {
        "Authorization": f"Bearer {token}",
        "Content-Type": "application/json",
        "Accept": JSON_ACCEPT,
        "User-Agent": USER_AGENT,
        "X-Snowflake-Authorization-Token-Type": token_type,
    }


def partition_headers(base_headers: dict[str, str], fmt: str) -> dict[str, str]:
    """GET partition headers. Do not copy POST ``Content-Type``; JAX-RS can then
    emit JSON for an Arrow partition (pyarrow: ``Not an Arrow file``).
    """
    headers = dict(base_headers)
    headers.pop("Content-Type", None)
    headers["Accept"] = ARROW_ACCEPT if is_arrow_format(fmt) else JSON_ACCEPT
    return headers


def statement_body(
    sql: str,
    *,
    database: str | None,
    schema: str | None,
    warehouse: str | None,
    role: str | None,
    result_format: str,
    timeout_s: int = 3600,
) -> dict:
    body = {
        "statement": sql,
        "timeout": timeout_s,
        "resultSetMetaData": {"format": sqlapi_format(result_format)},
    }
    if database:
        body["database"] = sql_object_name(database)
    if schema:
        body["schema"] = sql_object_name(schema)
    if warehouse:
        body["warehouse"] = sql_object_name(warehouse)
    if role:
        body["role"] = sql_object_name(role)
    return body


def result_format_of(payload: dict) -> str | None:
    meta = payload.get("resultSetMetaData") or {}
    fmt = meta.get("format")
    if fmt is None:
        return None
    return str(fmt).strip().lower() or None


def assert_result_format(payload: dict, requested: str) -> None:
    """Fail fast when SQL API ignored arrowv1."""
    got = result_format_of(payload)
    want = sqlapi_format(requested)
    if got is None:
        return
    if got != want:
        raise RuntimeError(
            f"SQL API returned resultSetMetaData.format={got!r} after requesting {want!r}. "
            "Arrow on /api/v2 needs account parameter ENABLE_SQL_API_ARROW_V1=true. "
            f"payload keys={list(payload)[:12]}"
        )


def metadata_num_rows(payload: dict) -> int | None:
    meta = payload.get("resultSetMetaData") or {}
    value = meta.get("numRows")
    if value is None:
        return None
    return int(value)


def partition_count(payload: dict) -> int:
    meta = payload.get("resultSetMetaData") or {}
    info = meta.get("partitionInfo") or []
    return len(info)


def json_row_count(payload: dict) -> int:
    data = payload.get("data")
    if not data:
        return 0
    return len(data)


def assert_row_count(actual: int, expected: int | None, source: str) -> None:
    if expected is None:
        if actual == 0:
            raise RuntimeError(
                f"{source}: row count is 0 and resultSetMetaData.numRows is missing"
            )
        return
    if actual != expected:
        raise RuntimeError(
            f"{source}: row count mismatch: fetched {actual}, "
            f"resultSetMetaData.numRows={expected}"
        )
    if actual == 0:
        raise RuntimeError(f"{source}: row count is 0")
