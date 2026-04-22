#ifndef PC_CARROWSTREAMTABLEITERATOR_HPP
#define PC_CARROWSTREAMTABLEITERATOR_HPP

#include <memory>

#include "ArrowTableConverter.hpp"
#include "CArrowIterator.hpp"
#include "Python/Common.hpp"
#include "logging.hpp"
#include "nanoarrow.h"
#include "nanoarrow.hpp"

namespace sf {

/**
 * Stream-based Arrow iterator that reads batches from an ArrowArrayStream
 * and exposes each batch as a (ArrowArray, ArrowSchema) pair suitable for
 * import into PyArrow via RecordBatch._import_from_c().
 *
 * Snowflake-specific type conversions (e.g. scaled NUMBER -> float64,
 * struct-encoded TIMESTAMP -> Arrow timestamp) are applied to every batch
 * through the shared ArrowTableConverter.
 *
 * Lifecycle (driven from the Cython wrapper ArrowDummyIterator):
 *   1. from_stream()   – validate stream, read schema, construct instance
 *   2. next()          – load next batch, convert, expose via export slots
 *   3. getArrowArrayPtr() / getArrowSchemaPtr()
 *                      – caller passes these to _import_from_c() which
 *                        consumes them (sets release to nullptr)
 *   4. repeat 2-3 until next() signals exhaustion (Py_None)
 */
class CArrowStreamTableIterator {
 public:
  /**
   * Factory method.  Validates the stream pointer, reads the schema, and
   * returns a ready-to-iterate instance.
   *
   * @param stream_ptr  Pointer to ArrowArrayStream as int64 (FFI from Python)
   * @param context     Python ArrowConverterContext carrying session timezone
   * @param number_to_decimal
   *   If true, scaled FIXED columns become Decimal128; otherwise float64
   * @param force_microsecond_precision
   *   If true, all TIMESTAMP columns are forced to microsecond precision,
   *   ensuring consistent schema across batches whose data may span
   *   outside the nanosecond-representable range (1677-2262)
   * @return Unique pointer to the iterator, or nullptr with a Python
   *         exception set on failure
   */
  static std::unique_ptr<CArrowStreamTableIterator> from_stream(
      int64_t stream_ptr, PyObject* context, bool number_to_decimal,
      bool force_microsecond_precision);

  /**
   * Advance to the next batch.
   *
   * @return ReturnVal with:
   *   - successObj = Py_True, exception = nullptr   → batch ready; call
   *     getArrowArrayPtr() / getArrowSchemaPtr() to retrieve it
   *   - successObj = Py_None, exception = nullptr   → stream exhausted
   *   - successObj = nullptr, exception != nullptr   → error occurred
   */
  ReturnVal next();

  /**
   * Raw pointer (as uintptr_t) to the converted ArrowArray for the
   * current batch.  Valid only immediately after next() returns Py_True.
   * Ownership transfers to the caller via _import_from_c().
   */
  uintptr_t getArrowArrayPtr();

  /**
   * Raw pointer (as uintptr_t) to the deep-copied ArrowSchema for the
   * current batch.  Same ownership semantics as getArrowArrayPtr().
   */
  uintptr_t getArrowSchemaPtr();

  /**
   * Raw pointer (as uintptr_t) to the stream schema after applying
   * Snowflake type conversions (e.g. struct → timestamp, int → float64).
   * Unlike getArrowSchemaPtr(), this is always valid—even when no batch
   * has been loaded—making it safe to call for empty result sets
   * (e.g. to build an empty PyArrow table with the correct column types).
   *
   * Internally synthesises a zero-row batch and runs it through
   * ArrowTableConverter so the schema matches what non-empty results
   * would produce.
   *
   * Ownership transfers to the caller via _import_from_c().
   */
  uintptr_t getConvertedSchemaPtr();

 private:
  CArrowStreamTableIterator(ArrowArrayStream* stream, ArrowSchema* schema,
                      PyObject* context, bool number_to_decimal,
                      bool force_microsecond_precision);

  /**
   * Pull the next non-empty batch from the stream into m_currentArray.
   * Releases the GIL during stream->get_next() so concurrent chunk
   * downloads (e.g. from S3) are not blocked by the Python interpreter.
   * Also initialises m_currentArrayView for the conversion step.
   *
   * @return true if a batch was loaded; false if exhausted or error
   */
  bool loadNextBatch();

  /**
   * Apply Snowflake type conversions to the current batch via
   * m_converter, then move the result into the export slots
   * (m_exportArray / m_exportSchema).
   * Sets a Python exception on failure (check py::checkPyError()).
   */
  void convertBatch();

  /** Owned handle to the ArrowArrayStream; released on destruction. */
  std::unique_ptr<ArrowArrayStream, void (*)(ArrowArrayStream*)> m_stream;

  /** Stream schema, read once at construction; immutable across batches. */
  nanoarrow::UniqueSchema m_schema;

  /** Raw batch from the most recent stream->get_next() call. */
  nanoarrow::UniqueArray m_currentArray;

  /** View over m_currentArray; rebuilt each loadNextBatch() for the
   *  converter to read column data through. */
  nanoarrow::UniqueArrayView m_currentArrayView;

  /** Converted batch ready for PyArrow import.  Populated by
   *  convertBatch(); consumed by _import_from_c() via getArrowArrayPtr(). */
  nanoarrow::UniqueArray m_exportArray;

  /** Deep-copied and possibly rewritten schema for the current batch.
   *  Each batch gets its own copy because conversion may alter child
   *  schemas (e.g. struct → timestamp).  Consumed via getArrowSchemaPtr(). */
  nanoarrow::UniqueSchema m_exportSchema;

  /** Deep copy of m_schema used by getStreamSchemaPtr().  Separate from
   *  m_exportSchema so it can be produced independently of batch iteration. */
  nanoarrow::UniqueSchema m_streamSchemaExport;

  /** Set to true once stream->get_next() returns an empty release sentinel. */
  bool m_streamExhausted = false;

  /** Number of top-level columns, cached from schema.n_children. */
  int64_t m_columnCount = 0;

  /** Python ArrowConverterContext; borrowed reference kept alive by the
   *  Cython wrapper which stores it as `self.arrow_context`. */
  PyObject* m_context;

  /** Applies Snowflake-specific type conversions per column. */
  std::unique_ptr<ArrowTableConverter> m_converter;

  /** Holds the most recent Python exception so the pointer remains valid
   *  until the next call into the iterator. */
  py::UniqueRef m_currentPyException;

  static Logger* logger;
};

}  // namespace sf

#endif  // PC_CARROWSTREAMTABLEITERATOR_HPP
