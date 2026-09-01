"""Shared LINEITEM SQL for SELECT perf tests (e2e + recorded HTTP)."""

TABLE = "SNOWFLAKE_SAMPLE_DATA.TPCH_SF100.LINEITEM"

_TEMPLATES = {
    "string": "SELECT L_COMMENT FROM {table} LIMIT {n}",
    "number": "SELECT L_LINENUMBER::INT FROM {table} LIMIT {n}",
    "date": "SELECT L_SHIPDATE FROM {table} LIMIT {n}",
    "float": "SELECT L_EXTENDEDPRICE FROM {table} LIMIT {n}",
    "double": "SELECT L_EXTENDEDPRICE::DOUBLE FROM {table} LIMIT {n}",
    "boolean": "SELECT (L_TAX > 0.04)::BOOLEAN FROM {table} LIMIT {n}",
    "timestamp_ntz": "SELECT L_SHIPDATE::TIMESTAMP_NTZ FROM {table} LIMIT {n}",
    "timestamp_tz": "SELECT L_SHIPDATE::TIMESTAMP_TZ FROM {table} LIMIT {n}",
    "time": (
        "SELECT TIME_FROM_PARTS(MOD(L_ORDERKEY, 24), MOD(L_PARTKEY, 60), MOD(L_SUPPKEY, 60)) "
        "FROM {table} LIMIT {n}"
    ),
    "binary": "SELECT TO_BINARY(L_COMMENT, 'UTF-8') FROM {table} LIMIT {n}",
    "15columns": """
            SELECT
                L_ORDERKEY,
                L_PARTKEY,
                L_SUPPKEY,
                L_LINENUMBER,
                L_QUANTITY,
                L_EXTENDEDPRICE,
                L_DISCOUNT,
                L_TAX,
                L_RETURNFLAG,
                L_LINESTATUS,
                L_SHIPDATE,
                L_COMMITDATE,
                L_RECEIPTDATE,
                L_SHIPINSTRUCT,
                L_COMMENT
            FROM {table}
            LIMIT {n}
        """,
}

TYPE_KEYS = tuple(_TEMPLATES)

# nodejs_bridge's Arrow decoder (column_reader.rs) has no TIME/TIMESTAMP_NTZ/
# TIMESTAMP_TZ logicalType support yet (SNOW-3946933/SNOW-3965562 family is
# landing decoders type-by-type). Keep this in one place so it stays correct
# as new decoders land.
NODEJS_UNSUPPORTED_TYPES = frozenset({"time", "timestamp_ntz", "timestamp_tz"})


def get_sql(type_key: str, n: int, *, ordered: bool = False) -> str:
    template = _TEMPLATES.get(type_key)
    if template is None:
        raise KeyError(f"Unknown query type {type_key!r}")
    if ordered:
        template = template.replace("LIMIT {n}", "ORDER BY L_ORDERKEY LIMIT {n}")
    return template.format(table=TABLE, n=n)
