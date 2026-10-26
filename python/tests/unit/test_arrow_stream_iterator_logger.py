from __future__ import annotations

import ctypes
import os
import subprocess
import sys
import threading
import time

import pytest


class _ArrowArrayStream(ctypes.Structure):
    _fields_ = [
        ("get_schema", ctypes.c_void_p),
        ("get_next", ctypes.c_void_p),
        ("get_last_error", ctypes.c_void_p),
        ("release", ctypes.c_void_p),
        ("private_data", ctypes.c_void_p),
    ]


def _stream(pa):
    schema = pa.schema(
        [pa.field("value", pa.int64(), metadata={"logicalType": "FIXED", "scale": "0", "precision": "18"})]
    )
    reader = pa.RecordBatchReader.from_batches(
        schema,
        [pa.record_batch([pa.array([1], type=pa.int64())], schema=schema)],
    )
    stream = _ArrowArrayStream()
    reader._export_to_c(ctypes.addressof(stream))
    return stream, reader


def _child_main() -> None:
    import pyarrow as pa

    from snowflake.connector._internal.arrow_context import ArrowConverterContext
    from snowflake.connector._internal.arrow_stream_iterator import ArrowStreamIterator
    from snowflake.connector._internal.logging import native_extension_logger

    streams = [_stream(pa), _stream(pa)]
    logger_initializing = threading.Event()
    original_get_logger = native_extension_logger.get_native_extension_logger

    def delayed_get_logger(*args, **kwargs):
        logger_initializing.set()
        time.sleep(0.2)
        return original_get_logger(*args, **kwargs)

    native_extension_logger.get_native_extension_logger = delayed_get_logger
    errors: list[BaseException] = []

    def build(index: int) -> None:
        try:
            stream, _reader = streams[index]
            assert list(ArrowStreamIterator(ctypes.addressof(stream), ArrowConverterContext())) == [(1,)]
        except BaseException as exc:
            errors.append(exc)

    first = threading.Thread(target=build, args=(0,), daemon=True)
    first.start()
    if not logger_initializing.wait(timeout=2):
        os._exit(3)

    second = threading.Thread(target=build, args=(1,), daemon=True)
    second.start()
    first.join(timeout=2)
    second.join(timeout=2)
    if first.is_alive() or second.is_alive():
        os._exit(2)
    if errors:
        os._exit(4)


def test_should_not_deadlock_when_logger_initializes_concurrently() -> None:
    pytest.importorskip("pyarrow")
    proc = subprocess.run(
        [sys.executable, __file__, "--child"],
        capture_output=True,
        text=True,
        timeout=10,
        check=False,
    )
    assert proc.returncode == 0, (
        f"Concurrent logger initialization failed (subprocess exit {proc.returncode}).\n"
        f"--- subprocess stderr ---\n{proc.stderr}\n"
        f"--- subprocess stdout ---\n{proc.stdout}"
    )


if __name__ == "__main__":
    _child_main()
