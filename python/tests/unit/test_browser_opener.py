from __future__ import annotations

import asyncio
import threading

import pytest

from snowflake.connector._internal.browser_opener import CallerThreadBrowserOpener, EventLoopBrowserOpener
from snowflake.connector.errors import OperationalError, ProgrammingError


class TestCallerThreadBrowserOpener:
    def test_runs_callback_on_thread_that_runs_init(self):
        seen: list[tuple[str, int]] = []
        opener = CallerThreadBrowserOpener(lambda url: seen.append((url, threading.get_ident())))

        opener.run_init(lambda: opener("https://snowflake.com/sso"))

        assert seen == [("https://snowflake.com/sso", threading.get_ident())]

    def test_callback_exception_is_raised_to_the_core_caller(self):
        def failing_callback(url):
            raise ValueError("idp form not found")

        opener = CallerThreadBrowserOpener(failing_callback)
        core_errors: list[Exception] = []

        def init():
            try:
                opener("https://snowflake.com/sso")
            except ValueError as error:
                core_errors.append(error)

        opener.run_init(init)

        assert [str(e) for e in core_errors] == ["idp form not found"]

    def test_init_exception_is_reraised(self):
        opener = CallerThreadBrowserOpener(lambda url: None)

        def init():
            raise RuntimeError("login failed")

        with pytest.raises(RuntimeError, match="^login failed$"):
            opener.run_init(init)

    def test_rejects_open_after_init_returns(self):
        opener = CallerThreadBrowserOpener(lambda url: None)
        opener.run_init(lambda: None)

        with pytest.raises(OperationalError, match="connect\\(\\) has finished or was interrupted"):
            opener("https://snowflake.com/sso")

    def test_keyboard_interrupt_unblocks_the_core_caller(self):
        def interrupting_callback(url):
            raise KeyboardInterrupt

        opener = CallerThreadBrowserOpener(interrupting_callback)
        core_errors: list[BaseException] = []

        def init():
            try:
                opener("https://snowflake.com/sso")
            except BaseException as error:
                core_errors.append(error)

        with pytest.raises(KeyboardInterrupt):
            opener.run_init(init)

        assert any(isinstance(error, KeyboardInterrupt) for error in core_errors)

    def test_async_callback_fails_explicitly(self):
        async def callback(url):
            return None

        opener = CallerThreadBrowserOpener(callback)
        core_errors: list[Exception] = []

        def init():
            try:
                opener("https://snowflake.com/sso")
            except Exception as error:
                core_errors.append(error)

        opener.run_init(init)

        assert len(core_errors) == 1
        assert isinstance(core_errors[0], ProgrammingError)
        assert "awaitable on the sync connector" in str(core_errors[0])


class TestEventLoopBrowserOpener:
    def test_awaits_async_callback_on_the_loop(self):
        seen: list[tuple[str, int]] = []

        async def callback(url):
            await asyncio.sleep(0)
            seen.append((url, threading.get_ident()))

        async def connect():
            opener = EventLoopBrowserOpener(callback, asyncio.get_running_loop())
            await asyncio.to_thread(opener, "https://snowflake.com/sso")

        asyncio.run(connect())

        assert seen == [("https://snowflake.com/sso", threading.get_ident())]

    def test_sync_callback_exception_is_raised_to_the_core_caller(self):
        def callback(url):
            raise ValueError("idp form not found")

        async def connect():
            opener = EventLoopBrowserOpener(callback, asyncio.get_running_loop())
            await asyncio.to_thread(opener, "https://snowflake.com/sso")

        with pytest.raises(ValueError, match="^idp form not found$"):
            asyncio.run(connect())
