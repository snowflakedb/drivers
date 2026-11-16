"""Routes core browser-open requests to a user ``open_external_browser_callback``.

The core invokes the opener from one of its worker threads. Callbacks such as a
Playwright sync page are bound to the thread that created them, so the sync
opener hands each URL back to the thread that called ``connect()`` and the
async opener schedules it on the connection's event loop.
"""

from __future__ import annotations

import asyncio
import inspect
import queue
import threading

from collections.abc import Callable
from concurrent.futures import Future
from typing import TYPE_CHECKING

from ..errors import OperationalError, ProgrammingError
from .api_client.bridge import sf_core_python


if TYPE_CHECKING:
    from .protobuf_gen.database_driver_v1_pb2 import ConnectionHandle

BrowserCallback = Callable[[str], object]

_SET_OPENER_ERRORS = {
    1: "the connection handle is not known to the core",
    2: "the native core was not initialized",
}


def register_browser_opener(conn_handle: ConnectionHandle, opener: Callable[[str], None]) -> None:
    status = sf_core_python.connection_set_browser_opener(conn_handle.id, conn_handle.magic, opener)
    if status != 0:
        reason = _SET_OPENER_ERRORS.get(status, f"status {status}")
        raise OperationalError(
            msg=f"Could not register open_external_browser_callback: {reason}. Create a new connection and retry."
        )


class CallerThreadBrowserOpener:
    """Runs the callback on the thread that calls :meth:`run_init`.

    The core only opens a browser during ``connection_init``. After
    ``run_init`` returns, a later open raises ``OperationalError`` instead of
    invoking the callback on a worker thread.
    """

    def __init__(self, callback: BrowserCallback) -> None:
        self._callback = callback
        self._requests: queue.Queue[tuple[str, Future[None]] | None] = queue.Queue()
        self._lock = threading.Lock()
        self._serving = True

    def __call__(self, url: str) -> None:
        done: Future[None] | None = None
        with self._lock:
            if self._serving:
                done = Future()
                self._requests.put((url, done))
        if done is None:
            raise OperationalError(
                msg="open_external_browser_callback was not run: connect() has finished or was interrupted. "
                "Pass the callback on a new connect() call."
            )
        done.result()

    def run_init(self, init: Callable[[], None]) -> None:
        errors: list[BaseException] = []

        def initialize() -> None:
            try:
                init()
            except BaseException as error:
                errors.append(error)
            finally:
                self._requests.put(None)

        init_thread = threading.Thread(target=initialize, name="snowflake-connection-init")
        init_thread.start()
        try:
            while (request := self._requests.get()) is not None:
                url, done = request
                try:
                    self._run_callback(url)
                except Exception as error:
                    done.set_exception(error)
                except BaseException as error:
                    done.set_exception(error)
                    raise
                else:
                    done.set_result(None)
        finally:
            self._stop_serving()
            init_thread.join()
        if errors:
            raise errors[0]

    def _run_callback(self, url: str) -> None:
        result = self._callback(url)
        if inspect.isawaitable(result):
            close = getattr(result, "close", None)
            if callable(close):
                close()
            raise ProgrammingError(
                msg="open_external_browser_callback returned an awaitable on the sync connector. "
                "Use a non-async function with snowflake.connector.connect, or pass the callback "
                "to snowflake.connector.aio.connect."
            )

    def _stop_serving(self) -> None:
        with self._lock:
            self._serving = False
        while True:
            try:
                request = self._requests.get_nowait()
            except queue.Empty:
                return
            if request is not None:
                request[1].set_exception(
                    OperationalError(msg="open_external_browser_callback was not run: connect() was interrupted.")
                )


class EventLoopBrowserOpener:
    """Runs the callback on ``loop``; the callback may return an awaitable."""

    def __init__(self, callback: BrowserCallback, loop: asyncio.AbstractEventLoop) -> None:
        self._callback = callback
        self._loop = loop

    def __call__(self, url: str) -> None:
        asyncio.run_coroutine_threadsafe(self._open(url), self._loop).result()

    async def _open(self, url: str) -> None:
        result = self._callback(url)
        if inspect.isawaitable(result):
            await result
