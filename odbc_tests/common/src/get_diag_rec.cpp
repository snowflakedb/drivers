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
