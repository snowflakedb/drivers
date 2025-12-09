#ifndef PC_ARROWBATCHITERATOR_HPP
#define PC_ARROWBATCHITERATOR_HPP

#include <memory>
#include <vector>

#include "CArrowIterator.hpp"  // For ReturnVal definition
#include "IColumnConverter.hpp"
#include "Python/Common.hpp"
#include "SnowflakeType.hpp"
#include "logging.hpp"
#include "nanoarrow.h"
#include "nanoarrow.hpp"

namespace sf {

// Forward declaration
std::shared_ptr<sf::IColumnConverter> getConverterFromSchema(ArrowSchema* schema,
                                                             ArrowArrayView* array,
                                                             PyObject* context, bool useNumpy,
                                                             Logger* logger);

/**
 * Arrow batch iterator for converting a single RecordBatch to Python rows.
 * Takes Arrow C Data Interface pointers and converts row-by-row.
 */
class CArrowBatchIterator {
 public:
  /**
   * Constructor - takes Arrow C Array and Schema
   * @param c_array Arrow C Array pointer from PyArrow RecordBatch
   * @param c_schema Arrow C Schema pointer from PyArrow RecordBatch
   * @param context Python context object for conversions
   * @param use_numpy Whether to use numpy types
   * @param check_error_on_every_column Check Python errors after each column
   */
  CArrowBatchIterator(ArrowArray* c_array, ArrowSchema* c_schema, PyObject* context,
                      PyObject* use_numpy, PyObject* check_error_on_every_column);

  /**
   * Destructor
   */
  virtual ~CArrowBatchIterator();

  /**
   * Get the next row as a Python tuple
   * @return ReturnVal with Python tuple or nullptr if exhausted
   */
  ReturnVal next();

  /**
   * Check if initialization was successful
   * @return ReturnVal indicating success or error
   */
  ReturnVal checkInitializationStatus();

  /**
   * Get total number of rows in this batch
   */
  int64_t getRowCount() const { return m_rowCount; }

  /**
   * Get current row index
   */
  int64_t getCurrentRowIndex() const { return m_currentRowIndex; }

 protected:
  /**
   * Create Python tuple object for current row
   */
  virtual void createRowPyObject();

  /** Pointer to the latest returned Python tuple (row) result */
  py::UniqueRef m_latestReturnedRow;

  /** List of column converters */
  std::vector<std::shared_ptr<sf::IColumnConverter>> m_columnConverters;

  /** Arrow schema */
  nanoarrow::UniqueSchema m_schema;

  /** Arrow array */
  nanoarrow::UniqueArray m_array;

  /** Arrow array view for efficient access */
  nanoarrow::UniqueArrayView m_arrayView;

  /** Current row index in batch (0-based) */
  int64_t m_currentRowIndex;

  /** Total number of rows in this batch */
  int64_t m_rowCount;

  /** Number of columns */
  int64_t m_columnCount;

  /** Arrow format convert context for the current session */
  PyObject* m_context;

  /** Whether to use numpy int64/float64/datetime */
  bool m_useNumpy;

  /** Check Python error after each column processing */
  bool m_checkErrorOnEveryColumn;

  /** Logger instance */
  static Logger* logger;

  /** Current Python exception if any */
  py::UniqueRef m_currentPyException;

 private:
  /**
   * Initialize column converters from schema
   */
  void initColumnConverters();
};

/**
 * Dictionary result variant - returns Python dicts instead of tuples
 */
class DictCArrowBatchIterator : public CArrowBatchIterator {
 public:
  DictCArrowBatchIterator(ArrowArray* c_array, ArrowSchema* c_schema, PyObject* context,
                          PyObject* use_numpy);

  ~DictCArrowBatchIterator() = default;

 private:
  void createRowPyObject() override;
};

}  // namespace sf

#endif  // PC_ARROWBATCHITERATOR_HPP

