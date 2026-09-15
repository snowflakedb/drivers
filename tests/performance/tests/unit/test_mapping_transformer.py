"""Keep/discard rules for recorded query-request stubs."""

from wiremock.mapping_transformer import MappingTransformer


def _query_mapping(*, rowtype_names, total=0, chunks=None, stats=None, sql_text=""):
    rowtype = [{"name": name} for name in rowtype_names]
    data = {
        "rowtype": rowtype,
        "total": total,
        "returned": total,
        "chunks": chunks or [],
        "sqlText": sql_text,
    }
    if stats is not None:
        data["stats"] = stats
    return {
        "request": {"urlPath": "/queries/v1/query-request", "method": "POST"},
        "response": {"body": __import__("json").dumps({"success": True, "data": data})},
    }


def test_keeps_named_column_select_with_chunks():
    mapping = _query_mapping(rowtype_names=["L_COMMENT"], total=1_000_000, chunks=[{"url": "s3://chunk"}])
    assert MappingTransformer._should_discard_mapping(mapping) is False


def test_keeps_placeholder_select():
    mapping = _query_mapping(rowtype_names=["?", "?", "?"], total=1)
    assert MappingTransformer._should_discard_mapping(mapping) is False


def test_keeps_insert_with_rows():
    mapping = _query_mapping(
        rowtype_names=[],
        total=1,
        stats={"numRowsInserted": 1000, "numRowsUpdated": 0, "numRowsDeleted": 0},
    )
    assert MappingTransformer._should_discard_mapping(mapping) is False


def test_discards_heartbeat_status():
    mapping = _query_mapping(rowtype_names=["status"], total=1)
    assert MappingTransformer._should_discard_mapping(mapping) is True


def test_discards_prepare_with_no_rows():
    mapping = _query_mapping(rowtype_names=[], total=0)
    assert MappingTransformer._should_discard_mapping(mapping) is True


def test_discards_empty_placeholder_select():
    mapping = _query_mapping(rowtype_names=["?", "?"], total=0)
    assert MappingTransformer._should_discard_mapping(mapping) is True


def test_discards_alter_session_text():
    mapping = _query_mapping(rowtype_names=["status"], total=1, sql_text="ALTER SESSION SET QUERY_RESULT_FORMAT='ARROW'")
    assert MappingTransformer._should_discard_mapping(mapping) is True
