#pragma once

#include <algorithm>
#include <stdexcept>
#include <string>

/// Enum for test types
enum class TestType { Select, PutGet };

/// Convert string to TestType enum
inline TestType parse_test_type(const std::string& str) {
  std::string lower = str;
  std::transform(lower.begin(), lower.end(), lower.begin(), ::tolower);

  if (lower == "select") {
    return TestType::Select;
  } else if (lower == "put_get") {
    return TestType::PutGet;
  } else {
    throw std::invalid_argument("Unknown test type: '" + str +
                                "'. Supported types: select, put_get");
  }
}

/// Convert TestType enum to string
inline std::string test_type_to_string(TestType type) {
  switch (type) {
    case TestType::Select:
      return "select";
    case TestType::PutGet:
      return "put_get";
    default:
      throw std::logic_error("Invalid test type enum value");
  }
}
