"""SQL query strings shared by sync and async connection implementations."""

from __future__ import annotations


COMMIT_SQL = "COMMIT"
ROLLBACK_SQL = "ROLLBACK"
SET_AUTOCOMMIT_SQL = "ALTER SESSION SET autocommit={autocommit}"
SET_CLIENT_PREFETCH_THREADS_SQL = "ALTER SESSION SET CLIENT_PREFETCH_THREADS = {value}"
CURRENT_VERSION_SQL = "SELECT CURRENT_VERSION() AS version"
