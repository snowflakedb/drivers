use pyo3::ffi;
use pyo3::prelude::*;
use pyo3::types::{PyList, PyTuple};

pub(crate) struct BatchConverter {
    columns: Vec<crate::arrow::converters::Column>,
    row_index: usize,
    row_count: usize,
    num_cols_ssize: ffi::Py_ssize_t,
}

impl BatchConverter {
    pub(crate) fn new(columns: Vec<crate::arrow::converters::Column>, row_count: usize) -> Self {
        let num_cols = columns.len();
        debug_assert!(num_cols <= ffi::Py_ssize_t::MAX as usize);
        Self {
            columns,
            row_index: 0,
            row_count,
            num_cols_ssize: num_cols as ffi::Py_ssize_t,
        }
    }

    pub(crate) fn exhausted() -> Self {
        Self::new(Vec::new(), 0)
    }

    #[inline]
    pub(crate) fn is_exhausted(&self) -> bool {
        self.row_index >= self.row_count
    }

    #[inline]
    fn cell_at<'py>(&self, py: Python<'py>, col: usize, row: usize) -> PyResult<Bound<'py, PyAny>> {
        self.columns[col].to_py(py, row)
    }

    pub(crate) fn materialize_row<'py>(
        &self,
        py: Python<'py>,
        row: usize,
    ) -> PyResult<Bound<'py, PyTuple>> {
        let num_cols = self.columns.len();

        // SAFETY: `PyTuple_New` returns a new owned tuple with NULL slots, or NULL
        // on allocation failure. We uniquely own it until `from_owned_ptr` / DECREF.
        let tuple = unsafe { ffi::PyTuple_New(self.num_cols_ssize) };
        if tuple.is_null() {
            return Err(PyErr::fetch(py));
        }

        for col in 0..num_cols {
            match self.cell_at(py, col, row) {
                Ok(value) => unsafe {
                    // SAFETY: exclusive owner; slot `col` is still NULL. SET_ITEM
                    // steals `value`'s owned reference into that slot.
                    ffi::PyTuple_SET_ITEM(tuple, col as ffi::Py_ssize_t, value.into_ptr());
                },
                Err(err) => {
                    // Remaining slots are NULL; pad so tuple dealloc is defined.
                    unsafe {
                        Self::pad_remaining_tuple_slots(tuple, col, num_cols);
                        ffi::Py_DECREF(tuple);
                    }
                    return Err(err);
                }
            }
        }

        // SAFETY: `tuple` is a `PyTuple` from `PyTuple_New` with every slot filled;
        // `from_owned_ptr` takes the owned reference, `cast_into_unchecked` skips
        // the type check because `PyTuple_New` cannot return a non-tuple.
        Ok(unsafe { Bound::from_owned_ptr(py, tuple).cast_into_unchecked() })
    }

    pub(crate) fn take_row(&mut self, py: Python<'_>) -> PyResult<Py<PyTuple>> {
        debug_assert!(self.row_index < self.row_count);
        let row = self.materialize_row(py, self.row_index)?;
        self.row_index += 1;
        Ok(row.unbind())
    }

    pub(crate) fn rows_remaining(&self) -> usize {
        self.row_count.saturating_sub(self.row_index)
    }

    pub(crate) fn column_count(&self) -> usize {
        self.columns.len()
    }

    pub(crate) fn append_rows_column_major<'py>(
        &mut self,
        py: Python<'py>,
        list: &mut BatchPyList<'py>,
        count: usize,
    ) -> PyResult<()> {
        let take = count.min(self.rows_remaining());
        if take == 0 {
            return Ok(());
        }

        let start_row = self.row_index;
        for col in 0..self.columns.len() {
            for row_offset in 0..take {
                let value = self.cell_at(py, col, start_row + row_offset)?;
                list.push_to_col(py, col, value)?;
            }
        }

        self.row_index += take;
        Ok(())
    }

    unsafe fn pad_remaining_tuple_slots(
        tuple: *mut ffi::PyObject,
        from_col: usize,
        num_cols: usize,
    ) {
        // SAFETY: caller guarantees `tuple` is an exclusively owned `PyTuple` with
        // NULL slots in `[from_col, num_cols)`.
        unsafe {
            let none = ffi::Py_None();
            for col in from_col..num_cols {
                ffi::Py_INCREF(none);
                ffi::PyTuple_SET_ITEM(tuple, col as ffi::Py_ssize_t, none);
            }
        }
    }
}

/// Owns a Python list of tuples while its slots are filled column-by-column.
pub(crate) struct BatchPyList<'py> {
    list: Bound<'py, PyList>,
    columns: usize,
    rows: usize,
    column_fills: Vec<usize>,
}

impl<'py> BatchPyList<'py> {
    pub(crate) fn new(py: Python<'py>, columns: usize) -> Self {
        Self {
            list: PyList::empty(py),
            columns,
            rows: 0,
            column_fills: vec![0; columns],
        }
    }

    pub(crate) fn len(&self) -> usize {
        self.rows
    }

    pub(crate) fn push_to_col(
        &mut self,
        py: Python<'py>,
        col: usize,
        value: Bound<'py, PyAny>,
    ) -> PyResult<()> {
        debug_assert!(col < self.columns);
        let row = self.column_fills[col];
        if row == self.rows {
            self.append_empty_tuple(py)?;
        }

        // SAFETY: `row` identifies an existing tuple exclusively owned by this
        // builder. This column's slot is NULL because each column advances once
        // per successful call. SET_ITEM steals `value`'s owned reference.
        unsafe {
            let tuple = ffi::PyList_GET_ITEM(self.list.as_ptr(), row as ffi::Py_ssize_t);
            ffi::PyTuple_SET_ITEM(tuple, col as ffi::Py_ssize_t, value.into_ptr());
        }
        self.column_fills[col] += 1;
        Ok(())
    }

    pub(crate) fn into_list(mut self) -> Py<PyList> {
        self.fill_null_slots();
        self.list.clone().unbind()
    }

    fn append_empty_tuple(&mut self, py: Python<'py>) -> PyResult<()> {
        debug_assert!(self.columns <= ffi::Py_ssize_t::MAX as usize);
        // SAFETY: `PyTuple_New` returns a new owned tuple with NULL slots. The
        // private list is exclusively owned. PyList_Append adds a reference;
        // the local reference is released after the append. On append failure,
        // slots are filled before releasing the tuple.
        unsafe {
            let tuple = ffi::PyTuple_New(self.columns as ffi::Py_ssize_t);
            if tuple.is_null() {
                return Err(PyErr::fetch(py));
            }
            if ffi::PyList_Append(self.list.as_ptr(), tuple) < 0 {
                BatchConverter::pad_remaining_tuple_slots(tuple, 0, self.columns);
                ffi::Py_DECREF(tuple);
                return Err(PyErr::fetch(py));
            }
            ffi::Py_DECREF(tuple);
        }
        self.rows += 1;
        Ok(())
    }

    fn fill_null_slots(&mut self) {
        // SAFETY: every list item was created by `append_empty_tuple` and is a
        // tuple exclusively owned by this builder. Unfilled slots are NULL.
        unsafe {
            let none = ffi::Py_None();
            for (col, filled) in self.column_fills.iter_mut().enumerate() {
                for row in *filled..self.rows {
                    let tuple = ffi::PyList_GET_ITEM(self.list.as_ptr(), row as ffi::Py_ssize_t);
                    ffi::Py_INCREF(none);
                    ffi::PyTuple_SET_ITEM(tuple, col as ffi::Py_ssize_t, none);
                }
                *filled = self.rows;
            }
        }
    }
}

impl Drop for BatchPyList<'_> {
    fn drop(&mut self) {
        self.fill_null_slots();
    }
}
