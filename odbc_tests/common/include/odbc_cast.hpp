#ifndef ODBC_CAST_HPP
#define ODBC_CAST_HPP

#include <sql.h>

#include <cstdint>

// Cast a C string literal or char* to SQLCHAR* for ODBC API calls.
inline SQLCHAR* sqlchar(const char* str) { return reinterpret_cast<SQLCHAR*>(const_cast<char*>(str)); }

// Integer-valued ODBC attribute / descriptor ValuePtr (SQLSetStmtAttr,
// SQLSetEnvAttr, SQLSetDescField, …). Widen through intptr_t first so
// MSVC C4312 is not raised on 64-bit builds when the source is a 32-bit int.
template <typename T>
inline SQLPOINTER sqlptr_value(T value) {
  return reinterpret_cast<SQLPOINTER>(static_cast<std::intptr_t>(value));
}

#endif  // ODBC_CAST_HPP
