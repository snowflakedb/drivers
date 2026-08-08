"""Unit tests for the real ResultMetadataV2 class (BD#43 partial fix).

Verifies: property access matches legacy interface, vector_dimension is read
from the proto dimension field, _is_nullable is a private attr (Snowpark reads
it directly), and fields is always None (proto limitation).
"""

from unittest.mock import MagicMock

import pytest

from snowflake.connector._internal.api_client.client_api import core_driver
from snowflake.connector._internal.cursor.result_metadata import ResultMetadata, ResultMetadataV2
from snowflake.connector._internal.protobuf_gen.database_driver_v1_pb2 import (
    ConnectionHandle,
    StatementHandle,
)
from snowflake.connector.cursor import SnowflakeCursor


def _mock_column(
    name: str = "col",
    col_type: str = "TEXT",
    nullable: bool = True,
    length: int | None = None,
    byte_length: int | None = None,
    precision: int | None = None,
    scale: int | None = None,
    dimension: int | None = None,
) -> MagicMock:
    col = MagicMock()
    col.name = name
    col.type = col_type
    col.nullable = nullable
    col.HasField.side_effect = lambda f: {
        "length": length is not None,
        "byte_length": byte_length is not None,
        "precision": precision is not None,
        "scale": scale is not None,
        "dimension": dimension is not None,
    }.get(f, False)
    col.length = length or 0
    col.byte_length = byte_length or 0
    col.precision = precision or 0
    col.scale = scale or 0
    col.dimension = dimension if dimension is not None else 0
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
        col = _mock_column(col_type="VECTOR", dimension=128)
        v2 = ResultMetadataV2.from_column(col)
        assert v2.vector_dimension == 128

    def test_vector_dimension_none_when_field_absent(self):
        col = _mock_column(col_type="FIXED")
        v2 = ResultMetadataV2.from_column(col)
        assert v2.vector_dimension is None

    def test_fields_always_none(self):
        # Proto has no nested column list — BD#43 remainder.
        col = _mock_column(col_type="OBJECT")
        v2 = ResultMetadataV2.from_column(col)
        assert v2.fields is None

    def test_nullable_and_name_populated(self):
        col = _mock_column(name="amount", col_type="FIXED", nullable=False)
        v2 = ResultMetadataV2.from_column(col)
        assert v2.name == "amount"
        assert v2.is_nullable is False

    def test_precision_and_scale_populated(self):
        col = _mock_column(col_type="FIXED", precision=18, scale=6)
        v2 = ResultMetadataV2.from_column(col)
        assert v2.precision == 18
        assert v2.scale == 6


class TestResultMetadataV2CreateDescription:
    def test_returns_none_for_none_result(self):
        assert ResultMetadataV2.create_description(None) is None

    def test_returns_none_for_result_with_no_columns(self):
        result = MagicMock()
        result.columns = []
        assert ResultMetadataV2.create_description(result) is None

    def test_returns_list_of_v2_objects(self):
        col = _mock_column(col_type="TEXT", length=100)
        result = MagicMock()
        result.columns = [col]
        desc = ResultMetadataV2.create_description(result)
        assert desc is not None
        assert len(desc) == 1
        assert isinstance(desc[0], ResultMetadataV2)
        assert desc[0].display_size == 100

    def test_vector_dimension_in_create_description(self):
        col = _mock_column(col_type="VECTOR", dimension=64)
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
        """_describe_internal() returns ResultMetadataV2 objects, not legacy ResultMetadata."""
        col = _mock_column(name="AMOUNT", col_type="FIXED", precision=18, scale=6)
        self._setup_prepare(mock_core_client, columns=[col])

        result = cursor._describe_internal("SELECT 1 AS AMOUNT")

        assert result is not None
        assert len(result) == 1
        assert isinstance(result[0], ResultMetadataV2)
        assert result[0].name == "AMOUNT"
        assert result[0].precision == 18
        assert result[0].scale == 6

    def test_returns_none_for_no_columns(self, cursor, mock_core_client):
        """_describe_internal() returns None when the statement produces no result set."""
        self._setup_prepare(mock_core_client, columns=[])

        assert cursor._describe_internal("INSERT INTO t VALUES (1)") is None

    def test_vector_dimension_populated(self, cursor, mock_core_client):
        """_describe_internal() propagates vector_dimension from the proto dimension field."""
        col = _mock_column(col_type="VECTOR", dimension=64)
        self._setup_prepare(mock_core_client, columns=[col])

        result = cursor._describe_internal("SELECT vec")

        assert result is not None
        assert result[0].vector_dimension == 64

    def test_display_size_populated_for_text(self, cursor, mock_core_client):
        """_describe_internal() populates display_size from length for TEXT columns."""
        col = _mock_column(col_type="TEXT", length=256)
        self._setup_prepare(mock_core_client, columns=[col])

        result = cursor._describe_internal("SELECT s")

        assert result is not None
        assert result[0].display_size == 256


class TestResultMetadataV2FromColumnFieldSizes:
    """Verify display_size and internal_size proto-field mapping (BD#56)."""

    def test_display_size_populated_for_text_column(self):
        col = _mock_column(col_type="TEXT", length=100)
        v2 = ResultMetadataV2.from_column(col)
        assert v2.display_size == 100

    def test_display_size_none_for_non_text_column(self):
        # FIXED with length present — type guard must suppress display_size.
        col = _mock_column(col_type="FIXED", length=100)
        v2 = ResultMetadataV2.from_column(col)
        assert v2.display_size is None

    def test_display_size_none_when_length_field_absent(self):
        col = _mock_column(col_type="TEXT")  # length=None → HasField("length") False
        v2 = ResultMetadataV2.from_column(col)
        assert v2.display_size is None

    def test_internal_size_populated_from_byte_length(self):
        col = _mock_column(col_type="TEXT", byte_length=400)
        v2 = ResultMetadataV2.from_column(col)
        assert v2.internal_size == 400

    def test_internal_size_none_when_byte_length_absent(self):
        col = _mock_column(col_type="TEXT")  # byte_length=None → HasField False
        v2 = ResultMetadataV2.from_column(col)
        assert v2.internal_size is None


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
