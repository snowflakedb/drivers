"""Unit tests for the callback-based async FFI in ProtoTransport.

These tests mock at the FFI boundary (sf_core_api_call_proto_async) so they
run without the Rust dylib loaded. They cover:

* successful single-call path (status 0)
* application-error path (status 1)
* transport-error path (status 2)
* unknown status -> ProtoTransportException
* cancellation safety -- callback firing into a canceled Future must not crash
* lifetime safety -- callback ref is pinned to the Future
* concurrency -- many in-flight calls resolve independently
* empty message edge case
"""

from __future__ import annotations

import asyncio
import ctypes

from typing import Any

import pytest

from snowflake.connector._internal.api_client import bridge
from snowflake.connector._internal.api_client.bridge import ProtoTransport
from snowflake.connector._internal.protobuf_gen.proto_exception import (
    ProtoTransportException,
)


class _FakeFFI:
    """Records FFI calls and lets the test fire the callback at will."""

    def __init__(self) -> None:
        self.calls: list[dict[str, Any]] = []
        self._next_id = 1

    def __call__(
        self,
        api_bytes: bytes,
        method_bytes: bytes,
        request_ptr: Any,
        request_len: int,
        callback: Any,
        user_data: Any,
    ) -> int:
        request_id = self._next_id
        self._next_id += 1
        self.calls.append(
            {
                "id": request_id,
                "api": api_bytes,
                "method": method_bytes,
                "request_len": request_len,
                "callback": callback,
                "user_data": user_data,
            }
        )
        return request_id

    def complete(self, index: int, status: int, payload: bytes) -> None:
        """Simulate Rust firing the callback for the index-th call."""
        call = self.calls[index]
        buf = (ctypes.c_ubyte * len(payload)).from_buffer_copy(payload)
        ptr = ctypes.cast(buf, ctypes.POINTER(ctypes.c_ubyte))
        call["callback"](call["user_data"], status, ptr, len(payload))


@pytest.fixture
def fake_ffi(monkeypatch: pytest.MonkeyPatch) -> _FakeFFI:
    """Patch sf_core_api_call_proto_async with a recording fake."""
    fake = _FakeFFI()
    monkeypatch.setattr(bridge, "sf_core_api_call_proto_async", fake)
    return fake


class TestSingleCall:
    """Verify the basic request-response flow."""

    def test_status_0_returns_response(self, fake_ffi: _FakeFFI) -> None:
        async def run() -> tuple[int, bytes]:
            transport = ProtoTransport()
            task = asyncio.create_task(transport.handle_message_async("DatabaseDriver", "database_new", b"req"))
            await asyncio.sleep(0)
            assert len(fake_ffi.calls) == 1
            fake_ffi.complete(0, 0, b"response-bytes")
            return await task

        status, response = asyncio.run(run())
        assert status == 0
        assert response == b"response-bytes"

    def test_status_1_returned_unchanged(self, fake_ffi: _FakeFFI) -> None:
        """Application errors flow through; the transport never raises on status 1."""

        async def run() -> tuple[int, bytes]:
            transport = ProtoTransport()
            task = asyncio.create_task(transport.handle_message_async("DatabaseDriver", "database_new", b"req"))
            await asyncio.sleep(0)
            fake_ffi.complete(0, 1, b"app-error-bytes")
            return await task

        status, response = asyncio.run(run())
        assert status == 1
        assert response == b"app-error-bytes"

    def test_status_2_returned_unchanged(self, fake_ffi: _FakeFFI) -> None:
        async def run() -> tuple[int, bytes]:
            transport = ProtoTransport()
            task = asyncio.create_task(transport.handle_message_async("DatabaseDriver", "database_new", b"req"))
            await asyncio.sleep(0)
            fake_ffi.complete(0, 2, b"transport-error-msg")
            return await task

        status, response = asyncio.run(run())
        assert status == 2
        assert response == b"transport-error-msg"

    def test_unknown_status_raises(self, fake_ffi: _FakeFFI) -> None:
        async def run() -> None:
            transport = ProtoTransport()
            task = asyncio.create_task(transport.handle_message_async("DatabaseDriver", "database_new", b"req"))
            await asyncio.sleep(0)
            fake_ffi.complete(0, 99, b"")
            await task

        with pytest.raises(ProtoTransportException, match="Unknown error code: 99"):
            asyncio.run(run())

    def test_request_passed_through(self, fake_ffi: _FakeFFI) -> None:
        """Verify api / method / request bytes reach the FFI correctly."""

        async def run() -> None:
            transport = ProtoTransport()
            task = asyncio.create_task(
                transport.handle_message_async("DatabaseDriver", "connection_init", b"\x01\x02\x03")
            )
            await asyncio.sleep(0)
            fake_ffi.complete(0, 0, b"")
            await task

        asyncio.run(run())
        call = fake_ffi.calls[0]
        assert call["api"] == b"DatabaseDriver"
        assert call["method"] == b"connection_init"
        assert call["request_len"] == 3


class TestCancellationSafety:
    """Cancellation must not crash. Two scenarios matter:

    1. Future cancelled BEFORE callback fires -> callback must not raise
       InvalidStateError when it tries to set_result on a cancelled Future.
    2. Future cancelled AFTER callback already fired -> normal cancellation,
       no special behaviour needed.
    """

    def test_callback_after_cancel_does_not_crash(self, fake_ffi: _FakeFFI) -> None:
        async def run() -> None:
            transport = ProtoTransport()
            task = asyncio.create_task(transport.handle_message_async("DatabaseDriver", "database_new", b"req"))
            await asyncio.sleep(0)
            assert len(fake_ffi.calls) == 1

            task.cancel()
            with pytest.raises(asyncio.CancelledError):
                await task

            # Rust fires the callback after cancellation (the race we're testing).
            fake_ffi.complete(0, 0, b"too-late-response")
            await asyncio.sleep(0)

        asyncio.run(run())

    def test_cancel_before_submit_works(self, fake_ffi: _FakeFFI) -> None:
        """Coroutine cancelled before reaching the FFI — no call submitted."""

        async def run() -> None:
            transport = ProtoTransport()
            coro = transport.handle_message_async("DatabaseDriver", "database_new", b"req")
            task = asyncio.create_task(coro)
            task.cancel()
            with pytest.raises(asyncio.CancelledError):
                await task
            assert len(fake_ffi.calls) == 0

        asyncio.run(run())


class TestLifetimeSafety:
    """The C callback object must be pinned to the Future so it outlives the Rust task."""

    def test_callback_pinned_to_future(self, fake_ffi: _FakeFFI) -> None:
        """Direct attribute check: the callback ref is attached to the Future.

        A weakref/GC test can't distinguish "pinned to future" from "kept alive
        by the awaiting frame". The direct check proves the pin exists, which is
        what protects against use-after-free on cancellation.
        """

        async def run() -> None:
            loop = asyncio.get_running_loop()
            original = loop.create_future
            captured: list[asyncio.Future] = []

            def spy() -> asyncio.Future:
                f = original()
                captured.append(f)
                return f

            loop.create_future = spy  # type: ignore[method-assign]
            try:
                transport = ProtoTransport()
                task = asyncio.create_task(transport.handle_message_async("DatabaseDriver", "database_new", b"req"))
                await asyncio.sleep(0)
            finally:
                loop.create_future = original  # type: ignore[method-assign]

            assert len(captured) == 1, "handle_message_async should create exactly one Future"
            future = captured[0]
            cb = fake_ffi.calls[0]["callback"]

            pinned = getattr(future, "_proto_transport_callback_ref", None)
            assert pinned is cb, (
                "callback not pinned to Future — without this, a cancelled coroutine "
                "could drop the only Python-side reference to the C callback while "
                "Rust still holds the function pointer (use-after-free)"
            )

            fake_ffi.complete(0, 0, b"done")
            await task

        asyncio.run(run())


class TestConcurrency:
    """Multiple in-flight calls must each resolve to their own response."""

    def test_many_concurrent_calls(self, fake_ffi: _FakeFFI) -> None:
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
            assert len(fake_ffi.calls) == n
            # Complete in reverse order to verify each Future tracks its own callback.
            for i in reversed(range(n)):
                fake_ffi.complete(i, 0, f"resp-{i}".encode())
            return await asyncio.gather(*tasks)

        results = asyncio.run(run())
        assert len(results) == 25
        for i, (status, response) in enumerate(results):
            assert status == 0
            assert response == f"resp-{i}".encode()


class TestEmptyMessage:
    """Regression: empty proto messages (e.g. ``DatabaseNewRequest()`` which
    serializes to ``b""``) must not produce a null/dangling pointer that the
    Rust side then dereferences.
    """

    def test_empty_message_passes_through(self, fake_ffi: _FakeFFI) -> None:
        async def run() -> tuple[int, bytes]:
            transport = ProtoTransport()
            task = asyncio.create_task(transport.handle_message_async("DatabaseDriver", "database_new", b""))
            await asyncio.sleep(0)
            assert len(fake_ffi.calls) == 1
            assert fake_ffi.calls[0]["request_len"] == 0
            fake_ffi.complete(0, 0, b"resp")
            return await task

        status, response = asyncio.run(run())
        assert status == 0
        assert response == b"resp"
