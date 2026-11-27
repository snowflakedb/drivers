from __future__ import annotations

from snowflake.connector.errors import OperationalError

from ..native_blocking import run_native_blocking
from ..protobuf_gen.proto_exception import ProtoTransportException


try:
    from snowflake.connector._core import sf_core_python
except ImportError as err:
    raise OperationalError(
        msg=(
            "Couldn't load the native library (sf_core_python). "
            "Ensure the package was installed from a pre-built wheel or built locally."
        )
    ) from err


class ProtoTransport:
    """Bridge between Python proto RPC calls and the Rust core via PyO3.

    :meth:`handle_message_async` — returns a native awaitable from Rust. The
        request runs as a tokio task; dropping the awaitable (which is what
        cancelling the surrounding asyncio task does) cancels the operation's
        token in core.

    :meth:`handle_message` — blocking call (releases the GIL). In a gevent-patched
        process the call runs on a gevent thread pool and the calling greenlet yields
        until it returns; see :mod:`native_blocking`.
    """

    async def handle_message_async(self, api: str, method: str, message: bytes) -> tuple[int, bytes]:
        result: tuple[int, bytes] = await sf_core_python.call_proto_async(api, method, message)  # type: ignore[assignment]
        status, response_bytes = result
        if status in (0, 1, 2):
            return status, response_bytes
        raise ProtoTransportException(f"Unknown error code: {status}")

    def handle_message(self, api: str, method: str, message: bytes) -> tuple[int, bytes]:
        status, response_bytes = run_native_blocking(sf_core_python.call_proto, api, method, message)
        if status in (0, 1, 2):
            return status, response_bytes
        raise ProtoTransportException(f"Unknown error code: {status}")
