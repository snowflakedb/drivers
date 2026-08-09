"""Unit tests for the async path in ProtoTransport (PyO3 bridge).

Mocks sf_core_python.call_proto_async so tests run without the Rust dylib.

Covers:
* status 0/1/2 pass through unchanged
* unknown status raises ProtoTransportException
* request args forwarded correctly
* cancellation raises CancelledError
* many concurrent calls resolve independently
* empty message edge case
"""

from __future__ import annotations

import asyncio

from typing import Any

import pytest

from snowflake.connector._internal.api_client.bridge import ProtoTransport
from snowflake.connector._internal.protobuf_gen.proto_exception import (
    ProtoTransportException,
)


class _FakeCore:
    """Simulates sf_core_python.call_proto_async as a simple awaitable.

    Each call records its arguments and returns a future. Tests resolve
    futures explicitly via complete(), or leave them pending for cancellation.
    """

    def __init__(self) -> None:
        self.calls: list[dict[str, Any]] = []
        self._futures: list[asyncio.Future[tuple[int, bytes]]] = []

    def call_proto_async(self, api: str, method: str, message: bytes) -> asyncio.Future[tuple[int, bytes]]:
        loop = asyncio.get_running_loop()
        fut: asyncio.Future[tuple[int, bytes]] = loop.create_future()
        self.calls.append({"api": api, "method": method, "message": message})
        self._futures.append(fut)
        return fut

    def complete(self, index: int, status: int, payload: bytes) -> None:
        fut = self._futures[index]
        if not fut.done():
            fut.set_result((status, payload))


@pytest.fixture
def fake_core(monkeypatch: pytest.MonkeyPatch) -> _FakeCore:
    fake = _FakeCore()
    import snowflake.connector._internal.api_client.bridge as bridge_mod

    monkeypatch.setattr(bridge_mod, "sf_core_python", fake)
    return fake


class TestSingleCall:
    """Verify the basic request-response flow."""

    @pytest.mark.parametrize(
        ("status", "payload"),
        [(0, b"response-bytes"), (1, b"app-error-bytes"), (2, b"transport-error-msg")],
        ids=["ok", "app-error", "transport-error"],
    )
    def test_known_status_returned_unchanged(self, fake_core: _FakeCore, status: int, payload: bytes) -> None:
        async def run() -> tuple[int, bytes]:
            transport = ProtoTransport()
            task = asyncio.create_task(transport.handle_message_async("DatabaseDriver", "database_new", b"req"))
            await asyncio.sleep(0)
            fake_core.complete(0, status, payload)
            return await task

        got_status, got_payload = asyncio.run(run())
        assert got_status == status
        assert got_payload == payload

    def test_unknown_status_raises(self, fake_core: _FakeCore) -> None:
        async def run() -> None:
            transport = ProtoTransport()
            task = asyncio.create_task(transport.handle_message_async("DatabaseDriver", "database_new", b"req"))
            await asyncio.sleep(0)
            fake_core.complete(0, 99, b"")
            await task

        with pytest.raises(ProtoTransportException, match="Unknown error code: 99"):
            asyncio.run(run())

    def test_request_args_forwarded(self, fake_core: _FakeCore) -> None:
        async def run() -> None:
            transport = ProtoTransport()
            task = asyncio.create_task(
                transport.handle_message_async("DatabaseDriver", "connection_init", b"\x01\x02\x03")
            )
            await asyncio.sleep(0)
            fake_core.complete(0, 0, b"")
            await task

        asyncio.run(run())
        call = fake_core.calls[0]
        assert call == {"api": "DatabaseDriver", "method": "connection_init", "message": b"\x01\x02\x03"}


class TestCancellation:
    def test_cancel_pending_raises_cancelled_error(self, fake_core: _FakeCore) -> None:
        async def run() -> None:
            transport = ProtoTransport()
            task = asyncio.create_task(transport.handle_message_async("DatabaseDriver", "database_new", b"req"))
            await asyncio.sleep(0)
            task.cancel()
            with pytest.raises(asyncio.CancelledError):
                await task

        asyncio.run(run())

    def test_cancel_before_await_submits_nothing(self, fake_core: _FakeCore) -> None:
        async def run() -> None:
            transport = ProtoTransport()
            task = asyncio.create_task(transport.handle_message_async("DatabaseDriver", "database_new", b"req"))
            task.cancel()
            with pytest.raises(asyncio.CancelledError):
                await task
            assert len(fake_core.calls) == 0

        asyncio.run(run())


class TestConcurrency:
    def test_many_concurrent_calls(self, fake_core: _FakeCore) -> None:
        async def run() -> list[tuple[int, bytes]]:
            transport = ProtoTransport()
            n = 25
            tasks = [
                asyncio.create_task(
                    transport.handle_message_async("DatabaseDriver", "database_new", f"req-{i}".encode())
                )
                for i in range(n)
            ]
            await asyncio.sleep(0)
            assert len(fake_core.calls) == n
            for i in reversed(range(n)):
                fake_core.complete(i, 0, f"resp-{i}".encode())
            return await asyncio.gather(*tasks)

        results = asyncio.run(run())
        assert len(results) == 25
        for i, (status, response) in enumerate(results):
            assert status == 0
            assert response == f"resp-{i}".encode()


class TestEmptyMessage:
    def test_empty_message_passes_through(self, fake_core: _FakeCore) -> None:
        async def run() -> tuple[int, bytes]:
            transport = ProtoTransport()
            task = asyncio.create_task(transport.handle_message_async("DatabaseDriver", "database_new", b""))
            await asyncio.sleep(0)
            assert fake_core.calls[0]["message"] == b""
            fake_core.complete(0, 0, b"resp")
            return await task

        status, response = asyncio.run(run())
        assert status == 0
        assert response == b"resp"
