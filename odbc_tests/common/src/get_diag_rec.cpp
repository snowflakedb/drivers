#include "get_diag_rec.hpp"

#include <string>
#include <vector>

// Core implementation: get diagnostic records from raw handle
std::vector<DiagRec> get_diag_rec(const SQLSMALLINT handle_type, const SQLHANDLE handle) {
  std::vector<DiagRec> records;
  SQLSMALLINT recNumber = 1;

  while (true) {
    SQLCHAR sqlState[6] = {};
    SQLINTEGER nativeError = 0;
    SQLCHAR messageText[8096] = {};
    SQLSMALLINT textLength = 0;

    const SQLRETURN ret = SQLGetDiagRec(handle_type, handle, recNumber, sqlState, &nativeError, messageText,
                                        sizeof(messageText), &textLength);
    if (ret == SQL_NO_DATA) {
      break;
    }

    if (ret != SQL_SUCCESS && ret != SQL_SUCCESS_WITH_INFO) {
      break;
    }

    DiagRec record;
    record.sqlState = std::string(reinterpret_cast<char*>(sqlState), 5);
    record.nativeError = nativeError;
    record.messageText = std::string(reinterpret_cast<char*>(messageText), textLength);
    records.push_back(record);
    recNumber++;
  }
  return records;
}

// Overload: get diagnostic records from HandleWrapper
std::vector<DiagRec> get_diag_rec(const HandleWrapper& wrapper) {
  return get_diag_rec(wrapper.getType(), wrapper.getHandle());
}
