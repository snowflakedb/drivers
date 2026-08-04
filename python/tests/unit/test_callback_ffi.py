"""Unit tests for the async path in ProtoTransport (PyO3 bridge).

These tests mock at the PyO3 boundary (sf_core_python.call_proto_async / cancel)
so they run without the Rust dylib loaded. They cover:

* successful single-call path (status 0)
* application-error path (status 1)
* transport-error path (status 2)
* unknown status -> ProtoTransportException
* cancellation propagation
* concurrency -- many in-flight calls resolve independently
* empty message edge case
"""

from __future__ import annotations

import asyncio

from collections.abc import Callable
from typing import Any

import pytest

from snowflake.connector._internal.api_client.bridge import ProtoTransport
from snowflake.connector._internal.protobuf_gen.proto_exception import (
    ProtoTransportException,
)


class _FakeCore:
    """Simulates sf_core_python.call_proto_async and sf_core_python.cancel."""

    def __init__(self) -> None:
        self.calls: list[dict[str, Any]] = []
        self.cancelled_ids: list[int] = []
        self._next_id = 1

    def call_proto_async(
        self,
        api: str,
        method: str,
        message: bytes,
        callback: Callable[[int, bytes], None],
    ) -> int:
        request_id = self._next_id
        self._next_id += 1
        self.calls.append(
            {
                "id": request_id,
                "api": api,
                "method": method,
                "message": message,
                "callback": callback,
            }
        )
        return request_id

    def cancel(self, async_handle: int) -> None:
        self.cancelled_ids.append(async_handle)

    def complete(self, index: int, status: int, payload: bytes) -> None:
        """Simulate Rust firing the callback for the index-th call."""
        call = self.calls[index]
        call["callback"](status, payload)


@pytest.fixture
def fake_core(monkeypatch: pytest.MonkeyPatch) -> _FakeCore:
    """Patch sf_core_python.call_proto_async and sf_core_python.cancel."""
    fake = _FakeCore()
    import snowflake.connector._internal.api_client.bridge as bridge_mod

    monkeypatch.setattr(bridge_mod, "sf_core_python", fake)
    return fake


class TestSingleCall:
    """Verify the basic request-response flow."""

    def test_status_0_returns_response(self, fake_core: _FakeCore) -> None:
        async def run() -> tuple[int, bytes]:
            transport = ProtoTransport()
            task = asyncio.create_task(transport.handle_message_async("DatabaseDriver", "database_new", b"req"))
            await asyncio.sleep(0)
            assert len(fake_core.calls) == 1
            fake_core.complete(0, 0, b"response-bytes")
            await asyncio.sleep(0)
            return await task

        status, response = asyncio.run(run())
        assert status == 0
        assert response == b"response-bytes"

    def test_status_1_returned_unchanged(self, fake_core: _FakeCore) -> None:
        async def run() -> tuple[int, bytes]:
            transport = ProtoTransport()
            task = asyncio.create_task(transport.handle_message_async("DatabaseDriver", "database_new", b"req"))
            await asyncio.sleep(0)
            fake_core.complete(0, 1, b"app-error-bytes")
            await asyncio.sleep(0)
            return await task

        status, response = asyncio.run(run())
        assert status == 1
        assert response == b"app-error-bytes"

    def test_status_2_returned_unchanged(self, fake_core: _FakeCore) -> None:
        async def run() -> tuple[int, bytes]:
            transport = ProtoTransport()
            task = asyncio.create_task(transport.handle_message_async("DatabaseDriver", "database_new", b"req"))
            await asyncio.sleep(0)
            fake_core.complete(0, 2, b"transport-error-msg")
            await asyncio.sleep(0)
            return await task

        status, response = asyncio.run(run())
        assert status == 2
        assert response == b"transport-error-msg"

    def test_unknown_status_raises(self, fake_core: _FakeCore) -> None:
        async def run() -> None:
            transport = ProtoTransport()
            task = asyncio.create_task(transport.handle_message_async("DatabaseDriver", "database_new", b"req"))
            await asyncio.sleep(0)
            fake_core.complete(0, 99, b"")
            await asyncio.sleep(0)
            await task

        with pytest.raises(ProtoTransportException, match="Unknown error code: 99"):
            asyncio.run(run())

    def test_request_passed_through(self, fake_core: _FakeCore) -> None:
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
        assert call["api"] == "DatabaseDriver"
        assert call["method"] == "connection_init"
        assert call["message"] == b"\x01\x02\x03"


class TestCancellationSafety:
    def test_cancel_propagates_request_id(self, fake_core: _FakeCore) -> None:
        async def run() -> None:
            transport = ProtoTransport()
            task = asyncio.create_task(transport.handle_message_async("DatabaseDriver", "database_new", b"req"))
            await asyncio.sleep(0)
            assert len(fake_core.calls) == 1

            task.cancel()
            with pytest.raises(asyncio.CancelledError):
                await task

        asyncio.run(run())
        assert fake_core.cancelled_ids == [1]

    def test_normal_completion_does_not_cancel(self, fake_core: _FakeCore) -> None:
        async def run() -> tuple[int, bytes]:
            transport = ProtoTransport()
            task = asyncio.create_task(transport.handle_message_async("DatabaseDriver", "database_new", b"req"))
            await asyncio.sleep(0)
            fake_core.complete(0, 0, b"ok")
            await asyncio.sleep(0)
            return await task

        asyncio.run(run())
        assert fake_core.cancelled_ids == []

    def test_cancel_before_submit_works(self, fake_core: _FakeCore) -> None:
        """Coroutine cancelled before reaching the bridge — no call submitted."""

        async def run() -> None:
            transport = ProtoTransport()
            coro = transport.handle_message_async("DatabaseDriver", "database_new", b"req")
            task = asyncio.create_task(coro)
            task.cancel()
            with pytest.raises(asyncio.CancelledError):
                await task
            assert len(fake_core.calls) == 0

        asyncio.run(run())
        assert fake_core.cancelled_ids == []


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
            await asyncio.sleep(0)
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
            assert len(fake_core.calls) == 1
            assert fake_core.calls[0]["message"] == b""
            fake_core.complete(0, 0, b"resp")
            await asyncio.sleep(0)
            return await task

        status, response = asyncio.run(run())
        assert status == 0
        assert response == b"resp"
