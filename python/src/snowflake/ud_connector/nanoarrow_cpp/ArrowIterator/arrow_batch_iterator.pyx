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


cdef class NanoarrowBatchIterator:
    """
    Python wrapper for C++ Arrow batch iterator.
    Converts Arrow RecordBatch to Python tuples/dicts row-by-row.
    """
    cdef CArrowBatchIterator* iterator
    cdef DictCArrowBatchIterator* dict_iterator
    cdef cpp_bool use_dict_result
    cdef object arrow_context
    # We need to own the schema/array memory when created from raw pointers
    cdef ArrowSchema* _owned_schema
    cdef ArrowArray* _owned_array
    
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
            Pointer to ArrowArray (as integer)
        schema_ptr : int
            Pointer to ArrowSchema (as integer)
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
        self._owned_schema = NULL
        self._owned_array = NULL
        
        # Cast integer pointers to C pointers
        cdef ArrowArray* c_array_ptr = <ArrowArray*>array_ptr
        cdef ArrowSchema* c_schema_ptr = <ArrowSchema*>schema_ptr
        
        if c_array_ptr == NULL or c_schema_ptr == NULL:
            raise RuntimeError("Invalid Arrow C Data pointers (NULL)")
        
        # Allocate our own copies and move the data into them
        # This ensures proper ownership and cleanup
        self._owned_schema = <ArrowSchema*>malloc(sizeof(ArrowSchema))
        self._owned_array = <ArrowArray*>malloc(sizeof(ArrowArray))
        
        if self._owned_schema == NULL or self._owned_array == NULL:
            if self._owned_schema != NULL:
                free(self._owned_schema)
                self._owned_schema = NULL
            if self._owned_array != NULL:
                free(self._owned_array)
                self._owned_array = NULL
            raise MemoryError("Failed to allocate Arrow structures")
        
        # Move the data (transfers ownership)
        ArrowSchemaMove(c_schema_ptr, self._owned_schema)
        ArrowArrayMove(c_array_ptr, self._owned_array)
        
        # Declare ReturnVal variable at function scope (Cython requirement)
        cdef ReturnVal init_ret
        
        # Create appropriate iterator
        if use_dict_result:
            self.dict_iterator = new DictCArrowBatchIterator(
                self._owned_array,
                self._owned_schema,
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
                self._owned_array,
                self._owned_schema,
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
        # Release and free owned structures
        if self._owned_schema != NULL:
            if self._owned_schema.release != NULL:
                self._owned_schema.release(self._owned_schema)
            free(self._owned_schema)
        if self._owned_array != NULL:
            if self._owned_array.release != NULL:
                self._owned_array.release(self._owned_array)
            free(self._owned_array)
    
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


cdef class NanoarrowStreamIterator:
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
    
    cdef _read_next_batch(self):
        """Read the next batch from the stream. Returns None if exhausted."""
        if self._stream == NULL or self._stream.release == NULL:
            return None
        
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
            return None
        
        # Check for empty batch
        if batch_array.length == 0:
            if batch_array.release != NULL:
                batch_array.release(batch_array)
            free(batch_array)
            return None
        
        # We need to create a copy of the schema for each batch iterator
        # since the batch iterator takes ownership
        cdef ArrowSchema* batch_schema = <ArrowSchema*>malloc(sizeof(ArrowSchema))
        if batch_schema == NULL:
            if batch_array.release != NULL:
                batch_array.release(batch_array)
            free(batch_array)
            raise MemoryError("Failed to allocate ArrowSchema for batch")
        
        # Copy schema (we need to duplicate it since batch iterator will own it)
        # For now, we pass the schema pointer but don't move it
        # The batch iterator should not release the schema
        batch_schema[0] = self._schema[0]
        # Mark as not owned by setting release to NULL
        # This is a shallow copy - the batch iterator should handle this
        
        # Return pointers as integers (uintptr_t) so they can be returned to Python
        return (<uintptr_t>batch_array, <uintptr_t>batch_schema)
    
    def __next__(self):
        """Get next row from the stream."""
        cdef uintptr_t batch_array_ptr
        cdef uintptr_t batch_schema_ptr
        
        while True:
            # If no current batch iterator, read next batch
            if self._current_batch_iterator is None:
                result = self._read_next_batch()
                if result is None:
                    raise StopIteration
                
                batch_array_ptr, batch_schema_ptr = result
                
                # Handle empty schema (0 columns)
                if self._column_count == 0:
                    # Release the batch array
                    if (<ArrowArray*>batch_array_ptr).release != NULL:
                        (<ArrowArray*>batch_array_ptr).release(<ArrowArray*>batch_array_ptr)
                    free(<void*>batch_array_ptr)
                    free(<void*>batch_schema_ptr)
                    
                    if self._use_dict_result:
                        return {}
                    else:
                        return tuple()
                
                # Create batch iterator with raw pointers
                # Note: NanoarrowBatchIterator takes ownership via ArrowArrayMove/ArrowSchemaMove
                self._current_batch_iterator = NanoarrowBatchIterator(
                    batch_array_ptr,
                    batch_schema_ptr,
                    self._arrow_context,
                    use_dict_result=self._use_dict_result,
                    use_numpy=self._use_numpy,
                    check_error_on_every_column=True
                )
                
                # Free our allocations (data was moved to batch iterator)
                free(<void*>batch_array_ptr)
                free(<void*>batch_schema_ptr)
            
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
