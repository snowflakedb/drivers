#include "get_diag_rec.hpp"

#include <iostream>
#include <string>
#include <vector>

std::vector<DiagRec> get_diag_rec(const HandleWrapper& wrapper) {
  SQLSMALLINT recNumber = 1;
  std::vector<DiagRec> records;

  while (true) {
    SQLCHAR sqlState[6] = {0};
    SQLINTEGER nativeError = 0;
    SQLCHAR messageText[8096] = {0};
    SQLSMALLINT textLength = 0;

    std::cout << "[get_diag_rec] nativeError: " << &nativeError << std::endl;
    std::cout << "[get_diag_rec] sqlState: " << &sqlState << std::endl;
    std::cout << "[get_diag_rec] messageText: " << (void*)messageText << std::endl;
    std::cout << "[get_diag_rec] textLength: " << &textLength << std::endl;

    SQLRETURN ret = SQLGetDiagRec(wrapper.getType(), wrapper.getHandle(), recNumber, sqlState,
                                  &nativeError, messageText, sizeof(messageText), &textLength);
    if (ret == SQL_NO_DATA) {
      break;  // No more data
    }

    REQUIRE(ret == SQL_SUCCESS);
    std::string messageStr((char*)messageText, textLength);
    std::string sqlStateStr((char*)sqlState, 5);

    DiagRec record = {};
    record.sqlState = sqlStateStr;
    record.nativeError = nativeError;
    record.messageText = messageStr;
    records.push_back(record);
    recNumber++;
  }
  return records;
}
