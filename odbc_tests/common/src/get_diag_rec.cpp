#include "get_diag_rec.hpp"

#include <iostream>
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
      std::cout << "SQLGetDiagRec: No more data" << std::endl;
      break;  // No more data
    }

    if (ret != SQL_SUCCESS && ret != SQL_SUCCESS_WITH_INFO) {
      std::cerr << "Warning: SQLGetDiagRec failed (returned " << ret << ") when retrieving diagnostic record #"
                << recNumber << std::endl;
      break;
    }

    std::cout << "SQLGetDiagRec: Successfully retrieved diagnostic record #" << recNumber << std::endl;
    std::cout << "SQLState: " << std::string(reinterpret_cast<char*>(sqlState), 5) << std::endl;
    std::cout << "Native Error: " << nativeError << std::endl;
    std::cout << "Message Text: " << std::string(reinterpret_cast<char*>(messageText), textLength) << std::endl;

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
