use std::sync::Arc;

use pyo3::ffi;
use pyo3::prelude::*;
use pyo3::types::PyList;

use crate::arrow::converters::RowShape;

pub(crate) struct BatchConverter {
    columns: Vec<crate::arrow::converters::Column>,
    row_shape: Arc<RowShape>,
    row_index: usize,
    row_count: usize,
    num_cols_ssize: ffi::Py_ssize_t,
}

impl BatchConverter {
    pub(crate) fn new(
        columns: Vec<crate::arrow::converters::Column>,
        row_count: usize,
        row_shape: Arc<RowShape>,
    ) -> Self {
        let num_cols = columns.len();
        debug_assert!(num_cols <= ffi::Py_ssize_t::MAX as usize);
        debug_assert!(match &*row_shape {
            RowShape::Tuple => true,
            RowShape::Dict { keys } => keys.len() == num_cols,
        });
        Self {
            columns,
            row_shape,
            row_index: 0,
            row_count,
            num_cols_ssize: num_cols as ffi::Py_ssize_t,
        }
    }

    pub(crate) fn exhausted() -> Self {
        Self::new(Vec::new(), 0, Arc::new(RowShape::Tuple))
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
    ) -> PyResult<Bound<'py, PyAny>> {
        let num_cols = self.columns.len();
        let row_obj = alloc_empty_row(py, &self.row_shape, self.num_cols_ssize)?;

        for col in 0..num_cols {
            match self.cell_at(py, col, row) {
                Ok(value) => {
                    if let Err(err) =
                        place_cell(py, &self.row_shape, row_obj, col, value.into_ptr())
                    {
                        unsafe { discard_row(&self.row_shape, row_obj, col + 1, num_cols) };
                        return Err(err);
                    }
                }
                Err(err) => {
                    unsafe { discard_row(&self.row_shape, row_obj, col, num_cols) };
                    return Err(err);
                }
            }
        }

        // SAFETY: `row_obj` is the owned tuple or dict we allocated above, with
        // every cell placed. `from_owned_ptr` takes that owned reference.
        Ok(unsafe { Bound::from_owned_ptr(py, row_obj) })
    }

    pub(crate) fn take_row(&mut self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        debug_assert!(self.row_index < self.row_count);
        let row = self.materialize_row(py, self.row_index)?;
        self.row_index += 1;
        Ok(row.unbind())
    }

    pub(crate) fn rows_remaining(&self) -> usize {
        self.row_count.saturating_sub(self.row_index)
    }

    pub(crate) fn row_list<'py>(&self, py: Python<'py>) -> BatchPyList<'py> {
        BatchPyList::new(py, self.columns.len(), Arc::clone(&self.row_shape))
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

/// Owns a Python list of rows while they are filled column-by-column.
pub(crate) struct BatchPyList<'py> {
    list: Bound<'py, PyList>,
    columns: usize,
    rows: usize,
    column_fills: Vec<usize>,
    row_shape: Arc<RowShape>,
}

impl<'py> BatchPyList<'py> {
    pub(crate) fn new(py: Python<'py>, columns: usize, row_shape: Arc<RowShape>) -> Self {
        Self {
            list: PyList::empty(py),
            columns,
            rows: 0,
            column_fills: vec![0; columns],
            row_shape,
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
            self.append_empty_row(py)?;
        }

        // SAFETY: `row` identifies an existing row exclusively owned by this
        // builder. Tuple slots for this column are still NULL because each
        // column advances once per successful call.
        unsafe {
            let row_obj = ffi::PyList_GET_ITEM(self.list.as_ptr(), row as ffi::Py_ssize_t);
            place_cell(py, &self.row_shape, row_obj, col, value.into_ptr())?;
        }
        self.column_fills[col] += 1;
        Ok(())
    }

    pub(crate) fn into_list(mut self) -> Py<PyList> {
        self.fill_null_slots();
        self.list.clone().unbind()
    }

    fn append_empty_row(&mut self, py: Python<'py>) -> PyResult<()> {
        debug_assert!(self.columns <= ffi::Py_ssize_t::MAX as usize);
        let row_obj = alloc_empty_row(py, &self.row_shape, self.columns as ffi::Py_ssize_t)?;
        // SAFETY: `row_obj` is a new owned tuple (NULL slots) or dict. The list is
        // exclusively owned by this builder. PyList_Append adds a reference; the
        // local reference is released after the append. On append failure, tuple
        // slots are padded before releasing the row.
        unsafe {
            if ffi::PyList_Append(self.list.as_ptr(), row_obj) < 0 {
                discard_row(&self.row_shape, row_obj, 0, self.columns);
                return Err(PyErr::fetch(py));
            }
            ffi::Py_DECREF(row_obj);
        }
        self.rows += 1;
        Ok(())
    }

    fn fill_null_slots(&mut self) {
        if !matches!(&*self.row_shape, RowShape::Tuple) {
            return;
        }
        // SAFETY: every new list item was created by `append_empty_row` as a
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

fn alloc_empty_row(
    py: Python<'_>,
    shape: &RowShape,
    num_cols_ssize: ffi::Py_ssize_t,
) -> PyResult<*mut ffi::PyObject> {
    // SAFETY: `PyTuple_New` / `PyDict_New` return a new owned object or NULL.
    let row_obj = unsafe {
        if matches!(shape, RowShape::Tuple) {
            ffi::PyTuple_New(num_cols_ssize)
        } else {
            ffi::PyDict_New()
        }
    };
    if row_obj.is_null() {
        Err(PyErr::fetch(py))
    } else {
        Ok(row_obj)
    }
}

fn place_cell(
    py: Python<'_>,
    shape: &RowShape,
    row_obj: *mut ffi::PyObject,
    col: usize,
    value: *mut ffi::PyObject,
) -> PyResult<()> {
    match shape {
        RowShape::Tuple => {
            // SAFETY: exclusive owner; slot `col` is still NULL. SET_ITEM steals
            // `value`'s owned reference into that slot.
            unsafe { ffi::PyTuple_SET_ITEM(row_obj, col as ffi::Py_ssize_t, value) };
            Ok(())
        }
        RowShape::Dict { keys } => {
            // SAFETY: `row_obj` is an owned dict; `keys[col]` is interned and kept
            // alive by `RowShape`. `PyDict_SetItem` increfs `value` and does not
            // steal, so the owned reference is released after the call.
            let result = unsafe { ffi::PyDict_SetItem(row_obj, keys[col].as_ptr(), value) };
            unsafe { ffi::Py_DECREF(value) };
            if result != 0 {
                Err(PyErr::fetch(py))
            } else {
                Ok(())
            }
        }
    }
}

unsafe fn discard_row(
    shape: &RowShape,
    row_obj: *mut ffi::PyObject,
    from_col: usize,
    num_cols: usize,
) {
    if matches!(shape, RowShape::Tuple) {
        unsafe { BatchConverter::pad_remaining_tuple_slots(row_obj, from_col, num_cols) };
    }
    unsafe { ffi::Py_DECREF(row_obj) };
}
