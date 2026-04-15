#include "CArrowStreamIterator.hpp"

#include <memory>
#include <string>
#include <vector>

namespace sf {



Logger* CArrowStreamIterator::logger = new Logger("snowflake.connector.CArrowStreamIterator");

std::unique_ptr<CArrowStreamIterator> CArrowStreamIterator::from_stream(int64_t stream_ptr,
                                                                        PyObject* context,
                                                                        bool use_numpy,
                                                                        bool use_dict_result) {
  auto* stream = reinterpret_cast<ArrowArrayStream*>(stream_ptr);

  // Validate stream pointer
  if (stream == nullptr || stream->release == nullptr) {
    std::string errorInfo = "[Snowflake Exception] Invalid ArrowArrayStream pointer";
    logger->error(__FILE__, __func__, __LINE__, errorInfo.c_str());
    PyErr_SetString(PyExc_Exception, errorInfo.c_str());
    return nullptr;
  }

  // Get schema from stream before creating the iterator
  ArrowSchema schema = {};
  int returnCode = stream->get_schema(stream, &schema);
  if (returnCode != 0) {
    const char* error_msg = stream->get_last_error(stream);
    std::string errorInfo = Logger::formatString(
        "[Snowflake Exception] error getting schema from stream: %s, error code: %d",
        error_msg ? error_msg : "unknown", returnCode);
    logger->error(__FILE__, __func__, __LINE__, errorInfo.c_str());
    PyErr_SetString(PyExc_Exception, errorInfo.c_str());
    return nullptr;
  }

  logger->debug(__FILE__, __func__, __LINE__, "CArrowStreamIterator initialized with %lld columns",
                schema.n_children);

  // Create the iterator with all data
  return std::unique_ptr<CArrowStreamIterator>(
      new CArrowStreamIterator(stream, &schema, context, use_numpy, use_dict_result));
}

namespace {
  void releaseArrowArrayStream(ArrowArrayStream* stream) {
    if (stream != nullptr && stream->release != nullptr) {
      stream->release(stream);
    }
  }
}  // namespace

CArrowStreamIterator::CArrowStreamIterator(ArrowArrayStream* stream, ArrowSchema* schema,
                                           PyObject* context, bool use_numpy, bool use_dict_result)
    : m_stream(stream, releaseArrowArrayStream),
      m_currentRowIndex(0),
      m_rowCount(0),
      m_columnCount(schema->n_children),
      m_context(context),
      m_useNumpy(use_numpy),
      m_useDictResult(use_dict_result),
      m_streamExhausted(false),
      m_totalRowsReturned(0) {
  // Move schema data into our UniqueSchema (transfers ownership)
  ArrowSchemaMove(schema, m_schema.get());
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
  ArrowArrayStream* stream = m_stream.get();
  {
    Py_BEGIN_ALLOW_THREADS
    returnCode = stream->get_next(stream, m_currentArray.get());
    Py_END_ALLOW_THREADS
  }

  if (returnCode != 0) {
    const char* error_msg = stream->get_last_error(stream);
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

ReturnVal CArrowStreamIterator::next() {
  while (m_currentRowIndex >= m_rowCount) {
    if (!loadNextBatch()) {
      return ReturnVal(nullptr, nullptr);
    }
  }

  PyObject* row = buildRowObject();
  if (row == nullptr) {
    if (py::checkPyError()) {
      PyObject *type, *val, *traceback;
      PyErr_Fetch(&type, &val, &traceback);
      PyErr_Clear();
      m_currentPyException.reset(val);
      Py_XDECREF(type);
      Py_XDECREF(traceback);
      return ReturnVal(nullptr, m_currentPyException.get());
    }
    return ReturnVal(nullptr, nullptr);
  }

  m_latestReturnedRow.reset(row);
  m_currentRowIndex++;
  m_totalRowsReturned++;
  return ReturnVal(m_latestReturnedRow.get(), nullptr);
}

PyObject* CArrowStreamIterator::nextN(int64_t size) {
  PyObject* pylist = PyList_New(size);
  if (pylist == nullptr) {
    return nullptr;
  }

  int64_t collected = 0;
  while (collected < size) {
    while (m_currentRowIndex >= m_rowCount) {
      if (!loadNextBatch()) {
        if (py::checkPyError()) {
          PyList_SetSlice(pylist, collected, size, nullptr);
          Py_DECREF(pylist);
          return nullptr;
        }
        if (PyList_SetSlice(pylist, collected, size, nullptr) != 0) {
          Py_DECREF(pylist);
          return nullptr;
        }
        return pylist;
      }
    }

    PyObject* row = buildRowObject();
    if (row == nullptr) {
      PyList_SetSlice(pylist, collected, size, nullptr);
      Py_DECREF(pylist);
      return nullptr;
    }

    PyList_SET_ITEM(pylist, collected, row);  // steals reference
    m_currentRowIndex++;
    m_totalRowsReturned++;
    collected++;
  }

  return pylist;
}

PyObject* CArrowStreamIterator::nextAll() {
  PyObject* pylist = PyList_New(0);
  if (pylist == nullptr) {
    return nullptr;
  }

  for (;;) {
    while (m_currentRowIndex >= m_rowCount) {
      if (!loadNextBatch()) {
        if (py::checkPyError()) {
          Py_DECREF(pylist);
          return nullptr;
        }
        return pylist;
      }
    }

    PyObject* row = buildRowObject();
    if (row == nullptr) {
      Py_DECREF(pylist);
      return nullptr;
    }

    if (PyList_Append(pylist, row) != 0) {
      Py_DECREF(row);
      Py_DECREF(pylist);
      return nullptr;
    }
    Py_DECREF(row);

    m_currentRowIndex++;
    m_totalRowsReturned++;
  }
}

PyObject* CArrowStreamIterator::buildRowObject() {
  if (m_columnCount == 0) {
    return m_useDictResult ? PyDict_New() : PyTuple_New(0);
  }

  if (m_useDictResult) {
    PyObject* pydict = PyDict_New();
    if (pydict == nullptr) {
      return nullptr;
    }

    for (int64_t colIdx = 0; colIdx < m_columnCount; ++colIdx) {
      const char* colName = m_schema->children[colIdx]->name;
      PyObject* val = m_columnConverters[colIdx]->toPyObject(m_currentRowIndex);

      if (py::checkPyError()) {
        logger->debug(__FILE__, __func__, __LINE__,
                      "Python error occurred during conversion of column %s", colName);
        Py_DECREF(pydict);
        return nullptr;
      }

      if (PyDict_SetItemString(pydict, colName, val) != 0) {
        Py_DECREF(val);
        Py_DECREF(pydict);
        return nullptr;
      }
      Py_DECREF(val);
    }
    return pydict;
  }

  PyObject* pytuple = PyTuple_New(m_columnCount);
  if (pytuple == nullptr) {
    return nullptr;
  }

  for (int64_t colIdx = 0; colIdx < m_columnCount; ++colIdx) {
    PyObject* val = m_columnConverters[colIdx]->toPyObject(m_currentRowIndex);

    if (py::checkPyError()) {
      logger->debug(__FILE__, __func__, __LINE__,
                    "Python error occurred during conversion of column %lld", colIdx);
      Py_DECREF(pytuple);
      return nullptr;
    }

    PyTuple_SET_ITEM(pytuple, colIdx, val);  // steals reference
  }
  return pytuple;
}

}  // namespace sf

