#include "ArrowTableConverter.hpp"

#include <cstring>
#include <string>

#include "Python/Common.hpp"
#include "SnowflakeType.hpp"
#include "Util/time.hpp"

namespace sf {

Logger* ArrowTableConverter::logger = new Logger("snowflake.connector.ArrowTableConverter");

ArrowTableConverter::ArrowTableConverter(bool number_to_decimal, bool force_microsecond_precision, const std::string& timezone)
    : m_convert_number_to_decimal(number_to_decimal),
      m_force_microsecond_precision(force_microsecond_precision),
      m_timezone(timezone) {}

// ---------------------------------------------------------------------------
// convertIfNeeded – top-level dispatch on Snowflake logical type
// ---------------------------------------------------------------------------

void ArrowTableConverter::convertIfNeeded(ArrowSchema* columnSchema,
                                          ArrowArrayView* columnArray) {
  ArrowSchemaView columnSchemaView;
  ArrowError error;
  int returnCode;

  returnCode = ArrowSchemaViewInit(&columnSchemaView, columnSchema, &error);
  SF_CHECK_ARROW_RC(returnCode,
                    "[Snowflake Exception] error initializing "
                    "ArrowSchemaView: %s, error code: %d",
                    ArrowErrorMessage(&error), returnCode);

  ArrowStringView snowflakeLogicalType;
  const char* metadata = columnSchema->metadata;
  returnCode = ArrowMetadataGetValue(metadata, ArrowCharView("logicalType"),
                                     &snowflakeLogicalType);
  SF_CHECK_ARROW_RC(returnCode,
                    "[Snowflake Exception] error getting 'logicalType' "
                    "from Arrow metadata, error code: %d",
                    returnCode);

  SnowflakeType::Type st = SnowflakeType::snowflakeTypeFromString(
      std::string(snowflakeLogicalType.data, snowflakeLogicalType.size_bytes));

  switch (st) {
    case SnowflakeType::Type::FIXED: {
      int scale = 0;
      ArrowStringView scaleString = ArrowCharView(nullptr);
      if (metadata != nullptr) {
        returnCode = ArrowMetadataGetValue(metadata, ArrowCharView("scale"),
                                           &scaleString);
        SF_CHECK_ARROW_RC(returnCode,
                          "[Snowflake Exception] error getting 'scale' "
                          "from Arrow metadata, error code: %d",
                          returnCode);
        scale = std::stoi(
            std::string(scaleString.data, scaleString.size_bytes));
      }
      if (scale > 0 &&
          columnSchemaView.type != NANOARROW_TYPE_DECIMAL128) {
        convertScaledFixedNumberColumn_nanoarrow(&columnSchemaView,
                                                 columnArray, scale);
      }
      break;
    }

    case SnowflakeType::Type::ANY:
    case SnowflakeType::Type::BINARY:
    case SnowflakeType::Type::BOOLEAN:
    case SnowflakeType::Type::CHAR:
    case SnowflakeType::Type::DATE:
    case SnowflakeType::Type::REAL:
    case SnowflakeType::Type::TEXT:
    case SnowflakeType::Type::INTERVAL_YEAR_MONTH:
    case SnowflakeType::Type::VARIANT:
    case SnowflakeType::Type::VECTOR:
      break;

    case SnowflakeType::Type::ARRAY: {
      switch (columnSchemaView.type) {
        case NANOARROW_TYPE_STRING:
          break;
        case NANOARROW_TYPE_LIST: {
          if (columnSchemaView.schema->n_children != 1) {
            PyErr_SetString(
                PyExc_Exception,
                Logger::formatString(
                    "[Snowflake Exception] invalid arrow schema for array "
                    "items: expected 1 child, got %d",
                    columnSchemaView.schema->n_children).c_str());
            break;
          }
          convertIfNeeded(columnSchemaView.schema->children[0],
                          columnArray->children[0]);
          break;
        }
        default:
          PyErr_SetString(
              PyExc_Exception,
              Logger::formatString(
                  "[Snowflake Exception] unknown arrow type(%s) "
                  "for ARRAY data in %s",
                  NANOARROW_TYPE_ENUM_STRING[columnSchemaView.type],
                  columnSchemaView.schema->name).c_str());
      }
      break;
    }

    case SnowflakeType::Type::MAP: {
      if (columnSchemaView.schema->n_children != 1) {
        PyErr_SetString(
            PyExc_Exception,
            Logger::formatString(
                "[Snowflake Exception] invalid arrow schema for map "
                "entries: expected 1 child, got %d",
                columnSchemaView.schema->n_children).c_str());
        break;
      }
      ArrowSchema* entries = columnSchemaView.schema->children[0];
      if (entries->n_children != 2) {
        PyErr_SetString(
            PyExc_Exception,
            Logger::formatString(
                "[Snowflake Exception] invalid arrow schema for map "
                "key/value: expected 2 entries, got %d",
                entries->n_children).c_str());
        break;
      }
      convertIfNeeded(entries->children[0],
                      columnArray->children[0]->children[0]);
      convertIfNeeded(entries->children[1],
                      columnArray->children[0]->children[1]);
      break;
    }

    case SnowflakeType::Type::OBJECT: {
      switch (columnSchemaView.type) {
        case NANOARROW_TYPE_STRING:
          break;
        case NANOARROW_TYPE_STRUCT:
          for (int i = 0; i < columnSchemaView.schema->n_children; i++) {
            convertIfNeeded(columnSchemaView.schema->children[i],
                            columnArray->children[i]);
          }
          break;
        default:
          PyErr_SetString(
              PyExc_Exception,
              Logger::formatString(
                  "[Snowflake Exception] unknown arrow type(%s) "
                  "for OBJECT data in %s",
                  NANOARROW_TYPE_ENUM_STRING[columnSchemaView.type],
                  columnSchemaView.schema->name).c_str());
      }
      break;
    }

    case SnowflakeType::Type::INTERVAL_DAY_TIME: {
      int scale = 9;
      if (metadata != nullptr) {
        ArrowStringView scaleString = ArrowCharView(nullptr);
        returnCode = ArrowMetadataGetValue(metadata, ArrowCharView("scale"),
                                           &scaleString);
        SF_CHECK_ARROW_RC(returnCode,
                          "[Snowflake Exception] error getting 'scale', "
                          "error code: %d",
                          returnCode);
        scale = std::stoi(
            std::string(scaleString.data, scaleString.size_bytes));
      }
      convertIntervalDayTimeColumn_nanoarrow(&columnSchemaView, columnArray,
                                             scale);
      break;
    }

    case SnowflakeType::Type::TIME: {
      int scale = 9;
      if (metadata != nullptr) {
        ArrowStringView scaleString = ArrowCharView(nullptr);
        returnCode = ArrowMetadataGetValue(metadata, ArrowCharView("scale"),
                                           &scaleString);
        SF_CHECK_ARROW_RC(returnCode,
                          "[Snowflake Exception] error getting 'scale', "
                          "error code: %d",
                          returnCode);
        scale = std::stoi(
            std::string(scaleString.data, scaleString.size_bytes));
      }
      convertTimeColumn_nanoarrow(&columnSchemaView, columnArray, scale);
      break;
    }

    case SnowflakeType::Type::TIMESTAMP_NTZ: {
      int scale = 9;
      if (metadata != nullptr) {
        ArrowStringView scaleString = ArrowCharView(nullptr);
        returnCode = ArrowMetadataGetValue(metadata, ArrowCharView("scale"),
                                           &scaleString);
        SF_CHECK_ARROW_RC(returnCode,
                          "[Snowflake Exception] error getting 'scale', "
                          "error code: %d",
                          returnCode);
        scale = std::stoi(
            std::string(scaleString.data, scaleString.size_bytes));
      }
      convertTimestampColumn_nanoarrow(&columnSchemaView, columnArray, scale);
      break;
    }

    case SnowflakeType::Type::TIMESTAMP_LTZ: {
      int scale = 9;
      if (metadata != nullptr) {
        ArrowStringView scaleString = ArrowCharView(nullptr);
        returnCode = ArrowMetadataGetValue(metadata, ArrowCharView("scale"),
                                           &scaleString);
        SF_CHECK_ARROW_RC(returnCode,
                          "[Snowflake Exception] error getting 'scale', "
                          "error code: %d",
                          returnCode);
        scale = std::stoi(
            std::string(scaleString.data, scaleString.size_bytes));
      }
      convertTimestampColumn_nanoarrow(&columnSchemaView, columnArray, scale,
                                       m_timezone);
      break;
    }

    case SnowflakeType::Type::TIMESTAMP_TZ: {
      int scale = 9;
      int byteLength = 16;
      if (metadata != nullptr) {
        ArrowStringView scaleString = ArrowCharView(nullptr);
        ArrowStringView byteLengthString = ArrowCharView(nullptr);
        returnCode = ArrowMetadataGetValue(metadata, ArrowCharView("scale"),
                                           &scaleString);
        SF_CHECK_ARROW_RC(returnCode,
                          "[Snowflake Exception] error getting 'scale', "
                          "error code: %d",
                          returnCode);
        returnCode = ArrowMetadataGetValue(metadata,
                                           ArrowCharView("byteLength"),
                                           &byteLengthString);
        SF_CHECK_ARROW_RC(returnCode,
                          "[Snowflake Exception] error getting 'byteLength', "
                          "error code: %d",
                          returnCode);
        scale = std::stoi(
            std::string(scaleString.data, scaleString.size_bytes));
        if (byteLengthString.data != nullptr) {
          byteLength = std::stoi(
              std::string(byteLengthString.data,
                          byteLengthString.size_bytes));
        }
      }
      convertTimestampTZColumn_nanoarrow(&columnSchemaView, columnArray,
                                         scale, byteLength, m_timezone);
      break;
    }

    default:
      PyErr_SetString(
          PyExc_Exception,
          Logger::formatString(
              "[Snowflake Exception] unknown Snowflake type: %s",
              snowflakeLogicalType.data).c_str());
  }
}

// ---------------------------------------------------------------------------
// Scaled fixed-number conversion
// ---------------------------------------------------------------------------

template <typename T>
double ArrowTableConverter::convertScaledFixedNumberToDouble(
    unsigned int scale, T originalValue) {
  if (scale < 9) {
    return static_cast<double>(originalValue) /
           sf::internal::powTenSB4[scale];
  }
  std::string valStr = std::to_string(originalValue);
  int negative = valStr.at(0) == '-' ? 1 : 0;
  unsigned int digits = valStr.length() - negative;
  if (digits <= scale) {
    int numZeroes = scale - digits + 1;
    valStr.insert(negative, std::string(numZeroes, '0'));
  }
  valStr.insert(valStr.length() - scale, ".");
  std::size_t offset = 0;
  return std::stod(valStr, &offset);
}

void ArrowTableConverter::convertScaledFixedNumberColumn_nanoarrow(
    ArrowSchemaView* field, ArrowArrayView* columnArray,
    unsigned int scale) {
  if (m_convert_number_to_decimal) {
    convertScaledFixedNumberColumnToDecimalColumn_nanoarrow(field, columnArray,
                                                           scale);
  } else {
    convertScaledFixedNumberColumnToDoubleColumn_nanoarrow(field, columnArray,
                                                          scale);
  }
}

void ArrowTableConverter::convertScaledFixedNumberColumnToDecimalColumn_nanoarrow(
    ArrowSchemaView* field, ArrowArrayView* columnArray,
    unsigned int scale) {
  int returnCode = 0;
  nanoarrow::UniqueSchema newUniqueField;
  nanoarrow::UniqueArray newUniqueArray;
  ArrowSchema* newSchema = newUniqueField.get();
  ArrowArray* newArray = newUniqueArray.get();
  ArrowError error;

  ArrowSchemaInit(newSchema);
  newSchema->flags &= (field->schema->flags & ARROW_FLAG_NULLABLE);
  returnCode = ArrowSchemaSetTypeDecimal(newSchema, NANOARROW_TYPE_DECIMAL128,
                                         38, scale);
  SF_CHECK_ARROW_RC(returnCode,
                    "[Snowflake Exception] error setting schema type "
                    "decimal, error code: %d",
                    returnCode);
  returnCode = ArrowSchemaSetName(newSchema, field->schema->name);
  SF_CHECK_ARROW_RC(returnCode,
                    "[Snowflake Exception] error setting schema name, "
                    "error code: %d",
                    returnCode);

  returnCode = ArrowArrayInitFromSchema(newArray, newSchema, &error);
  SF_CHECK_ARROW_RC(returnCode,
                    "[Snowflake Exception] error initializing array from "
                    "schema: %s, error code: %d",
                    ArrowErrorMessage(&error), returnCode);
  returnCode = ArrowArrayStartAppending(newArray);
  SF_CHECK_ARROW_RC(returnCode,
                    "[Snowflake Exception] error starting array appending, "
                    "error code: %d",
                    returnCode);

  for (int64_t rowIdx = 0; rowIdx < columnArray->array->length; rowIdx++) {
    if (ArrowArrayViewIsNull(columnArray, rowIdx)) {
      returnCode = ArrowArrayAppendNull(newArray, 1);
    } else {
      auto originalVal = ArrowArrayViewGetIntUnsafe(columnArray, rowIdx);
      ArrowDecimal arrowDecimal;
      ArrowDecimalInit(&arrowDecimal, 128, 38, scale);
      ArrowDecimalSetInt(&arrowDecimal, originalVal);
      returnCode = ArrowArrayAppendDecimal(newArray, &arrowDecimal);
    }
    SF_CHECK_ARROW_RC(returnCode,
                      "[Snowflake Exception] error appending decimal value, "
                      "error code: %d",
                      returnCode);
  }

  returnCode = ArrowArrayFinishBuildingDefault(newArray, &error);
  SF_CHECK_ARROW_RC(returnCode,
                    "[Snowflake Exception] error finishing array: %s, "
                    "error code: %d",
                    ArrowErrorMessage(&error), returnCode);
  field->schema->release(field->schema);
  ArrowSchemaMove(newSchema, field->schema);
  columnArray->array->release(columnArray->array);
  ArrowArrayMove(newArray, columnArray->array);
}

void ArrowTableConverter::convertScaledFixedNumberColumnToDoubleColumn_nanoarrow(
    ArrowSchemaView* field, ArrowArrayView* columnArray,
    unsigned int scale) {
  int returnCode = 0;
  nanoarrow::UniqueSchema newUniqueField;
  nanoarrow::UniqueArray newUniqueArray;
  ArrowSchema* newSchema = newUniqueField.get();
  ArrowArray* newArray = newUniqueArray.get();
  ArrowError error;

  ArrowSchemaInit(newSchema);
  newSchema->flags &= (field->schema->flags & ARROW_FLAG_NULLABLE);
  returnCode = ArrowSchemaSetType(newSchema, NANOARROW_TYPE_DOUBLE);
  SF_CHECK_ARROW_RC(returnCode,
                    "[Snowflake Exception] error setting schema type double, "
                    "error code: %d",
                    returnCode);
  returnCode = ArrowSchemaSetName(newSchema, field->schema->name);
  SF_CHECK_ARROW_RC(returnCode,
                    "[Snowflake Exception] error setting schema name, "
                    "error code: %d",
                    returnCode);

  returnCode = ArrowArrayInitFromSchema(newArray, newSchema, &error);
  SF_CHECK_ARROW_RC(returnCode,
                    "[Snowflake Exception] error initializing array from "
                    "schema: %s, error code: %d",
                    ArrowErrorMessage(&error), returnCode);

  for (int64_t rowIdx = 0; rowIdx < columnArray->array->length; rowIdx++) {
    if (ArrowArrayViewIsNull(columnArray, rowIdx)) {
      returnCode = ArrowArrayAppendNull(newArray, 1);
    } else {
      auto originalVal = ArrowArrayViewGetIntUnsafe(columnArray, rowIdx);
      returnCode = ArrowArrayAppendDouble(
          newArray, convertScaledFixedNumberToDouble(scale, originalVal));
    }
    SF_CHECK_ARROW_RC(returnCode,
                      "[Snowflake Exception] error appending double value, "
                      "error code: %d",
                      returnCode);
  }

  returnCode = ArrowArrayFinishBuildingDefault(newArray, &error);
  SF_CHECK_ARROW_RC(returnCode,
                    "[Snowflake Exception] error finishing array: %s, "
                    "error code: %d",
                    ArrowErrorMessage(&error), returnCode);
  field->schema->release(field->schema);
  ArrowSchemaMove(newSchema, field->schema);
  columnArray->array->release(columnArray->array);
  ArrowArrayMove(newArray, columnArray->array);
}

// ---------------------------------------------------------------------------
// Interval Day-Time
// ---------------------------------------------------------------------------

void ArrowTableConverter::convertIntervalDayTimeColumn_nanoarrow(
    ArrowSchemaView* field, ArrowArrayView* columnArray, int scale) {
  int returnCode = 0;
  nanoarrow::UniqueSchema newUniqueField;
  nanoarrow::UniqueArray newUniqueArray;
  ArrowSchema* newSchema = newUniqueField.get();
  ArrowArray* newArray = newUniqueArray.get();
  ArrowError error;

  ArrowSchemaInit(newSchema);
  newSchema->flags &= (field->schema->flags & ARROW_FLAG_NULLABLE);
  returnCode = ArrowSchemaSetTypeDateTime(newSchema, NANOARROW_TYPE_DURATION,
                                          NANOARROW_TIME_UNIT_NANO, nullptr);
  SF_CHECK_ARROW_RC(returnCode,
                    "[Snowflake Exception] error setting schema DateTime, "
                    "error code: %d",
                    returnCode);
  returnCode = ArrowSchemaSetName(newSchema, field->schema->name);
  SF_CHECK_ARROW_RC(returnCode,
                    "[Snowflake Exception] error setting schema name, "
                    "error code: %d",
                    returnCode);

  returnCode = ArrowArrayInitFromSchema(newArray, newSchema, &error);
  SF_CHECK_ARROW_RC(returnCode,
                    "[Snowflake Exception] error initializing array from "
                    "schema: %s, error code: %d",
                    ArrowErrorMessage(&error), returnCode);
  returnCode = ArrowArrayStartAppending(newArray);
  SF_CHECK_ARROW_RC(returnCode,
                    "[Snowflake Exception] error starting appending, "
                    "error code: %d",
                    returnCode);

  for (int64_t rowIdx = 0; rowIdx < columnArray->array->length; rowIdx++) {
    if (ArrowArrayViewIsNull(columnArray, rowIdx)) {
      returnCode = ArrowArrayAppendNull(newArray, 1);
    } else {
      ArrowDecimal arrowDecimal;
      ArrowDecimalInit(&arrowDecimal, 128, 38, 0);
      ArrowArrayViewGetDecimalUnsafe(columnArray, rowIdx, &arrowDecimal);
      returnCode =
          ArrowArrayAppendInt(newArray, ArrowDecimalGetIntUnsafe(&arrowDecimal));
    }
    SF_CHECK_ARROW_RC(returnCode,
                      "[Snowflake Exception] error appending interval value, "
                      "error code: %d",
                      returnCode);
  }

  returnCode = ArrowArrayFinishBuildingDefault(newArray, &error);
  SF_CHECK_ARROW_RC(returnCode,
                    "[Snowflake Exception] error finishing array: %s, "
                    "error code: %d",
                    ArrowErrorMessage(&error), returnCode);
  field->schema->release(field->schema);
  ArrowSchemaMove(newSchema, field->schema);
  columnArray->array->release(columnArray->array);
  ArrowArrayMove(newArray, columnArray->array);
}

// ---------------------------------------------------------------------------
// Time
// ---------------------------------------------------------------------------

void ArrowTableConverter::convertTimeColumn_nanoarrow(
    ArrowSchemaView* field, ArrowArrayView* columnArray, int scale) {
  int returnCode = 0;
  nanoarrow::UniqueSchema newUniqueField;
  nanoarrow::UniqueArray newUniqueArray;
  ArrowSchema* newSchema = newUniqueField.get();
  ArrowArray* newArray = newUniqueArray.get();
  ArrowError error;

  ArrowSchemaInit(newSchema);
  newSchema->flags &= (field->schema->flags & ARROW_FLAG_NULLABLE);

  int64_t powTenSB4Val = 1;
  if (scale == 0) {
    returnCode = ArrowSchemaSetTypeDateTime(
        newSchema, NANOARROW_TYPE_TIME32, NANOARROW_TIME_UNIT_SECOND, nullptr);
  } else if (scale <= 3) {
    returnCode = ArrowSchemaSetTypeDateTime(
        newSchema, NANOARROW_TYPE_TIME32, NANOARROW_TIME_UNIT_MILLI, nullptr);
    powTenSB4Val = sf::internal::powTenSB4[3 - scale];
  } else if (scale <= 6) {
    returnCode = ArrowSchemaSetTypeDateTime(
        newSchema, NANOARROW_TYPE_TIME64, NANOARROW_TIME_UNIT_MICRO, nullptr);
    powTenSB4Val = sf::internal::powTenSB4[6 - scale];
  } else {
    returnCode = ArrowSchemaSetTypeDateTime(
        newSchema, NANOARROW_TYPE_TIME64, NANOARROW_TIME_UNIT_MICRO, nullptr);
    powTenSB4Val = sf::internal::powTenSB4[scale - 6];
  }
  SF_CHECK_ARROW_RC(returnCode,
                    "[Snowflake Exception] error setting schema DateTime, "
                    "error code: %d",
                    returnCode);
  returnCode = ArrowSchemaSetName(newSchema, field->schema->name);
  SF_CHECK_ARROW_RC(returnCode,
                    "[Snowflake Exception] error setting schema name, "
                    "error code: %d",
                    returnCode);

  returnCode = ArrowArrayInitFromSchema(newArray, newSchema, &error);
  SF_CHECK_ARROW_RC(returnCode,
                    "[Snowflake Exception] error initializing array: %s, "
                    "error code: %d",
                    ArrowErrorMessage(&error), returnCode);
  returnCode = ArrowArrayStartAppending(newArray);
  SF_CHECK_ARROW_RC(returnCode,
                    "[Snowflake Exception] error starting appending, "
                    "error code: %d",
                    returnCode);

  for (int64_t rowIdx = 0; rowIdx < columnArray->array->length; rowIdx++) {
    if (ArrowArrayViewIsNull(columnArray, rowIdx)) {
      returnCode = ArrowArrayAppendNull(newArray, 1);
    } else {
      auto val = ArrowArrayViewGetIntUnsafe(columnArray, rowIdx);
      val = (scale <= 6) ? (val * powTenSB4Val) : (val / powTenSB4Val);
      returnCode = ArrowArrayAppendInt(newArray, val);
    }
    SF_CHECK_ARROW_RC(returnCode,
                      "[Snowflake Exception] error appending time value, "
                      "error code: %d",
                      returnCode);
  }

  returnCode = ArrowArrayFinishBuildingDefault(newArray, &error);
  SF_CHECK_ARROW_RC(returnCode,
                    "[Snowflake Exception] error finishing array: %s, "
                    "error code: %d",
                    ArrowErrorMessage(&error), returnCode);
  field->schema->release(field->schema);
  ArrowSchemaMove(newSchema, field->schema);
  columnArray->array->release(columnArray->array);
  ArrowArrayMove(newArray, columnArray->array);
}

// ---------------------------------------------------------------------------
// Nanosecond timestamp overflow helper
// ---------------------------------------------------------------------------

static bool _checkNanosecondTimestampOverflowAndDownscale(
    ArrowArrayView* columnArray, ArrowArrayView* epochArray,
    ArrowArrayView* fractionArray) {
  int powTenSB4 = sf::internal::powTenSB4[9];
  for (int64_t rowIdx = 0; rowIdx < columnArray->array->length; rowIdx++) {
    if (!ArrowArrayViewIsNull(columnArray, rowIdx)) {
      int64_t epoch = ArrowArrayViewGetIntUnsafe(epochArray, rowIdx);
      int64_t fraction = ArrowArrayViewGetIntUnsafe(fractionArray, rowIdx);
      if (epoch > (INT64_MAX / powTenSB4) ||
          epoch < (INT64_MIN / powTenSB4)) {
        if (fraction % 1000 != 0) {
          std::string errorInfo = Logger::formatString(
              "The total number of nanoseconds %d%d overflows int64 range. "
              "If you use a timestamp with the nanosecond part over 6-digits "
              "in the Snowflake database, the timestamp must be between "
              "'1677-09-21 00:12:43.145224192' and '2262-04-11 "
              "23:47:16.854775807' to not overflow.",
              epoch, fraction);
          throw std::overflow_error(errorInfo.c_str());
        }
        return true;
      }
    }
  }
  return false;
}

// ---------------------------------------------------------------------------
// Timestamp NTZ / LTZ
// ---------------------------------------------------------------------------

void ArrowTableConverter::convertTimestampColumn_nanoarrow(
    ArrowSchemaView* field, ArrowArrayView* columnArray, int scale,
    const std::string& timezone) {
  int returnCode = 0;
  nanoarrow::UniqueSchema newUniqueField;
  nanoarrow::UniqueArray newUniqueArray;
  ArrowSchema* newSchema = newUniqueField.get();
  ArrowArray* newArray = newUniqueArray.get();
  ArrowError error;

  ArrowSchemaInit(newSchema);
  newSchema->flags &= (field->schema->flags & ARROW_FLAG_NULLABLE);

  const char* tz = timezone.empty() ? nullptr : timezone.c_str();

  ArrowArrayView* epochArray = nullptr;
  ArrowArrayView* fractionArray = nullptr;
  bool has_overflow = m_force_microsecond_precision;
  if (!m_force_microsecond_precision && scale > 6 &&
      field->type == NANOARROW_TYPE_STRUCT) {
    for (int64_t i = 0; i < field->schema->n_children; i++) {
      const char* name = field->schema->children[i]->name;
      if (std::strcmp(name, internal::FIELD_NAME_EPOCH.c_str()) == 0)
        epochArray = columnArray->children[i];
      else if (std::strcmp(name, internal::FIELD_NAME_FRACTION.c_str()) == 0)
        fractionArray = columnArray->children[i];
    }
    has_overflow = _checkNanosecondTimestampOverflowAndDownscale(
        columnArray, epochArray, fractionArray);
  }

  if (scale <= 6) {
    auto timeunit = NANOARROW_TIME_UNIT_SECOND;
    int64_t powTenSB4Val = 1;
    if (scale == 0) {
      timeunit = NANOARROW_TIME_UNIT_SECOND;
    } else if (scale <= 3) {
      timeunit = NANOARROW_TIME_UNIT_MILLI;
      powTenSB4Val = sf::internal::powTenSB4[3 - scale];
    } else {
      timeunit = NANOARROW_TIME_UNIT_MICRO;
      powTenSB4Val = sf::internal::powTenSB4[6 - scale];
    }
    returnCode = ArrowSchemaSetTypeDateTime(
        newSchema, NANOARROW_TYPE_TIMESTAMP, timeunit, tz);
    SF_CHECK_ARROW_RC(returnCode,
                      "[Snowflake Exception] error setting schema DateTime, "
                      "error code: %d",
                      returnCode);
    returnCode = ArrowSchemaSetName(newSchema, field->schema->name);
    SF_CHECK_ARROW_RC(returnCode,
                      "[Snowflake Exception] error setting schema name, "
                      "error code: %d",
                      returnCode);
    returnCode = ArrowArrayInitFromSchema(newArray, newSchema, &error);
    SF_CHECK_ARROW_RC(returnCode,
                      "[Snowflake Exception] error initializing array: %s, "
                      "error code: %d",
                      ArrowErrorMessage(&error), returnCode);

    for (int64_t rowIdx = 0; rowIdx < columnArray->array->length; rowIdx++) {
      if (ArrowArrayViewIsNull(columnArray, rowIdx)) {
        returnCode = ArrowArrayAppendNull(newArray, 1);
      } else {
        int64_t val =
            ArrowArrayViewGetIntUnsafe(columnArray, rowIdx) * powTenSB4Val;
        returnCode = ArrowArrayAppendInt(newArray, val);
      }
      SF_CHECK_ARROW_RC(returnCode,
                        "[Snowflake Exception] error appending timestamp, "
                        "error code: %d",
                        returnCode);
    }
  } else {
    auto timeunit =
        has_overflow ? NANOARROW_TIME_UNIT_MICRO : NANOARROW_TIME_UNIT_NANO;
    returnCode = ArrowSchemaSetTypeDateTime(
        newSchema, NANOARROW_TYPE_TIMESTAMP, timeunit, tz);
    SF_CHECK_ARROW_RC(returnCode,
                      "[Snowflake Exception] error setting schema DateTime, "
                      "error code: %d",
                      returnCode);
    returnCode = ArrowSchemaSetName(newSchema, field->schema->name);
    SF_CHECK_ARROW_RC(returnCode,
                      "[Snowflake Exception] error setting schema name, "
                      "error code: %d",
                      returnCode);
    returnCode = ArrowArrayInitFromSchema(newArray, newSchema, &error);
    SF_CHECK_ARROW_RC(returnCode,
                      "[Snowflake Exception] error initializing array: %s, "
                      "error code: %d",
                      ArrowErrorMessage(&error), returnCode);

    if (field->type == NANOARROW_TYPE_STRUCT) {
      epochArray = nullptr;
      fractionArray = nullptr;
      for (int64_t i = 0; i < field->schema->n_children; i++) {
        const char* name = field->schema->children[i]->name;
        if (std::strcmp(name, internal::FIELD_NAME_EPOCH.c_str()) == 0)
          epochArray = columnArray->children[i];
        else if (std::strcmp(name, internal::FIELD_NAME_FRACTION.c_str()) == 0)
          fractionArray = columnArray->children[i];
      }
      for (int64_t rowIdx = 0; rowIdx < columnArray->array->length; rowIdx++) {
        if (ArrowArrayViewIsNull(columnArray, rowIdx)) {
          returnCode = ArrowArrayAppendNull(newArray, 1);
        } else {
          int64_t epoch = ArrowArrayViewGetIntUnsafe(epochArray, rowIdx);
          int64_t frac = ArrowArrayViewGetIntUnsafe(fractionArray, rowIdx);
          int64_t val = has_overflow
                            ? (epoch * sf::internal::powTenSB4[6] + frac / 1000)
                            : (epoch * sf::internal::powTenSB4[9] + frac);
          returnCode = ArrowArrayAppendInt(newArray, val);
        }
        SF_CHECK_ARROW_RC(returnCode,
                          "[Snowflake Exception] error appending timestamp, "
                          "error code: %d",
                          returnCode);
      }
    } else if (field->type == NANOARROW_TYPE_INT64) {
      for (int64_t rowIdx = 0; rowIdx < columnArray->array->length; rowIdx++) {
        if (ArrowArrayViewIsNull(columnArray, rowIdx)) {
          returnCode = ArrowArrayAppendNull(newArray, 1);
        } else {
          int64_t val = ArrowArrayViewGetIntUnsafe(columnArray, rowIdx) *
                        sf::internal::powTenSB4[9 - scale];
          returnCode = ArrowArrayAppendInt(newArray, val);
        }
        SF_CHECK_ARROW_RC(returnCode,
                          "[Snowflake Exception] error appending timestamp, "
                          "error code: %d",
                          returnCode);
      }
    }
  }

  returnCode = ArrowArrayFinishBuildingDefault(newArray, &error);
  SF_CHECK_ARROW_RC(returnCode,
                    "[Snowflake Exception] error finishing array: %s, "
                    "error code: %d",
                    ArrowErrorMessage(&error), returnCode);
  field->schema->release(field->schema);
  ArrowSchemaMove(newSchema, field->schema);
  columnArray->array->release(columnArray->array);
  ArrowArrayMove(newArray, columnArray->array);
}

// ---------------------------------------------------------------------------
// Timestamp TZ
// ---------------------------------------------------------------------------

void ArrowTableConverter::convertTimestampTZColumn_nanoarrow(
    ArrowSchemaView* field, ArrowArrayView* columnArray, int scale,
    int byteLength, const std::string& timezone) {
  int returnCode = 0;
  nanoarrow::UniqueSchema newUniqueField;
  nanoarrow::UniqueArray newUniqueArray;
  ArrowSchema* newSchema = newUniqueField.get();
  ArrowArray* newArray = newUniqueArray.get();
  ArrowError error;

  ArrowSchemaInit(newSchema);
  newSchema->flags &= (field->schema->flags & ARROW_FLAG_NULLABLE);

  ArrowArrayView* epochArray = nullptr;
  ArrowArrayView* fractionArray = nullptr;
  for (int64_t i = 0; i < field->schema->n_children; i++) {
    const char* name = field->schema->children[i]->name;
    if (std::strcmp(name, internal::FIELD_NAME_EPOCH.c_str()) == 0)
      epochArray = columnArray->children[i];
    else if (std::strcmp(name, internal::FIELD_NAME_FRACTION.c_str()) == 0)
      fractionArray = columnArray->children[i];
  }

  bool has_overflow = m_force_microsecond_precision;
  if (!m_force_microsecond_precision && scale > 6 && byteLength == 16) {
    has_overflow = _checkNanosecondTimestampOverflowAndDownscale(
        columnArray, epochArray, fractionArray);
  }

  auto timeunit = NANOARROW_TIME_UNIT_SECOND;
  if (scale == 0)
    timeunit = NANOARROW_TIME_UNIT_SECOND;
  else if (scale <= 3)
    timeunit = NANOARROW_TIME_UNIT_MILLI;
  else if (scale <= 6)
    timeunit = NANOARROW_TIME_UNIT_MICRO;
  else
    timeunit =
        has_overflow ? NANOARROW_TIME_UNIT_MICRO : NANOARROW_TIME_UNIT_NANO;

  const char* tz = timezone.empty() ? nullptr : timezone.c_str();
  returnCode = ArrowSchemaSetTypeDateTime(
      newSchema, NANOARROW_TYPE_TIMESTAMP, timeunit, tz);
  SF_CHECK_ARROW_RC(returnCode,
                    "[Snowflake Exception] error setting schema DateTime, "
                    "error code: %d",
                    returnCode);
  returnCode = ArrowSchemaSetName(newSchema, field->schema->name);
  SF_CHECK_ARROW_RC(returnCode,
                    "[Snowflake Exception] error setting schema name, "
                    "error code: %d",
                    returnCode);
  returnCode = ArrowArrayInitFromSchema(newArray, newSchema, &error);
  SF_CHECK_ARROW_RC(returnCode,
                    "[Snowflake Exception] error initializing array: %s, "
                    "error code: %d",
                    ArrowErrorMessage(&error), returnCode);

  for (int64_t rowIdx = 0; rowIdx < columnArray->array->length; rowIdx++) {
    if (ArrowArrayViewIsNull(columnArray, rowIdx)) {
      returnCode = ArrowArrayAppendNull(newArray, 1);
    } else {
      int64_t epoch = ArrowArrayViewGetIntUnsafe(epochArray, rowIdx);
      if (byteLength == 8) {
        int64_t val;
        if (scale == 0)
          val = epoch;
        else if (scale <= 3)
          val = epoch * sf::internal::powTenSB4[3 - scale];
        else if (scale <= 6)
          val = epoch * sf::internal::powTenSB4[6 - scale];
        else
          val = has_overflow ? (epoch * sf::internal::powTenSB4[6])
                             : (epoch * sf::internal::powTenSB4[9 - scale]);
        returnCode = ArrowArrayAppendInt(newArray, val);
      } else if (byteLength == 16) {
        int64_t frac = ArrowArrayViewGetIntUnsafe(fractionArray, rowIdx);
        int64_t val;
        if (scale == 0)
          val = epoch;
        else if (scale <= 3)
          val = epoch * sf::internal::powTenSB4[3 - scale] +
                frac / sf::internal::powTenSB4[6];
        else if (scale <= 6)
          val = epoch * sf::internal::powTenSB4[6] +
                frac / sf::internal::powTenSB4[3];
        else
          val = has_overflow
                    ? (epoch * sf::internal::powTenSB4[6] + frac / 1000)
                    : (epoch * sf::internal::powTenSB4[9] + frac);
        returnCode = ArrowArrayAppendInt(newArray, val);
      } else {
        PyErr_SetString(
            PyExc_Exception,
            Logger::formatString(
                "[Snowflake Exception] unknown byteLength(%d) for "
                "TIMESTAMP_TZ",
                byteLength).c_str());
        return;
      }
    }
    SF_CHECK_ARROW_RC(returnCode,
                      "[Snowflake Exception] error appending timestamp_tz, "
                      "error code: %d",
                      returnCode);
  }

  returnCode = ArrowArrayFinishBuildingDefault(newArray, &error);
  SF_CHECK_ARROW_RC(returnCode,
                    "[Snowflake Exception] error finishing array: %s, "
                    "error code: %d",
                    ArrowErrorMessage(&error), returnCode);
  field->schema->release(field->schema);
  ArrowSchemaMove(newSchema, field->schema);
  columnArray->array->release(columnArray->array);
  ArrowArrayMove(newArray, columnArray->array);
}

}  // namespace sf
