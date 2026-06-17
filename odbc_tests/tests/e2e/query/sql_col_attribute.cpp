#include <cstring>
#include <string>

#include <catch2/catch_test_macros.hpp>

#include "Connection.hpp"
#include "Schema.hpp"
#include "compatibility.hpp"
#include "get_diag_rec.hpp"
#include "odbc_matchers.hpp"

// =============================================================================
// Tests for SQLColAttribute (ODBC 3.x) based on ODBC specification:
// https://learn.microsoft.com/en-us/sql/odbc/reference/syntax/sqlcolattribute-function
//
// SQLColAttribute is the ODBC 3.x replacement for SQLColAttributes.
// It uses SQL_DESC_* field identifiers and returns string-valued attributes
// via CharacterAttributePtr and numeric attributes via NumericAttributePtr.
// =============================================================================

static SQLLEN get_numeric_attr(const HandleWrapper& stmt, SQLUSMALLINT col, SQLUSMALLINT field_id) {
  SQLLEN value = 0;
  SQLRETURN ret = SQLColAttribute(stmt.getHandle(), col, field_id, NULL, 0, NULL, &value);
  REQUIRE_THAT(OdbcResult(ret, stmt), OdbcMatchers::IsSuccess());
  return value;
}

static std::string get_string_attr(const HandleWrapper& stmt, SQLUSMALLINT col, SQLUSMALLINT field_id) {
  SQLCHAR buffer[512] = {0};
  SQLSMALLINT str_len = 0;
  SQLRETURN ret = SQLColAttribute(stmt.getHandle(), col, field_id, buffer, sizeof(buffer), &str_len, NULL);
  REQUIRE_THAT(OdbcResult(ret, stmt), OdbcMatchers::IsSuccess());
  return std::string(reinterpret_cast<char*>(buffer));
}

// =============================================================================
// Per-type attribute coverage
// =============================================================================

TEST_CASE("SQLColAttribute returns correct attributes for VARCHAR.", "[query][col_attribute]") {
  // Given A table with a VARCHAR(100) column is queried
  bool is_ascii = is_ascii_locale();
  Connection conn;
  Schema::use_temp_session_schema(conn);
  conn.execute("CREATE TEMPORARY TABLE t (val VARCHAR(100))");
  auto stmt = conn.execute("SELECT val FROM t");

  // When SQLColAttribute is called for each descriptor field
  CHECK(get_numeric_attr(stmt, 1, SQL_DESC_CONCISE_TYPE) == SQL_VARCHAR);
  CHECK(get_numeric_attr(stmt, 1, SQL_DESC_TYPE) == SQL_VARCHAR);
  // Then All metadata attributes should match expected values for VARCHAR
  CHECK(get_string_attr(stmt, 1, SQL_DESC_TYPE_NAME) == "VARCHAR");
  CHECK(get_string_attr(stmt, 1, SQL_DESC_LOCAL_TYPE_NAME) == "VARCHAR");
  CHECK(get_numeric_attr(stmt, 1, SQL_DESC_LENGTH) == 100);
  CHECK(get_numeric_attr(stmt, 1, SQL_DESC_PRECISION) == 100);
  CHECK(get_numeric_attr(stmt, 1, SQL_DESC_SCALE) == 0);
  CHECK(get_numeric_attr(stmt, 1, SQL_DESC_DISPLAY_SIZE) == 100);
  UNIX_ONLY {
    if (is_ascii) {
      CHECK(get_numeric_attr(stmt, 1, SQL_DESC_OCTET_LENGTH) == 100);
    } else {
      CHECK(get_numeric_attr(stmt, 1, SQL_DESC_OCTET_LENGTH) == 400);
    }
  }
  WINDOWS_ONLY { CHECK(get_numeric_attr(stmt, 1, SQL_DESC_OCTET_LENGTH) == 100); }
  CHECK(get_numeric_attr(stmt, 1, SQL_DESC_NUM_PREC_RADIX) == 0);
  CHECK(get_numeric_attr(stmt, 1, SQL_DESC_UNSIGNED) == SQL_TRUE);
  CHECK(get_numeric_attr(stmt, 1, SQL_DESC_CASE_SENSITIVE) == SQL_TRUE);
  CHECK(get_string_attr(stmt, 1, SQL_DESC_LITERAL_PREFIX) == "'");
  CHECK(get_string_attr(stmt, 1, SQL_DESC_LITERAL_SUFFIX) == "'");
  CHECK(get_numeric_attr(stmt, 1, SQL_DESC_SEARCHABLE) == SQL_PRED_SEARCHABLE);
  CHECK(get_numeric_attr(stmt, 1, SQL_DESC_UPDATABLE) == SQL_ATTR_READWRITE_UNKNOWN);
  CHECK(get_numeric_attr(stmt, 1, SQL_DESC_AUTO_UNIQUE_VALUE) == SQL_FALSE);
  CHECK(get_numeric_attr(stmt, 1, SQL_DESC_FIXED_PREC_SCALE) == SQL_FALSE);
  CHECK(get_numeric_attr(stmt, 1, SQL_DESC_NULLABLE) == SQL_NULLABLE);
  CHECK(get_numeric_attr(stmt, 1, SQL_DESC_UNNAMED) == SQL_NAMED);
}

TEST_CASE("SQLColAttribute returns correct attributes for NUMBER.", "[query][col_attribute]") {
  // Given A table with a NUMBER(10,2) column is queried
  Connection conn;
  Schema::use_temp_session_schema(conn);
  conn.execute("CREATE TEMPORARY TABLE t (val NUMBER(10,2))");
  auto stmt = conn.execute("SELECT val FROM t");

  // When SQLColAttribute is called for each descriptor field
  CHECK(get_numeric_attr(stmt, 1, SQL_DESC_CONCISE_TYPE) == SQL_DECIMAL);
  CHECK(get_numeric_attr(stmt, 1, SQL_DESC_TYPE) == SQL_DECIMAL);
  // Then All metadata attributes should match expected values for NUMBER
  CHECK(get_string_attr(stmt, 1, SQL_DESC_TYPE_NAME) == "DECIMAL");
  CHECK(get_string_attr(stmt, 1, SQL_DESC_LOCAL_TYPE_NAME) == "DECIMAL");
  CHECK(get_numeric_attr(stmt, 1, SQL_DESC_LENGTH) == 10);
  CHECK(get_numeric_attr(stmt, 1, SQL_DESC_PRECISION) == 10);
  CHECK(get_numeric_attr(stmt, 1, SQL_DESC_SCALE) == 2);
  CHECK(get_numeric_attr(stmt, 1, SQL_DESC_DISPLAY_SIZE) == 136);
  CHECK(get_numeric_attr(stmt, 1, SQL_DESC_OCTET_LENGTH) == 136);
  CHECK(get_numeric_attr(stmt, 1, SQL_DESC_NUM_PREC_RADIX) == 10);
  CHECK(get_numeric_attr(stmt, 1, SQL_DESC_UNSIGNED) == SQL_FALSE);
  CHECK(get_numeric_attr(stmt, 1, SQL_DESC_CASE_SENSITIVE) == SQL_FALSE);
  CHECK(get_string_attr(stmt, 1, SQL_DESC_LITERAL_PREFIX) == "");
  CHECK(get_string_attr(stmt, 1, SQL_DESC_LITERAL_SUFFIX) == "");
  CHECK(get_numeric_attr(stmt, 1, SQL_DESC_SEARCHABLE) == SQL_ALL_EXCEPT_LIKE);
  CHECK(get_numeric_attr(stmt, 1, SQL_DESC_UPDATABLE) == SQL_ATTR_READWRITE_UNKNOWN);
  CHECK(get_numeric_attr(stmt, 1, SQL_DESC_AUTO_UNIQUE_VALUE) == SQL_FALSE);
  CHECK(get_numeric_attr(stmt, 1, SQL_DESC_FIXED_PREC_SCALE) == SQL_FALSE);
  CHECK(get_numeric_attr(stmt, 1, SQL_DESC_NULLABLE) == SQL_NULLABLE);
  CHECK(get_numeric_attr(stmt, 1, SQL_DESC_UNNAMED) == SQL_NAMED);
}

TEST_CASE("SQLColAttribute returns correct attributes for BOOLEAN.", "[query][col_attribute]") {
  // Given A table with a BOOLEAN column is queried
  Connection conn;
  Schema::use_temp_session_schema(conn);
  conn.execute("CREATE TEMPORARY TABLE t (val BOOLEAN)");
  auto stmt = conn.execute("SELECT val FROM t");

  // When SQLColAttribute is called for each descriptor field
  CHECK(get_numeric_attr(stmt, 1, SQL_DESC_CONCISE_TYPE) == SQL_BIT);
  CHECK(get_numeric_attr(stmt, 1, SQL_DESC_TYPE) == SQL_BIT);
  // Then All metadata attributes should match expected values for BOOLEAN
  CHECK(get_string_attr(stmt, 1, SQL_DESC_TYPE_NAME) == "BIT");
  CHECK(get_string_attr(stmt, 1, SQL_DESC_LOCAL_TYPE_NAME) == "BIT");
  CHECK(get_numeric_attr(stmt, 1, SQL_DESC_LENGTH) == 1);
  CHECK(get_numeric_attr(stmt, 1, SQL_DESC_PRECISION) == 1);
  CHECK(get_numeric_attr(stmt, 1, SQL_DESC_SCALE) == 0);
  CHECK(get_numeric_attr(stmt, 1, SQL_DESC_DISPLAY_SIZE) == 1);
  CHECK(get_numeric_attr(stmt, 1, SQL_DESC_OCTET_LENGTH) == 1);
  CHECK(get_numeric_attr(stmt, 1, SQL_DESC_NUM_PREC_RADIX) == 0);
  CHECK(get_numeric_attr(stmt, 1, SQL_DESC_UNSIGNED) == SQL_TRUE);
  CHECK(get_numeric_attr(stmt, 1, SQL_DESC_CASE_SENSITIVE) == SQL_FALSE);
  CHECK(get_string_attr(stmt, 1, SQL_DESC_LITERAL_PREFIX) == "");
  CHECK(get_string_attr(stmt, 1, SQL_DESC_LITERAL_SUFFIX) == "");
  CHECK(get_numeric_attr(stmt, 1, SQL_DESC_SEARCHABLE) == SQL_ALL_EXCEPT_LIKE);
  CHECK(get_numeric_attr(stmt, 1, SQL_DESC_UPDATABLE) == SQL_ATTR_READWRITE_UNKNOWN);
  CHECK(get_numeric_attr(stmt, 1, SQL_DESC_AUTO_UNIQUE_VALUE) == SQL_FALSE);
  CHECK(get_numeric_attr(stmt, 1, SQL_DESC_FIXED_PREC_SCALE) == SQL_FALSE);
  CHECK(get_numeric_attr(stmt, 1, SQL_DESC_NULLABLE) == SQL_NULLABLE);
  CHECK(get_numeric_attr(stmt, 1, SQL_DESC_UNNAMED) == SQL_NAMED);
}

TEST_CASE("SQLColAttribute returns correct attributes for DATE.", "[query][col_attribute]") {
  // Given A table with a DATE column is queried
  Connection conn;
  Schema::use_temp_session_schema(conn);
  conn.execute("CREATE TEMPORARY TABLE t (val DATE)");
  auto stmt = conn.execute("SELECT val FROM t");

  // When SQLColAttribute is called for each descriptor field
  CHECK(get_numeric_attr(stmt, 1, SQL_DESC_CONCISE_TYPE) == SQL_TYPE_DATE);
  CHECK(get_numeric_attr(stmt, 1, SQL_DESC_TYPE) == SQL_DATETIME);
  // Then All metadata attributes should match expected values for DATE
  CHECK(get_string_attr(stmt, 1, SQL_DESC_TYPE_NAME) == "TYPE_DATE");
  CHECK(get_string_attr(stmt, 1, SQL_DESC_LOCAL_TYPE_NAME) == "TYPE_DATE");
  CHECK(get_numeric_attr(stmt, 1, SQL_DESC_LENGTH) == 10);
  CHECK(get_numeric_attr(stmt, 1, SQL_DESC_PRECISION) == 0);
  CHECK(get_numeric_attr(stmt, 1, SQL_DESC_SCALE) == 0);
  CHECK(get_numeric_attr(stmt, 1, SQL_DESC_DISPLAY_SIZE) == 10);
  CHECK(get_numeric_attr(stmt, 1, SQL_DESC_OCTET_LENGTH) == 6);
  CHECK(get_numeric_attr(stmt, 1, SQL_DESC_NUM_PREC_RADIX) == 0);
  CHECK(get_numeric_attr(stmt, 1, SQL_DESC_UNSIGNED) == SQL_TRUE);
  CHECK(get_numeric_attr(stmt, 1, SQL_DESC_CASE_SENSITIVE) == SQL_FALSE);
  CHECK(get_string_attr(stmt, 1, SQL_DESC_LITERAL_PREFIX) == "'");
  CHECK(get_string_attr(stmt, 1, SQL_DESC_LITERAL_SUFFIX) == "'");
  CHECK(get_numeric_attr(stmt, 1, SQL_DESC_SEARCHABLE) == SQL_ALL_EXCEPT_LIKE);
  CHECK(get_numeric_attr(stmt, 1, SQL_DESC_UPDATABLE) == SQL_ATTR_READWRITE_UNKNOWN);
  CHECK(get_numeric_attr(stmt, 1, SQL_DESC_AUTO_UNIQUE_VALUE) == SQL_FALSE);
  CHECK(get_numeric_attr(stmt, 1, SQL_DESC_FIXED_PREC_SCALE) == SQL_FALSE);
  CHECK(get_numeric_attr(stmt, 1, SQL_DESC_NULLABLE) == SQL_NULLABLE);
  CHECK(get_numeric_attr(stmt, 1, SQL_DESC_UNNAMED) == SQL_NAMED);
}

TEST_CASE("SQLColAttribute returns correct attributes for TIME.", "[query][col_attribute]") {
  // Given A table with a TIME(9) column is queried
  Connection conn;
  Schema::use_temp_session_schema(conn);
  conn.execute("CREATE TEMPORARY TABLE t (val TIME(9))");
  auto stmt = conn.execute("SELECT val FROM t");

  // When SQLColAttribute is called for each descriptor field
  CHECK(get_numeric_attr(stmt, 1, SQL_DESC_CONCISE_TYPE) == SQL_TYPE_TIME);
  CHECK(get_numeric_attr(stmt, 1, SQL_DESC_TYPE) == SQL_DATETIME);
  // Then All metadata attributes should match expected values for TIME
  CHECK(get_string_attr(stmt, 1, SQL_DESC_TYPE_NAME) == "TYPE_TIME");
  CHECK(get_string_attr(stmt, 1, SQL_DESC_LOCAL_TYPE_NAME) == "TYPE_TIME");
  CHECK(get_numeric_attr(stmt, 1, SQL_DESC_DISPLAY_SIZE) == 18);
  CHECK(get_numeric_attr(stmt, 1, SQL_DESC_OCTET_LENGTH) == 6);
  CHECK(get_numeric_attr(stmt, 1, SQL_DESC_NUM_PREC_RADIX) == 0);
  CHECK(get_numeric_attr(stmt, 1, SQL_DESC_UNSIGNED) == SQL_TRUE);
  CHECK(get_numeric_attr(stmt, 1, SQL_DESC_CASE_SENSITIVE) == SQL_FALSE);
  CHECK(get_string_attr(stmt, 1, SQL_DESC_LITERAL_PREFIX) == "'");
  CHECK(get_string_attr(stmt, 1, SQL_DESC_LITERAL_SUFFIX) == "'");
  CHECK(get_numeric_attr(stmt, 1, SQL_DESC_SEARCHABLE) == SQL_ALL_EXCEPT_LIKE);
  CHECK(get_numeric_attr(stmt, 1, SQL_DESC_UPDATABLE) == SQL_ATTR_READWRITE_UNKNOWN);
  CHECK(get_numeric_attr(stmt, 1, SQL_DESC_AUTO_UNIQUE_VALUE) == SQL_FALSE);
  CHECK(get_numeric_attr(stmt, 1, SQL_DESC_FIXED_PREC_SCALE) == SQL_FALSE);
  CHECK(get_numeric_attr(stmt, 1, SQL_DESC_NULLABLE) == SQL_NULLABLE);
  CHECK(get_numeric_attr(stmt, 1, SQL_DESC_UNNAMED) == SQL_NAMED);
}

TEST_CASE("SQLColAttribute returns correct attributes for TIMESTAMP_NTZ.", "[query][col_attribute]") {
  // Given A table with a TIMESTAMP_NTZ(9) column is queried
  Connection conn;
  Schema::use_temp_session_schema(conn);
  conn.execute("CREATE TEMPORARY TABLE t (val TIMESTAMP_NTZ(9))");
  auto stmt = conn.execute("SELECT val FROM t");

  // When SQLColAttribute is called for each descriptor field
  CHECK(get_numeric_attr(stmt, 1, SQL_DESC_CONCISE_TYPE) == SQL_TYPE_TIMESTAMP);
  CHECK(get_numeric_attr(stmt, 1, SQL_DESC_TYPE) == SQL_DATETIME);
  // Then All metadata attributes should match expected values for TIMESTAMP_NTZ
  CHECK(get_string_attr(stmt, 1, SQL_DESC_TYPE_NAME) == "TYPE_TIMESTAMP");
  CHECK(get_string_attr(stmt, 1, SQL_DESC_LOCAL_TYPE_NAME) == "TYPE_TIMESTAMP");
  CHECK(get_numeric_attr(stmt, 1, SQL_DESC_DISPLAY_SIZE) == 29);
  CHECK(get_numeric_attr(stmt, 1, SQL_DESC_OCTET_LENGTH) == 16);
  CHECK(get_numeric_attr(stmt, 1, SQL_DESC_NUM_PREC_RADIX) == 0);
  CHECK(get_numeric_attr(stmt, 1, SQL_DESC_UNSIGNED) == SQL_TRUE);
  CHECK(get_numeric_attr(stmt, 1, SQL_DESC_CASE_SENSITIVE) == SQL_FALSE);
  CHECK(get_string_attr(stmt, 1, SQL_DESC_LITERAL_PREFIX) == "'");
  CHECK(get_string_attr(stmt, 1, SQL_DESC_LITERAL_SUFFIX) == "'");
  CHECK(get_numeric_attr(stmt, 1, SQL_DESC_SEARCHABLE) == SQL_ALL_EXCEPT_LIKE);
  CHECK(get_numeric_attr(stmt, 1, SQL_DESC_UPDATABLE) == SQL_ATTR_READWRITE_UNKNOWN);
  CHECK(get_numeric_attr(stmt, 1, SQL_DESC_AUTO_UNIQUE_VALUE) == SQL_FALSE);
  CHECK(get_numeric_attr(stmt, 1, SQL_DESC_FIXED_PREC_SCALE) == SQL_FALSE);
  CHECK(get_numeric_attr(stmt, 1, SQL_DESC_NULLABLE) == SQL_NULLABLE);
  CHECK(get_numeric_attr(stmt, 1, SQL_DESC_UNNAMED) == SQL_NAMED);
}

TEST_CASE("SQLColAttribute returns correct attributes for TIMESTAMP_LTZ.", "[query][col_attribute]") {
  // Given A table with a TIMESTAMP_LTZ(9) column is queried
  Connection conn;
  Schema::use_temp_session_schema(conn);
  conn.execute("CREATE TEMPORARY TABLE t (val TIMESTAMP_LTZ(9))");
  auto stmt = conn.execute("SELECT val FROM t");

  // When SQLColAttribute is called for each descriptor field
  CHECK(get_numeric_attr(stmt, 1, SQL_DESC_CONCISE_TYPE) == SQL_TYPE_TIMESTAMP);
  CHECK(get_numeric_attr(stmt, 1, SQL_DESC_TYPE) == SQL_DATETIME);
  // Then All metadata attributes should match expected values for TIMESTAMP_LTZ
  CHECK(get_string_attr(stmt, 1, SQL_DESC_TYPE_NAME) == "TYPE_TIMESTAMP");
  CHECK(get_string_attr(stmt, 1, SQL_DESC_LOCAL_TYPE_NAME) == "TYPE_TIMESTAMP");
  CHECK(get_numeric_attr(stmt, 1, SQL_DESC_DISPLAY_SIZE) == 29);
  CHECK(get_numeric_attr(stmt, 1, SQL_DESC_OCTET_LENGTH) == 16);
  CHECK(get_numeric_attr(stmt, 1, SQL_DESC_NUM_PREC_RADIX) == 0);
  CHECK(get_numeric_attr(stmt, 1, SQL_DESC_UNSIGNED) == SQL_TRUE);
  CHECK(get_numeric_attr(stmt, 1, SQL_DESC_CASE_SENSITIVE) == SQL_FALSE);
  CHECK(get_string_attr(stmt, 1, SQL_DESC_LITERAL_PREFIX) == "'");
  CHECK(get_string_attr(stmt, 1, SQL_DESC_LITERAL_SUFFIX) == "'");
  CHECK(get_numeric_attr(stmt, 1, SQL_DESC_SEARCHABLE) == SQL_ALL_EXCEPT_LIKE);
  CHECK(get_numeric_attr(stmt, 1, SQL_DESC_UPDATABLE) == SQL_ATTR_READWRITE_UNKNOWN);
  CHECK(get_numeric_attr(stmt, 1, SQL_DESC_AUTO_UNIQUE_VALUE) == SQL_FALSE);
  CHECK(get_numeric_attr(stmt, 1, SQL_DESC_FIXED_PREC_SCALE) == SQL_FALSE);
  CHECK(get_numeric_attr(stmt, 1, SQL_DESC_NULLABLE) == SQL_NULLABLE);
  CHECK(get_numeric_attr(stmt, 1, SQL_DESC_UNNAMED) == SQL_NAMED);
}

TEST_CASE("SQLColAttribute returns correct attributes for TIMESTAMP_TZ.", "[query][col_attribute]") {
  // Given A table with a TIMESTAMP_TZ(9) column is queried
  Connection conn;
  Schema::use_temp_session_schema(conn);
  conn.execute("CREATE TEMPORARY TABLE t (val TIMESTAMP_TZ(9))");
  auto stmt = conn.execute("SELECT val FROM t");

  // When SQLColAttribute is called for each descriptor field
  CHECK(get_numeric_attr(stmt, 1, SQL_DESC_CONCISE_TYPE) == SQL_TYPE_TIMESTAMP);
  CHECK(get_numeric_attr(stmt, 1, SQL_DESC_TYPE) == SQL_DATETIME);
  // Then All metadata attributes should match expected values for TIMESTAMP_TZ
  CHECK(get_string_attr(stmt, 1, SQL_DESC_TYPE_NAME) == "TYPE_TIMESTAMP");
  CHECK(get_string_attr(stmt, 1, SQL_DESC_LOCAL_TYPE_NAME) == "TYPE_TIMESTAMP");
  CHECK(get_numeric_attr(stmt, 1, SQL_DESC_DISPLAY_SIZE) == 29);
  CHECK(get_numeric_attr(stmt, 1, SQL_DESC_OCTET_LENGTH) == 16);
  CHECK(get_numeric_attr(stmt, 1, SQL_DESC_NUM_PREC_RADIX) == 0);
  CHECK(get_numeric_attr(stmt, 1, SQL_DESC_UNSIGNED) == SQL_TRUE);
  CHECK(get_numeric_attr(stmt, 1, SQL_DESC_CASE_SENSITIVE) == SQL_FALSE);
  CHECK(get_string_attr(stmt, 1, SQL_DESC_LITERAL_PREFIX) == "'");
  CHECK(get_string_attr(stmt, 1, SQL_DESC_LITERAL_SUFFIX) == "'");
  CHECK(get_numeric_attr(stmt, 1, SQL_DESC_SEARCHABLE) == SQL_ALL_EXCEPT_LIKE);
  CHECK(get_numeric_attr(stmt, 1, SQL_DESC_UPDATABLE) == SQL_ATTR_READWRITE_UNKNOWN);
  CHECK(get_numeric_attr(stmt, 1, SQL_DESC_AUTO_UNIQUE_VALUE) == SQL_FALSE);
  CHECK(get_numeric_attr(stmt, 1, SQL_DESC_FIXED_PREC_SCALE) == SQL_FALSE);
  CHECK(get_numeric_attr(stmt, 1, SQL_DESC_NULLABLE) == SQL_NULLABLE);
  CHECK(get_numeric_attr(stmt, 1, SQL_DESC_UNNAMED) == SQL_NAMED);
}

TEST_CASE("SQLColAttribute returns correct attributes for BINARY.", "[query][col_attribute]") {
  // Given A table with a BINARY column is queried
  Connection conn;
  Schema::use_temp_session_schema(conn);
  conn.execute("CREATE TEMPORARY TABLE t (val BINARY)");
  auto stmt = conn.execute("SELECT val FROM t");

  // When SQLColAttribute is called for each descriptor field
  CHECK(get_numeric_attr(stmt, 1, SQL_DESC_CONCISE_TYPE) == SQL_BINARY);
  CHECK(get_numeric_attr(stmt, 1, SQL_DESC_TYPE) == SQL_BINARY);
  // Then All metadata attributes should match expected values for BINARY
  CHECK(get_string_attr(stmt, 1, SQL_DESC_TYPE_NAME) == "BINARY");
  CHECK(get_string_attr(stmt, 1, SQL_DESC_LOCAL_TYPE_NAME) == "BINARY");
  CHECK(get_numeric_attr(stmt, 1, SQL_DESC_LENGTH) == 8388608);
  CHECK(get_numeric_attr(stmt, 1, SQL_DESC_PRECISION) == 0);
  CHECK(get_numeric_attr(stmt, 1, SQL_DESC_SCALE) == 0);
  CHECK(get_numeric_attr(stmt, 1, SQL_DESC_DISPLAY_SIZE) == 2 * 8388608);
  CHECK(get_numeric_attr(stmt, 1, SQL_DESC_OCTET_LENGTH) == 8388608);
  CHECK(get_numeric_attr(stmt, 1, SQL_DESC_NUM_PREC_RADIX) == 0);
  CHECK(get_numeric_attr(stmt, 1, SQL_DESC_UNSIGNED) == SQL_TRUE);
  CHECK(get_numeric_attr(stmt, 1, SQL_DESC_CASE_SENSITIVE) == SQL_FALSE);
  CHECK(get_string_attr(stmt, 1, SQL_DESC_LITERAL_PREFIX) == "0x");
  CHECK(get_string_attr(stmt, 1, SQL_DESC_LITERAL_SUFFIX) == "");
  CHECK(get_numeric_attr(stmt, 1, SQL_DESC_SEARCHABLE) == SQL_ALL_EXCEPT_LIKE);
  CHECK(get_numeric_attr(stmt, 1, SQL_DESC_UPDATABLE) == SQL_ATTR_READWRITE_UNKNOWN);
  CHECK(get_numeric_attr(stmt, 1, SQL_DESC_AUTO_UNIQUE_VALUE) == SQL_FALSE);
  CHECK(get_numeric_attr(stmt, 1, SQL_DESC_FIXED_PREC_SCALE) == SQL_FALSE);
  CHECK(get_numeric_attr(stmt, 1, SQL_DESC_NULLABLE) == SQL_NULLABLE);
  CHECK(get_numeric_attr(stmt, 1, SQL_DESC_UNNAMED) == SQL_NAMED);
}

TEST_CASE("SQLColAttribute returns correct attributes for BINARY with explicit size.", "[query][col_attribute]") {
  // Given A table with a BINARY(100) column is queried
  Connection conn;
  Schema::use_temp_session_schema(conn);
  conn.execute("CREATE TEMPORARY TABLE t (val BINARY(100))");
  auto stmt = conn.execute("SELECT val FROM t");

  // When SQLColAttribute is called for size-dependent descriptor fields
  CHECK(get_numeric_attr(stmt, 1, SQL_DESC_CONCISE_TYPE) == SQL_BINARY);
  CHECK(get_numeric_attr(stmt, 1, SQL_DESC_TYPE) == SQL_BINARY);
  // Then Size-related attributes should reflect the declared BINARY(100) size
  CHECK(get_numeric_attr(stmt, 1, SQL_DESC_LENGTH) == 100);
  CHECK(get_numeric_attr(stmt, 1, SQL_DESC_DISPLAY_SIZE) == 200);
  CHECK(get_numeric_attr(stmt, 1, SQL_DESC_OCTET_LENGTH) == 100);
}

TEST_CASE("SQLColAttribute returns correct attributes for FLOAT.", "[query][col_attribute]") {
  // Given A table with a FLOAT column is queried
  Connection conn;
  Schema::use_temp_session_schema(conn);
  conn.execute("CREATE TEMPORARY TABLE t (val FLOAT)");
  auto stmt = conn.execute("SELECT val FROM t");

  // When SQLColAttribute is called for each descriptor field
  CHECK(get_numeric_attr(stmt, 1, SQL_DESC_CONCISE_TYPE) == SQL_DOUBLE);
  CHECK(get_numeric_attr(stmt, 1, SQL_DESC_TYPE) == SQL_DOUBLE);
  // Then All metadata attributes should match expected values for FLOAT
  CHECK(get_string_attr(stmt, 1, SQL_DESC_TYPE_NAME) == "DOUBLE");
  CHECK(get_string_attr(stmt, 1, SQL_DESC_LOCAL_TYPE_NAME) == "DOUBLE");
  CHECK(get_numeric_attr(stmt, 1, SQL_DESC_DISPLAY_SIZE) == 24);
  CHECK(get_numeric_attr(stmt, 1, SQL_DESC_OCTET_LENGTH) == 8);
  CHECK(get_numeric_attr(stmt, 1, SQL_DESC_NUM_PREC_RADIX) == 2);
  CHECK(get_numeric_attr(stmt, 1, SQL_DESC_UNSIGNED) == SQL_FALSE);
  CHECK(get_numeric_attr(stmt, 1, SQL_DESC_CASE_SENSITIVE) == SQL_FALSE);
  CHECK(get_string_attr(stmt, 1, SQL_DESC_LITERAL_PREFIX) == "");
  CHECK(get_string_attr(stmt, 1, SQL_DESC_LITERAL_SUFFIX) == "");
  CHECK(get_numeric_attr(stmt, 1, SQL_DESC_SEARCHABLE) == SQL_ALL_EXCEPT_LIKE);
  CHECK(get_numeric_attr(stmt, 1, SQL_DESC_UPDATABLE) == SQL_ATTR_READWRITE_UNKNOWN);
  CHECK(get_numeric_attr(stmt, 1, SQL_DESC_AUTO_UNIQUE_VALUE) == SQL_FALSE);
  CHECK(get_numeric_attr(stmt, 1, SQL_DESC_FIXED_PREC_SCALE) == SQL_FALSE);
  CHECK(get_numeric_attr(stmt, 1, SQL_DESC_NULLABLE) == SQL_NULLABLE);
  CHECK(get_numeric_attr(stmt, 1, SQL_DESC_UNNAMED) == SQL_NAMED);
}

TEST_CASE("SQLColAttribute returns correct attributes for DECFLOAT.", "[query][col_attribute]") {
  // Given A table with a DECFLOAT column is queried
  Connection conn;
  Schema::use_temp_session_schema(conn);
  conn.execute("CREATE TEMPORARY TABLE t (val DECFLOAT)");
  auto stmt = conn.execute("SELECT val FROM t");

  // When SQLColAttribute is called for each descriptor field
  CHECK(get_numeric_attr(stmt, 1, SQL_DESC_CONCISE_TYPE) == SQL_NUMERIC);
  CHECK(get_numeric_attr(stmt, 1, SQL_DESC_TYPE) == SQL_NUMERIC);
  // Then All metadata attributes should match expected values for DECFLOAT
  CHECK(get_string_attr(stmt, 1, SQL_DESC_TYPE_NAME) == "NUMERIC");
  CHECK(get_string_attr(stmt, 1, SQL_DESC_LOCAL_TYPE_NAME) == "NUMERIC");
  CHECK(get_numeric_attr(stmt, 1, SQL_DESC_DISPLAY_SIZE) == 136);
  CHECK(get_numeric_attr(stmt, 1, SQL_DESC_OCTET_LENGTH) == 136);
  CHECK(get_numeric_attr(stmt, 1, SQL_DESC_NUM_PREC_RADIX) == 10);
  CHECK(get_numeric_attr(stmt, 1, SQL_DESC_UNSIGNED) == SQL_FALSE);
  CHECK(get_numeric_attr(stmt, 1, SQL_DESC_CASE_SENSITIVE) == SQL_FALSE);
  CHECK(get_string_attr(stmt, 1, SQL_DESC_LITERAL_PREFIX) == "");
  CHECK(get_string_attr(stmt, 1, SQL_DESC_LITERAL_SUFFIX) == "");
  CHECK(get_numeric_attr(stmt, 1, SQL_DESC_SEARCHABLE) == SQL_ALL_EXCEPT_LIKE);
  CHECK(get_numeric_attr(stmt, 1, SQL_DESC_UPDATABLE) == SQL_ATTR_READWRITE_UNKNOWN);
  CHECK(get_numeric_attr(stmt, 1, SQL_DESC_AUTO_UNIQUE_VALUE) == SQL_FALSE);
  CHECK(get_numeric_attr(stmt, 1, SQL_DESC_FIXED_PREC_SCALE) == SQL_FALSE);
  CHECK(get_numeric_attr(stmt, 1, SQL_DESC_NULLABLE) == SQL_NULLABLE);
  CHECK(get_numeric_attr(stmt, 1, SQL_DESC_UNNAMED) == SQL_NAMED);
}

// =============================================================================
// Nullable behavior
// =============================================================================

TEST_CASE("SQLColAttribute returns SQL_NULLABLE for nullable column.", "[query][col_attribute]") {
  // Given A table with a nullable column is queried
  Connection conn;
  Schema::use_temp_session_schema(conn);
  conn.execute("CREATE TEMPORARY TABLE t (val VARCHAR(50))");
  auto stmt = conn.execute("SELECT val FROM t");

  // When SQLColAttribute is called with SQL_DESC_NULLABLE
  SQLLEN nullable = get_numeric_attr(stmt, 1, SQL_DESC_NULLABLE);
  // Then The call should succeed and return SQL_NULLABLE
  CHECK(nullable == SQL_NULLABLE);
}

TEST_CASE("SQLColAttribute returns SQL_NO_NULLS for NOT NULL column.", "[query][col_attribute]") {
  // Given A table with a NOT NULL column is queried
  Connection conn;
  Schema::use_temp_session_schema(conn);
  conn.execute("CREATE TEMPORARY TABLE t (val VARCHAR(50) NOT NULL)");
  auto stmt = conn.execute("SELECT val FROM t");

  // When SQLColAttribute is called with SQL_DESC_NULLABLE
  SQLLEN nullable = get_numeric_attr(stmt, 1, SQL_DESC_NULLABLE);
  // Then The call should succeed and return SQL_NO_NULLS
  CHECK(nullable == SQL_NO_NULLS);
}

// =============================================================================
// Column name and label
// =============================================================================

TEST_CASE("SQLColAttribute returns column name via SQL_DESC_NAME.", "[query][col_attribute]") {
  // Given A query with a named column is executed
  Connection conn;
  auto stmt = conn.execute("SELECT 42 AS MY_COLUMN");

  // When SQLColAttribute is called with SQL_DESC_NAME
  SQLCHAR buffer[128] = {0};
  SQLSMALLINT str_len = 0;
  SQLRETURN ret = SQLColAttribute(stmt.getHandle(), 1, SQL_DESC_NAME, buffer, sizeof(buffer), &str_len, NULL);

  // Then The call should succeed and return the column name
  REQUIRE_THAT(OdbcResult(ret, stmt), OdbcMatchers::IsSuccess());
  CHECK(std::string(reinterpret_cast<char*>(buffer)) == "MY_COLUMN");
  CHECK(str_len == 9);
}

TEST_CASE("SQLColAttribute returns column label via SQL_DESC_LABEL.", "[query][col_attribute]") {
  // Given A query with a labeled column is executed
  Connection conn;
  auto stmt = conn.execute("SELECT 42 AS MY_LABEL");

  // When SQLColAttribute is called with SQL_DESC_LABEL
  std::string label = get_string_attr(stmt, 1, SQL_DESC_LABEL);

  // Then The call should succeed and return the column label
  CHECK(label == "MY_LABEL");
}

TEST_CASE("SQLColAttribute returns empty table/schema/catalog names.", "[query][col_attribute]") {
  // Given A query is executed
  Connection conn;
  auto stmt = conn.execute("SELECT 42 AS val");

  // When SQLColAttribute is called with SQL_DESC_TABLE_NAME, SQL_DESC_BASE_TABLE_NAME, SQL_DESC_SCHEMA_NAME,
  // SQL_DESC_CATALOG_NAME
  CHECK(get_string_attr(stmt, 1, SQL_DESC_TABLE_NAME) == "");
  // Then Each should return an empty string
  CHECK(get_string_attr(stmt, 1, SQL_DESC_BASE_TABLE_NAME) == "");
  CHECK(get_string_attr(stmt, 1, SQL_DESC_SCHEMA_NAME) == "");
  CHECK(get_string_attr(stmt, 1, SQL_DESC_CATALOG_NAME) == "");
}

TEST_CASE("SQLColAttribute returns base column name via SQL_DESC_BASE_COLUMN_NAME.", "[query][col_attribute]") {
  // Given A query with a named column is executed
  Connection conn;
  auto stmt = conn.execute("SELECT 42 AS MY_COL");

  // When SQLColAttribute is called with SQL_DESC_BASE_COLUMN_NAME
  std::string base_col = get_string_attr(stmt, 1, SQL_DESC_BASE_COLUMN_NAME);
  // Then The call should succeed and return the column name
  CHECK(base_col == "MY_COL");
}

// =============================================================================
// SQL_DESC_COUNT
// =============================================================================

TEST_CASE("SQLColAttribute returns column count via SQL_DESC_COUNT.", "[query][col_attribute]") {
  // Given A multi-column query is executed
  Connection conn;
  auto stmt = conn.execute("SELECT 1 AS a, 2 AS b, 3 AS c");

  // When SQLColAttribute is called with SQL_DESC_COUNT
  SQLLEN num_attr = 0;
  SQLRETURN ret = SQLColAttribute(stmt.getHandle(), 1, SQL_DESC_COUNT, NULL, 0, NULL, &num_attr);

  // Then The call should succeed and return 3
  REQUIRE_THAT(OdbcResult(ret, stmt), OdbcMatchers::IsSuccess());
  CHECK(num_attr == 3);
}

// =============================================================================
// ODBC 2.x aliases via SQLColAttribute
// =============================================================================

TEST_CASE("SQLColAttribute returns same values for ODBC 2.x aliases as 3.x equivalents.", "[query][col_attribute]") {
  // Given A query with a named column is executed
  Connection conn;
  Schema::use_temp_session_schema(conn);
  conn.execute("CREATE TEMPORARY TABLE t (val VARCHAR(50))");
  auto stmt = conn.execute("SELECT val FROM t");

  // When SQLColAttribute is called with SQL_COLUMN_NAME and SQL_DESC_NAME
  SQLCHAR buf_2x[256] = {0};
  SQLCHAR buf_3x[256] = {0};
  SQLSMALLINT len_2x = 0;
  SQLSMALLINT len_3x = 0;
  SQLLEN num_2x = 0;
  SQLLEN num_3x = 0;

  OLD_DRIVER_ONLY("BD#80") NON_IODBC UNIX_ONLY {
    SQLRETURN ret = SQLColAttribute(stmt.getHandle(), 1, SQL_COLUMN_NAME, buf_2x, sizeof(buf_2x), &len_2x, NULL);
    REQUIRE_THAT(OdbcResult(ret, stmt), OdbcMatchers::IsSuccess());
    CHECK(std::string(reinterpret_cast<char*>(buf_2x)).empty());

    ret = SQLColAttribute(stmt.getHandle(), 1, SQL_COLUMN_NULLABLE, NULL, 0, NULL, &num_2x);
    REQUIRE_THAT(OdbcResult(ret, stmt), OdbcMatchers::IsError() && OdbcMatchers::HasSqlState("HY091"));
  }

  // Then Both should return the same values
  NEW_DRIVER_ONLY("BD#80") {
    SQLRETURN ret = SQLColAttribute(stmt.getHandle(), 1, SQL_COLUMN_NAME, buf_2x, sizeof(buf_2x), &len_2x, NULL);
    REQUIRE_THAT(OdbcResult(ret, stmt), OdbcMatchers::IsSuccess());

    ret = SQLColAttribute(stmt.getHandle(), 1, SQL_DESC_NAME, buf_3x, sizeof(buf_3x), &len_3x, NULL);
    REQUIRE_THAT(OdbcResult(ret, stmt), OdbcMatchers::IsSuccess());

    CHECK(std::string(reinterpret_cast<char*>(buf_2x)) == std::string(reinterpret_cast<char*>(buf_3x)));

    ret = SQLColAttribute(stmt.getHandle(), 1, SQL_COLUMN_NULLABLE, NULL, 0, NULL, &num_2x);
    REQUIRE_THAT(OdbcResult(ret, stmt), OdbcMatchers::IsSuccess());

    ret = SQLColAttribute(stmt.getHandle(), 1, SQL_DESC_NULLABLE, NULL, 0, NULL, &num_3x);
    REQUIRE_THAT(OdbcResult(ret, stmt), OdbcMatchers::IsSuccess());

    CHECK(num_2x == num_3x);
  }
}

// =============================================================================
// Prepared state support
// =============================================================================

TEST_CASE("SQLColAttribute returns column name after SQLPrepare without SQLExecute.", "[query][col_attribute]") {
  // Given A SELECT statement is prepared but not executed
  Connection conn;
  auto stmt = conn.createStatement();
  SQLRETURN ret =
      SQLPrepare(stmt.getHandle(), reinterpret_cast<SQLCHAR*>(const_cast<char*>("SELECT 42 AS MY_COL")), SQL_NTS);
  REQUIRE_THAT(OdbcResult(ret, stmt), OdbcMatchers::IsSuccess());

  // When SQLColAttribute is called with SQL_DESC_NAME
  SQLCHAR buffer[128] = {0};
  SQLSMALLINT str_len = 0;
  ret = SQLColAttribute(stmt.getHandle(), 1, SQL_DESC_NAME, buffer, sizeof(buffer), &str_len, NULL);

  // Then The call should succeed and return the column name
  REQUIRE_THAT(OdbcResult(ret, stmt), OdbcMatchers::IsSuccess());
  CHECK(std::string(reinterpret_cast<char*>(buffer)) == "MY_COL");
}

TEST_CASE("SQLColAttribute returns column count after SQLPrepare without SQLExecute.", "[query][col_attribute]") {
  // Given A multi-column SELECT is prepared but not executed
  Connection conn;
  auto stmt = conn.createStatement();
  SQLRETURN ret =
      SQLPrepare(stmt.getHandle(), reinterpret_cast<SQLCHAR*>(const_cast<char*>("SELECT 1 AS a, 2 AS b")), SQL_NTS);
  REQUIRE_THAT(OdbcResult(ret, stmt), OdbcMatchers::IsSuccess());

  // When SQLColAttribute is called with SQL_DESC_COUNT
  SQLLEN num_attr = 0;
  ret = SQLColAttribute(stmt.getHandle(), 1, SQL_DESC_COUNT, NULL, 0, NULL, &num_attr);

  // Then The call should succeed and return the column count
  REQUIRE_THAT(OdbcResult(ret, stmt), OdbcMatchers::IsSuccess());
  CHECK(num_attr == 2);
}

TEST_CASE("SQLColAttribute returns type after SQLPrepare without SQLExecute.", "[query][col_attribute]") {
  // Given A SELECT returning a numeric literal is prepared but not executed
  Connection conn;
  auto stmt = conn.createStatement();
  SQLRETURN ret = SQLPrepare(stmt.getHandle(), reinterpret_cast<SQLCHAR*>(const_cast<char*>("SELECT 42")), SQL_NTS);
  REQUIRE_THAT(OdbcResult(ret, stmt), OdbcMatchers::IsSuccess());

  // When SQLColAttribute is called with SQL_DESC_TYPE
  SQLLEN num_attr = 0;
  ret = SQLColAttribute(stmt.getHandle(), 1, SQL_DESC_TYPE, NULL, 0, NULL, &num_attr);

  // Then The call should succeed and return the SQL type
  REQUIRE_THAT(OdbcResult(ret, stmt), OdbcMatchers::IsSuccess());
  CHECK(num_attr == SQL_DECIMAL);
}

// =============================================================================
// String truncation
// =============================================================================

TEST_CASE("SQLColAttribute returns SQL_SUCCESS_WITH_INFO with 01004 on string truncation.", "[query][col_attribute]") {
  // Given A query with a named column is executed
  Connection conn;
  auto stmt = conn.execute("SELECT 42 AS MY_COLUMN");

  // When SQLColAttribute is called with a buffer too small for the column name
  SQLCHAR buffer[4] = {0};
  SQLSMALLINT str_len = 0;
  SQLRETURN ret = SQLColAttribute(stmt.getHandle(), 1, SQL_DESC_NAME, buffer, sizeof(buffer), &str_len, NULL);

  // clang-format off
  // Then The call should return SQL_SUCCESS_WITH_INFO with SQLSTATE 01004 and StringLengthPtr should contain the full untruncated length
  REQUIRE_THAT(OdbcResult(ret, stmt), OdbcMatchers::IsSuccessWithInfo() && OdbcMatchers::HasSqlState("01004"));
  // clang-format on
  WINDOWS_ONLY { CHECK(str_len == 18); }
  NON_IODBC UNIX_ONLY { CHECK(str_len == 9); }
  IODBC_ONLY {
    // iODBC reports the truncated length (bytes written) rather than the full untruncated length
    CHECK(str_len == 3);
  }
}

// =============================================================================
// Error cases
// =============================================================================

TEST_CASE("SQLColAttribute returns 07009 for column number 0.", "[query][col_attribute]") {
  // Given A query is executed
  Connection conn;
  auto stmt = conn.execute("SELECT 42 AS value");

  // When SQLColAttribute is called with column number 0
  SQLLEN num_attr = 0;
  SQLRETURN ret = SQLColAttribute(stmt.getHandle(), 0, SQL_DESC_NAME, NULL, 0, NULL, &num_attr);

  // Then The call should return SQL_ERROR with SQLSTATE 07009
  REQUIRE_THAT(OdbcResult(ret, stmt), OdbcMatchers::IsError() && OdbcMatchers::HasSqlState("07009"));
}

TEST_CASE("SQLColAttribute returns 07009 for out-of-range column number.", "[query][col_attribute]") {
  // Given A single-column query is executed
  Connection conn;
  auto stmt = conn.execute("SELECT 42 AS value");

  // When SQLColAttribute is called with a column number beyond the result set
  SQLLEN num_attr = 0;
  SQLRETURN ret = SQLColAttribute(stmt.getHandle(), 100, SQL_DESC_TYPE, NULL, 0, NULL, &num_attr);

  // Then The call should return SQL_ERROR with SQLSTATE 07009
  REQUIRE_THAT(OdbcResult(ret, stmt), OdbcMatchers::IsError() && OdbcMatchers::HasSqlState("07009"));
}

TEST_CASE("SQLColAttribute returns HY010 before prepare or execute.", "[query][col_attribute]") {
  // Given A statement handle exists but no query has been prepared or executed
  Connection conn;
  auto stmt = conn.createStatement();

  // When SQLColAttribute is called without any prepare or execute
  SQLLEN num_attr = 0;
  SQLRETURN ret = SQLColAttribute(stmt.getHandle(), 1, SQL_DESC_TYPE, NULL, 0, NULL, &num_attr);

  // Then The call should return SQL_ERROR with SQLSTATE HY010
  REQUIRE_THAT(OdbcResult(ret, stmt), OdbcMatchers::IsError() && OdbcMatchers::HasSqlState("HY010"));
}

TEST_CASE("SQLColAttribute returns HY091 for unrecognized field identifier.", "[query][col_attribute]") {
  // Given A query is executed
  Connection conn;
  auto stmt = conn.execute("SELECT 42 AS value");

  // When SQLColAttribute is called with an invalid field identifier
  SQLLEN num_attr = 0;
  SQLRETURN ret = SQLColAttribute(stmt.getHandle(), 1, 65535, NULL, 0, NULL, &num_attr);

  // Then The call should return SQL_ERROR with SQLSTATE HY091
  REQUIRE_THAT(OdbcResult(ret, stmt), OdbcMatchers::IsError() && OdbcMatchers::HasSqlState("HY091"));
}

// =============================================================================
// Cross-function consistency: SQLColAttribute vs SQLGetDescField(IRD)
// =============================================================================

TEST_CASE("SQLColAttribute and SQLGetDescField(IRD) return consistent values.", "[query][col_attribute][consistency]") {
  // Given A table with multiple column types is queried
  Connection conn;
  Schema::use_temp_session_schema(conn);
  conn.execute(
      "CREATE TEMPORARY TABLE t ("
      "  str_col VARCHAR(50),"
      "  num_col NUMBER(10,2),"
      "  date_col DATE,"
      "  ts_col TIMESTAMP_NTZ(9),"
      "  bin_col BINARY(100)"
      ")");
  auto stmt = conn.execute("SELECT str_col, num_col, date_col, ts_col, bin_col FROM t");

  // Get the IRD handle
  SQLHDESC ird = nullptr;
  SQLRETURN ret = SQLGetStmtAttr(stmt.getHandle(), SQL_ATTR_IMP_ROW_DESC, &ird, 0, nullptr);
  REQUIRE_ODBC(ret, stmt);

  // When Both functions are called for each column and numeric descriptor field
  for (SQLUSMALLINT col = 1; col <= 5; col++) {
    INFO("Column " << col);

    // Fields that SQLGetDescField returns as SQLSMALLINT
    SQLUSMALLINT smallint_fields[] = {SQL_DESC_TYPE,  SQL_DESC_CONCISE_TYPE, SQL_DESC_NULLABLE, SQL_DESC_PRECISION,
                                      SQL_DESC_SCALE, SQL_DESC_UNSIGNED,     SQL_DESC_UNNAMED};

    for (auto field_id : smallint_fields) {
      INFO("Field " << field_id);

      SQLLEN col_attr_value = 0;
      ret = SQLColAttribute(stmt.getHandle(), col, field_id, NULL, 0, NULL, &col_attr_value);
      REQUIRE_ODBC(ret, stmt);

      SQLSMALLINT desc_field_value = 0;
      ret = SQLGetDescField(ird, col, field_id, &desc_field_value, 0, nullptr);
      REQUIRE_ODBC(ret, stmt);

      CHECK(col_attr_value == static_cast<SQLLEN>(desc_field_value));
    }

    // Fields that SQLGetDescField returns as SQLLEN
    {
      INFO("Field SQL_DESC_OCTET_LENGTH");

      SQLLEN col_attr_value = 0;
      ret = SQLColAttribute(stmt.getHandle(), col, SQL_DESC_OCTET_LENGTH, NULL, 0, NULL, &col_attr_value);
      REQUIRE_ODBC(ret, stmt);

      SQLLEN desc_field_value = 0;
      ret = SQLGetDescField(ird, col, SQL_DESC_OCTET_LENGTH, &desc_field_value, 0, nullptr);
      REQUIRE_ODBC(ret, stmt);

      CHECK(col_attr_value == desc_field_value);
    }

    // String fields: SQL_DESC_NAME, SQL_DESC_TYPE_NAME
    SQLUSMALLINT string_fields[] = {SQL_DESC_NAME, SQL_DESC_TYPE_NAME};
    for (auto field_id : string_fields) {
      INFO("String field " << field_id);

      SQLCHAR col_attr_buf[256] = {0};
      SQLSMALLINT col_attr_len = 0;
      ret = SQLColAttribute(stmt.getHandle(), col, field_id, col_attr_buf, sizeof(col_attr_buf), &col_attr_len, NULL);
      REQUIRE_ODBC(ret, stmt);

      SQLCHAR desc_field_buf[256] = {0};
      SQLINTEGER desc_field_len = 0;
      ret = SQLGetDescField(ird, col, field_id, desc_field_buf, sizeof(desc_field_buf), &desc_field_len);
      REQUIRE_ODBC(ret, stmt);

      // Then Both functions should return the same string
      CHECK(std::string(reinterpret_cast<char*>(col_attr_buf)) == std::string(reinterpret_cast<char*>(desc_field_buf)));
    }
  }
}

// =============================================================================
// Cross-function consistency: SQLColAttribute vs SQLDescribeCol
// =============================================================================

TEST_CASE("SQLColAttribute and SQLDescribeCol return consistent values.", "[query][col_attribute][consistency]") {
  // Given A table with multiple column types is queried
  Connection conn;
  Schema::use_temp_session_schema(conn);
  conn.execute(
      "CREATE TEMPORARY TABLE t ("
      "  str_col VARCHAR(50),"
      "  num_col NUMBER(10,2),"
      "  date_col DATE,"
      "  ts_col TIMESTAMP_NTZ(9),"
      "  bin_col BINARY(100)"
      ")");
  auto stmt = conn.execute("SELECT str_col, num_col, date_col, ts_col, bin_col FROM t");

  // When Both functions are called for each column
  for (SQLUSMALLINT col = 1; col <= 5; col++) {
    INFO("Column " << col);

    // Call SQLDescribeCol
    SQLCHAR desc_name[256] = {0};
    SQLSMALLINT desc_name_len = 0;
    SQLSMALLINT desc_data_type = 0;
    SQLULEN desc_col_size = 0;
    SQLSMALLINT desc_decimal_digits = 0;
    SQLSMALLINT desc_nullable = 0;
    SQLRETURN ret = SQLDescribeCol(stmt.getHandle(), col, desc_name, sizeof(desc_name), &desc_name_len, &desc_data_type,
                                   &desc_col_size, &desc_decimal_digits, &desc_nullable);
    REQUIRE_ODBC(ret, stmt);

    // Call SQLColAttribute for corresponding fields
    std::string col_attr_name = get_string_attr(stmt, col, SQL_DESC_NAME);
    SQLLEN col_attr_concise_type = get_numeric_attr(stmt, col, SQL_DESC_CONCISE_TYPE);
    SQLLEN col_attr_length = get_numeric_attr(stmt, col, SQL_DESC_LENGTH);
    SQLLEN col_attr_scale = get_numeric_attr(stmt, col, SQL_DESC_SCALE);
    SQLLEN col_attr_nullable = get_numeric_attr(stmt, col, SQL_DESC_NULLABLE);

    // Then SQLDescribeCol.ColumnName == SQLColAttribute(SQL_DESC_NAME)
    CHECK(std::string(reinterpret_cast<char*>(desc_name)) == col_attr_name);
    // Then SQLDescribeCol.DataType == SQLColAttribute(SQL_DESC_CONCISE_TYPE)
    CHECK(static_cast<SQLLEN>(desc_data_type) == col_attr_concise_type);
    // Then SQLDescribeCol.ColumnSize == SQLColAttribute(SQL_DESC_LENGTH)
    CHECK(static_cast<SQLLEN>(desc_col_size) == col_attr_length);
    // Then SQLDescribeCol.DecimalDigits == SQLColAttribute(SQL_DESC_SCALE)
    CHECK(static_cast<SQLLEN>(desc_decimal_digits) == col_attr_scale);
    // Then SQLDescribeCol.Nullable == SQLColAttribute(SQL_DESC_NULLABLE)
    CHECK(static_cast<SQLLEN>(desc_nullable) == col_attr_nullable);
  }
}

// =============================================================================
// NUMBER precision/scale variations
// =============================================================================

TEST_CASE("SQLColAttribute returns correct precision/scale for NUMBER(38,0).", "[query][col_attribute]") {
  // Given A table with a NUMBER(38,0) column is queried
  Connection conn;
  Schema::use_temp_session_schema(conn);
  conn.execute("CREATE TEMPORARY TABLE t (val NUMBER(38,0))");
  auto stmt = conn.execute("SELECT val FROM t");

  // When SQLColAttribute is called for precision and scale
  CHECK(get_numeric_attr(stmt, 1, SQL_DESC_CONCISE_TYPE) == SQL_DECIMAL);
  // Then Precision should be 38 and scale 0
  CHECK(get_numeric_attr(stmt, 1, SQL_DESC_PRECISION) == 38);
  CHECK(get_numeric_attr(stmt, 1, SQL_DESC_SCALE) == 0);
  CHECK(get_numeric_attr(stmt, 1, SQL_DESC_LENGTH) == 38);
}

TEST_CASE("SQLColAttribute returns correct precision/scale for NUMBER(1,0).", "[query][col_attribute]") {
  // Given A table with a NUMBER(1,0) column is queried
  Connection conn;
  Schema::use_temp_session_schema(conn);
  conn.execute("CREATE TEMPORARY TABLE t (val NUMBER(1,0))");
  auto stmt = conn.execute("SELECT val FROM t");

  // When SQLColAttribute is called for precision and scale
  CHECK(get_numeric_attr(stmt, 1, SQL_DESC_CONCISE_TYPE) == SQL_DECIMAL);
  // Then Precision should be 1 and scale 0
  CHECK(get_numeric_attr(stmt, 1, SQL_DESC_PRECISION) == 1);
  CHECK(get_numeric_attr(stmt, 1, SQL_DESC_SCALE) == 0);
  CHECK(get_numeric_attr(stmt, 1, SQL_DESC_LENGTH) == 1);
}

TEST_CASE("SQLColAttribute returns correct precision/scale for NUMBER(38,18).", "[query][col_attribute]") {
  // Given A table with a NUMBER(38,18) column is queried
  Connection conn;
  Schema::use_temp_session_schema(conn);
  conn.execute("CREATE TEMPORARY TABLE t (val NUMBER(38,18))");
  auto stmt = conn.execute("SELECT val FROM t");

  // When SQLColAttribute is called for precision and scale
  CHECK(get_numeric_attr(stmt, 1, SQL_DESC_CONCISE_TYPE) == SQL_DECIMAL);
  // Then Precision should be 38 and scale 18
  CHECK(get_numeric_attr(stmt, 1, SQL_DESC_PRECISION) == 38);
  CHECK(get_numeric_attr(stmt, 1, SQL_DESC_SCALE) == 18);
  CHECK(get_numeric_attr(stmt, 1, SQL_DESC_LENGTH) == 38);
}

// =============================================================================
// TIME/TIMESTAMP scale variations
// =============================================================================

TEST_CASE("SQLColAttribute returns correct display size for TIME(0).", "[query][col_attribute]") {
  // Given A table with a TIME(0) column is queried
  Connection conn;
  Schema::use_temp_session_schema(conn);
  conn.execute("CREATE TEMPORARY TABLE t (val TIME(0))");
  auto stmt = conn.execute("SELECT val FROM t");

  // When SQLColAttribute is called for display size
  CHECK(get_numeric_attr(stmt, 1, SQL_DESC_CONCISE_TYPE) == SQL_TYPE_TIME);
  // Then Display size should reflect no fractional seconds (HH:MM:SS = 8)
  CHECK(get_numeric_attr(stmt, 1, SQL_DESC_DISPLAY_SIZE) == 8);
}

TEST_CASE("SQLColAttribute returns correct display size for TIMESTAMP_NTZ(3).", "[query][col_attribute]") {
  // Given A table with a TIMESTAMP_NTZ(3) column is queried
  Connection conn;
  Schema::use_temp_session_schema(conn);
  conn.execute("CREATE TEMPORARY TABLE t (val TIMESTAMP_NTZ(3))");
  auto stmt = conn.execute("SELECT val FROM t");

  // When SQLColAttribute is called for display size
  CHECK(get_numeric_attr(stmt, 1, SQL_DESC_CONCISE_TYPE) == SQL_TYPE_TIMESTAMP);
  // Then Display size should reflect millisecond precision (YYYY-MM-DD HH:MM:SS.fff = 23)
  CHECK(get_numeric_attr(stmt, 1, SQL_DESC_DISPLAY_SIZE) == 23);
}

// =============================================================================
// BINARY/VARCHAR size edge cases
// =============================================================================

TEST_CASE("SQLColAttribute returns correct attributes for BINARY(1).", "[query][col_attribute]") {
  // Given A table with a BINARY(1) column is queried
  Connection conn;
  Schema::use_temp_session_schema(conn);
  conn.execute("CREATE TEMPORARY TABLE t (val BINARY(1))");
  auto stmt = conn.execute("SELECT val FROM t");

  // When SQLColAttribute is called for size-dependent fields
  CHECK(get_numeric_attr(stmt, 1, SQL_DESC_CONCISE_TYPE) == SQL_BINARY);
  // Then Size attributes should reflect 1 byte
  CHECK(get_numeric_attr(stmt, 1, SQL_DESC_LENGTH) == 1);
  CHECK(get_numeric_attr(stmt, 1, SQL_DESC_DISPLAY_SIZE) == 2);
  CHECK(get_numeric_attr(stmt, 1, SQL_DESC_OCTET_LENGTH) == 1);
}

TEST_CASE("SQLColAttribute returns correct attributes for VARCHAR(1).", "[query][col_attribute]") {
  // Given A table with a VARCHAR(1) column is queried
  Connection conn;
  Schema::use_temp_session_schema(conn);
  conn.execute("CREATE TEMPORARY TABLE t (val VARCHAR(1))");
  auto stmt = conn.execute("SELECT val FROM t");

  // When SQLColAttribute is called for size-dependent fields
  CHECK(get_numeric_attr(stmt, 1, SQL_DESC_CONCISE_TYPE) == SQL_VARCHAR);
  // Then Size attributes should reflect 1 character
  CHECK(get_numeric_attr(stmt, 1, SQL_DESC_LENGTH) == 1);
  CHECK(get_numeric_attr(stmt, 1, SQL_DESC_DISPLAY_SIZE) == 1);
  CHECK(get_numeric_attr(stmt, 1, SQL_DESC_PRECISION) == 1);
}

TEST_CASE("SQLColAttribute returns correct attributes for VARCHAR(16777216).", "[query][col_attribute]") {
  // Given A table with a VARCHAR(16777216) column (Snowflake max) is queried
  Connection conn;
  Schema::use_temp_session_schema(conn);
  conn.execute("CREATE TEMPORARY TABLE t (val VARCHAR(16777216))");
  auto stmt = conn.execute("SELECT val FROM t");

  // When SQLColAttribute is called for size-dependent fields
  CHECK(get_numeric_attr(stmt, 1, SQL_DESC_CONCISE_TYPE) == SQL_VARCHAR);
  // Then Size attributes should reflect 16MB characters
  CHECK(get_numeric_attr(stmt, 1, SQL_DESC_LENGTH) == 16777216);
  CHECK(get_numeric_attr(stmt, 1, SQL_DESC_DISPLAY_SIZE) == 16777216);
}

// =============================================================================
// StringLengthPtr validation
// =============================================================================

TEST_CASE("SQLColAttribute returns correct StringLengthPtr for string attributes.", "[query][col_attribute]") {
  // Given A table with multiple column types is queried
  Connection conn;
  Schema::use_temp_session_schema(conn);
  conn.execute(
      "CREATE TEMPORARY TABLE t ("
      "  str_col VARCHAR(50),"
      "  num_col NUMBER(10,2),"
      "  date_col DATE,"
      "  bin_col BINARY(100)"
      ")");
  auto stmt = conn.execute("SELECT str_col, num_col, date_col, bin_col FROM t");

  // When SQL_DESC_TYPE_NAME is queried for each column
  SQLCHAR buffer[256] = {0};
  SQLSMALLINT str_len = 0;
  SQLRETURN ret;
  // Then StringLengthPtr should match the string length

  // VARCHAR -> "VARCHAR" (7 chars)
  str_len = 0;
  ret = SQLColAttribute(stmt.getHandle(), 1, SQL_DESC_TYPE_NAME, buffer, sizeof(buffer), &str_len, NULL);
  REQUIRE_ODBC(ret, stmt);
  CHECK(str_len == 7);

  // NUMBER -> "DECIMAL" (7 chars)
  str_len = 0;
  ret = SQLColAttribute(stmt.getHandle(), 2, SQL_DESC_TYPE_NAME, buffer, sizeof(buffer), &str_len, NULL);
  REQUIRE_ODBC(ret, stmt);
  CHECK(str_len == 7);

  // DATE -> "TYPE_DATE" (9 chars)
  str_len = 0;
  ret = SQLColAttribute(stmt.getHandle(), 3, SQL_DESC_TYPE_NAME, buffer, sizeof(buffer), &str_len, NULL);
  REQUIRE_ODBC(ret, stmt);
  CHECK(str_len == 9);

  // BINARY -> "BINARY" (6 chars)
  str_len = 0;
  ret = SQLColAttribute(stmt.getHandle(), 4, SQL_DESC_TYPE_NAME, buffer, sizeof(buffer), &str_len, NULL);
  REQUIRE_ODBC(ret, stmt);
  CHECK(str_len == 6);

  // Verify SQL_DESC_LITERAL_PREFIX StringLengthPtr:

  // VARCHAR prefix "'" (1 char)
  str_len = 0;
  ret = SQLColAttribute(stmt.getHandle(), 1, SQL_DESC_LITERAL_PREFIX, buffer, sizeof(buffer), &str_len, NULL);
  REQUIRE_ODBC(ret, stmt);
  CHECK(str_len == 1);

  // NUMBER prefix "" (0 chars)
  str_len = 0;
  ret = SQLColAttribute(stmt.getHandle(), 2, SQL_DESC_LITERAL_PREFIX, buffer, sizeof(buffer), &str_len, NULL);
  REQUIRE_ODBC(ret, stmt);
  CHECK(str_len == 0);

  // BINARY prefix "0x" (2 chars)
  str_len = 0;
  ret = SQLColAttribute(stmt.getHandle(), 4, SQL_DESC_LITERAL_PREFIX, buffer, sizeof(buffer), &str_len, NULL);
  REQUIRE_ODBC(ret, stmt);
  CHECK(str_len == 2);
}

// =============================================================================
// Multi-column index validation
// =============================================================================

TEST_CASE("SQLColAttribute returns correct attributes for each column in a multi-column result.",
          "[query][col_attribute]") {
  // Given A table with diverse types is queried
  Connection conn;
  Schema::use_temp_session_schema(conn);
  conn.execute(
      "CREATE TEMPORARY TABLE t ("
      "  c1 VARCHAR(20),"
      "  c2 NUMBER(5,1),"
      "  c3 BOOLEAN,"
      "  c4 DATE,"
      "  c5 TIMESTAMP_NTZ(6),"
      "  c6 FLOAT"
      ")");
  auto stmt = conn.execute("SELECT c1, c2, c3, c4, c5, c6 FROM t");

  // When SQLColAttribute is called for SQL_DESC_CONCISE_TYPE on each column
  SQLLEN count = 0;
  // Then Each column index should return the correct type
  CHECK(get_numeric_attr(stmt, 1, SQL_DESC_CONCISE_TYPE) == SQL_VARCHAR);
  CHECK(get_numeric_attr(stmt, 2, SQL_DESC_CONCISE_TYPE) == SQL_DECIMAL);
  CHECK(get_numeric_attr(stmt, 3, SQL_DESC_CONCISE_TYPE) == SQL_BIT);
  CHECK(get_numeric_attr(stmt, 4, SQL_DESC_CONCISE_TYPE) == SQL_TYPE_DATE);
  CHECK(get_numeric_attr(stmt, 5, SQL_DESC_CONCISE_TYPE) == SQL_TYPE_TIMESTAMP);
  CHECK(get_numeric_attr(stmt, 6, SQL_DESC_CONCISE_TYPE) == SQL_DOUBLE);

  // Verify SQL_DESC_NAME for each column:
  CHECK(get_string_attr(stmt, 1, SQL_DESC_NAME) == "C1");
  CHECK(get_string_attr(stmt, 2, SQL_DESC_NAME) == "C2");
  CHECK(get_string_attr(stmt, 3, SQL_DESC_NAME) == "C3");
  CHECK(get_string_attr(stmt, 4, SQL_DESC_NAME) == "C4");
  CHECK(get_string_attr(stmt, 5, SQL_DESC_NAME) == "C5");
  CHECK(get_string_attr(stmt, 6, SQL_DESC_NAME) == "C6");

  // Verify SQL_DESC_COUNT:
  count = 0;
  SQLRETURN ret = SQLColAttribute(stmt.getHandle(), 1, SQL_DESC_COUNT, NULL, 0, NULL, &count);
  REQUIRE_ODBC(ret, stmt);
  CHECK(count == 6);
}
