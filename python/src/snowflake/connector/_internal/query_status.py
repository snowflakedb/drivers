"""Map Core ``ConnectionGetQueryStatusResponse`` to public ``QueryStatusInfo``."""

from __future__ import annotations

from typing import TYPE_CHECKING

from ..constants import QueryStatus
from ..query_status import QueryStatusInfo
from .logging import get_logger


if TYPE_CHECKING:
    from .protobuf_gen.database_driver_v1_services import ConnectionGetQueryStatusResponse


logger = get_logger(__name__)


def parse_query_status(status_name: str) -> QueryStatus:
    """Map a wire status name to :class:`QueryStatus`, treating unknowns as ``NO_DATA``."""
    try:
        return QueryStatus[status_name]
    except KeyError:
        logger.warning("Unknown query status %r; treating as NO_DATA", status_name)
        return QueryStatus.NO_DATA


def query_status_info_from_response(
    response: ConnectionGetQueryStatusResponse,
    status: QueryStatus,
    *,
    query_id_fallback: str = "",
) -> QueryStatusInfo:
    """Build :class:`QueryStatusInfo` from a Core response and a parsed status enum."""
    return QueryStatusInfo(
        status=status,
        query_id=response.query_id or query_id_fallback,
        session_id=response.session_id,
        sql_text=response.sql_text,
        state=response.state,
        start_time=response.start_time,
        end_time=response.end_time,
        total_duration=response.total_duration,
        error_code=response.error_code if response.HasField("error_code") else None,
        error_message=response.error_message if response.HasField("error_message") else None,
        warehouse_id=response.warehouse_id,
        warehouse_name=response.warehouse_name if response.HasField("warehouse_name") else None,
        warehouse_external_size=(
            response.warehouse_external_size if response.HasField("warehouse_external_size") else None
        ),
        warehouse_server_type=(response.warehouse_server_type if response.HasField("warehouse_server_type") else None),
    )
