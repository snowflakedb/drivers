#ifndef ODBC_CAST_HPP
#define ODBC_CAST_HPP

#include <sql.h>

// Cast a C string literal or char* to SQLCHAR* for ODBC API calls.
inline SQLCHAR* sqlchar(const char* str) { return reinterpret_cast<SQLCHAR*>(const_cast<char*>(str)); }

#endif  // ODBC_CAST_HPP
