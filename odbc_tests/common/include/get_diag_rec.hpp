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
inline std::string get_sqlstate(const HandleWrapper& handle_wrapper) {
  auto records = get_diag_rec(handle_wrapper);
  if (records.empty()) {
    return "";
  }
  return records[0].sqlState;
}

#endif  // GET_DIAG_REC_HPP
