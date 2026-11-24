"""Smoke tests for the connector inside a stored procedure."""

from __future__ import annotations

import pytest


# The XP backend is not registered inside the stored procedure yet (ExecPlatform
# XpDriverProviders.cpp), so connections fail with code -1002. Remove the xfail
# once the backend is wired; strict=True makes the unexpected pass fail the run.
BACKEND_NOT_READY = pytest.mark.xfail(
    strict=True,
    reason="XP backend is not registered inside the stored procedure yet (code -1002)",
)


@BACKEND_NOT_READY
def test_should_select_one(cursor):
    # When Query "SELECT 1" is executed
    cursor.execute("SELECT 1")

    # Then the result should be a single row with value 1
    assert cursor.fetchall() == [(1,)]


@BACKEND_NOT_READY
def test_should_select_one_on_new_connection(connection_factory):
    # Given a connection opened inside the stored procedure
    with connection_factory() as conn, conn.cursor() as cur:
        # When Query "SELECT 1" is executed
        cur.execute("SELECT 1")

        # Then the result should be a single row with value 1
        assert cur.fetchone() == (1,)
