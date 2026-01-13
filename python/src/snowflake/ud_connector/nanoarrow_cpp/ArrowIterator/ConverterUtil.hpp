#ifndef PC_CONVERTERUTIL_HPP
#define PC_CONVERTERUTIL_HPP

#include <memory>

#include "IColumnConverter.hpp"
#include "logging.hpp"
#include "nanoarrow.h"

namespace sf {

/**
 * Factory function to create appropriate column converter based on Arrow schema.
 *
 * @param schema Arrow schema for the column
 * @param array Arrow array view for the column data
 * @param context Python context object for conversions
 * @param useNumpy Whether to use numpy types for numeric data
 * @param logger Logger instance for error reporting
 * @return Shared pointer to column converter, or nullptr on error
 */
std::shared_ptr<sf::IColumnConverter> getConverterFromSchema(ArrowSchema* schema,
                                                             ArrowArrayView* array,
                                                             PyObject* context, bool useNumpy,
                                                             Logger* logger);

}  // namespace sf

#endif  // PC_CONVERTERUTIL_HPP

