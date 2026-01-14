# distutils: language = c++
# cython: language_level=3

from cpython.ref cimport PyObject
from libc.stdint cimport int64_t
from libcpp.memory cimport unique_ptr


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
