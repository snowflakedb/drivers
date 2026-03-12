#ifndef GET_DESCRIPTOR_HPP
#define GET_DESCRIPTOR_HPP

#include <sql.h>

#include <catch2/catch_test_macros.hpp>

// Retrieves an implicit descriptor handle from a statement attribute.
// Typical attrs: SQL_ATTR_APP_ROW_DESC, SQL_ATTR_APP_PARAM_DESC,
//                SQL_ATTR_IMP_ROW_DESC, SQL_ATTR_IMP_PARAM_DESC
inline SQLHDESC get_descriptor(const SQLHSTMT stmt, const SQLINTEGER attr) {
  SQLHDESC desc = SQL_NULL_HDESC;
  const SQLRETURN ret = SQLGetStmtAttr(stmt, attr, &desc, 0, nullptr);
  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(desc != SQL_NULL_HDESC);
  return desc;
}

#endif  // GET_DESCRIPTOR_HPP
