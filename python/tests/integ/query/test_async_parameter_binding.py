"""Single-parameter execute for qmark, numeric, and pyformat, including aio."""

from __future__ import annotations

from tests.conftest import run_against_sync_and_async, with_paramstyle


pytestmark = run_against_sync_and_async


class TestAsyncParameterBindingSNOW4017504:
    @with_paramstyle("qmark")
    def test_should_bind_single_qmark_parameter(self, cursor):
        cursor.execute("SELECT ?", (42,))
        assert cursor.fetchone() == (42,)

    @with_paramstyle("numeric")
    def test_should_bind_single_numeric_parameter(self, cursor):
        cursor.execute("SELECT :1", (42,))
        assert cursor.fetchone() == (42,)

    @with_paramstyle("pyformat")
    def test_should_bind_single_pyformat_parameter(self, cursor):
        cursor.execute("SELECT %s", (42,))
        assert cursor.fetchone() == (42,)
