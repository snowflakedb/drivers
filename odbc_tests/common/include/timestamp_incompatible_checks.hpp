#ifndef TIMESTAMP_INCOMPATIBLE_CHECKS_HPP
#define TIMESTAMP_INCOMPATIBLE_CHECKS_HPP

#include <sql.h>
#include <sqlext.h>
#include <sqltypes.h>

#include "HandleWrapper.hpp"
#include "conversion_checks.hpp"

template <typename T>
inline void check_incompat(const StatementHandleWrapper& stmt, SQLUSMALLINT col, SQLSMALLINT target_type) {
  T value = {};
  check_incompatible_conversion(stmt, col, target_type, &value, sizeof(value));
}

#endif  // TIMESTAMP_INCOMPATIBLE_CHECKS_HPP
