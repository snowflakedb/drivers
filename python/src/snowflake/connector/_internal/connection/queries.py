"""SQL query strings shared by sync and async connection implementations."""

from __future__ import annotations


COMMIT_SQL = "COMMIT"
ROLLBACK_SQL = "ROLLBACK"
SET_AUTOCOMMIT_SQL = "ALTER SESSION SET autocommit={autocommit}"
CURRENT_VERSION_SQL = "SELECT CURRENT_VERSION() AS version"
