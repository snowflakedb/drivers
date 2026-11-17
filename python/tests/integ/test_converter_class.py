"""``converter_class`` on the fetch path, run against both the new and the reference driver.

The reference driver only consults ``converter_class`` for JSON result chunks, so its
session is switched to JSON; the new driver always fetches Arrow.
"""

import json

import pytest

from tests.compatibility import is_new_driver, is_old_driver
from tests.conftest import run_against_sync_and_async


pytestmark = run_against_sync_and_async

if is_new_driver():
    from snowflake.connector._internal.type_codes import ARRAY, OBJECT, VARIANT
    from snowflake.connector.converter import ArrowConverterContext

    class VariantToDictConverter(ArrowConverterContext):
        deserialized_types = frozenset({VARIANT, OBJECT, ARRAY})

        def deserialize(self, value, column):
            return json.loads(value) if isinstance(value, str) else value

else:
    from snowflake.connector.converter import SnowflakeConverter

    class VariantToDictConverter(SnowflakeConverter):
        def _VARIANT_to_python(self, _):
            return json.loads

        _OBJECT_to_python = _VARIANT_to_python
        _ARRAY_to_python = _VARIANT_to_python


QUERY = """
    select
        1 as n,
        parse_json('{"a": 1, "b": [1, 2]}') as v,
        object_construct('k', 'v') as o,
        array_construct(1, 2, 3) as a,
        null::variant as nv,
        'text' as t
"""
EXPECTED = (1, {"a": 1, "b": [1, 2]}, {"k": "v"}, [1, 2, 3], None, "text")


@pytest.fixture
def converting_connection(connection_factory):
    with connection_factory(converter_class=VariantToDictConverter) as conn:
        if is_old_driver():
            conn.cursor().execute("alter session set python_connector_query_result_format='JSON'")
        yield conn


def test_default_converter_returns_semi_structured_values_as_text(connection):
    row = connection.cursor().execute(QUERY).fetchone()

    assert [json.loads(value) for value in row[1:4]] == list(EXPECTED[1:4])
    assert all(isinstance(value, str) for value in row[1:4])
    assert row[4] is None


def test_variant_to_dict_converter_fetchone(converting_connection):
    assert converting_connection.cursor().execute(QUERY).fetchone() == EXPECTED


MANY_ROWS = """
    select object_construct('i', i) as v
    from (select seq4() as i from table(generator(rowcount => 5)))
    order by i
"""


def test_variant_to_dict_converter_fetchmany(converting_connection):
    cur = converting_connection.cursor().execute(MANY_ROWS)

    assert cur.fetchmany(2) == [({"i": 0},), ({"i": 1},)]
    assert cur.fetchmany(2) == [({"i": 2},), ({"i": 3},)]


def test_variant_to_dict_converter_fetchall(converting_connection):
    cur = converting_connection.cursor().execute(MANY_ROWS)

    assert cur.fetchall() == [({"i": i},) for i in range(5)]


def test_variant_to_dict_converter_iteration(converting_connection):
    cur = converting_connection.cursor().execute(QUERY)

    assert list(cur) == [EXPECTED]
