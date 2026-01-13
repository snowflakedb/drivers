#ifndef PC_ARROWSTREAMITERATOR_HPP
#define PC_ARROWSTREAMITERATOR_HPP

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
 * Arrow stream iterator that reads directly from an ArrowArrayStream pointer.
 * This eliminates the need for PyArrow to read batches - C++ reads directly from the stream.
 */
class CArrowStreamIterator {
 public:
  /**
   * Constructor - takes ArrowArrayStream pointer directly
   * @param stream_ptr Pointer to ArrowArrayStream (as integer from Python)
   * @param context Python context object for conversions
   * @param use_numpy Whether to use numpy types
   * @param use_dict_result Whether to return dicts instead of tuples
   */
  CArrowStreamIterator(int64_t stream_ptr, PyObject* context, PyObject* use_numpy,
                       PyObject* use_dict_result);

  /**
   * Destructor - releases the stream
   */
  virtual ~CArrowStreamIterator();

  /**
   * Get the next row as a Python tuple or dict
   * @return ReturnVal with Python object or nullptr if exhausted
   */
  ReturnVal next();

  /**
   * Check if initialization was successful
   * @return ReturnVal indicating success or error
   */
  ReturnVal checkInitializationStatus();

 protected:
  /**
   * Load the next batch from the stream
   * @return true if a batch was loaded, false if stream exhausted
   */
  bool loadNextBatch();

  /**
   * Initialize column converters for the current batch
   */
  void initColumnConverters();

  /**
   * Create Python tuple object for current row
   */
  void createRowPyObject();

  /**
   * Create Python dict object for current row
   */
  void createDictRowPyObject();

  /** The Arrow stream we're reading from */
  ArrowArrayStream* m_stream;

  /** Whether we own the stream (and should release it) */
  bool m_ownsStream;

  /** Schema read from stream */
  nanoarrow::UniqueSchema m_schema;

  /** Current batch array */
  nanoarrow::UniqueArray m_currentArray;

  /** Current batch array view */
  nanoarrow::UniqueArrayView m_currentArrayView;

  /** Pointer to the latest returned Python row result */
  py::UniqueRef m_latestReturnedRow;

  /** List of column converters for current batch */
  std::vector<std::shared_ptr<sf::IColumnConverter>> m_columnConverters;

  /** Current row index in current batch (0-based) */
  int64_t m_currentRowIndex;

  /** Total number of rows in current batch */
  int64_t m_rowCount;

  /** Number of columns */
  int64_t m_columnCount;

  /** Arrow format convert context for the current session */
  PyObject* m_context;

  /** Whether to use numpy int64/float64/datetime */
  bool m_useNumpy;

  /** Whether to return dicts instead of tuples */
  bool m_useDictResult;

  /** Logger instance */
  static Logger* logger;

  /** Current Python exception if any */
  py::UniqueRef m_currentPyException;

  /** Whether initialization succeeded */
  bool m_initialized;

  /** Whether stream is exhausted */
  bool m_streamExhausted;

  /** Total rows returned so far (for debugging) */
  int64_t m_totalRowsReturned;
};

}  // namespace sf

#endif  // PC_ARROWSTREAMITERATOR_HPP

