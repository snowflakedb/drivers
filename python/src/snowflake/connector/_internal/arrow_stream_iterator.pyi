"""Type stubs for the Cython arrow_stream_iterator extension module.

The actual implementation is compiled from
nanoarrow_cpp/ArrowIterator/arrow_stream_iterator.pyx at wheel-build time.
"""

from __future__ import annotations

from collections.abc import Iterator
from typing import Any

class ArrowStreamIterator(Iterator[Any]):
    """Python wrapper for C++ Arrow stream iterator.

    Reads directly from an ArrowArrayStream pointer.
    """

    def __init__(
        self,
        stream_ptr: int,
        arrow_context: Any,
        use_dict_result: bool = False,
        use_numpy: bool = False,
    ) -> None: ...
    def __iter__(self) -> ArrowStreamIterator: ...
    def __next__(self) -> Any: ...
