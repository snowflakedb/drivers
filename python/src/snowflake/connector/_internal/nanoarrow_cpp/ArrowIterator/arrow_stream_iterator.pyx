# distutils: language = c++
# cython: language_level=3

from cpython.ref cimport PyObject
from libc.stdint cimport int64_t, uintptr_t
from libcpp.memory cimport unique_ptr

try:
    import pyarrow as pa
except ImportError:
    pass


# Import ReturnVal from C++ (defined in CArrowIterator.hpp)
cdef extern from "CArrowIterator.hpp" namespace "sf":
    cdef cppclass ReturnVal:
        PyObject* successObj
        PyObject* exception


# Import the C++ stream iterator
cdef extern from "CArrowStreamIterator.hpp" namespace "sf":
    cdef cppclass CArrowStreamIterator:
        @staticmethod
        unique_ptr[CArrowStreamIterator] from_stream(
            int64_t stream_ptr,
            PyObject* context,
            bint use_numpy,
            bint use_dict_result
        )
        ReturnVal next()
        object nextN(int64_t size)
        object nextAll()


cdef extern from "CArrowStreamTableIterator.hpp" namespace "sf":
    cdef cppclass CArrowStreamTableIterator:
        @staticmethod
        unique_ptr[CArrowStreamTableIterator] from_stream(
            int64_t stream_ptr,
            PyObject* context,
            bint number_to_decimal,
            bint force_microsecond_precision,
        )
        ReturnVal next()
        uintptr_t getArrowArrayPtr()
        uintptr_t getArrowSchemaPtr()
        uintptr_t getConvertedSchemaPtr()


cdef class ArrowStreamIterator:
    """
    Python wrapper for C++ Arrow stream iterator.

    Reads directly from an ArrowArrayStream pointer. The C++ implementation
    uses Py_BEGIN_ALLOW_THREADS/Py_END_ALLOW_THREADS to release the GIL during
    potentially blocking I/O operations (e.g., fetching data chunks from S3).
    """
    cdef unique_ptr[CArrowStreamIterator] iterator
    cdef object arrow_context

    def __cinit__(
        self,
        int64_t stream_ptr,
        object arrow_context,
        object use_dict_result=False,
        object use_numpy=False
    ):
        """
        Initialize the stream iterator.

        Parameters
        ----------
        stream_ptr : int
            Pointer to ArrowArrayStream (as integer)
        arrow_context : ArrowConverterContext
            Context object for conversions
        use_dict_result : bool
            If True, return dicts instead of tuples
        use_numpy : bool
            If True, use numpy types for numeric data
        """
        self.arrow_context = arrow_context

        # Create the C++ stream iterator using factory method
        self.iterator = CArrowStreamIterator.from_stream(
            stream_ptr,
            <PyObject*>arrow_context,
            use_numpy,
            use_dict_result
        )

        # Check if creation failed (nullptr returned)
        if self.iterator.get() == NULL:
            raise RuntimeError("Failed to initialize stream iterator")

    def __iter__(self):
        return self

    def __next__(self):
        """Get next row from stream."""
        cdef ReturnVal ret

        ret = self.iterator.get().next()

        # Check for exception
        if ret.exception != NULL:
            error_msg = <object>ret.exception
            raise RuntimeError(f"Error converting row: {error_msg}")

        # Check for end of iteration
        if ret.successObj == NULL:
            raise StopIteration

        # Return the row
        row = <object>ret.successObj
        return row

    def fetch_many(self, int64_t size):
        """Fetch up to `size` rows as a list in a single C++ call."""
        try:
            return self.iterator.get().nextN(size)
        except Exception as e:
            raise RuntimeError(f"Error converting row: {e}") from e

    def fetch_all(self):
        """Fetch all remaining rows as a list in a single C++ call."""
        try:
            return self.iterator.get().nextAll()
        except Exception as e:
            raise RuntimeError(f"Error converting row: {e}") from e

cdef class ArrowStreamTableIterator:
    """
    Python iterator that reads batches from an ArrowArrayStream and yields
    one pyarrow.RecordBatch per batch, with Snowflake type conversions
    applied via ArrowTableConverter.
    """
    cdef unique_ptr[CArrowStreamTableIterator] iterator
    cdef object arrow_context

    def __cinit__(
        self,
        int64_t stream_ptr,
        object arrow_context,
        bint number_to_decimal = False,
        bint force_microsecond_precision = False,
    ):
        self.arrow_context = arrow_context

        self.iterator = CArrowStreamTableIterator.from_stream(
            stream_ptr,
            <PyObject*>arrow_context,
            number_to_decimal,
            force_microsecond_precision,
        )
        if self.iterator.get() == NULL:
            raise RuntimeError("Failed to initialize ArrowStreamTableIterator")

    def __iter__(self):
        return self

    def __next__(self):
        cdef ReturnVal ret
        cdef uintptr_t array_ptr
        cdef uintptr_t schema_ptr

        ret = self.iterator.get().next()

        if ret.exception != NULL:
            error_msg = <object>ret.exception
            raise RuntimeError(f"Error in dummy iterator: {error_msg}")

        if ret.successObj == NULL:
            raise StopIteration

        if <object>ret.successObj is None:
            raise StopIteration

        array_ptr = self.iterator.get().getArrowArrayPtr()
        schema_ptr = self.iterator.get().getArrowSchemaPtr()

        return pa.RecordBatch._import_from_c(array_ptr, schema_ptr)

    def get_converted_schema(self):
        cdef uintptr_t schema_ptr = self.iterator.get().getConvertedSchemaPtr()

        return pa.Schema._import_from_c(schema_ptr)
