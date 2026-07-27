"""Logger wrapper used by bundled native extensions (nanoarrow C++).

The extension imports this module via pybind (see
``_internal/nanoarrow_cpp/Logging/logging.cpp``) and calls
``NativeExtensionLogger.log()`` so it can pass C++-side file / function / line
information through :class:`.core_logger.CoreLogger`, which routes every log
through sf_core.
"""

from __future__ import annotations

import logging
import warnings

from typing import Any

from .core_logger import CoreLogger


def get_native_extension_logger(name: str) -> NativeExtensionLogger:
    return NativeExtensionLogger(CoreLogger(name))


class NativeExtensionLogger:
    """Thin wrapper around :class:`.core_logger.CoreLogger` for native code.

    Native extensions pass explicit file, function, and line metadata from C++
    rather than relying on Python stack frames.
    """

    def __init__(self, core_logger: CoreLogger) -> None:
        self._logger = core_logger

    def debug(
        self,
        msg: str,
        path_name: str | None = None,
        func_name: str | None = None,
        *args: Any,
        **kwargs: Any,
    ) -> None:
        self.log(logging.DEBUG, msg, *args, path_name=path_name, func_name=func_name, **kwargs)

    def info(
        self,
        msg: str,
        path_name: str | None = None,
        func_name: str | None = None,
        *args: Any,
        **kwargs: Any,
    ) -> None:
        self.log(logging.INFO, msg, *args, path_name=path_name, func_name=func_name, **kwargs)

    def warning(
        self,
        msg: str,
        path_name: str | None = None,
        func_name: str | None = None,
        *args: Any,
        **kwargs: Any,
    ) -> None:
        self.log(logging.WARNING, msg, *args, path_name=path_name, func_name=func_name, **kwargs)

    def warn(
        self,
        msg: str,
        path_name: str | None = None,
        func_name: str | None = None,
        *args: Any,
        **kwargs: Any,
    ) -> None:
        warnings.warn(
            "The 'warn' method is deprecated, use 'warning' instead",
            DeprecationWarning,
            stacklevel=2,
        )
        self.warning(msg, *args, path_name=path_name, func_name=func_name, **kwargs)  # type: ignore[misc]

    def error(
        self,
        msg: str,
        path_name: str | None = None,
        func_name: str | None = None,
        *args: Any,
        **kwargs: Any,
    ) -> None:
        self.log(logging.ERROR, msg, *args, path_name=path_name, func_name=func_name, **kwargs)

    def exception(
        self,
        msg: str,
        path_name: str | None = None,
        func_name: str | None = None,
        *args: Any,
        exc_info: bool = True,
        **kwargs: Any,
    ) -> None:
        self.error(msg, *args, path_name=path_name, func_name=func_name, exc_info=exc_info, **kwargs)  # type: ignore[misc]

    def critical(
        self,
        msg: str,
        path_name: str | None = None,
        func_name: str | None = None,
        *args: Any,
        **kwargs: Any,
    ) -> None:
        self.log(logging.CRITICAL, msg, *args, path_name=path_name, func_name=func_name, **kwargs)

    fatal = critical

    def log(
        self,
        level: int,
        msg: str,
        *args: Any,
        path_name: str | None = None,
        func_name: str | None = None,
        line_num: int = 0,
        **kwargs: Any,
    ) -> None:
        """Generalized log method for native extension callers.

        Args:
            level: Logging level.
            msg: Logging message.
            path_name: Absolute or relative path of the file where the logger gets called.
            func_name: Function inside which the logger gets called.
            line_num: Line number at which the logger gets called.
        """
        if not isinstance(level, int):
            if logging.raiseExceptions:  # type: ignore[unreachable]
                raise TypeError("level must be an integer")
            return
        exc_info = kwargs.pop("exc_info", False)
        self._logger._emit_with_location(
            level,
            msg,
            args,
            exc_info=exc_info,
            file=path_name or "",
            line=line_num,
            function=func_name or "",
        )
