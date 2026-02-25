#ifndef GET_DIAG_REC_HPP
#define GET_DIAG_REC_HPP

#include <string>
#include <vector>

#include "HandleWrapper.hpp"

struct DiagRec {
  std::string sqlState;
  SQLINTEGER nativeError;
  std::string messageText;
};

// Get diagnostic records from a raw ODBC handle
std::vector<DiagRec> get_diag_rec(SQLSMALLINT handle_type, SQLHANDLE handle);

// Get diagnostic records from a HandleWrapper
std::vector<DiagRec> get_diag_rec(const HandleWrapper& wrapper);

// Helper to extract SQLSTATE from the first diagnostic record
inline std::string get_sqlstate(const SQLSMALLINT handle_type, const SQLHANDLE handle) {
  auto records = get_diag_rec(handle_type, handle);
  if (records.empty()) {
    return "";
  }
  return records[0].sqlState;
}

inline std::string get_sqlstate(const HandleWrapper& handle_wrapper) {
  return get_sqlstate(handle_wrapper.getType(), handle_wrapper.getHandle());
}

#endif  // GET_DIAG_REC_HPP
