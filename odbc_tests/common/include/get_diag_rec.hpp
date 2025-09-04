#include <sql.h>

#include <iostream>
#include <string>
#include <vector>

#include "HandleWrapper.hpp"

struct DiagRec {
  std::string sqlState;
  SQLINTEGER nativeError;
  std::string messageText;
};

std::vector<DiagRec> get_diag_rec(const HandleWrapper& wrapper);
