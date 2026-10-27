#ifndef CATALOG_IRD_HPP
#define CATALOG_IRD_HPP

#include <sql.h>
#include <sqlext.h>

#include <cstring>
#include <initializer_list>
#include <string>

#include <catch2/catch_test_macros.hpp>

inline std::string catalog_string_attr(SQLHSTMT stmt, SQLUSMALLINT column, SQLUSMALLINT fieldId) {
  INFO("col " << column << " field " << fieldId);
  char name[64];
  std::memset(name, 0xFF, sizeof(name));
  SQLSMALLINT nameLen = -1;
  const SQLRETURN ret = SQLColAttribute(stmt, column, fieldId, name, sizeof(name), &nameLen, nullptr);
  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(nameLen >= 0);
  REQUIRE(static_cast<size_t>(nameLen) < sizeof(name));
  REQUIRE(name[nameLen] == '\0');
  return std::string(name, static_cast<size_t>(nameLen));
}

inline void require_catalog_type_name(SQLHSTMT stmt, SQLUSMALLINT column, SQLUSMALLINT fieldId, const char* typeName) {
  REQUIRE(catalog_string_attr(stmt, column, fieldId) == typeName);
}

inline SQLLEN catalog_numeric_attr(SQLHSTMT stmt, SQLUSMALLINT column, SQLUSMALLINT fieldId) {
  SQLLEN numeric;
  std::memset(&numeric, 0xFF, sizeof(numeric));
  const SQLRETURN ret = SQLColAttribute(stmt, column, fieldId, nullptr, 0, nullptr, &numeric);
  REQUIRE(ret == SQL_SUCCESS);
  return numeric;
}

inline void require_catalog_integer_ird(SQLHSTMT stmt, SQLUSMALLINT column, SQLLEN displaySize, SQLLEN octetLength,
                                        const char* typeName, bool unsignedColumn = false) {
  INFO("col " << column);
  REQUIRE(catalog_numeric_attr(stmt, column, SQL_DESC_DISPLAY_SIZE) == displaySize);
  REQUIRE(catalog_numeric_attr(stmt, column, SQL_DESC_OCTET_LENGTH) == octetLength);
  REQUIRE(catalog_numeric_attr(stmt, column, SQL_DESC_UNSIGNED) == (unsignedColumn ? SQL_TRUE : SQL_FALSE));

  require_catalog_type_name(stmt, column, SQL_DESC_TYPE_NAME, typeName);
  const std::string localTypeName = std::string(unsignedColumn ? "U" : "S") + typeName;
  require_catalog_type_name(stmt, column, SQL_DESC_LOCAL_TYPE_NAME, localTypeName.c_str());
}

inline SQLLEN catalog_wide_octet_length(SQLLEN charLength) { return charLength * 4; }

inline void require_catalog_wide_column(SQLHSTMT stmt, SQLUSMALLINT column) {
  INFO("col " << column);
  const SQLLEN charLength = catalog_numeric_attr(stmt, column, SQL_DESC_DISPLAY_SIZE);
  REQUIRE(catalog_numeric_attr(stmt, column, SQL_DESC_LENGTH) == charLength);
  REQUIRE(catalog_numeric_attr(stmt, column, SQL_DESC_OCTET_LENGTH) == catalog_wide_octet_length(charLength));
  const std::string typeName = catalog_string_attr(stmt, column, SQL_DESC_TYPE_NAME);
  REQUIRE(catalog_string_attr(stmt, column, SQL_DESC_LOCAL_TYPE_NAME) == typeName);
}

inline void require_catalog_wide_octet_length(SQLHSTMT stmt, SQLUSMALLINT column, SQLLEN charLength) {
  REQUIRE(catalog_numeric_attr(stmt, column, SQL_DESC_DISPLAY_SIZE) == charLength);
  require_catalog_wide_column(stmt, column);
}

inline void require_catalog_varchar_ird(SQLHSTMT stmt, SQLUSMALLINT column) {
  INFO("col " << column);
  REQUIRE(catalog_numeric_attr(stmt, column, SQL_DESC_DISPLAY_SIZE) == 20);
  require_catalog_type_name(stmt, column, SQL_DESC_TYPE_NAME, "VARCHAR");
  require_catalog_type_name(stmt, column, SQL_DESC_LOCAL_TYPE_NAME, "VARCHAR");
}

inline void require_catalog_wide_columns(SQLHSTMT stmt, std::initializer_list<SQLUSMALLINT> columns) {
  for (const SQLUSMALLINT column : columns) {
    require_catalog_wide_column(stmt, column);
  }
}

#endif
