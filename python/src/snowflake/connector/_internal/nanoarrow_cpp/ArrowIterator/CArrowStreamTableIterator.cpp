#include "CArrowStreamTableIterator.hpp"

#include <memory>
#include <string>

#include "Python/Common.hpp"

namespace sf {

Logger* CArrowStreamTableIterator::logger =
    new Logger("snowflake.connector.CArrowStreamTableIterator");

namespace {
  void releaseStream(ArrowArrayStream* s) {
    if (s && s->release) s->release(s);
  }
}  // namespace

// ---------------------------------------------------------------------------
// Factory / constructor
// ---------------------------------------------------------------------------

std::unique_ptr<CArrowStreamTableIterator> CArrowStreamTableIterator::from_stream(int64_t stream_ptr, PyObject* context, bool number_to_decimal, bool force_microsecond_precision) {
  auto* stream = reinterpret_cast<ArrowArrayStream*>(stream_ptr);
  if (!stream || !stream->release) {
    PyErr_SetString(PyExc_Exception, "[Snowflake Exception] Invalid ArrowArrayStream pointer");
    return nullptr;
  }

  ArrowSchema schema{};
  int rc = stream->get_schema(stream, &schema);
  if (rc != 0) {
    const char* msg = stream->get_last_error(stream);
    std::string err = Logger::formatString(
        "[Snowflake Exception] error getting schema: %s, error code: %d", msg ? msg : "unknown", rc);
    logger->error(__FILE__, __func__, __LINE__, err.c_str());
    PyErr_SetString(PyExc_Exception, err.c_str());
    return nullptr;
  }

  return std::unique_ptr<CArrowStreamTableIterator>(
    new CArrowStreamTableIterator(stream, &schema, context, number_to_decimal, force_microsecond_precision));
}

CArrowStreamTableIterator::CArrowStreamTableIterator(ArrowArrayStream* stream, ArrowSchema* schema, PyObject* context, bool number_to_decimal, bool force_microsecond_precision)
    : m_stream(stream, releaseStream),
      m_columnCount(schema->n_children),
      m_context(context) {
  ArrowSchemaMove(schema, m_schema.get());

  char* timezone = nullptr;
  py::UniqueRef tz(PyObject_GetAttrString(m_context, "_timezone"));
  PyArg_Parse(tz.get(), "z", &timezone);

  m_converter = std::unique_ptr<ArrowTableConverter>(
    new ArrowTableConverter(number_to_decimal, force_microsecond_precision, timezone ? std::string(timezone) : std::string()));
}

// ---------------------------------------------------------------------------
// Export accessors
// ---------------------------------------------------------------------------

uintptr_t CArrowStreamTableIterator::getArrowArrayPtr() {
  return reinterpret_cast<uintptr_t>(m_exportArray.get());
}

uintptr_t CArrowStreamTableIterator::getArrowSchemaPtr() {
  return reinterpret_cast<uintptr_t>(m_exportSchema.get());
}

uintptr_t CArrowStreamTableIterator::getConvertedSchemaPtr() {
  m_streamSchemaExport.reset();
  int rc = ArrowSchemaDeepCopy(m_schema.get(), m_streamSchemaExport.get());
  if (rc != NANOARROW_OK) {
    std::string err = Logger::formatString(
        "[Snowflake Exception] error deep-copying stream schema, error code: %d", rc);
    logger->error(__FILE__, __func__, __LINE__, err.c_str());
    PyErr_SetString(PyExc_RuntimeError, err.c_str());
    return 0;
  }

  ArrowError error;

  nanoarrow::UniqueArray emptyArray;
  rc = ArrowArrayInitFromSchema(emptyArray.get(), m_schema.get(), &error);
  if (rc != NANOARROW_OK) {
    std::string err = Logger::formatString(
        "[Snowflake Exception] error initializing empty array for schema conversion: %s, error code: %d",
        ArrowErrorMessage(&error), rc);
    logger->error(__FILE__, __func__, __LINE__, err.c_str());
    PyErr_SetString(PyExc_RuntimeError, err.c_str());
    return 0;
  }
  rc = ArrowArrayFinishBuildingDefault(emptyArray.get(), &error);
  if (rc != NANOARROW_OK) {
    std::string err = Logger::formatString(
        "[Snowflake Exception] error finishing empty array: %s, error code: %d",
        ArrowErrorMessage(&error), rc);
    logger->error(__FILE__, __func__, __LINE__, err.c_str());
    PyErr_SetString(PyExc_RuntimeError, err.c_str());
    return 0;
  }

  nanoarrow::UniqueArrayView emptyView;
  rc = ArrowArrayViewInitFromSchema(emptyView.get(), m_schema.get(), &error);
  if (rc != NANOARROW_OK) {
    std::string err = Logger::formatString(
        "[Snowflake Exception] error initializing empty array view: %s, error code: %d",
        ArrowErrorMessage(&error), rc);
    logger->error(__FILE__, __func__, __LINE__, err.c_str());
    PyErr_SetString(PyExc_RuntimeError, err.c_str());
    return 0;
  }
  rc = ArrowArrayViewSetArray(emptyView.get(), emptyArray.get(), &error);
  if (rc != NANOARROW_OK) {
    std::string err = Logger::formatString(
        "[Snowflake Exception] error setting empty array view: %s, error code: %d",
        ArrowErrorMessage(&error), rc);
    logger->error(__FILE__, __func__, __LINE__, err.c_str());
    PyErr_SetString(PyExc_RuntimeError, err.c_str());
    return 0;
  }

  for (int64_t col = 0; col < m_columnCount; col++) {
    m_converter->convertIfNeeded(m_streamSchemaExport->children[col], emptyView->children[col]);
    if (py::checkPyError()) return 0;
  }

  return reinterpret_cast<uintptr_t>(m_streamSchemaExport.get());
}

// ---------------------------------------------------------------------------
// Stream loading
// ---------------------------------------------------------------------------

bool CArrowStreamTableIterator::loadNextBatch() {
  if (m_streamExhausted) return false;

  m_currentArray.reset();
  m_currentArrayView.reset();

  int returnCode;
  ArrowArrayStream* stream = m_stream.get();
  {
    Py_BEGIN_ALLOW_THREADS
    returnCode = stream->get_next(stream, m_currentArray.get());
    Py_END_ALLOW_THREADS
  }

  if (returnCode != NANOARROW_OK) {
    const char* msg = stream->get_last_error(stream);
    std::string err = Logger::formatString(
        "[Snowflake Exception] error getting next batch: %s, error code: %d", msg ? msg : "unknown", returnCode);
    logger->error(__FILE__, __func__, __LINE__, err.c_str());
    PyErr_SetString(PyExc_Exception, err.c_str());
    return false;
  }

  if (m_currentArray->release == nullptr) {
    m_streamExhausted = true;
    return false;
  }

  if (m_currentArray->length == 0) {
    return loadNextBatch();
  }

  ArrowError error;
  returnCode = ArrowArrayViewInitFromSchema(m_currentArrayView.get(), m_schema.get(), &error);
  if (returnCode != NANOARROW_OK) {
    std::string err = Logger::formatString(
        "[Snowflake Exception] error initializing ArrowArrayView: %s, error code: %d",
        ArrowErrorMessage(&error), returnCode);
    logger->error(__FILE__, __func__, __LINE__, err.c_str());
    PyErr_SetString(PyExc_Exception, err.c_str());
    return false;
  }

  returnCode = ArrowArrayViewSetArray(m_currentArrayView.get(), m_currentArray.get(), &error);
  if (returnCode != NANOARROW_OK) {
    std::string err = Logger::formatString(
        "[Snowflake Exception] error setting ArrowArrayView: %s, error code: %d",
        ArrowErrorMessage(&error), returnCode);
    logger->error(__FILE__, __func__, __LINE__, err.c_str());
    PyErr_SetString(PyExc_Exception, err.c_str());
    return false;
  }

  return true;
}

// ---------------------------------------------------------------------------
// convertBatch()
// ---------------------------------------------------------------------------

void CArrowStreamTableIterator::convertBatch() {
  m_exportArray.reset();
  m_exportSchema.reset();

  int rc = ArrowSchemaDeepCopy(m_schema.get(), m_exportSchema.get());
  SF_CHECK_ARROW_RC(rc, "[Snowflake Exception] error deep-copying schema, error code: %d", rc);

  for (int64_t col = 0; col < m_columnCount; col++) {
    m_converter->convertIfNeeded(m_exportSchema->children[col], m_currentArrayView->children[col]);
    if (py::checkPyError()) return;
  }

  ArrowArrayMove(m_currentArray.get(), m_exportArray.get());
}

// ---------------------------------------------------------------------------
// next()
// ---------------------------------------------------------------------------

ReturnVal CArrowStreamTableIterator::next() {
  if (!loadNextBatch()) {
    if (py::checkPyError()) {
      PyObject *type, *val, *traceback;
      PyErr_Fetch(&type, &val, &traceback);
      PyErr_Clear();
      m_currentPyException.reset(val);
      Py_XDECREF(type);
      Py_XDECREF(traceback);
      return ReturnVal(nullptr, m_currentPyException.get());
    }
    return ReturnVal(Py_None, nullptr);
  }

  convertBatch();

  if (py::checkPyError()) {
    PyObject *type, *val, *traceback;
    PyErr_Fetch(&type, &val, &traceback);
    PyErr_Clear();
    m_currentPyException.reset(val);
    Py_XDECREF(type);
    Py_XDECREF(traceback);
    return ReturnVal(nullptr, m_currentPyException.get());
  }

  return ReturnVal(Py_True, nullptr);
}

}  // namespace sf
