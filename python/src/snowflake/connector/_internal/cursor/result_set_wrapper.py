from __future__ import annotations

from ...errors import ProgrammingError
from ..api_client.client_api import core_driver
from ..errorcode import ER_NO_DATA_FOUND
from ..logging import get_logger
from ..protobuf_gen.database_driver_v1_pb2 import ResultSetHandle


logger = get_logger(__name__)


class ResultSetWrapperBase:
    """Owns a ``ResultSetHandle`` and guards against misuse.

    Lifecycle: ``replace()`` releases the previous handle before adopting a
    new one; ``release()`` frees the current handle.  ``__del__`` acts as a
    safety net for handles that were never explicitly released.

    The core layer supports repeated ``result_set_get_stream`` calls — each
    invocation builds a fresh Arrow stream from the stored ``RowsetData``.

    Release/replace are always synchronous (pure FFI handle deletion, no network I/O).
    """

    __slots__ = ("_handle",)

    def __init__(self, handle: ResultSetHandle | None = None) -> None:
        self._handle: ResultSetHandle | None = handle

    def __del__(self) -> None:
        self._do_release()

    def replace(self, new_handle: ResultSetHandle | None) -> None:
        """Release the current handle (if any) and adopt *new_handle*."""
        self._do_release()
        self._handle = new_handle

    def release(self) -> None:
        """Explicitly release the handle."""
        self._do_release()

    def _do_release(self) -> None:
        handle = self._take_handle()
        if handle is None:
            return
        try:
            core_driver.result_set_release(result_set_handle=handle)
        except Exception as e:
            logger.warning("Failed to release ResultSet handle: %s", type(e).__name__)
            logger.debug("Failed to release ResultSet handle", exc_info=True)

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
