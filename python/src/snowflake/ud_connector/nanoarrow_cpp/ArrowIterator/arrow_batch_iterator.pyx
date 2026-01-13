# distutils: language = c++
# cython: language_level=3

from cpython.ref cimport PyObject
from libc.stdint cimport int64_t
from libcpp cimport bool as cpp_bool

# Import Arrow C Data Interface structures
cdef extern from "nanoarrow.h":
    ctypedef struct ArrowArray:
        pass
    
    ctypedef struct ArrowSchema:
        pass
    
    cdef int ArrowArrayMove(ArrowArray* src, ArrowArray* dst)
    cdef int ArrowSchemaMove(ArrowSchema* src, ArrowSchema* dst)

# Import our C++ classes for batch conversion
cdef extern from "CArrowBatchIterator.hpp" namespace "sf":
    cdef cppclass ReturnVal:
        PyObject* successObj
        PyObject* exception
    
    cdef cppclass CArrowBatchIterator:
        CArrowBatchIterator(
            ArrowArray* c_array,
            ArrowSchema* c_schema,
            PyObject* context,
            PyObject* use_numpy,
            PyObject* check_error_on_every_column
        )
        ReturnVal next()
        ReturnVal checkInitializationStatus()
        int64_t getRowCount()
        int64_t getCurrentRowIndex()
    
    cdef cppclass DictCArrowBatchIterator:
        DictCArrowBatchIterator(
            ArrowArray* c_array,
            ArrowSchema* c_schema,
            PyObject* context,
            PyObject* use_numpy
        )
        ReturnVal next()
        ReturnVal checkInitializationStatus()
        int64_t getRowCount()
        int64_t getCurrentRowIndex()


cdef class PyArrowBatchIterator:
    """
    Python wrapper for C++ Arrow batch iterator.
    Converts Arrow RecordBatch to Python tuples/dicts row-by-row.
    """
    cdef CArrowBatchIterator* iterator
    cdef DictCArrowBatchIterator* dict_iterator
    cdef cpp_bool use_dict_result
    cdef object arrow_context
    
    def __cinit__(
        self,
        object record_batch,
        object arrow_context,
        object use_dict_result=False,
        object use_numpy=False,
        object check_error_on_every_column=True
    ):
        """
        Initialize the batch iterator.
        
        Parameters
        ----------
        record_batch : pyarrow.RecordBatch
            The Arrow RecordBatch to iterate over
        arrow_context : ArrowConverterContext
            Context object for conversions
        use_dict_result : bool
            If True, return dicts instead of tuples
        use_numpy : bool
            If True, use numpy types for numeric data
        check_error_on_every_column : bool
            If True, check for Python errors after each column conversion
        """
        cdef ArrowArray c_array
        cdef ArrowSchema c_schema
        
        self.use_dict_result = use_dict_result
        self.arrow_context = arrow_context
        self.iterator = NULL
        self.dict_iterator = NULL
        
        # Export RecordBatch to Arrow C Data Interface
        try:
            # PyArrow 14.0+ uses __arrow_c_array__
            # Note: __arrow_c_array__() returns (schema_capsule, array_capsule) - schema first!
            c_schema_capsule, c_array_capsule = record_batch.__arrow_c_array__()
        except AttributeError:
            # Fallback for older PyArrow versions
            raise RuntimeError(
                "PyArrow version too old. Need pyarrow >= 14.0 with "
                "__arrow_c_array__ support"
            )
        
        # Extract pointers from PyCapsules
        cdef ArrowArray* c_array_ptr = <ArrowArray*>PyCapsule_GetPointer(
            c_array_capsule, "arrow_array"
        )
        cdef ArrowSchema* c_schema_ptr = <ArrowSchema*>PyCapsule_GetPointer(
            c_schema_capsule, "arrow_schema"
        )
        
        if c_array_ptr == NULL or c_schema_ptr == NULL:
            raise RuntimeError("Failed to extract Arrow C pointers from RecordBatch")
        
        # Declare ReturnVal variable at function scope (Cython requirement)
        cdef ReturnVal init_ret
        
        # Create appropriate iterator
        if use_dict_result:
            self.dict_iterator = new DictCArrowBatchIterator(
                c_array_ptr,
                c_schema_ptr,
                <PyObject*>arrow_context,
                <PyObject*>use_numpy
            )
            
            # Check initialization
            init_ret = self.dict_iterator.checkInitializationStatus()
            if init_ret.exception != NULL:
                error_msg = <object>init_ret.exception
                raise RuntimeError(f"Failed to initialize batch iterator: {error_msg}")
        else:
            self.iterator = new CArrowBatchIterator(
                c_array_ptr,
                c_schema_ptr,
                <PyObject*>arrow_context,
                <PyObject*>use_numpy,
                <PyObject*>check_error_on_every_column
            )
            
            # Check initialization
            init_ret = self.iterator.checkInitializationStatus()
            if init_ret.exception != NULL:
                error_msg = <object>init_ret.exception
                raise RuntimeError(f"Failed to initialize batch iterator: {error_msg}")
    
    def __dealloc__(self):
        if self.iterator != NULL:
            del self.iterator
        if self.dict_iterator != NULL:
            del self.dict_iterator
    
    def __iter__(self):
        return self
    
    def __next__(self):
        """Get next row from batch."""
        cdef ReturnVal ret
        
        if self.use_dict_result:
            ret = self.dict_iterator.next()
        else:
            ret = self.iterator.next()
        
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
    
    def get_row_count(self):
        """Get total number of rows in this batch."""
        if self.use_dict_result:
            return self.dict_iterator.getRowCount()
        else:
            return self.iterator.getRowCount()
    
    def get_current_index(self):
        """Get current row index (0-based)."""
        if self.use_dict_result:
            return self.dict_iterator.getCurrentRowIndex()
        else:
            return self.iterator.getCurrentRowIndex()


# Import the C++ stream iterator
cdef extern from "CArrowStreamIterator.hpp" namespace "sf":
    cdef cppclass CArrowStreamIterator:
        CArrowStreamIterator(
            int64_t stream_ptr,
            PyObject* context,
            PyObject* use_numpy,
            PyObject* use_dict_result
        )
        ReturnVal next()
        ReturnVal checkInitializationStatus()


cdef class PyArrowStreamIterator:
    """
    Python wrapper for C++ Arrow stream iterator.
    
    Reads directly from an ArrowArrayStream pointer. The C++ implementation
    uses Py_BEGIN_ALLOW_THREADS/Py_END_ALLOW_THREADS to release the GIL during
    potentially blocking I/O operations (e.g., fetching data chunks from S3).
    """
    cdef CArrowStreamIterator* iterator
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
        self.iterator = NULL
        
        # Declare ReturnVal variable at function scope (Cython requirement)
        cdef ReturnVal init_ret
        
        # Create the C++ stream iterator
        self.iterator = new CArrowStreamIterator(
            stream_ptr,
            <PyObject*>arrow_context,
            <PyObject*>use_numpy,
            <PyObject*>use_dict_result
        )
        
        # Check initialization
        init_ret = self.iterator.checkInitializationStatus()
        if init_ret.exception != NULL:
            error_msg = <object>init_ret.exception
            raise RuntimeError(f"Failed to initialize stream iterator: {error_msg}")
    
    def __dealloc__(self):
        if self.iterator != NULL:
            del self.iterator
    
    def __iter__(self):
        return self
    
    def __next__(self):
        """Get next row from stream."""
        cdef ReturnVal ret
        
        ret = self.iterator.next()
        
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


# Import PyCapsule functions
cdef extern from "Python.h":
    void* PyCapsule_GetPointer(object capsule, const char* name) except NULL

