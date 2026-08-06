from __future__ import annotations

import asyncio

from snowflake.connector.errors import OperationalError

from ..protobuf_gen.proto_exception import ProtoTransportException


try:
    from snowflake.connector._core import sf_core_python
except ImportError as err:
    raise OperationalError(
        msg=(
            "Couldn't load core driver dependency (sf_core_python). "
            "Ensure the package was installed from a pre-built wheel or built locally."
        )
    ) from err


class ProtoTransport:
    """Bridge between Python proto RPC calls and the Rust core via PyO3.

    :meth:`handle_message_async` - spawns the request on the tokio runtime.
        A Python callback resolves an asyncio.Future when the task completes.
        Cancellation signals the Rust CancellationToken so the waiter skips the
        callback; in-flight work is not aborted until SNOW-3675196.

    :meth:`handle_message` — blocking call (releases the GIL).
    """

    async def handle_message_async(self, api: str, method: str, message: bytes) -> tuple[int, bytes]:
        loop = asyncio.get_running_loop()
        future: asyncio.Future[tuple[int, bytes]] = loop.create_future()

        def on_response(status: int, response_bytes: bytes) -> None:
            def _set() -> None:
                # The Future may have been canceled while Rust was working.
                if not future.done():
                    future.set_result((status, response_bytes))

            loop.call_soon_threadsafe(_set)

        async_handle = sf_core_python.call_proto_async(
            api,
            method,
            message,
            on_response,
        )
        if async_handle == 0:
            raise OperationalError(
                msg=(
                    "call_proto_async failed: core not initialized. "
                    "Ensure the package was installed from a pre-built wheel or built locally."
                )
            )

        try:
            status, response_bytes = await future
        except asyncio.CancelledError:
            sf_core_python.cancel(async_handle)
            raise

        if status in (0, 1, 2):
            return status, response_bytes

        raise ProtoTransportException(f"Unknown error code: {status}")

    def handle_message(self, api: str, method: str, message: bytes) -> tuple[int, bytes]:
        status, response_bytes = sf_core_python.call_proto(api, method, message)
        if status in (0, 1, 2):
            return status, response_bytes
        raise ProtoTransportException(f"Unknown error code: {status}")
