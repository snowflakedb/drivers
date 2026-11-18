"""Query status metadata returned by ``Connection.get_query_status_info``."""

from __future__ import annotations

from dataclasses import dataclass

from ._internal.connection.connection import ConnectionMixin
from .constants import QueryStatus


@dataclass(frozen=True)
class QueryStatusInfo:
    """Server-reported status and execution metadata for a query.

    Times and ``total_duration`` are milliseconds (epoch ms for start/end).
    Optional warehouse and error fields are ``None`` when the monitoring
    response omits them. ``Connection.get_query_status`` still returns only
    the :class:`QueryStatus` enum.
    """

    status: QueryStatus
    query_id: str
    session_id: int
    sql_text: str
    state: str
    start_time: int
    end_time: int
    total_duration: int
    error_code: int | None
    error_message: str | None
    warehouse_id: int
    warehouse_name: str | None
    warehouse_external_size: str | None
    warehouse_server_type: str | None

    def is_still_running(self) -> bool:
        return ConnectionMixin.is_still_running(self.status)

    def is_an_error(self) -> bool:
        return ConnectionMixin.is_an_error(self.status)

    def is_success(self) -> bool:
        return self.status == QueryStatus.SUCCESS
