"""Unit tests for the real ResultMetadataV2 class (BD#54).

Verifies: property access matches the old-driver interface, vector_dimension is read
from the proto dimension field, _is_nullable is a private attr (Snowpark reads
it directly), and fields is built recursively from the proto nested column list.
"""

from unittest.mock import MagicMock

import pytest

from snowflake.connector._internal.api_client.client_api import core_driver
from snowflake.connector._internal.cursor.result_metadata import ResultMetadata, ResultMetadataV2
from snowflake.connector._internal.protobuf_gen.database_driver_v1_pb2 import (
    ColumnMetadata,
    ConnectionHandle,
    StatementHandle,
)
from snowflake.connector._internal.type_codes import FIELD_ID_TO_NAME
from snowflake.connector.cursor import SnowflakeCursor


def _column(
    name: str = "col",
    col_type: str = "TEXT",
    nullable: bool = True,
    length: int | None = None,
    byte_length: int | None = None,
    precision: int | None = None,
    scale: int | None = None,
    dimension: int | None = None,
    fields: list[ColumnMetadata] | None = None,
) -> ColumnMetadata:
    """Build a real proto ``ColumnMetadata``.

    A real message rather than a ``MagicMock`` so that ``HasField`` presence and
    the empty-vs-absent distinction on the repeated ``fields`` list behave the way
    they do in production. A mock reports an unset repeated field as truthy.
    """
    col = ColumnMetadata(name=name, type=col_type, nullable=nullable)
    for attr, value in (
        ("length", length),
        ("byte_length", byte_length),
        ("precision", precision),
        ("scale", scale),
        ("dimension", dimension),
    ):
        if value is not None:
            setattr(col, attr, value)
    if fields:
        col.fields.extend(fields)
    return col


class TestResultMetadataV2IsDistinctClass:
    def test_is_not_alias_of_result_metadata(self):
        assert ResultMetadataV2 is not ResultMetadata

    def test_is_not_a_named_tuple(self):
        assert not issubclass(ResultMetadataV2, tuple)


class TestResultMetadataV2Properties:
    def test_all_properties_accessible(self):
        v2 = ResultMetadataV2(
            name="col",
            type_code=2,
            is_nullable=True,
            display_size=None,
            internal_size=16,
            precision=10,
            scale=2,
            vector_dimension=3,
            fields=None,
        )
        assert v2.name == "col"
        assert v2.type_code == 2
        assert v2.is_nullable is True
        assert v2.display_size is None
        assert v2.internal_size == 16
        assert v2.precision == 10
        assert v2.scale == 2
        assert v2.vector_dimension == 3
        assert v2.fields is None

    def test_is_nullable_accessible_as_private_attr(self):
        # Snowpark reads ._is_nullable directly on nested element metadata.
        v2 = ResultMetadataV2(name=None, type_code=1, is_nullable=False)
        assert v2._is_nullable is False
        assert v2.is_nullable is False


class TestResultMetadataV2FromColumn:
    def test_vector_dimension_populated_from_dimension_field(self):
        col = _column(col_type="VECTOR", dimension=128)
        v2 = ResultMetadataV2.from_column(col)
        assert v2.vector_dimension == 128

    def test_vector_dimension_none_when_field_absent(self):
        col = _column(col_type="FIXED")
        v2 = ResultMetadataV2.from_column(col)
        assert v2.vector_dimension is None

    def test_fields_none_when_proto_carries_no_nested_columns(self):
        col = _column(col_type="OBJECT")
        v2 = ResultMetadataV2.from_column(col)
        assert v2.fields is None

    def test_nullable_and_name_populated(self):
        col = _column(name="amount", col_type="FIXED", nullable=False)
        v2 = ResultMetadataV2.from_column(col)
        assert v2.name == "amount"
        assert v2.is_nullable is False

    def test_precision_and_scale_populated(self):
        col = _column(col_type="FIXED", precision=18, scale=6)
        v2 = ResultMetadataV2.from_column(col)
        assert v2.precision == 18
        assert v2.scale == 6

    def test_map_type_code_matches_old_driver(self):
        """A MAP column's type_code is MAP, not the TEXT fallback for an unmapped type name.

        The old driver's FIELD_TYPES table places MAP right after VECTOR at the same
        index (constants.py). Before SNOW-3895458 the new driver had no MAP entry in
        its type-code table, so get_type_code("MAP") fell through to the TEXT default
        and a MAP column was indistinguishable from an actual TEXT column via type_code
        alone. A live-server round-trip isn't currently exercisable for MAP describe/
        prepare due to the pre-existing SNOW-4052969; this pins the type-code mapping
        directly.
        """
        col = _column(col_type="MAP")
        v2 = ResultMetadataV2.from_column(col)
        assert FIELD_ID_TO_NAME[v2.type_code] == "MAP"


class TestResultMetadataV2CreateDescription:
    def test_returns_none_for_none_result(self):
        assert ResultMetadataV2.create_description(None) is None

    def test_returns_none_for_result_with_no_columns(self):
        result = MagicMock()
        result.columns = []
        assert ResultMetadataV2.create_description(result) is None

    def test_returns_list_of_v2_objects(self):
        col = _column(col_type="TEXT", length=100)
        result = MagicMock()
        result.columns = [col]
        desc = ResultMetadataV2.create_description(result)
        assert desc is not None
        assert len(desc) == 1
        assert isinstance(desc[0], ResultMetadataV2)
        assert desc[0].display_size == 100

    def test_vector_dimension_in_create_description(self):
        col = _column(col_type="VECTOR", dimension=64)
        result = MagicMock()
        result.columns = [col]
        desc = ResultMetadataV2.create_description(result)
        assert desc[0].vector_dimension == 64


class TestResultMetadataV2Equality:
    def test_equal_instances(self):
        a = ResultMetadataV2(name="x", type_code=2, is_nullable=True, precision=10, scale=0)
        b = ResultMetadataV2(name="x", type_code=2, is_nullable=True, precision=10, scale=0)
        assert a == b

    def test_not_equal_when_vector_dimension_differs(self):
        a = ResultMetadataV2(name="x", type_code=7, is_nullable=False, vector_dimension=3)
        b = ResultMetadataV2(name="x", type_code=7, is_nullable=False, vector_dimension=4)
        assert a != b


@pytest.fixture
def mock_core_client():
    """Patch core_driver.client for tests that drive cursor methods."""
    mock = MagicMock()
    old = core_driver._client
    core_driver.client = mock
    yield mock
    core_driver.client = old


class TestDescribeInternal:
    """Unit tests for SnowflakeCursor._describe_internal.

    _describe_internal follows the same prepare path as describe() but returns
    list[ResultMetadataV2] instead of list[ResultMetadata].  The mocking
    pattern mirrors TestDescribe in test_cursor.py: supply a real
    ConnectionHandle + StatementHandle so that the statement() context manager
    can allocate/release handles via the mocked core_driver client.
    """

    @pytest.fixture
    def mock_connection(self, mock_core_client):
        conn = MagicMock()
        conn.conn_handle = ConnectionHandle(id=1)
        conn.is_closed.return_value = False
        mock_core_client.statement_new.return_value.stmt_handle = StatementHandle(id=1)
        return conn

    @pytest.fixture
    def cursor(self, mock_connection, no_native_stream_ops):
        return SnowflakeCursor(mock_connection)

    def _setup_prepare(self, mock_core_client, columns=None):
        result = MagicMock()
        result.columns = columns or []
        result.stream.value = (42).to_bytes(8, byteorder="little", signed=False)
        result.query_id = ""
        result.query = ""
        result.sql_state = None
        mock_core_client.statement_prepare.return_value.result = result
        return result

    def test_returns_list_of_result_metadata_v2(self, cursor, mock_core_client):
        """_describe_internal() returns ResultMetadataV2 objects, not old-driver ResultMetadata."""
        col = _column(name="AMOUNT", col_type="FIXED", precision=18, scale=6)
        self._setup_prepare(mock_core_client, columns=[col])

        result = cursor._describe_internal("SELECT 1 AS AMOUNT")

        assert result is not None
        assert len(result) == 1
        assert isinstance(result[0], ResultMetadataV2)
        assert result[0].name == "AMOUNT"
        assert result[0].precision == 18
        assert result[0].scale == 6

    def test_returns_none_for_no_columns(self, cursor, mock_core_client):
        """_describe_internal() returns None when the prepare result carries no columns.

        Statement text is irrelevant here — the mocked prepare result drives the
        outcome. A real DML describe does return a synthetic row-count column, so
        this must not be read as "DML describes to None".
        """
        self._setup_prepare(mock_core_client, columns=[])

        assert cursor._describe_internal("SELECT 1") is None

    def test_vector_dimension_populated(self, cursor, mock_core_client):
        """_describe_internal() propagates vector_dimension from the proto dimension field."""
        col = _column(col_type="VECTOR", dimension=64)
        self._setup_prepare(mock_core_client, columns=[col])

        result = cursor._describe_internal("SELECT vec")

        assert result is not None
        assert result[0].vector_dimension == 64

    def test_display_size_populated_for_text(self, cursor, mock_core_client):
        """_describe_internal() populates display_size from length for TEXT columns."""
        col = _column(col_type="TEXT", length=256)
        self._setup_prepare(mock_core_client, columns=[col])

        result = cursor._describe_internal("SELECT s")

        assert result is not None
        assert result[0].display_size == 256

    def test_fields_populated_from_nested_proto_columns(self, cursor, mock_core_client):
        """_describe_internal() carries the nested field list through to ResultMetadataV2."""
        col = _column(col_type="VECTOR", dimension=3, fields=[_column(name="", col_type="FIXED")])
        self._setup_prepare(mock_core_client, columns=[col])

        result = cursor._describe_internal("SELECT vec")

        assert result[0].fields is not None
        assert FIELD_ID_TO_NAME[result[0].fields[0].type_code] == "FIXED"


class TestDescriptionInternalProperty:
    """Unit tests for the ``_description_internal`` cursor property.

    Snowpark's ``get_new_description`` probes for this attribute and falls back to
    the V1 ``description`` when it is absent, which drops ``fields`` and
    ``vector_dimension``. These tests pin the attribute's presence and its
    relationship to ``description``.
    """

    @pytest.fixture
    def mock_connection(self, mock_core_client):
        conn = MagicMock()
        conn.conn_handle = ConnectionHandle(id=1)
        conn.is_closed.return_value = False
        mock_core_client.statement_new.return_value.stmt_handle = StatementHandle(id=1)
        return conn

    @pytest.fixture
    def cursor(self, mock_connection, no_native_stream_ops):
        return SnowflakeCursor(mock_connection)

    def test_attribute_is_present_on_the_cursor(self, cursor):
        # hasattr() is the exact probe Snowpark's get_new_description performs.
        assert hasattr(cursor, "_description_internal")

    def test_none_before_any_statement_runs(self, cursor):
        assert cursor._description_internal is None

    def test_returns_v2_objects_carrying_nested_fields(self, cursor, mock_core_client):
        col = _column(name="ARR", col_type="ARRAY", fields=[_column(name="", col_type="FIXED", nullable=True)])
        result = MagicMock()
        result.columns = [col]
        result.stream.value = (42).to_bytes(8, byteorder="little", signed=False)
        result.query_id = ""
        result.query = ""
        result.sql_state = None
        mock_core_client.statement_prepare.return_value.result = result

        cursor._describe_internal("SELECT arr")

        internal = cursor._description_internal
        assert internal is not None
        assert isinstance(internal[0], ResultMetadataV2)
        assert internal[0].fields[0]._is_nullable is True

    def test_v1_description_is_the_same_columns_downcast(self, cursor, mock_core_client):
        col = _column(name="AMOUNT", col_type="FIXED", precision=18, scale=6, nullable=False)
        result = MagicMock()
        result.columns = [col]
        result.stream.value = (42).to_bytes(8, byteorder="little", signed=False)
        result.query_id = ""
        result.query = ""
        result.sql_state = None
        mock_core_client.statement_prepare.return_value.result = result

        cursor._describe_internal("SELECT 1 AS AMOUNT")

        v1 = cursor.description
        v2 = cursor._description_internal
        assert [c.name for c in v1] == [c.name for c in v2]
        assert all(isinstance(c, ResultMetadata) for c in v1)
        assert v1[0].precision == v2[0].precision == 18
        assert v1[0].is_nullable is v2[0].is_nullable is False


class TestResultMetadataV2NestedFields:
    """Verify ``fields`` is built from the proto nested column list.

    The shapes mirror what Snowpark's ``convert_metadata_to_sp_type`` consumes:
    one sub-field for VECTOR and ARRAY, two for MAP, and one named sub-field per
    attribute for a structured OBJECT.
    """

    def test_vector_element_type_exposed_as_single_sub_field(self):
        col = _column(col_type="VECTOR", dimension=768, fields=[_column(name="", col_type="REAL", nullable=False)])
        v2 = ResultMetadataV2.from_column(col)
        assert v2.vector_dimension == 768
        assert v2.fields is not None
        assert len(v2.fields) == 1
        assert FIELD_ID_TO_NAME[v2.fields[0].type_code] == "REAL"

    def test_array_element_sub_field_exposes_private_nullable_flag(self):
        # Snowpark reads fields[0]._is_nullable to decide ArrayType(contains_null=...).
        col = _column(col_type="ARRAY", fields=[_column(name="", col_type="TEXT", nullable=True, length=100)])
        v2 = ResultMetadataV2.from_column(col)
        assert v2.fields[0]._is_nullable is True
        assert v2.fields[0].internal_size == 100

    def test_map_exposes_key_and_value_sub_fields_in_order(self):
        col = _column(
            col_type="MAP",
            fields=[
                _column(name="", col_type="TEXT", nullable=False, length=16777216),
                _column(name="", col_type="FIXED", nullable=True, precision=38, scale=0),
            ],
        )
        v2 = ResultMetadataV2.from_column(col)
        assert len(v2.fields) == 2
        assert FIELD_ID_TO_NAME[v2.fields[0].type_code] == "TEXT"
        assert FIELD_ID_TO_NAME[v2.fields[1].type_code] == "FIXED"
        assert v2.fields[1]._is_nullable is True

    def test_structured_object_sub_fields_keep_their_names(self):
        col = _column(
            col_type="OBJECT",
            fields=[
                _column(name="city", col_type="TEXT", nullable=False, length=100),
                _column(name="zip", col_type="FIXED", nullable=True, precision=38, scale=0),
            ],
        )
        v2 = ResultMetadataV2.from_column(col)
        assert [f.name for f in v2.fields] == ["city", "zip"]
        assert v2.fields[0].is_nullable is False

    def test_sub_field_name_is_none_when_proto_leaves_it_empty(self):
        # The old driver merges {"name": None, **f}; the proto renders an absent
        # name as "", which must map back to None rather than an empty string.
        col = _column(col_type="ARRAY", fields=[_column(name="", col_type="FIXED")])
        v2 = ResultMetadataV2.from_column(col)
        assert v2.fields[0].name is None

    def test_top_level_name_is_preserved_verbatim(self):
        col = _column(name="ARR", col_type="ARRAY", fields=[_column(name="", col_type="FIXED")])
        v2 = ResultMetadataV2.from_column(col)
        assert v2.name == "ARR"

    def test_nested_structured_fields_recurse(self):
        inner = _column(
            name="",
            col_type="OBJECT",
            fields=[_column(name="city", col_type="TEXT", nullable=False, length=100)],
        )
        col = _column(name="ADDRESSES", col_type="ARRAY", fields=[inner])
        v2 = ResultMetadataV2.from_column(col)
        element = v2.fields[0]
        assert FIELD_ID_TO_NAME[element.type_code] == "OBJECT"
        assert element.fields is not None
        assert element.fields[0].name == "city"
        assert element.fields[0].fields is None

    def test_fields_none_for_semi_structured_column_without_nested_list(self):
        # Snowpark treats a falsy `fields` as "fall back to the legacy type", so an
        # unset repeated field must surface as None, not an empty list.
        col = _column(col_type="ARRAY")
        v2 = ResultMetadataV2.from_column(col)
        assert v2.fields is None

    def test_create_description_populates_fields(self):
        col = _column(col_type="VECTOR", dimension=3, fields=[_column(name="", col_type="FIXED")])
        result = MagicMock()
        result.columns = [col]
        desc = ResultMetadataV2.create_description(result)
        assert desc[0].fields is not None
        assert FIELD_ID_TO_NAME[desc[0].fields[0].type_code] == "FIXED"

    def test_v1_downcast_drops_nested_fields(self):
        col = _column(col_type="ARRAY", fields=[_column(name="", col_type="FIXED")])
        v1 = ResultMetadataV2.from_column(col)._to_result_metadata_v1()
        assert not hasattr(v1, "fields")


class TestResultMetadataV2FromColumnFieldSizes:
    """Verify display_size and internal_size proto-field mapping.

    display_size is new-driver-only, populated for text types (BD#90). internal_size
    carries the char count on both drivers — old-driver semantics, matched here.
    """

    def test_display_size_populated_for_text_column(self):
        col = _column(col_type="TEXT", length=100)
        v2 = ResultMetadataV2.from_column(col)
        assert v2.display_size == 100

    def test_display_size_none_for_non_text_column(self):
        # FIXED with length present — type guard must suppress display_size.
        col = _column(col_type="FIXED", length=100)
        v2 = ResultMetadataV2.from_column(col)
        assert v2.display_size is None

    def test_display_size_none_when_length_field_absent(self):
        col = _column(col_type="TEXT")  # length=None → HasField("length") False
        v2 = ResultMetadataV2.from_column(col)
        assert v2.display_size is None

    def test_internal_size_populated_from_length(self):
        # internal_size carries the char count (old-driver semantics), not byte_length.
        col = _column(col_type="TEXT", length=100, byte_length=400)
        v2 = ResultMetadataV2.from_column(col)
        assert v2.internal_size == 100

    def test_internal_size_none_when_length_absent(self):
        col = _column(col_type="TEXT")  # length=None → HasField("length") False
        v2 = ResultMetadataV2.from_column(col)
        assert v2.internal_size is None

    def test_internal_size_populated_for_binary_column(self):
        # BINARY has no display_size (type-gated to text), but internal_size must
        # still carry a real value here — unlike display_size, it is not type-gated.
        col = _column(col_type="BINARY", length=8)
        v2 = ResultMetadataV2.from_column(col)
        assert v2.display_size is None
        assert v2.internal_size == 8

    @pytest.mark.parametrize(
        "col_type",
        [
            "FIXED",
            "DATE",
            "TIME",
            "TIMESTAMP_NTZ",
            "TIMESTAMP_LTZ",
            "TIMESTAMP_TZ",
            "VARIANT",
            "OBJECT",
            "ARRAY",
            "BOOLEAN",
        ],
    )
    def test_internal_size_none_for_types_without_length(self, col_type):
        # The old driver never populates internal_size for these types; the proto never
        # sets `length` for them either, so HasField("length") is False.
        col = _column(col_type=col_type)
        v2 = ResultMetadataV2.from_column(col)
        assert v2.internal_size is None


class TestResultMetadataFromColumnFieldSizes:
    """Mirrors TestResultMetadataV2FromColumnFieldSizes for the V1 NamedTuple's
    own from_column — the size-field mapping is duplicated in both."""

    def test_internal_size_populated_from_length(self):
        col = _column(col_type="TEXT", length=100, byte_length=400)
        v1 = ResultMetadata.from_column(col)
        assert v1.internal_size == 100

    def test_internal_size_none_when_length_absent(self):
        col = _column(col_type="TEXT")
        v1 = ResultMetadata.from_column(col)
        assert v1.internal_size is None

    def test_internal_size_populated_for_binary_column(self):
        col = _column(col_type="BINARY", length=8)
        v1 = ResultMetadata.from_column(col)
        assert v1.display_size is None
        assert v1.internal_size == 8


class TestResultMetadataV2ToV1:
    """Verify _to_result_metadata_v1 downcast to PEP 249 ResultMetadata."""

    def test_returns_result_metadata_instance(self):
        v2 = ResultMetadataV2(name="col", type_code=2, is_nullable=True)
        assert isinstance(v2._to_result_metadata_v1(), ResultMetadata)

    def test_all_fields_round_trip(self):
        v2 = ResultMetadataV2(
            name="amount",
            type_code=0,
            is_nullable=False,
            display_size=None,
            internal_size=16,
            precision=18,
            scale=6,
            vector_dimension=128,
            fields=None,
        )
        v1 = v2._to_result_metadata_v1()
        assert v1.name == "amount"
        assert v1.type_code == 0
        assert v1.is_nullable is False
        assert v1.display_size is None
        assert v1.internal_size == 16
        assert v1.precision == 18
        assert v1.scale == 6

    def test_none_fields_pass_through(self):
        v2 = ResultMetadataV2(name="x", type_code=1, is_nullable=True)
        v1 = v2._to_result_metadata_v1()
        assert v1.display_size is None
        assert v1.internal_size is None
        assert v1.precision is None
        assert v1.scale is None

    def test_vector_dimension_not_in_v1(self):
        # ResultMetadata is a NamedTuple — vector_dimension must not leak into it.
        v2 = ResultMetadataV2(name="v", type_code=7, is_nullable=False, vector_dimension=3)
        v1 = v2._to_result_metadata_v1()
        assert not hasattr(v1, "vector_dimension")


class TestResultMetadataV2Protocol:
    """Protocol contracts: repr format, equality edge cases."""

    def test_repr_contains_required_fields(self):
        # Regression anchor — format must survive refactors.
        v2 = ResultMetadataV2(name="col", type_code=2, is_nullable=True, vector_dimension=3)
        r = repr(v2)
        assert r.startswith("ResultMetadataV2(")
        assert "name=" in r
        assert "type_code=" in r
        assert "is_nullable=" in r
        assert "vector_dimension=" in r
        assert "fields=" in r

    def test_eq_with_non_v2_returns_not_implemented(self):
        v2 = ResultMetadataV2(name="x", type_code=2, is_nullable=True)
        assert v2.__eq__("not-a-v2") is NotImplemented
        assert v2.__eq__(42) is NotImplemented

    def test_eq_recurses_into_non_none_fields(self):
        # Manually construct non-None fields to confirm equality recurses.
        child_a = ResultMetadataV2(name="elem", type_code=0, is_nullable=False)
        child_b = ResultMetadataV2(name="elem", type_code=2, is_nullable=False)
        a = ResultMetadataV2(name="arr", type_code=12, is_nullable=True, fields=[child_a])
        b = ResultMetadataV2(name="arr", type_code=12, is_nullable=True, fields=[child_b])
        assert a != b

    def test_eq_with_matching_non_none_fields(self):
        child = ResultMetadataV2(name="elem", type_code=0, is_nullable=True)
        a = ResultMetadataV2(name="arr", type_code=12, is_nullable=True, fields=[child])
        b = ResultMetadataV2(name="arr", type_code=12, is_nullable=True, fields=[child])
        assert a == b
