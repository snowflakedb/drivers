from __future__ import annotations

import logging

from ...errors import ProgrammingError
from ..api_client.client_api import core_driver
from ..errorcode import ER_NO_DATA_FOUND
from ..protobuf_gen.database_driver_v1_pb2 import ResultSetHandle


logger = logging.getLogger(__name__)


class ResultSetWrapperBase:
    """Owns a ``ResultSetHandle`` and guards against misuse.

    Lifecycle: ``replace()`` releases the previous handle before adopting a
    new one; ``release()`` frees the current handle.  ``__del__`` acts as a
    safety net for handles that were never explicitly released.

    The core layer supports repeated ``result_set_get_stream`` calls — each
    invocation builds a fresh Arrow stream from the stored ``RowsetData``.

    Sync and async specializations live in :mod:`snowflake.connector.cursor._result_set_wrapper`
    and :mod:`snowflake.connector.aio.cursor._result_set_wrapper` respectively.
    """

    __slots__ = ("_handle",)

    def __init__(self, handle: ResultSetHandle | None = None) -> None:
        self._handle: ResultSetHandle | None = handle

    def __del__(self) -> None:
        # __del__ cannot be a coroutine, so the safety-net release stays
        # synchronous (via the sync core_driver). Explicit teardown should go
        # through release()/replace().
        handle = self._handle
        if handle is None:
            return
        self._handle = None
        try:
            core_driver.result_set_release(result_set_handle=handle)
        except Exception:
            logger.warning("Failed to release ResultSet handle", exc_info=True)

    def _take_handle(self) -> ResultSetHandle | None:
        handle = self._handle
        if handle is None:
            return None
        self._handle = None
        return handle

    def _require_handle(self) -> ResultSetHandle:
        if self._handle is None:
            raise ProgrammingError(
                msg="No results available (not produced by this query)",
                errno=ER_NO_DATA_FOUND,
            )
        return self._handle
