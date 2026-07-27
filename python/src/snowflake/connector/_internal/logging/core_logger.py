"""Round-trip logging: Python -> core -> Python.

:class:`CoreLogger` is a drop-in replacement for a module ``logging.Logger``.
Wrapper logs are sent to sf_core over a direct FFI call so every tracing layer
(file, OTLP, Snowflake in-band telemetry) sees them, then come back through the
FFI callback onto the originating module logger. See
``doc/logging/logging-architecture.md``.

This lives in its own module (rather than in :mod:`config`) so the FFI binding
``sf_core_log_event`` can be imported at module top level. ``logging.config`` is
imported at package load, before the c_api layer — which itself imports
``logging`` — is ready; keeping ``CoreLogger`` here avoids that import cycle.
:func:`logging.get_logger` imports this module lazily, once the package is up.
"""

from __future__ import annotations

import logging
import sys
import traceback

from typing import TYPE_CHECKING

from ..api_client.c_api._init import sf_core_log_event
from .config import LoggingConfiguration


if TYPE_CHECKING:
    from typing import Any

# sf_core_log_event return codes (see ``sf_core::logging::c_api``).
_CORE_DELIVERED = 0  # event accepted by the tracing pipeline


def _py_level_to_callback(level: int) -> int:
    """Map a stdlib logging level to the sf_core FFI callback level encoding."""
    if level >= logging.ERROR:
        return 0
    if level >= logging.WARNING:
        return 1
    if level >= logging.INFO:
        return 2
    return 3  # DEBUG - finest level supported


class CoreLogger:
    """Drop-in replacement for a module ``logging.Logger`` that routes records
    through sf_core.

    The wrapped stdlib logger remains the single source of truth for levels:
    :meth:`is_enabled_for` gates every FFI call, so a filtered message costs
    nothing.  When troubleshooting mode is active the pre-filter is bypassed
    and all events reach the core pipeline regardless of the stdlib level.
    """

    def __init__(self, name: str) -> None:
        self._name = name
        self._py_logger = logging.getLogger(name)

    @property
    def name(self) -> str:
        return self._name

    def is_enabled_for(self, level: int) -> bool:
        return self._is_level_enabled(level)

    def _is_level_enabled(self, py_level: int) -> bool:
        cfg = LoggingConfiguration._instance
        return (cfg is not None and cfg.is_troubleshooting_enabled()) or self._py_logger.isEnabledFor(py_level)

    def _format_message(self, msg: str, args: tuple[object, ...], exc_info: Any) -> str:
        try:
            formatted = msg % args if args else msg
        except Exception:
            # A mismatched format string / args is a caller bug; logging must
            # never raise into user code (same contract as the stdlib logger we
            # replace, which swallows format errors in ``record.getMessage``).
            # Still emit something useful rather than dropping the record.
            formatted = f"{msg} {args!r}"
        if exc_info:
            if exc_info is True:
                exc_info = sys.exc_info()
            if isinstance(exc_info, BaseException):
                exc_info = (type(exc_info), exc_info, exc_info.__traceback__)
            if exc_info and exc_info[0] is not None:
                formatted += "".join(traceback.format_exception(*exc_info))
        return formatted

    def _emit(
        self,
        py_level: int,
        msg: str,
        args: tuple[object, ...],
        *,
        exc_info: Any = False,
        stacklevel: int = 1,
    ) -> None:
        """Emit a Python-originated log, resolving the caller's location.

        Used by the public ``debug``/``info``/… methods, where the call site is
        a Python stack frame.
        """
        if not self._is_level_enabled(py_level):
            return
        # frame 0 is _emit, frame 1 is the public CoreLogger method (info/log/...)
        # stacklevel counts frames above that method
        frame = sys._getframe(1 + stacklevel)
        self._dispatch(
            py_level,
            self._format_message(msg, args, exc_info),
            frame.f_code.co_filename,
            frame.f_lineno,
            frame.f_code.co_name,
        )

    def _emit_with_location(
        self,
        py_level: int,
        msg: str,
        args: tuple[object, ...],
        *,
        exc_info: Any = False,
        file: str,
        line: int,
        function: str,
    ) -> None:
        """Emit a log with a caller-supplied source location.

        Used by native extensions, which pass explicit C++ file/function/line
        instead of a resolvable Python stack frame.
        """
        if not self._is_level_enabled(py_level):
            return
        self._dispatch(py_level, self._format_message(msg, args, exc_info), file, line, function)

    def _dispatch(
        self,
        py_level: int,
        message: str,
        source_file: str,
        source_line: int,
        source_function: str,
    ) -> None:
        """Send a fully-resolved record to sf_core, falling back to the stdlib
        logger when the pipeline is not live.
        """
        try:
            status = sf_core_log_event(
                level=_py_level_to_callback(py_level),
                message=message,
                file=source_file,
                line=source_line,
                function=source_function,
                logger_name=self._name,
            )
        except Exception:
            # FFI unusable (e.g. torn down during interpreter shutdown). Logging
            # must never raise into user code (same contract as
            # ``logging.Handler.handle`` and :func:`logging.safe_log`); treat as
            # not delivered and fall back below.
            status = None

        if status == _CORE_DELIVERED:
            return

        # sf_core is not up yet (early-import events, before ``sf_core_init``
        # marks the pipeline live) or the FFI is gone: emit straight onto the
        # stdlib logger so the record is not lost. The FFI return code — not a
        # Python-side flag — is the source of truth for whether it is live.
        record = self._py_logger.makeRecord(
            self._name,
            py_level,
            source_file,
            source_line,
            message,
            (),
            None,
            source_function,
        )
        self._py_logger.handle(record)

    def debug(self, msg: str, *args: object, exc_info: Any = False) -> None:
        self._emit(logging.DEBUG, msg, args, exc_info=exc_info)

    def info(self, msg: str, *args: object, exc_info: Any = False) -> None:
        self._emit(logging.INFO, msg, args, exc_info=exc_info)

    def warning(self, msg: str, *args: object, exc_info: Any = False) -> None:
        self._emit(logging.WARNING, msg, args, exc_info=exc_info)

    def error(self, msg: str, *args: object, exc_info: Any = False) -> None:
        self._emit(logging.ERROR, msg, args, exc_info=exc_info)

    def exception(self, msg: str, *args: object) -> None:
        self._emit(logging.ERROR, msg, args, exc_info=True)

    def log(self, level: int, msg: str, *args: object, exc_info: Any = False, stacklevel: int = 1) -> None:
        self._emit(level, msg, args, exc_info=exc_info, stacklevel=stacklevel)

    def safe_log(self, level: int, msg: str, *args: object, exc_info: bool = False) -> None:
        """Best-effort log call — never raises.

        Use on cleanup paths that run during interpreter shutdown (``atexit``,
        ``__del__``) where the ``logging`` module itself may be partially torn
        down and the sf_core FFI may already be gone. Bypasses the round-trip
        and logs straight onto the stdlib logger. ``stacklevel=2`` points the
        record at the caller, not at this method.

        For all other call sites prefer the standard ``logger.<level>(...)``
        methods so genuine logging misconfiguration is not silently hidden.
        """
        try:
            self._py_logger.log(level, msg, *args, exc_info=exc_info, stacklevel=2)
        except Exception:
            pass
