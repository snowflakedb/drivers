#ifndef PC_ARROWTABLECONVERTER_HPP
#define PC_ARROWTABLECONVERTER_HPP

#include <string>

#include "CArrowIterator.hpp"
#include "logging.hpp"
#include "nanoarrow.h"
#include "nanoarrow.hpp"

namespace sf {

/**
 * Stateless-ish helper that applies Snowflake-specific type conversions to
 * Arrow columns in-place.  Both CArrowTableIterator (IPC-based) and
 * CArrowStreamTableIterator (stream-based) delegate all per-column
 * conversion to this class so the logic lives in a single place.
 *
 * Construction captures the three session-level knobs that affect
 * conversion behaviour; individual columns are converted via
 * convertIfNeeded().
 */
class ArrowTableConverter {
 public:
    ArrowTableConverter(bool number_to_decimal, bool force_microsecond_precision, const std::string& timezone);

  /**
   * Inspect the Snowflake logical-type metadata on @p columnSchema and
   * convert @p columnArray in-place when necessary.
   *
   * Sets a Python exception on failure (caller should check
   * py::checkPyError() after the call).
   */
  void convertIfNeeded(ArrowSchema* columnSchema, ArrowArrayView* columnArray);

 private:
  void convertScaledFixedNumberColumn_nanoarrow(ArrowSchemaView* field, ArrowArrayView* columnArray, unsigned int scale);

  void convertScaledFixedNumberColumnToDecimalColumn_nanoarrow(ArrowSchemaView* field, ArrowArrayView* columnArray, unsigned int scale);

  void convertScaledFixedNumberColumnToDoubleColumn_nanoarrow(ArrowSchemaView* field, ArrowArrayView* columnArray, unsigned int scale);

  void convertTimeColumn_nanoarrow(ArrowSchemaView* field, ArrowArrayView* columnArray, int scale);

  void convertIntervalDayTimeColumn_nanoarrow(ArrowSchemaView* field, ArrowArrayView* columnArray, int scale);

  void convertTimestampColumn_nanoarrow(ArrowSchemaView* field, ArrowArrayView* columnArray, int scale, const std::string& timezone = "");

  void convertTimestampTZColumn_nanoarrow(ArrowSchemaView* field, ArrowArrayView* columnArray, int scale, int byteLength, const std::string& timezone);

  template <typename T>
  double convertScaledFixedNumberToDouble(unsigned int scale, T originalValue);

  const bool m_convert_number_to_decimal;
  const bool m_force_microsecond_precision;
  const std::string m_timezone;

  static Logger* logger;
};

}  // namespace sf

#endif  // PC_ARROWTABLECONVERTER_HPP
