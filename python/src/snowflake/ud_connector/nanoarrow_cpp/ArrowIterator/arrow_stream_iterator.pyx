# distutils: language = c++
# cython: language_level=3

from cpython.ref cimport PyObject
from libc.stdint cimport int64_t, uintptr_t
from libc.stdlib cimport malloc, free
from libcpp cimport bool as cpp_bool

# Import Arrow C Data Interface structures from nanoarrow
cdef extern from "nanoarrow.h":
    ctypedef struct ArrowArray:
        int64_t length
        void (*release)(ArrowArray*)
    
    ctypedef struct ArrowSchema:
        int64_t n_children
        void (*release)(ArrowSchema*)
    
    # Arrow C Stream Interface
    ctypedef struct ArrowArrayStream:
        int (*get_schema)(ArrowArrayStream*, ArrowSchema* out) noexcept
        int (*get_next)(ArrowArrayStream*, ArrowArray* out) noexcept
        const char* (*get_last_error)(ArrowArrayStream*) noexcept
        void (*release)(ArrowArrayStream*) noexcept
        void* private_data
    
    cdef int ArrowArrayMove(ArrowArray* src, ArrowArray* dst)
    cdef int ArrowSchemaMove(ArrowSchema* src, ArrowSchema* dst)
    
    cdef int NANOARROW_OK


# Import our C++ classes
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


cdef class ArrowBatchIterator:
    """
    Python wrapper for C++ Arrow batch iterator.
    Converts Arrow RecordBatch to Python tuples/dicts row-by-row.
    
    Schema is always borrowed from the stream iterator (caller retains ownership).
    Array ownership is always transferred to this iterator.
    """
    cdef CArrowBatchIterator* iterator
    cdef DictCArrowBatchIterator* dict_iterator
    cdef cpp_bool use_dict_result
    cdef object arrow_context
    # Array storage - we allocate, C++ takes ownership via ArrowArrayMove
    cdef ArrowArray* _array_storage
    
    def __cinit__(
        self,
        uintptr_t array_ptr,
        uintptr_t schema_ptr,
        object arrow_context,
        object use_dict_result=False,
        object use_numpy=False,
        object check_error_on_every_column=True
    ):
        """
        Initialize the batch iterator from Arrow C Data Interface pointers.
        
        Parameters
        ----------
        array_ptr : int
            Pointer to ArrowArray (as integer) - ownership transferred
        schema_ptr : int
            Pointer to ArrowSchema (as integer) - borrowed, caller retains ownership
        arrow_context : ArrowConverterContext
            Context object for conversions
        use_dict_result : bool
            If True, return dicts instead of tuples
        use_numpy : bool
            If True, use numpy types for numeric data
        check_error_on_every_column : bool
            If True, check for Python errors after each column conversion
        """
        self.use_dict_result = use_dict_result
        self.arrow_context = arrow_context
        self.iterator = NULL
        self.dict_iterator = NULL
        self._array_storage = NULL
        
        # Cast pointers
        cdef ArrowArray* c_array_ptr = <ArrowArray*>array_ptr
        cdef ArrowSchema* c_schema_ptr = <ArrowSchema*>schema_ptr
        
        if c_array_ptr == NULL or c_schema_ptr == NULL:
            raise RuntimeError("Invalid Arrow C Data pointers (NULL)")
        
        # Allocate storage for array and move data into it
        # C++ will take ownership via ArrowArrayMove
        self._array_storage = <ArrowArray*>malloc(sizeof(ArrowArray))
        if self._array_storage == NULL:
            raise MemoryError("Failed to allocate ArrowArray")
        
        # Move array data (transfers ownership to our storage, then to C++)
        ArrowArrayMove(c_array_ptr, self._array_storage)
        
        # Declare ReturnVal variable at function scope (Cython requirement)
        cdef ReturnVal init_ret
        
        # Create appropriate iterator
        # Schema is borrowed - C++ just stores the pointer
        if use_dict_result:
            self.dict_iterator = new DictCArrowBatchIterator(
                self._array_storage,
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
                self._array_storage,
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
        # Delete C++ iterators - they handle array cleanup via RAII
        # Schema is borrowed, so it's not released here
        if self.iterator != NULL:
            del self.iterator
        if self.dict_iterator != NULL:
            del self.dict_iterator
        # Free our array storage (data was moved to C++)
        if self._array_storage != NULL:
            free(self._array_storage)
    
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


cdef class ArrowStreamIterator:
    """
    Python wrapper that iterates over an Arrow C Stream Interface.
    
    This class directly consumes an ArrowArrayStream pointer from the Rust core,
    without requiring pyarrow. It yields rows one by one across all batches.
    """
    cdef ArrowArrayStream* _stream
    cdef ArrowSchema* _schema
    cdef cpp_bool _stream_owned
    cdef cpp_bool _schema_initialized
    cdef object _arrow_context
    cdef cpp_bool _use_dict_result
    cdef cpp_bool _use_numpy
    cdef object _current_batch_iterator
    cdef int64_t _column_count
    
    def __cinit__(
        self,
        uintptr_t stream_ptr,
        object arrow_context,
        object use_dict_result=False,
        object use_numpy=False
    ):
        """
        Initialize the stream iterator from an Arrow C Stream Interface pointer.
        
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
        self._stream = <ArrowArrayStream*>stream_ptr
        self._schema = NULL
        self._stream_owned = True
        self._schema_initialized = False
        self._arrow_context = arrow_context
        self._use_dict_result = use_dict_result
        self._use_numpy = use_numpy
        self._current_batch_iterator = None
        self._column_count = 0
        
        if self._stream == NULL:
            raise RuntimeError("Invalid ArrowArrayStream pointer (NULL)")
        
        if self._stream.release == NULL:
            raise RuntimeError("ArrowArrayStream has already been released")
        
        # Get the schema from the stream
        self._schema = <ArrowSchema*>malloc(sizeof(ArrowSchema))
        if self._schema == NULL:
            raise MemoryError("Failed to allocate ArrowSchema")
        
        # Initialize schema to empty state
        self._schema.release = NULL
        
        cdef int ret = self._stream.get_schema(self._stream, self._schema)
        if ret != 0:
            error_msg = "Unknown error"
            if self._stream.get_last_error != NULL:
                last_error = self._stream.get_last_error(self._stream)
                if last_error != NULL:
                    error_msg = last_error.decode('utf-8')
            free(self._schema)
            self._schema = NULL
            raise RuntimeError(f"Failed to get schema from stream: {error_msg}")
        
        self._schema_initialized = True
        self._column_count = self._schema.n_children if self._schema.n_children else 0
    
    def __dealloc__(self):
        # Release schema if we own it
        if self._schema != NULL:
            if self._schema.release != NULL:
                self._schema.release(self._schema)
            free(self._schema)
        
        # Release stream if we own it
        if self._stream_owned and self._stream != NULL:
            if self._stream.release != NULL:
                self._stream.release(self._stream)
    
    def __iter__(self):
        return self
    
    cdef ArrowArray* _read_next_batch(self) except NULL:
        """Read the next batch from the stream. Returns NULL if exhausted."""
        if self._stream == NULL or self._stream.release == NULL:
            return NULL
        
        # Allocate array for the batch
        cdef ArrowArray* batch_array = <ArrowArray*>malloc(sizeof(ArrowArray))
        if batch_array == NULL:
            raise MemoryError("Failed to allocate ArrowArray for batch")
        
        # Initialize to empty state
        batch_array.release = NULL
        
        cdef int ret = self._stream.get_next(self._stream, batch_array)
        if ret != 0:
            error_msg = "Unknown error"
            if self._stream.get_last_error != NULL:
                last_error = self._stream.get_last_error(self._stream)
                if last_error != NULL:
                    error_msg = last_error.decode('utf-8')
            free(batch_array)
            raise RuntimeError(f"Failed to get next batch from stream: {error_msg}")
        
        # Check if stream is exhausted (array.release will be NULL)
        if batch_array.release == NULL:
            free(batch_array)
            return NULL
        
        # Check for empty batch
        if batch_array.length == 0:
            if batch_array.release != NULL:
                batch_array.release(batch_array)
            free(batch_array)
            return NULL
        
        return batch_array
    
    def __next__(self):
        """Get next row from the stream."""
        cdef ArrowArray* batch_array
        
        while True:
            # If no current batch iterator, read next batch
            if self._current_batch_iterator is None:
                batch_array = self._read_next_batch()
                if batch_array == NULL:
                    raise StopIteration
                
                # Handle empty schema (0 columns)
                if self._column_count == 0:
                    # Release the batch array
                    if batch_array.release != NULL:
                        batch_array.release(batch_array)
                    free(batch_array)
                    
                    if self._use_dict_result:
                        return {}
                    else:
                        return tuple()
                
                # Create batch iterator
                # - Array ownership is transferred (ArrowBatchIterator moves it)
                # - Schema is borrowed (stream owns it)
                self._current_batch_iterator = ArrowBatchIterator(
                    <uintptr_t>batch_array,
                    <uintptr_t>self._schema,  # Pass schema as int - borrowed
                    self._arrow_context,
                    use_dict_result=self._use_dict_result,
                    use_numpy=self._use_numpy,
                    check_error_on_every_column=True
                )
                
                # Free our array allocation (data was moved to batch iterator)
                free(batch_array)
            
            # Try to get next row from current batch
            try:
                return next(self._current_batch_iterator)
            except StopIteration:
                # Batch exhausted, get next batch
                self._current_batch_iterator = None
                continue
    
    @property
    def column_count(self):
        """Get the number of columns in the result."""
        return self._column_count
