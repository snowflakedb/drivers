"""Native waits must not freeze a gevent hub.

The offload decision is tested against fake gevent modules, which keeps
``monkey.patch_all()`` out of the pytest process. The end-to-end tests run a
real gevent hub in a subprocess.
"""

from __future__ import annotations

import contextvars
import logging
import subprocess
import sys
import textwrap
import threading
import types

from typing import Any

import pytest

from snowflake.connector._internal import arrow_stream_utils, native_blocking
from snowflake.connector._internal.api_client.bridge import ProtoTransport
from snowflake.connector._internal.arrow_context import ArrowConverterContext
from snowflake.connector._internal.native_blocking import (
    DEFAULT_THREADPOOL_SIZE,
    THREADPOOL_SIZE_ENV_VAR,
    OffloadedRowIterator,
    OffloadedTableIterator,
    offload_row_iterator,
    offload_table_iterator,
    run_native_blocking,
)
from snowflake.connector._internal.protobuf_gen.proto_exception import (
    ProtoTransportException,
)
from snowflake.connector.errors import OperationalError


requires_real_gevent = pytest.mark.skipif(
    sys.version_info >= (3, 14),
    reason="gevent is a test dependency only for Python < 3.14 (no free-threaded wheels)",
)


class _Interrupt(BaseException):
    """Stands in for ``GreenletExit`` and ``gevent.Timeout``."""


class _Pool:
    def __init__(self, size: int, hub: Any = None) -> None:
        self.size = size
        self.maxsize = size
        self.hub = hub
        self.calls = 0
        self.pending = 0
        self.interrupt: BaseException | None = None

    def __len__(self) -> int:
        return self.pending

    def apply(self, func: Any, args: tuple[Any, ...] = (), kwds: dict[str, Any] | None = None) -> Any:
        self.calls += 1
        if self.interrupt is not None:
            raise self.interrupt
        box: dict[str, Any] = {}

        def target() -> None:
            try:
                box["value"] = func(*args, **(kwds or {}))
            except BaseException as exc:
                box["error"] = exc

        worker = threading.Thread(target=target)
        worker.start()
        worker.join()
        if "error" in box:
            raise box["error"]
        return box["value"]


class _Hub:
    def __init__(self, thread_ident: int) -> None:
        self.thread_ident = thread_ident


class _FakeGevent:
    def __init__(self, hub: _Hub) -> None:
        self.hub = hub
        self.pools: list[_Pool] = []

    @property
    def pool(self) -> _Pool:
        assert len(self.pools) == 1
        return self.pools[0]

    @property
    def calls(self) -> int:
        return sum(pool.calls for pool in self.pools)


def _module(name: str, **attrs: Any) -> types.ModuleType:
    module = types.ModuleType(name)
    for attr, value in attrs.items():
        setattr(module, attr, value)
    return module


def _install_fake_gevent(
    monkeypatch: pytest.MonkeyPatch,
    *,
    patched: bool = True,
    on_hub: bool = False,
    hub_exists: bool = True,
    hub_thread_ident: int | None = None,
    hub_local: bool = True,
) -> _FakeGevent:
    hub = _Hub(threading.get_ident() if hub_thread_ident is None else hub_thread_ident)
    fake = _FakeGevent(hub)
    current = hub if on_hub else object()

    def make_pool(size: int, hub: Any = None) -> _Pool:
        pool = _Pool(size, hub)
        fake.pools.append(pool)
        return pool

    def get_original(module: str, name: str) -> Any:
        assert (module, name) == ("_thread", "get_ident")
        return threading.get_ident

    modules = {
        "gevent": _module("gevent", getcurrent=lambda: current, get_hub=lambda: hub),
        "gevent._hub_local": _module("gevent._hub_local", get_hub_if_exists=lambda: hub if hub_exists else None),
        "gevent.monkey": _module(
            "gevent.monkey",
            is_module_patched=lambda name: patched and name == "socket",
            get_original=get_original,
        ),
        "gevent.threadpool": _module("gevent.threadpool", ThreadPool=make_pool),
    }
    if not hub_local:
        del modules["gevent._hub_local"]
    for name, module in modules.items():
        monkeypatch.setitem(sys.modules, name, module)
    return fake


class _Rows:
    def __init__(self, rows: list[Any]) -> None:
        self._rows = list(rows)
        self.fetch_many_sizes: list[int] = []

    def __iter__(self) -> _Rows:
        return self

    def __next__(self) -> Any:
        raise AssertionError("the offloading wrapper must load rows through fetch_many")

    def fetch_many(self, size: int) -> list[Any]:
        self.fetch_many_sizes.append(size)
        taken, self._rows = self._rows[:size], self._rows[size:]
        return taken

    def fetch_all(self) -> list[Any]:
        taken, self._rows = self._rows, []
        return taken


class _Batches:
    def __init__(self, batches: list[Any]) -> None:
        self._batches = iter(batches)
        self.schema_calls = 0

    def __iter__(self) -> _Batches:
        return self

    def __next__(self) -> Any:
        return next(self._batches)

    def get_converted_schema(self) -> str:
        self.schema_calls += 1
        return "schema"


def test_run_native_blocking_calls_fn_directly_without_gevent() -> None:
    assert run_native_blocking(lambda value: value + 1, 3) == 4


def test_run_native_blocking_uses_driver_pool_when_gevent_patched(monkeypatch: pytest.MonkeyPatch) -> None:
    fake = _install_fake_gevent(monkeypatch)
    assert run_native_blocking(lambda value: value * 2, 5) == 10
    assert fake.pool.calls == 1
    assert fake.pool.hub is fake.hub
    assert fake.pool.size == DEFAULT_THREADPOOL_SIZE


def test_run_native_blocking_runs_fn_on_another_thread(monkeypatch: pytest.MonkeyPatch) -> None:
    _install_fake_gevent(monkeypatch)
    assert run_native_blocking(threading.get_ident) != threading.get_ident()


def test_run_native_blocking_reuses_one_pool_per_hub(monkeypatch: pytest.MonkeyPatch) -> None:
    fake = _install_fake_gevent(monkeypatch)
    run_native_blocking(lambda: 1)
    run_native_blocking(lambda: 2)
    assert fake.pool.calls == 2


@pytest.mark.parametrize(
    "overrides",
    [
        {"patched": False},
        {"on_hub": True},
        {"hub_exists": False},
        {"hub_thread_ident": threading.get_ident() + 1},
    ],
    ids=["sockets-not-patched", "caller-is-hub", "no-hub-on-thread", "hub-owned-by-other-thread"],
)
def test_run_native_blocking_runs_inline(monkeypatch: pytest.MonkeyPatch, overrides: dict[str, Any]) -> None:
    fake = _install_fake_gevent(monkeypatch, **overrides)
    assert run_native_blocking(threading.get_ident) == threading.get_ident()
    assert fake.pools == []


def test_run_native_blocking_logs_when_the_pool_is_saturated(
    monkeypatch: pytest.MonkeyPatch, caplog: pytest.LogCaptureFixture
) -> None:
    monkeypatch.setenv(THREADPOOL_SIZE_ENV_VAR, "2")
    fake = _install_fake_gevent(monkeypatch)
    run_native_blocking(lambda: None)
    fake.pool.pending = 2
    with caplog.at_level(logging.DEBUG, logger=native_blocking.logger.name):
        run_native_blocking(lambda: None)
    assert "gevent pool saturated" in caplog.text


def test_run_native_blocking_falls_back_to_get_hub_without_hub_local(monkeypatch: pytest.MonkeyPatch) -> None:
    fake = _install_fake_gevent(monkeypatch, hub_local=False)
    monkeypatch.delitem(sys.modules, "gevent._hub_local", raising=False)
    assert run_native_blocking(lambda: 1) == 1
    assert fake.pool.calls == 1


def test_run_native_blocking_propagates_context_variables(monkeypatch: pytest.MonkeyPatch) -> None:
    _install_fake_gevent(monkeypatch)
    var: contextvars.ContextVar[str] = contextvars.ContextVar("var", default="unset")
    var.set("caller")
    assert run_native_blocking(var.get) == "caller"


def test_run_native_blocking_reraises_original_exception(monkeypatch: pytest.MonkeyPatch) -> None:
    fake = _install_fake_gevent(monkeypatch)

    def native_failure() -> None:
        raise KeyError("boom")

    with pytest.raises(KeyError, match="boom") as excinfo:
        run_native_blocking(native_failure)
    assert "native_failure" in [entry.name for entry in excinfo.traceback]
    assert fake.pool.calls == 1


def test_threadpool_size_comes_from_environment(monkeypatch: pytest.MonkeyPatch) -> None:
    monkeypatch.setenv(THREADPOOL_SIZE_ENV_VAR, "7")
    fake = _install_fake_gevent(monkeypatch)
    run_native_blocking(lambda: None)
    assert fake.pool.size == 7


@pytest.mark.parametrize("raw", ["0", "-3", "many"])
def test_invalid_threadpool_size_falls_back_to_default(
    monkeypatch: pytest.MonkeyPatch, caplog: pytest.LogCaptureFixture, raw: str
) -> None:
    monkeypatch.setenv(THREADPOOL_SIZE_ENV_VAR, raw)
    fake = _install_fake_gevent(monkeypatch)
    with caplog.at_level(logging.WARNING, logger=native_blocking.logger.name):
        run_native_blocking(lambda: None)
    assert fake.pool.size == DEFAULT_THREADPOOL_SIZE
    assert f"Ignoring {THREADPOOL_SIZE_ENV_VAR}={raw!r}" in caplog.text


def test_handle_message_offloads_call_proto(monkeypatch: pytest.MonkeyPatch) -> None:
    fake = _install_fake_gevent(monkeypatch)
    seen: list[tuple[str, str, bytes]] = []

    def call_proto(api: str, method: str, message: bytes) -> tuple[int, bytes]:
        seen.append((api, method, message))
        return (0, b"ok")

    import snowflake.connector._internal.api_client.bridge as bridge_mod

    monkeypatch.setattr(bridge_mod, "sf_core_python", types.SimpleNamespace(call_proto=call_proto))
    status, payload = ProtoTransport().handle_message("DatabaseDriver", "database_new", b"req")
    assert (status, payload) == (0, b"ok")
    assert seen == [("DatabaseDriver", "database_new", b"req")]
    assert fake.pool.calls == 1


def test_handle_message_unknown_status_raises(monkeypatch: pytest.MonkeyPatch) -> None:
    fake = _install_fake_gevent(monkeypatch)
    import snowflake.connector._internal.api_client.bridge as bridge_mod

    monkeypatch.setattr(
        bridge_mod,
        "sf_core_python",
        types.SimpleNamespace(call_proto=lambda api, method, message: (9, b"x")),
    )
    with pytest.raises(ProtoTransportException, match="Unknown error code"):
        ProtoTransport().handle_message("DatabaseDriver", "database_new", b"req")
    assert fake.pool.calls == 1


def test_offload_iterators_are_identity_without_gevent() -> None:
    rows, batches = _Rows([1]), _Batches([1])
    assert offload_row_iterator(rows) is rows
    assert offload_table_iterator(batches) is batches


def test_offload_iterators_wrap_when_gevent_patched(monkeypatch: pytest.MonkeyPatch) -> None:
    _install_fake_gevent(monkeypatch)
    assert isinstance(offload_row_iterator(_Rows([])), OffloadedRowIterator)
    assert isinstance(offload_table_iterator(_Batches([])), OffloadedTableIterator)


def test_row_iterator_loads_one_batch_per_pool_round_trip(monkeypatch: pytest.MonkeyPatch) -> None:
    fake = _install_fake_gevent(monkeypatch)
    source = _Rows(list(range(2500)))
    assert list(offload_row_iterator(source)) == list(range(2500))
    assert source.fetch_many_sizes == [1024, 1024, 1024, 1024]
    assert fake.calls == 4


def test_row_iterator_stays_exhausted(monkeypatch: pytest.MonkeyPatch) -> None:
    _install_fake_gevent(monkeypatch)
    iterator = offload_row_iterator(_Rows([1]))
    assert next(iterator) == 1
    for _ in range(2):
        with pytest.raises(StopIteration):
            next(iterator)


def test_row_iterator_fetch_many_and_fetch_all_drain_the_buffer_first(monkeypatch: pytest.MonkeyPatch) -> None:
    fake = _install_fake_gevent(monkeypatch)
    iterator = offload_row_iterator(_Rows(list(range(3000))))
    assert next(iterator) == 0
    assert iterator.fetch_many(2) == [1, 2]
    calls_before = fake.calls
    assert iterator.fetch_many(1000) == list(range(3, 1003))
    assert fake.calls == calls_before
    assert iterator.fetch_many(30) == list(range(1003, 1033))
    assert iterator.fetch_all() == list(range(1033, 3000))


def test_row_iterator_exception_does_not_poison_the_iterator(monkeypatch: pytest.MonkeyPatch) -> None:
    _install_fake_gevent(monkeypatch)
    source = _Rows([1, 2])
    iterator = offload_row_iterator(source)
    healthy = source.fetch_many

    def failing(size: int) -> list[Any]:
        raise OperationalError(msg="native failure")

    monkeypatch.setattr(source, "fetch_many", failing)
    with pytest.raises(OperationalError, match="native failure"):
        next(iterator)
    monkeypatch.setattr(source, "fetch_many", healthy)
    assert next(iterator) == 1


def test_table_iterator_offloads_each_batch(monkeypatch: pytest.MonkeyPatch) -> None:
    fake = _install_fake_gevent(monkeypatch)
    source = _Batches(["a", "b"])
    iterator = offload_table_iterator(source)
    assert list(iterator) == ["a", "b"]
    assert fake.calls == 3


def test_table_iterator_schema_is_not_offloaded(monkeypatch: pytest.MonkeyPatch) -> None:
    fake = _install_fake_gevent(monkeypatch)
    iterator = offload_table_iterator(_Batches([]))
    assert iterator.get_converted_schema() == "schema"
    assert fake.pools == []


@pytest.mark.parametrize("kind", ["row", "table"])
def test_interrupted_fetch_makes_the_iterator_refuse_further_calls(monkeypatch: pytest.MonkeyPatch, kind: str) -> None:
    fake = _install_fake_gevent(monkeypatch)
    iterator: Any = offload_row_iterator(_Rows([1])) if kind == "row" else offload_table_iterator(_Batches(["a"]))
    run_native_blocking(lambda: None)
    fake.pool.interrupt = _Interrupt()
    with pytest.raises(_Interrupt):
        next(iterator)
    fake.pool.interrupt = None
    with pytest.raises(OperationalError, match="interrupted"):
        next(iterator)


class _NativeRows(_Rows):
    def __init__(self, stream_ptr: int = 0, **kwargs: Any) -> None:
        super().__init__([])

    @classmethod
    def from_result_set(cls, handle_id: int, magic: int, **kwargs: Any) -> _NativeRows:
        return cls()


class _NativeBatches(_Batches):
    def __init__(self, stream_ptr: int = 0, **kwargs: Any) -> None:
        super().__init__([])

    @classmethod
    def from_result_set(cls, handle_id: int, magic: int, **kwargs: Any) -> _NativeBatches:
        return cls()


def _create_row_cython() -> Any:
    return arrow_stream_utils.create_row_iterator(1, context=ArrowConverterContext())


def _create_row_native() -> Any:
    return arrow_stream_utils.create_row_iterator(1, context=ArrowConverterContext(), use_core_arrow=True)


def _create_row_from_result_set() -> Any:
    handle = types.SimpleNamespace(id=1, magic=2)
    return arrow_stream_utils.create_row_iterator_from_result_set(handle, context=ArrowConverterContext())  # type: ignore[arg-type]


def _create_table_cython() -> Any:
    return arrow_stream_utils.create_table_iterator(1, context=ArrowConverterContext())


def _create_table_native() -> Any:
    return arrow_stream_utils.create_table_iterator(1, context=ArrowConverterContext(), use_core_arrow=True)


def _create_table_from_result_set() -> Any:
    handle = types.SimpleNamespace(id=1, magic=2)
    return arrow_stream_utils.create_table_iterator_from_result_set(handle, context=ArrowConverterContext())  # type: ignore[arg-type]


@pytest.mark.parametrize(
    ("create", "wrapper"),
    [
        (_create_row_cython, OffloadedRowIterator),
        (_create_row_native, OffloadedRowIterator),
        (_create_row_from_result_set, OffloadedRowIterator),
        (_create_table_cython, OffloadedTableIterator),
        (_create_table_native, OffloadedTableIterator),
        (_create_table_from_result_set, OffloadedTableIterator),
    ],
)
def test_arrow_stream_utils_factories_wrap_iterators_only_under_gevent(
    monkeypatch: pytest.MonkeyPatch, create: Any, wrapper: type
) -> None:
    monkeypatch.setattr(arrow_stream_utils, "CythonArrowStreamIterator", lambda *args, **kwargs: _Rows([]))
    monkeypatch.setattr(arrow_stream_utils, "CythonArrowStreamTableIterator", lambda *args, **kwargs: _Batches([]))
    monkeypatch.setattr(
        arrow_stream_utils,
        "sf_core_python",
        types.SimpleNamespace(
            native_arrow_enabled=lambda: True,
            ArrowStreamIterator=_NativeRows,
            ArrowStreamTableIterator=_NativeBatches,
        ),
    )
    assert not isinstance(create(), wrapper)

    fake = _install_fake_gevent(monkeypatch)
    assert isinstance(create(), wrapper)
    expects_offloaded_construction = create in {
        _create_row_native,
        _create_row_from_result_set,
        _create_table_native,
        _create_table_from_result_set,
    }
    assert fake.calls == (1 if expects_offloaded_construction else 0)


_GEVENT_PRELUDE = textwrap.dedent(
    """
    from gevent import monkey

    monkey.patch_all()

    import sys
    import threading
    import time
    import types

    import gevent

    blocking_sleep = monkey.get_original("time", "sleep")
    real_lock = monkey.get_original("_thread", "allocate_lock")
    hub_thread = monkey.get_original("_thread", "get_ident")()
    ticks = []


    def heartbeat():
        while True:
            ticks.append(time.monotonic())
            gevent.sleep(0.05)


    def start_heartbeat():
        beat = gevent.spawn(heartbeat)
        gevent.sleep(0)
        return beat


    def fail(message):
        sys.exit(message)


    def wait_for(condition, timeout=10.0):
        deadline = time.monotonic() + timeout
        while not condition():
            if time.monotonic() > deadline:
                fail("timed out waiting for the native call")
            gevent.sleep(0.01)
    """
)


def _run_gevent_script(body: str) -> None:
    proc = subprocess.run(
        [sys.executable, "-c", _GEVENT_PRELUDE + textwrap.dedent(body)],
        capture_output=True,
        text=True,
        check=False,
        timeout=30,
    )
    assert proc.returncode == 0, proc.stdout + proc.stderr


@requires_real_gevent
def test_blocking_wait_without_offload_starves_the_heartbeat() -> None:
    _run_gevent_script(
        """
        beat = start_heartbeat()
        blocking_sleep(0.4)
        beat.kill()
        if len(ticks) > 2:
            fail(f"expected the heartbeat to starve, it ran {len(ticks)} times")
        """
    )


@requires_real_gevent
def test_handle_message_does_not_block_gevent_loop() -> None:
    _run_gevent_script(
        """
        from snowflake.connector._internal import native_blocking
        from snowflake.connector._internal.api_client import bridge as bridge_mod
        from snowflake.connector._internal.api_client.bridge import ProtoTransport

        ran_on = []

        def slow_call_proto(api, method, message):
            ran_on.append(threading.get_ident())
            blocking_sleep(0.4)
            return (0, b"ok")

        bridge_mod.sf_core_python = types.SimpleNamespace(call_proto=slow_call_proto)

        beat = start_heartbeat()
        start = time.monotonic()
        status, payload = ProtoTransport().handle_message("DatabaseDriver", "database_new", b"req")
        elapsed = time.monotonic() - start
        beat.kill()
        if (status, payload) != (0, b"ok"):
            fail(f"unexpected result {status!r} {payload!r}")
        if elapsed < 0.3:
            fail(f"call returned too fast: {elapsed}")
        if len(ticks) < 5:
            fail(f"gevent loop blocked: heartbeat ran {len(ticks)} times")
        if not ran_on or ran_on[0] == hub_thread:
            fail("the native call ran on the hub thread")
        if len(native_blocking._pools) != 1:
            fail("the driver pool was not used")
        """
    )


@requires_real_gevent
def test_row_fetch_does_not_block_gevent_loop() -> None:
    _run_gevent_script(
        """
        from snowflake.connector._internal.native_blocking import offload_row_iterator

        class Rows:
            def __init__(self):
                self.rows = list(range(1500))

            def __iter__(self):
                return self

            def fetch_many(self, size):
                blocking_sleep(0.2)
                taken, self.rows = self.rows[:size], self.rows[size:]
                return taken

            def fetch_all(self):
                blocking_sleep(0.2)
                taken, self.rows = self.rows, []
                return taken

        beat = start_heartbeat()
        iterator = offload_row_iterator(Rows())
        rows = list(iterator)
        beat.kill()
        if rows != list(range(1500)):
            fail("rows differ from the source")
        if len(ticks) < 5:
            fail(f"gevent loop blocked: heartbeat ran {len(ticks)} times")
        """
    )


@requires_real_gevent
def test_table_fetch_does_not_block_gevent_loop() -> None:
    _run_gevent_script(
        """
        from snowflake.connector._internal.native_blocking import offload_table_iterator

        class Batches:
            def __init__(self):
                self.batches = iter(["a", "b"])

            def __iter__(self):
                return self

            def __next__(self):
                blocking_sleep(0.2)
                return next(self.batches)

            def get_converted_schema(self):
                return "schema"

        beat = start_heartbeat()
        batches = list(offload_table_iterator(Batches()))
        beat.kill()
        if batches != ["a", "b"]:
            fail(f"unexpected batches {batches!r}")
        if len(ticks) < 5:
            fail(f"gevent loop blocked: heartbeat ran {len(ticks)} times")
        """
    )


@requires_real_gevent
def test_killed_greenlet_leaves_a_clear_error_on_the_iterator() -> None:
    _run_gevent_script(
        """
        from snowflake.connector._internal.native_blocking import offload_row_iterator
        from snowflake.connector.errors import OperationalError

        release = real_lock()
        release.acquire()
        started = []
        finished = []

        class Rows:
            def __iter__(self):
                return self

            def fetch_many(self, size):
                started.append(True)
                release.acquire()
                finished.append(True)
                return [1]

        iterator = offload_row_iterator(Rows())
        reader = gevent.spawn(next, iterator)
        wait_for(lambda: started)
        reader.kill()
        wait_for(lambda: reader.dead)
        try:
            next(iterator)
        except OperationalError as exc:
            if "interrupted" not in str(exc):
                fail(f"unexpected message: {exc}")
        else:
            fail("the iterator accepted a call after an abandoned fetch")
        release.release()
        wait_for(lambda: finished)
        """
    )


@requires_real_gevent
def test_gevent_timeout_leaves_a_clear_error_on_the_iterator() -> None:
    _run_gevent_script(
        """
        from snowflake.connector._internal.native_blocking import offload_table_iterator
        from snowflake.connector.errors import OperationalError

        release = real_lock()
        release.acquire()
        finished = []

        class Batches:
            def __iter__(self):
                return self

            def __next__(self):
                release.acquire()
                finished.append(True)
                return "a"

            def get_converted_schema(self):
                return "schema"

        iterator = offload_table_iterator(Batches())
        try:
            with gevent.Timeout(0.1):
                next(iterator)
        except gevent.Timeout:
            pass
        else:
            fail("the timeout did not fire")
        try:
            next(iterator)
        except OperationalError:
            pass
        else:
            fail("the iterator accepted a call after an abandoned fetch")
        release.release()
        wait_for(lambda: finished)
        """
    )
