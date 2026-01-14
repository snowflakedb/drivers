#include "ConverterUtil.hpp"

#include <memory>
#include <string>

#include "ArrayConverter.hpp"
#include "BinaryConverter.hpp"
#include "BooleanConverter.hpp"
#include "CArrowIterator.hpp"
#include "DateConverter.hpp"
#include "DecFloatConverter.hpp"
#include "DecimalConverter.hpp"
#include "FixedSizeListConverter.hpp"
#include "FloatConverter.hpp"
#include "IntConverter.hpp"
#include "IntervalConverter.hpp"
#include "MapConverter.hpp"
#include "ObjectConverter.hpp"
#include "SnowflakeType.hpp"
#include "StringConverter.hpp"
#include "TimeConverter.hpp"
#include "TimeStampConverter.hpp"
#include "nanoarrow.hpp"

namespace sf {

std::shared_ptr<sf::IColumnConverter> getConverterFromSchema(ArrowSchema* schema,
                                                             ArrowArrayView* array,
                                                             PyObject* context, bool useNumpy,
                                                             Logger* logger) {
  std::shared_ptr<sf::IColumnConverter> converter = nullptr;
  ArrowSchemaView schemaView;
  ArrowError error;
  int returnCode = 0;

  returnCode = ArrowSchemaViewInit(&schemaView, schema, &error);
  SF_CHECK_ARROW_RC_AND_RETURN(returnCode, nullptr,
                               "[Snowflake Exception] error initializing "
                               "ArrowSchemaView: %s, error code: %d",
                               ArrowErrorMessage(&error), returnCode);

  struct ArrowStringView snowflakeLogicalType = ArrowCharView(nullptr);
  const char* metadata = schema->metadata;
  returnCode = ArrowMetadataGetValue(metadata, ArrowCharView("logicalType"), &snowflakeLogicalType);

  SF_CHECK_ARROW_RC_AND_RETURN(returnCode, nullptr,
                               "[Snowflake Exception] error getting 'logicalType' from "
                               "Arrow metadata, error code: %d",
                               returnCode);

  SnowflakeType::Type sfType = SnowflakeType::snowflakeTypeFromString(
      std::string(snowflakeLogicalType.data, snowflakeLogicalType.size_bytes));

  switch (sfType) {
    case SnowflakeType::Type::FIXED: {
      struct ArrowStringView scaleString = ArrowCharView(nullptr);
      struct ArrowStringView precisionString = ArrowCharView(nullptr);
      int scale = 0;
      int precision = 38;
      if (metadata != nullptr) {
        returnCode = ArrowMetadataGetValue(metadata, ArrowCharView("scale"), &scaleString);
        SF_CHECK_ARROW_RC_AND_RETURN(returnCode, nullptr,
                                     "[Snowflake Exception] error getting 'scale' from "
                                     "Arrow metadata, error code: %d",
                                     returnCode);
        returnCode = ArrowMetadataGetValue(metadata, ArrowCharView("precision"), &precisionString);
        SF_CHECK_ARROW_RC_AND_RETURN(returnCode, nullptr,
                                     "[Snowflake Exception] error getting 'precision' "
                                     "from Arrow metadata, error code: %d",
                                     returnCode);
        scale = std::stoi(std::string(scaleString.data, scaleString.size_bytes));
        precision = std::stoi(std::string(precisionString.data, precisionString.size_bytes));
      }

      switch (schemaView.type) {
#define _SF_INIT_FIXED_CONVERTER(ARROW_TYPE)                                                       \
  case ArrowType::ARROW_TYPE: {                                                                    \
    if (scale > 0) {                                                                               \
      if (useNumpy) {                                                                              \
        converter = std::make_shared<sf::NumpyDecimalConverter>(array, precision, scale, context); \
      } else {                                                                                     \
        converter = std::make_shared<sf::DecimalFromIntConverter>(array, precision, scale);        \
      }                                                                                            \
    } else {                                                                                       \
      if (useNumpy) {                                                                              \
        converter = std::make_shared<sf::NumpyIntConverter>(array, context);                       \
      } else {                                                                                     \
        converter = std::make_shared<sf::IntConverter>(array);                                     \
      }                                                                                            \
    }                                                                                              \
    break;                                                                                         \
  }
        _SF_INIT_FIXED_CONVERTER(NANOARROW_TYPE_INT8)
        _SF_INIT_FIXED_CONVERTER(NANOARROW_TYPE_INT16)
        _SF_INIT_FIXED_CONVERTER(NANOARROW_TYPE_INT32)
        _SF_INIT_FIXED_CONVERTER(NANOARROW_TYPE_INT64)
#undef _SF_INIT_FIXED_CONVERTER

        case ArrowType::NANOARROW_TYPE_DECIMAL128: {
          converter = std::make_shared<sf::DecimalFromDecimalConverter>(context, array, scale);
          break;
        }

        default: {
          std::string errorInfo = Logger::formatString(
              "[Snowflake Exception] unknown arrow internal data type(%d) "
              "for FIXED data",
              NANOARROW_TYPE_ENUM_STRING[schemaView.type]);
          logger->error(__FILE__, __func__, __LINE__, errorInfo.c_str());
          PyErr_SetString(PyExc_Exception, errorInfo.c_str());
          break;
        }
      }
      break;
    }

    case SnowflakeType::Type::ANY:
    case SnowflakeType::Type::CHAR:
    case SnowflakeType::Type::TEXT:
    case SnowflakeType::Type::VARIANT: {
      converter = std::make_shared<sf::StringConverter>(array);
      break;
    }

    case SnowflakeType::Type::BOOLEAN: {
      converter = std::make_shared<sf::BooleanConverter>(array);
      break;
    }

    case SnowflakeType::Type::REAL: {
      if (useNumpy) {
        converter = std::make_shared<sf::NumpyFloat64Converter>(array, context);
      } else {
        converter = std::make_shared<sf::FloatConverter>(array);
      }
      break;
    }

    case SnowflakeType::Type::DATE: {
      if (useNumpy) {
        converter = std::make_shared<sf::NumpyDateConverter>(array, context);
      } else {
        converter = std::make_shared<sf::DateConverter>(array);
      }
      break;
    }

    case SnowflakeType::Type::BINARY: {
      converter = std::make_shared<sf::BinaryConverter>(array);
      break;
    }

    case SnowflakeType::Type::TIME: {
      int scale = 9;
      if (metadata != nullptr) {
        struct ArrowStringView scaleString = ArrowCharView(nullptr);
        returnCode = ArrowMetadataGetValue(metadata, ArrowCharView("scale"), &scaleString);
        SF_CHECK_ARROW_RC_AND_RETURN(returnCode, nullptr,
                                     "[Snowflake Exception] error getting 'scale' from "
                                     "Arrow metadata, error code: %d",
                                     returnCode);
        scale = std::stoi(std::string(scaleString.data, scaleString.size_bytes));
      }
      switch (schemaView.type) {
        case NANOARROW_TYPE_INT32:
        case NANOARROW_TYPE_INT64: {
          converter = std::make_shared<sf::TimeConverter>(array, scale);
          break;
        }

        default: {
          std::string errorInfo = Logger::formatString(
              "[Snowflake Exception] unknown arrow internal data type(%d) "
              "for TIME data",
              NANOARROW_TYPE_ENUM_STRING[schemaView.type]);
          logger->error(__FILE__, __func__, __LINE__, errorInfo.c_str());
          PyErr_SetString(PyExc_Exception, errorInfo.c_str());
          return nullptr;
        }
      }
      break;
    }

    case SnowflakeType::Type::TIMESTAMP_NTZ: {
      int scale = 9;
      if (metadata != nullptr) {
        struct ArrowStringView scaleString = ArrowCharView(nullptr);
        returnCode = ArrowMetadataGetValue(metadata, ArrowCharView("scale"), &scaleString);
        SF_CHECK_ARROW_RC_AND_RETURN(returnCode, nullptr,
                                     "[Snowflake Exception] error getting 'scale' from "
                                     "Arrow metadata, error code: %d",
                                     returnCode);
        scale = std::stoi(std::string(scaleString.data, scaleString.size_bytes));
      }
      switch (schemaView.type) {
        case NANOARROW_TYPE_INT64: {
          if (useNumpy) {
            converter =
                std::make_shared<sf::NumpyOneFieldTimeStampNTZConverter>(array, scale, context);
          } else {
            converter = std::make_shared<sf::OneFieldTimeStampNTZConverter>(array, scale, context);
          }
          break;
        }

        case NANOARROW_TYPE_STRUCT: {
          if (useNumpy) {
            converter = std::make_shared<sf::NumpyTwoFieldTimeStampNTZConverter>(array, &schemaView,
                                                                                 scale, context);
          } else {
            converter = std::make_shared<sf::TwoFieldTimeStampNTZConverter>(array, &schemaView,
                                                                            scale, context);
          }
          break;
        }

        default: {
          std::string errorInfo = Logger::formatString(
              "[Snowflake Exception] unknown arrow internal data type(%d) "
              "for TIMESTAMP_NTZ data",
              NANOARROW_TYPE_ENUM_STRING[schemaView.type]);
          logger->error(__FILE__, __func__, __LINE__, errorInfo.c_str());
          PyErr_SetString(PyExc_Exception, errorInfo.c_str());
          break;
        }
      }
      break;
    }

    case SnowflakeType::Type::TIMESTAMP_LTZ: {
      int scale = 9;
      if (metadata != nullptr) {
        struct ArrowStringView scaleString = ArrowCharView(nullptr);
        returnCode = ArrowMetadataGetValue(metadata, ArrowCharView("scale"), &scaleString);
        SF_CHECK_ARROW_RC_AND_RETURN(returnCode, nullptr,
                                     "[Snowflake Exception] error getting 'scale' from "
                                     "Arrow metadata, error code: %d",
                                     returnCode);
        scale = std::stoi(std::string(scaleString.data, scaleString.size_bytes));
      }
      switch (schemaView.type) {
        case NANOARROW_TYPE_INT64: {
          converter = std::make_shared<sf::OneFieldTimeStampLTZConverter>(array, scale, context);
          break;
        }

        case NANOARROW_TYPE_STRUCT: {
          converter = std::make_shared<sf::TwoFieldTimeStampLTZConverter>(array, &schemaView, scale,
                                                                          context);
          break;
        }

        default: {
          std::string errorInfo = Logger::formatString(
              "[Snowflake Exception] unknown arrow internal data type(%d) "
              "for TIMESTAMP_LTZ data",
              NANOARROW_TYPE_ENUM_STRING[schemaView.type]);
          logger->error(__FILE__, __func__, __LINE__, errorInfo.c_str());
          PyErr_SetString(PyExc_Exception, errorInfo.c_str());
          break;
        }
      }
      break;
    }

    case SnowflakeType::Type::TIMESTAMP_TZ: {
      struct ArrowStringView scaleString = ArrowCharView(nullptr);
      struct ArrowStringView byteLengthString = ArrowCharView(nullptr);
      int scale = 9;
      int byteLength = 16;
      if (metadata != nullptr) {
        returnCode = ArrowMetadataGetValue(metadata, ArrowCharView("scale"), &scaleString);
        SF_CHECK_ARROW_RC_AND_RETURN(returnCode, nullptr,
                                     "[Snowflake Exception] error getting 'scale' from "
                                     "Arrow metadata, error code: %d",
                                     returnCode);
        returnCode =
            ArrowMetadataGetValue(metadata, ArrowCharView("byteLength"), &byteLengthString);
        SF_CHECK_ARROW_RC_AND_RETURN(returnCode, nullptr,
                                     "[Snowflake Exception] error getting 'byteLength' "
                                     "from Arrow metadata, error code: %d",
                                     returnCode);
        scale = std::stoi(std::string(scaleString.data, scaleString.size_bytes));

        // Byte Length may be unset if TIMESTAMP_TZ is the child of a structured
        // type. In this case rely on the default value.
        if (byteLengthString.data != nullptr) {
          byteLength = std::stoi(std::string(byteLengthString.data, byteLengthString.size_bytes));
        }
      }
      switch (byteLength) {
        case 8: {
          converter = std::make_shared<sf::TwoFieldTimeStampTZConverter>(array, &schemaView, scale,
                                                                         context);
          break;
        }

        case 16: {
          converter = std::make_shared<sf::ThreeFieldTimeStampTZConverter>(array, &schemaView,
                                                                           scale, context);
          break;
        }

        default: {
          std::string errorInfo = Logger::formatString(
              "[Snowflake Exception] unknown arrow internal data type(%d) "
              "for TIMESTAMP_TZ data",
              NANOARROW_TYPE_ENUM_STRING[schemaView.type]);
          logger->error(__FILE__, __func__, __LINE__, errorInfo.c_str());
          PyErr_SetString(PyExc_Exception, errorInfo.c_str());
          break;
        }
      }

      break;
    }

    case SnowflakeType::Type::ARRAY: {
      switch (schemaView.type) {
        case NANOARROW_TYPE_STRING:
          converter = std::make_shared<sf::StringConverter>(array);
          break;
        case NANOARROW_TYPE_LIST:
          converter = std::make_shared<sf::ArrayConverter>(&schemaView, array, context, useNumpy);
          break;
        default: {
          std::string errorInfo = Logger::formatString(
              "[Snowflake Exception] unknown arrow internal data type(%d) "
              "for ARRAY data in %s",
              NANOARROW_TYPE_ENUM_STRING[schemaView.type], schemaView.schema->name);
          logger->error(__FILE__, __func__, __LINE__, errorInfo.c_str());
          PyErr_SetString(PyExc_Exception, errorInfo.c_str());
          break;
        }
      }
      break;
    }

    case SnowflakeType::Type::MAP: {
      converter = std::make_shared<sf::MapConverter>(&schemaView, array, context, useNumpy);
      break;
    }

    case SnowflakeType::Type::OBJECT: {
      switch (schemaView.type) {
        case NANOARROW_TYPE_STRING:
          converter = std::make_shared<sf::StringConverter>(array);
          break;
        case NANOARROW_TYPE_STRUCT:
          converter = std::make_shared<sf::ObjectConverter>(&schemaView, array, context, useNumpy);
          break;
        default: {
          std::string errorInfo = Logger::formatString(
              "[Snowflake Exception] unknown arrow internal data type(%d) "
              "for OBJECT data in %s",
              NANOARROW_TYPE_ENUM_STRING[schemaView.type], schemaView.schema->name);
          logger->error(__FILE__, __func__, __LINE__, errorInfo.c_str());
          PyErr_SetString(PyExc_Exception, errorInfo.c_str());
          break;
        }
      }
      break;
    }

    case SnowflakeType::Type::VECTOR: {
      converter = std::make_shared<sf::FixedSizeListConverter>(array);
      break;
    }

    case SnowflakeType::Type::DECFLOAT: {
      converter = std::make_shared<sf::DecFloatConverter>(*array, schemaView, *context, useNumpy);
      break;
    }

    case SnowflakeType::Type::INTERVAL_YEAR_MONTH: {
      converter = std::make_shared<sf::IntervalYearMonthConverter>(array, context, useNumpy);
      break;
    }

    case SnowflakeType::Type::INTERVAL_DAY_TIME: {
      switch (schemaView.type) {
        case NANOARROW_TYPE_INT64:
          converter = std::make_shared<sf::IntervalDayTimeConverterInt>(array, context, useNumpy);
          break;
        case NANOARROW_TYPE_DECIMAL128:
          converter =
              std::make_shared<sf::IntervalDayTimeConverterDecimal>(array, context, useNumpy);
          break;
        default: {
          std::string errorInfo = Logger::formatString(
              "[Snowflake Exception] unknown arrow internal data type(%d) "
              "for INTERVAL_DAY_TIME data in %s",
              NANOARROW_TYPE_ENUM_STRING[schemaView.type], schemaView.schema->name);
          logger->error(__FILE__, __func__, __LINE__, errorInfo.c_str());
          PyErr_SetString(PyExc_Exception, errorInfo.c_str());
          break;
        }
      }
      break;
    }

    default: {
      std::string errorInfo =
          Logger::formatString("[Snowflake Exception] unknown snowflake data type : %d", sfType);
      logger->error(__FILE__, __func__, __LINE__, errorInfo.c_str());
      PyErr_SetString(PyExc_Exception, errorInfo.c_str());
      break;
    }
  }
  return converter;
}

}  // namespace sf

