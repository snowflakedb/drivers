#include "CArrowBatchConverter.hpp"

#include <memory>

#include "CArrowChunkIterator.hpp"  // For getConverterFromSchema

namespace sf {

Logger* CArrowBatchConverter::logger = new Logger("snowflake.connector.CArrowBatchConverter");

CArrowBatchConverter::CArrowBatchConverter(ArrowArray* c_array, ArrowSchema* c_schema,
                                           PyObject* context, PyObject* use_numpy,
                                           PyObject* check_error_on_every_column)
    : m_currentRowIndex(0),
      m_rowCount(0),
      m_columnCount(0),
      m_context(context),
      m_useNumpy(use_numpy == Py_True),
      m_checkErrorOnEveryColumn(check_error_on_every_column == Py_True) {
  int returnCode = 0;
  ArrowError error;

  // Move Arrow C Data structures into RAII wrappers
  ArrowSchemaMove(c_schema, m_schema.get());
  ArrowArrayMove(c_array, m_array.get());

  // Validate we got valid data
  if (m_schema->release == nullptr || m_array->release == nullptr) {
    std::string errorInfo = "[Snowflake Exception] Invalid Arrow C Data: schema or array is null";
    logger->error(__FILE__, __func__, __LINE__, errorInfo.c_str());
    PyErr_SetString(PyExc_Exception, errorInfo.c_str());
    return;
  }

  // Get row count
  m_rowCount = m_array->length;
  m_columnCount = m_schema->n_children;

  logger->debug(__FILE__, __func__, __LINE__,
                "CArrowBatchConverter initialized: rows=%lld, columns=%lld", m_rowCount,
                m_columnCount);

  // Initialize array view for efficient access
  returnCode = ArrowArrayViewInitFromSchema(m_arrayView.get(), m_schema.get(), &error);
  if (returnCode != NANOARROW_OK) {
    std::string errorInfo = Logger::formatString(
        "[Snowflake Exception] error initializing ArrowArrayView: %s, error "
        "code: %d",
        ArrowErrorMessage(&error), returnCode);
    logger->error(__FILE__, __func__, __LINE__, errorInfo.c_str());
    PyErr_SetString(PyExc_Exception, errorInfo.c_str());
    return;
  }

  returnCode = ArrowArrayViewSetArray(m_arrayView.get(), m_array.get(), &error);
  if (returnCode != NANOARROW_OK) {
    std::string errorInfo = Logger::formatString(
        "[Snowflake Exception] error setting ArrowArrayView: %s, error code: "
        "%d",
        ArrowErrorMessage(&error), returnCode);
    logger->error(__FILE__, __func__, __LINE__, errorInfo.c_str());
    PyErr_SetString(PyExc_Exception, errorInfo.c_str());
    return;
  }

  // Initialize column converters
  initColumnConverters();
}

CArrowBatchConverter::~CArrowBatchConverter() {
  // RAII handles cleanup
}

void CArrowBatchConverter::initColumnConverters() {
  m_columnConverters.clear();
  m_columnConverters.reserve(m_columnCount);

  for (int64_t i = 0; i < m_columnCount; ++i) {
    ArrowSchema* columnSchema = m_schema->children[i];
    ArrowArrayView* columnArrayView = m_arrayView->children[i];

    auto converter =
        getConverterFromSchema(columnSchema, columnArrayView, m_context, m_useNumpy, logger);
    if (converter == nullptr) {
      std::string errorInfo = Logger::formatString(
          "[Snowflake Exception] Failed to create converter for column %lld", i);
      logger->error(__FILE__, __func__, __LINE__, errorInfo.c_str());
      PyErr_SetString(PyExc_Exception, errorInfo.c_str());
      return;
    }
    m_columnConverters.push_back(converter);
  }

  logger->debug(__FILE__, __func__, __LINE__, "Initialized %zu column converters",
                m_columnConverters.size());
}

ReturnVal CArrowBatchConverter::checkInitializationStatus() {
  if (PyErr_Occurred()) {
    PyObject *type, *val, *traceback;
    PyErr_Fetch(&type, &val, &traceback);
    PyErr_Clear();
    m_currentPyException.reset(val);
    Py_XDECREF(type);
    Py_XDECREF(traceback);
    return ReturnVal(nullptr, m_currentPyException.get());
  }

  if (m_columnConverters.size() != static_cast<size_t>(m_columnCount)) {
    std::string errorInfo = "[Snowflake Exception] Column converter initialization failed";
    logger->error(__FILE__, __func__, __LINE__, errorInfo.c_str());
    PyErr_SetString(PyExc_Exception, errorInfo.c_str());
    m_currentPyException.reset(PyErr_Occurred());
    return ReturnVal(nullptr, m_currentPyException.get());
  }

  return ReturnVal(Py_True, nullptr);
}

ReturnVal CArrowBatchConverter::next() {
  // Check if we've exhausted all rows
  if (m_currentRowIndex >= m_rowCount) {
    return ReturnVal(nullptr, nullptr);  // Signal end of iteration
  }

  // Convert current row to Python object
  createRowPyObject();

  // Check for Python errors during conversion
  SF_CHECK_PYTHON_ERR();

  // Increment row counter
  m_currentRowIndex++;

  // Return the row
  return ReturnVal(m_latestReturnedRow.get(), nullptr);
}

void CArrowBatchConverter::createRowPyObject() {
  PyObject* pytuple = PyTuple_New(m_columnCount);

  for (int64_t colIdx = 0; colIdx < m_columnCount; ++colIdx) {
    PyObject* val = m_columnConverters[colIdx]->toPyObject(m_currentRowIndex);

    if (m_checkErrorOnEveryColumn && py::checkPyError()) {
      logger->debug(__FILE__, __func__, __LINE__,
                    "Python error occurred during conversion of column %lld", colIdx);
      Py_DECREF(pytuple);
      return;
    }

    PyTuple_SET_ITEM(pytuple, colIdx, val);
  }

  m_latestReturnedRow.reset(pytuple);
}

// Dictionary variant implementation
DictCArrowBatchConverter::DictCArrowBatchConverter(ArrowArray* c_array, ArrowSchema* c_schema,
                                                   PyObject* context, PyObject* use_numpy)
    : CArrowBatchConverter(c_array, c_schema, context, use_numpy, Py_False) {}

void DictCArrowBatchConverter::createRowPyObject() {
  PyObject* pydict = PyDict_New();

  for (int64_t colIdx = 0; colIdx < m_columnCount; ++colIdx) {
    const char* colName = m_schema->children[colIdx]->name;
    PyObject* val = m_columnConverters[colIdx]->toPyObject(m_currentRowIndex);

    if (py::checkPyError()) {
      logger->debug(__FILE__, __func__, __LINE__,
                    "Python error occurred during conversion of column %s", colName);
      Py_DECREF(pydict);
      return;
    }

    PyDict_SetItemString(pydict, colName, val);
    Py_DECREF(val);  // PyDict_SetItemString increments reference
  }

  m_latestReturnedRow.reset(pydict);
}

}  // namespace sf
