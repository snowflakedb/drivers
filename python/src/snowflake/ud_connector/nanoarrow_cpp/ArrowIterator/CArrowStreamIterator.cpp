#include "CArrowStreamIterator.hpp"

#include <memory>
#include <string>
#include <vector>

namespace sf {

Logger* CArrowStreamIterator::logger = new Logger("snowflake.connector.CArrowStreamIterator");

CArrowStreamIterator::CArrowStreamIterator(int64_t stream_ptr, PyObject* context,
                                           PyObject* use_numpy, PyObject* use_dict_result)
    : m_stream(reinterpret_cast<ArrowArrayStream*>(stream_ptr)),
      m_ownsStream(true),
      m_currentRowIndex(0),
      m_rowCount(0),
      m_columnCount(0),
      m_context(context),
      m_useNumpy(use_numpy == Py_True),
      m_useDictResult(use_dict_result == Py_True),
      m_initialized(false),
      m_streamExhausted(false),
      m_totalRowsReturned(0) {
  int returnCode = 0;

  // Validate stream pointer
  if (m_stream == nullptr || m_stream->release == nullptr) {
    std::string errorInfo = "[Snowflake Exception] Invalid ArrowArrayStream pointer";
    logger->error(__FILE__, __func__, __LINE__, errorInfo.c_str());
    PyErr_SetString(PyExc_Exception, errorInfo.c_str());
    return;
  }

  // Get schema from stream
  returnCode = m_stream->get_schema(m_stream, m_schema.get());
  if (returnCode != 0) {
    const char* error_msg = m_stream->get_last_error(m_stream);
    std::string errorInfo = Logger::formatString(
        "[Snowflake Exception] error getting schema from stream: %s, error code: %d",
        error_msg ? error_msg : "unknown", returnCode);
    logger->error(__FILE__, __func__, __LINE__, errorInfo.c_str());
    PyErr_SetString(PyExc_Exception, errorInfo.c_str());
    return;
  }

  m_columnCount = m_schema->n_children;
  logger->debug(__FILE__, __func__, __LINE__,
                "CArrowStreamIterator initialized with %lld columns", m_columnCount);

  m_initialized = true;
}

CArrowStreamIterator::~CArrowStreamIterator() {
  // Release the stream if we own it
  if (m_ownsStream && m_stream != nullptr && m_stream->release != nullptr) {
    m_stream->release(m_stream);
    m_stream = nullptr;
  }
}

bool CArrowStreamIterator::loadNextBatch() {
  if (m_streamExhausted) {
    return false;
  }

  // Reset current batch state
  m_currentRowIndex = 0;
  m_rowCount = 0;
  m_columnConverters.clear();

  // Create new array for next batch
  m_currentArray.reset();
  m_currentArrayView.reset();

  // Get next batch from stream
  // IMPORTANT: Release the GIL during get_next() to avoid deadlocks.
  // Similar practice happens in pyarrow.
  int returnCode;
  {
    Py_BEGIN_ALLOW_THREADS
    returnCode = m_stream->get_next(m_stream, m_currentArray.get());
    Py_END_ALLOW_THREADS
  }

  if (returnCode != 0) {
    const char* error_msg = m_stream->get_last_error(m_stream);
    std::string errorInfo = Logger::formatString(
        "[Snowflake Exception] error getting next batch from stream: %s, error code: %d",
        error_msg ? error_msg : "unknown", returnCode);
    logger->error(__FILE__, __func__, __LINE__, errorInfo.c_str());
    PyErr_SetString(PyExc_Exception, errorInfo.c_str());
    return false;
  }

  // Check if stream is exhausted (release is null means no more data)
  if (m_currentArray->release == nullptr) {
    m_streamExhausted = true;
    logger->debug(__FILE__, __func__, __LINE__, "Stream exhausted");
    return false;
  }

  m_rowCount = m_currentArray->length;
  logger->debug(__FILE__, __func__, __LINE__, "Loaded batch with %lld rows", m_rowCount);

  // Handle empty batches
  if (m_rowCount == 0) {
    return loadNextBatch();  // Try next batch
  }

  // Initialize array view
  ArrowError error;
  returnCode = ArrowArrayViewInitFromSchema(m_currentArrayView.get(), m_schema.get(), &error);
  if (returnCode != NANOARROW_OK) {
    std::string errorInfo = Logger::formatString(
        "[Snowflake Exception] error initializing ArrowArrayView: %s, error code: %d",
        ArrowErrorMessage(&error), returnCode);
    logger->error(__FILE__, __func__, __LINE__, errorInfo.c_str());
    PyErr_SetString(PyExc_Exception, errorInfo.c_str());
    return false;
  }

  returnCode = ArrowArrayViewSetArray(m_currentArrayView.get(), m_currentArray.get(), &error);
  if (returnCode != NANOARROW_OK) {
    std::string errorInfo = Logger::formatString(
        "[Snowflake Exception] error setting ArrowArrayView: %s, error code: %d",
        ArrowErrorMessage(&error), returnCode);
    logger->error(__FILE__, __func__, __LINE__, errorInfo.c_str());
    PyErr_SetString(PyExc_Exception, errorInfo.c_str());
    return false;
  }

  // Initialize column converters for this batch
  initColumnConverters();

  return true;
}

void CArrowStreamIterator::initColumnConverters() {
  m_columnConverters.clear();
  m_columnConverters.reserve(m_columnCount);

  for (int64_t i = 0; i < m_columnCount; ++i) {
    ArrowSchema* columnSchema = m_schema->children[i];
    ArrowArrayView* columnArrayView = m_currentArrayView->children[i];

    auto converter =
        getConverterFromSchema(columnSchema, columnArrayView, m_context, m_useNumpy, logger);
    if (converter == nullptr) {
      std::string errorInfo =
          Logger::formatString("[Snowflake Exception] Failed to create converter for column %lld", i);
      logger->error(__FILE__, __func__, __LINE__, errorInfo.c_str());
      PyErr_SetString(PyExc_Exception, errorInfo.c_str());
      return;
    }
    m_columnConverters.push_back(converter);
  }

  logger->debug(__FILE__, __func__, __LINE__, "Initialized %zu column converters",
                m_columnConverters.size());
}

ReturnVal CArrowStreamIterator::checkInitializationStatus() {
  if (PyErr_Occurred()) {
    PyObject *type, *val, *traceback;
    PyErr_Fetch(&type, &val, &traceback);
    PyErr_Clear();
    m_currentPyException.reset(val);
    Py_XDECREF(type);
    Py_XDECREF(traceback);
    return ReturnVal(nullptr, m_currentPyException.get());
  }

  if (!m_initialized) {
    std::string errorInfo = "[Snowflake Exception] Stream iterator initialization failed";
    logger->error(__FILE__, __func__, __LINE__, errorInfo.c_str());
    PyErr_SetString(PyExc_Exception, errorInfo.c_str());
    m_currentPyException.reset(PyErr_Occurred());
    return ReturnVal(nullptr, m_currentPyException.get());
  }

  return ReturnVal(Py_True, nullptr);
}

ReturnVal CArrowStreamIterator::next() {
  // Check if we need to load the next batch
  while (m_currentRowIndex >= m_rowCount) {
    if (!loadNextBatch()) {
      // Stream exhausted
      return ReturnVal(nullptr, nullptr);
    }
  }

  // Handle empty schema (no columns)
  if (m_columnCount == 0) {
    if (m_useDictResult) {
      m_latestReturnedRow.reset(PyDict_New());
    } else {
      m_latestReturnedRow.reset(PyTuple_New(0));
    }
    m_currentRowIndex++;
    m_totalRowsReturned++;
    return ReturnVal(m_latestReturnedRow.get(), nullptr);
  }

  // Convert current row to Python object
  if (m_useDictResult) {
    createDictRowPyObject();
  } else {
    createRowPyObject();
  }

  // Check for Python errors during conversion
  if (py::checkPyError()) {
    PyObject *type, *val, *traceback;
    PyErr_Fetch(&type, &val, &traceback);
    PyErr_Clear();
    m_currentPyException.reset(val);
    Py_XDECREF(type);
    Py_XDECREF(traceback);
    return ReturnVal(nullptr, m_currentPyException.get());
  }

  // Increment row counter
  m_currentRowIndex++;
  m_totalRowsReturned++;

  // Return the row
  return ReturnVal(m_latestReturnedRow.get(), nullptr);
}

void CArrowStreamIterator::createRowPyObject() {
  PyObject* pytuple = PyTuple_New(m_columnCount);

  for (int64_t colIdx = 0; colIdx < m_columnCount; ++colIdx) {
    PyObject* val = m_columnConverters[colIdx]->toPyObject(m_currentRowIndex);

    if (py::checkPyError()) {
      logger->debug(__FILE__, __func__, __LINE__,
                    "Python error occurred during conversion of column %lld", colIdx);
      Py_DECREF(pytuple);
      return;
    }

    PyTuple_SET_ITEM(pytuple, colIdx, val);
  }

  m_latestReturnedRow.reset(pytuple);
}

void CArrowStreamIterator::createDictRowPyObject() {
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

