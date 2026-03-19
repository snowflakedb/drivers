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
// SQL_DESC_NAME
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

// =============================================================================
// SQL_DESC_LABEL
// =============================================================================

TEST_CASE("SQLColAttribute returns column label via SQL_DESC_LABEL.", "[query][col_attribute]") {
  // Given A query with a labeled column is executed
  Connection conn;
  auto stmt = conn.execute("SELECT 42 AS MY_LABEL");

  // When SQLColAttribute is called with SQL_DESC_LABEL
  SQLCHAR buffer[128] = {0};
  SQLSMALLINT str_len = 0;
  SQLRETURN ret = SQLColAttribute(stmt.getHandle(), 1, SQL_DESC_LABEL, buffer, sizeof(buffer), &str_len, NULL);

  // Then The call should succeed and return the column label
  REQUIRE_THAT(OdbcResult(ret, stmt), OdbcMatchers::IsSuccess());
  CHECK(std::string(reinterpret_cast<char*>(buffer)) == "MY_LABEL");
}

// =============================================================================
// SQL_DESC_TYPE_NAME
// =============================================================================

TEST_CASE("SQLColAttribute returns type name for VARCHAR via SQL_DESC_TYPE_NAME.", "[query][col_attribute]") {
  // Given A table with a VARCHAR column is queried
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);
  conn.execute("CREATE TABLE t (val VARCHAR(100))");
  auto stmt = conn.execute("SELECT val FROM t");

  // When SQLColAttribute is called with SQL_DESC_TYPE_NAME
  std::string type_name = get_string_attr(stmt, 1, SQL_DESC_TYPE_NAME);

  // Then The call should succeed and return a non-empty type name
  CHECK(!type_name.empty());
}

TEST_CASE("SQLColAttribute returns type name for NUMBER via SQL_DESC_TYPE_NAME.", "[query][col_attribute]") {
  // Given A table with a NUMBER column is queried
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);
  conn.execute("CREATE TABLE t (val NUMBER(10,2))");
  auto stmt = conn.execute("SELECT val FROM t");

  // When SQLColAttribute is called with SQL_DESC_TYPE_NAME
  std::string type_name = get_string_attr(stmt, 1, SQL_DESC_TYPE_NAME);

  // Then The call should succeed and return a non-empty type name
  CHECK(!type_name.empty());
}

// =============================================================================
// SQL_DESC_BASE_COLUMN_NAME
// =============================================================================

TEST_CASE("SQLColAttribute returns base column name via SQL_DESC_BASE_COLUMN_NAME.", "[query][col_attribute]") {
  // Given A table column is queried directly
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);
  conn.execute("CREATE TABLE t (col1 INT)");
  auto stmt = conn.execute("SELECT col1 FROM t");

  // When SQLColAttribute is called with SQL_DESC_BASE_COLUMN_NAME
  SQLCHAR buffer[128] = {0};
  SQLSMALLINT str_len = 0;
  SQLRETURN ret =
      SQLColAttribute(stmt.getHandle(), 1, SQL_DESC_BASE_COLUMN_NAME, buffer, sizeof(buffer), &str_len, NULL);

  // Then The call should succeed
  REQUIRE_THAT(OdbcResult(ret, stmt), OdbcMatchers::IsSuccess());
}

// =============================================================================
// SQL_DESC_TABLE_NAME
// =============================================================================

TEST_CASE("SQLColAttribute returns table name via SQL_DESC_TABLE_NAME.", "[query][col_attribute]") {
  // Given A table column is queried directly
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);
  conn.execute("CREATE TABLE t (val INT)");
  auto stmt = conn.execute("SELECT val FROM t");

  // When SQLColAttribute is called with SQL_DESC_TABLE_NAME
  SQLCHAR buffer[256] = {0};
  SQLSMALLINT str_len = 0;
  SQLRETURN ret = SQLColAttribute(stmt.getHandle(), 1, SQL_DESC_TABLE_NAME, buffer, sizeof(buffer), &str_len, NULL);

  // Then The call should succeed
  REQUIRE_THAT(OdbcResult(ret, stmt), OdbcMatchers::IsSuccess());
}

// =============================================================================
// SQL_DESC_BASE_TABLE_NAME
// =============================================================================

TEST_CASE("SQLColAttribute returns base table name via SQL_DESC_BASE_TABLE_NAME.", "[query][col_attribute]") {
  // Given A table column is queried directly
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);
  conn.execute("CREATE TABLE t (val INT)");
  auto stmt = conn.execute("SELECT val FROM t");

  // When SQLColAttribute is called with SQL_DESC_BASE_TABLE_NAME
  SQLCHAR buffer[256] = {0};
  SQLSMALLINT str_len = 0;
  SQLRETURN ret =
      SQLColAttribute(stmt.getHandle(), 1, SQL_DESC_BASE_TABLE_NAME, buffer, sizeof(buffer), &str_len, NULL);

  // Then The call should succeed
  REQUIRE_THAT(OdbcResult(ret, stmt), OdbcMatchers::IsSuccess());
}

// =============================================================================
// SQL_DESC_CATALOG_NAME
// =============================================================================

TEST_CASE("SQLColAttribute returns catalog name via SQL_DESC_CATALOG_NAME.", "[query][col_attribute]") {
  // Given A query is executed
  Connection conn;
  auto stmt = conn.execute("SELECT 42 AS value");

  // When SQLColAttribute is called with SQL_DESC_CATALOG_NAME
  SQLCHAR buffer[256] = {0};
  SQLSMALLINT str_len = 0;
  SQLRETURN ret = SQLColAttribute(stmt.getHandle(), 1, SQL_DESC_CATALOG_NAME, buffer, sizeof(buffer), &str_len, NULL);

  // Then The call should succeed
  REQUIRE_THAT(OdbcResult(ret, stmt), OdbcMatchers::IsSuccess());
}

// =============================================================================
// SQL_DESC_SCHEMA_NAME
// =============================================================================

TEST_CASE("SQLColAttribute returns schema name via SQL_DESC_SCHEMA_NAME.", "[query][col_attribute]") {
  // Given A query is executed
  Connection conn;
  auto stmt = conn.execute("SELECT 42 AS value");

  // When SQLColAttribute is called with SQL_DESC_SCHEMA_NAME
  SQLCHAR buffer[256] = {0};
  SQLSMALLINT str_len = 0;
  SQLRETURN ret = SQLColAttribute(stmt.getHandle(), 1, SQL_DESC_SCHEMA_NAME, buffer, sizeof(buffer), &str_len, NULL);

  // Then The call should succeed
  REQUIRE_THAT(OdbcResult(ret, stmt), OdbcMatchers::IsSuccess());
}

// =============================================================================
// SQL_DESC_LITERAL_PREFIX
// =============================================================================

TEST_CASE("SQLColAttribute returns literal prefix for VARCHAR via SQL_DESC_LITERAL_PREFIX.", "[query][col_attribute]") {
  // Given A table with a VARCHAR column is queried
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);
  conn.execute("CREATE TABLE t (val VARCHAR(50))");
  auto stmt = conn.execute("SELECT val FROM t");

  // When SQLColAttribute is called with SQL_DESC_LITERAL_PREFIX
  SQLCHAR buffer[128] = {0};
  SQLSMALLINT str_len = 0;
  SQLRETURN ret = SQLColAttribute(stmt.getHandle(), 1, SQL_DESC_LITERAL_PREFIX, buffer, sizeof(buffer), &str_len, NULL);

  // Then The call should succeed
  REQUIRE_THAT(OdbcResult(ret, stmt), OdbcMatchers::IsSuccess());
}

// =============================================================================
// SQL_DESC_LITERAL_SUFFIX
// =============================================================================

TEST_CASE("SQLColAttribute returns literal suffix for VARCHAR via SQL_DESC_LITERAL_SUFFIX.", "[query][col_attribute]") {
  // Given A table with a VARCHAR column is queried
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);
  conn.execute("CREATE TABLE t (val VARCHAR(50))");
  auto stmt = conn.execute("SELECT val FROM t");

  // When SQLColAttribute is called with SQL_DESC_LITERAL_SUFFIX
  SQLCHAR buffer[128] = {0};
  SQLSMALLINT str_len = 0;
  SQLRETURN ret = SQLColAttribute(stmt.getHandle(), 1, SQL_DESC_LITERAL_SUFFIX, buffer, sizeof(buffer), &str_len, NULL);

  // Then The call should succeed
  REQUIRE_THAT(OdbcResult(ret, stmt), OdbcMatchers::IsSuccess());
}

// =============================================================================
// SQL_DESC_LOCAL_TYPE_NAME
// =============================================================================

TEST_CASE("SQLColAttribute returns local type name via SQL_DESC_LOCAL_TYPE_NAME.", "[query][col_attribute]") {
  // Given A table with a VARCHAR column is queried
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);
  conn.execute("CREATE TABLE t (val VARCHAR(50))");
  auto stmt = conn.execute("SELECT val FROM t");

  // When SQLColAttribute is called with SQL_DESC_LOCAL_TYPE_NAME
  SQLCHAR buffer[128] = {0};
  SQLSMALLINT str_len = 0;
  SQLRETURN ret =
      SQLColAttribute(stmt.getHandle(), 1, SQL_DESC_LOCAL_TYPE_NAME, buffer, sizeof(buffer), &str_len, NULL);

  // Then The call should succeed
  REQUIRE_THAT(OdbcResult(ret, stmt), OdbcMatchers::IsSuccess());
}

// =============================================================================
// SQL_DESC_TYPE
// =============================================================================

TEST_CASE("SQLColAttribute returns SQL_DECIMAL type for numeric literal.", "[query][col_attribute]") {
  // Given A query returning a numeric literal is executed
  Connection conn;
  auto stmt = conn.execute("SELECT 42");

  // When SQLColAttribute is called with SQL_DESC_TYPE
  SQLLEN num_attr = 0;
  SQLRETURN ret = SQLColAttribute(stmt.getHandle(), 1, SQL_DESC_TYPE, NULL, 0, NULL, &num_attr);

  // Then The call should succeed and return SQL_DECIMAL
  REQUIRE_THAT(OdbcResult(ret, stmt), OdbcMatchers::IsSuccess());
  CHECK(num_attr == SQL_DECIMAL);
}

// =============================================================================
// SQL_DESC_CONCISE_TYPE
// =============================================================================

TEST_CASE("SQLColAttribute returns SQL_VARCHAR concise type for VARCHAR.", "[query][col_attribute]") {
  // Given A table with a VARCHAR column is queried
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);
  conn.execute("CREATE TABLE t (val VARCHAR(100))");
  auto stmt = conn.execute("SELECT val FROM t");

  // When SQLColAttribute is called with SQL_DESC_CONCISE_TYPE
  SQLLEN num_attr = 0;
  SQLRETURN ret = SQLColAttribute(stmt.getHandle(), 1, SQL_DESC_CONCISE_TYPE, NULL, 0, NULL, &num_attr);

  // Then The call should succeed and return SQL_VARCHAR
  REQUIRE_THAT(OdbcResult(ret, stmt), OdbcMatchers::IsSuccess());
  CHECK(num_attr == SQL_VARCHAR);
}

// =============================================================================
// SQL_DESC_NULLABLE
// =============================================================================

TEST_CASE("SQLColAttribute returns SQL_NULLABLE for nullable column.", "[query][col_attribute]") {
  // Given A table with a nullable column is queried
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);
  conn.execute("CREATE TABLE t (val VARCHAR(50))");
  auto stmt = conn.execute("SELECT val FROM t");

  // When SQLColAttribute is called with SQL_DESC_NULLABLE
  SQLLEN num_attr = -1;
  SQLRETURN ret = SQLColAttribute(stmt.getHandle(), 1, SQL_DESC_NULLABLE, NULL, 0, NULL, &num_attr);

  // Then The call should succeed and return SQL_NULLABLE
  REQUIRE_THAT(OdbcResult(ret, stmt), OdbcMatchers::IsSuccess());
  CHECK(num_attr == SQL_NULLABLE);
}

TEST_CASE("SQLColAttribute returns SQL_NO_NULLS for NOT NULL column.", "[query][col_attribute]") {
  // Given A table with a NOT NULL column is queried
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);
  conn.execute("CREATE TABLE t (val VARCHAR(50) NOT NULL)");
  auto stmt = conn.execute("SELECT val FROM t");

  // When SQLColAttribute is called with SQL_DESC_NULLABLE
  SQLLEN num_attr = -1;
  SQLRETURN ret = SQLColAttribute(stmt.getHandle(), 1, SQL_DESC_NULLABLE, NULL, 0, NULL, &num_attr);

  // Then The call should succeed and return SQL_NO_NULLS
  REQUIRE_THAT(OdbcResult(ret, stmt), OdbcMatchers::IsSuccess());
  CHECK(num_attr == SQL_NO_NULLS);
}

// =============================================================================
// SQL_DESC_PRECISION
// =============================================================================

TEST_CASE("SQLColAttribute returns precision for NUMBER(10,2).", "[query][col_attribute]") {
  // Given A table with a NUMBER(10,2) column is queried
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);
  conn.execute("CREATE TABLE t (val NUMBER(10,2))");
  auto stmt = conn.execute("SELECT val FROM t");

  // When SQLColAttribute is called with SQL_DESC_PRECISION
  SQLLEN num_attr = 0;
  SQLRETURN ret = SQLColAttribute(stmt.getHandle(), 1, SQL_DESC_PRECISION, NULL, 0, NULL, &num_attr);

  // Then The call should succeed and return 10
  REQUIRE_THAT(OdbcResult(ret, stmt), OdbcMatchers::IsSuccess());
  CHECK(num_attr == 10);
}

// =============================================================================
// SQL_DESC_SCALE
// =============================================================================

TEST_CASE("SQLColAttribute returns scale for NUMBER(10,4).", "[query][col_attribute]") {
  // Given A table with a NUMBER(10,4) column is queried
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);
  conn.execute("CREATE TABLE t (val NUMBER(10,4))");
  auto stmt = conn.execute("SELECT val FROM t");

  // When SQLColAttribute is called with SQL_DESC_SCALE
  SQLLEN num_attr = 0;
  SQLRETURN ret = SQLColAttribute(stmt.getHandle(), 1, SQL_DESC_SCALE, NULL, 0, NULL, &num_attr);

  // Then The call should succeed and return 4
  REQUIRE_THAT(OdbcResult(ret, stmt), OdbcMatchers::IsSuccess());
  CHECK(num_attr == 4);
}

// =============================================================================
// SQL_DESC_LENGTH
// =============================================================================

TEST_CASE("SQLColAttribute returns length for VARCHAR(200).", "[query][col_attribute]") {
  // Given A table with a VARCHAR(200) column is queried
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);
  conn.execute("CREATE TABLE t (val VARCHAR(200))");
  auto stmt = conn.execute("SELECT val FROM t");

  // When SQLColAttribute is called with SQL_DESC_LENGTH
  SQLLEN num_attr = 0;
  SQLRETURN ret = SQLColAttribute(stmt.getHandle(), 1, SQL_DESC_LENGTH, NULL, 0, NULL, &num_attr);

  // Then The call should succeed and return 200
  REQUIRE_THAT(OdbcResult(ret, stmt), OdbcMatchers::IsSuccess());
  CHECK(num_attr == 200);
}

// =============================================================================
// SQL_DESC_OCTET_LENGTH
// =============================================================================

TEST_CASE("SQLColAttribute returns positive octet length for VARCHAR.", "[query][col_attribute]") {
  // Given A table with a VARCHAR(100) column is queried
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);
  conn.execute("CREATE TABLE t (val VARCHAR(100))");
  auto stmt = conn.execute("SELECT val FROM t");

  // When SQLColAttribute is called with SQL_DESC_OCTET_LENGTH
  SQLLEN num_attr = 0;
  SQLRETURN ret = SQLColAttribute(stmt.getHandle(), 1, SQL_DESC_OCTET_LENGTH, NULL, 0, NULL, &num_attr);

  // Then The call should succeed and return a positive value
  REQUIRE_THAT(OdbcResult(ret, stmt), OdbcMatchers::IsSuccess());
  CHECK(num_attr > 0);
}

// =============================================================================
// SQL_DESC_DISPLAY_SIZE
// =============================================================================

TEST_CASE("SQLColAttribute returns reasonable display size for VARCHAR.", "[query][col_attribute]") {
  // Given A table with a VARCHAR(100) column is queried
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);
  conn.execute("CREATE TABLE t (val VARCHAR(100))");
  auto stmt = conn.execute("SELECT val FROM t");

  // When SQLColAttribute is called with SQL_DESC_DISPLAY_SIZE
  SQLLEN num_attr = 0;
  SQLRETURN ret = SQLColAttribute(stmt.getHandle(), 1, SQL_DESC_DISPLAY_SIZE, NULL, 0, NULL, &num_attr);

  // Then The call should succeed and return a value >= 100
  REQUIRE_THAT(OdbcResult(ret, stmt), OdbcMatchers::IsSuccess());
  CHECK(num_attr >= 100);
}

TEST_CASE("SQLColAttribute returns display size for NUMBER.", "[query][col_attribute]") {
  // Given A table with a NUMBER(10,2) column is queried
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);
  conn.execute("CREATE TABLE t (val NUMBER(10,2))");
  auto stmt = conn.execute("SELECT val FROM t");

  // When SQLColAttribute is called with SQL_DESC_DISPLAY_SIZE
  SQLLEN num_attr = 0;
  SQLRETURN ret = SQLColAttribute(stmt.getHandle(), 1, SQL_DESC_DISPLAY_SIZE, NULL, 0, NULL, &num_attr);

  // Then The call should succeed and return a positive value
  REQUIRE_THAT(OdbcResult(ret, stmt), OdbcMatchers::IsSuccess());
  CHECK(num_attr > 0);
}

// =============================================================================
// SQL_DESC_NUM_PREC_RADIX
// =============================================================================

TEST_CASE("SQLColAttribute returns num prec radix of 10 for NUMBER.", "[query][col_attribute]") {
  // Given A table with a NUMBER(10,2) column is queried
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);
  conn.execute("CREATE TABLE t (val NUMBER(10,2))");
  auto stmt = conn.execute("SELECT val FROM t");

  // When SQLColAttribute is called with SQL_DESC_NUM_PREC_RADIX
  SQLLEN num_attr = 0;
  SQLRETURN ret = SQLColAttribute(stmt.getHandle(), 1, SQL_DESC_NUM_PREC_RADIX, NULL, 0, NULL, &num_attr);

  // Then The call should succeed and return 10
  REQUIRE_THAT(OdbcResult(ret, stmt), OdbcMatchers::IsSuccess());
  CHECK(num_attr == 10);
}

TEST_CASE("SQLColAttribute returns num prec radix of 0 for VARCHAR.", "[query][col_attribute]") {
  // Given A table with a VARCHAR column is queried
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);
  conn.execute("CREATE TABLE t (val VARCHAR(50))");
  auto stmt = conn.execute("SELECT val FROM t");

  // When SQLColAttribute is called with SQL_DESC_NUM_PREC_RADIX
  SQLLEN num_attr = -1;
  SQLRETURN ret = SQLColAttribute(stmt.getHandle(), 1, SQL_DESC_NUM_PREC_RADIX, NULL, 0, NULL, &num_attr);

  // Then The call should succeed and return 0
  REQUIRE_THAT(OdbcResult(ret, stmt), OdbcMatchers::IsSuccess());
  CHECK(num_attr == 0);
}

// =============================================================================
// SQL_DESC_UNSIGNED
// =============================================================================

TEST_CASE("SQLColAttribute returns SQL_FALSE for signed numeric column.", "[query][col_attribute]") {
  // Given A table with a NUMBER column is queried
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);
  conn.execute("CREATE TABLE t (val NUMBER(10,2))");
  auto stmt = conn.execute("SELECT val FROM t");

  // When SQLColAttribute is called with SQL_DESC_UNSIGNED
  SQLLEN num_attr = -1;
  SQLRETURN ret = SQLColAttribute(stmt.getHandle(), 1, SQL_DESC_UNSIGNED, NULL, 0, NULL, &num_attr);

  // Then The call should succeed and return SQL_FALSE
  REQUIRE_THAT(OdbcResult(ret, stmt), OdbcMatchers::IsSuccess());
  CHECK(num_attr == SQL_FALSE);
}

TEST_CASE("SQLColAttribute returns SQL_TRUE for non-numeric column.", "[query][col_attribute]") {
  // Given A table with a VARCHAR column is queried
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);
  conn.execute("CREATE TABLE t (val VARCHAR(50))");
  auto stmt = conn.execute("SELECT val FROM t");

  // When SQLColAttribute is called with SQL_DESC_UNSIGNED
  SQLLEN num_attr = -1;
  SQLRETURN ret = SQLColAttribute(stmt.getHandle(), 1, SQL_DESC_UNSIGNED, NULL, 0, NULL, &num_attr);

  // Then The call should succeed and return SQL_TRUE
  REQUIRE_THAT(OdbcResult(ret, stmt), OdbcMatchers::IsSuccess());
  CHECK(num_attr == SQL_TRUE);
}

// =============================================================================
// SQL_DESC_SEARCHABLE
// =============================================================================

TEST_CASE("SQLColAttribute returns searchable value for column.", "[query][col_attribute]") {
  // Given A table with a VARCHAR column is queried
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);
  conn.execute("CREATE TABLE t (val VARCHAR(50))");
  auto stmt = conn.execute("SELECT val FROM t");

  // When SQLColAttribute is called with SQL_DESC_SEARCHABLE
  SQLLEN num_attr = 0;
  SQLRETURN ret = SQLColAttribute(stmt.getHandle(), 1, SQL_DESC_SEARCHABLE, NULL, 0, NULL, &num_attr);

  // Then The call should succeed and return a searchable classification
  REQUIRE_THAT(OdbcResult(ret, stmt), OdbcMatchers::IsSuccess());
  CHECK(num_attr >= SQL_PRED_BASIC);
}

// =============================================================================
// SQL_DESC_UPDATABLE
// =============================================================================

TEST_CASE("SQLColAttribute returns updatability for column.", "[query][col_attribute]") {
  // Given A table with a VARCHAR column is queried
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);
  conn.execute("CREATE TABLE t (val VARCHAR(50))");
  auto stmt = conn.execute("SELECT val FROM t");

  // When SQLColAttribute is called with SQL_DESC_UPDATABLE
  SQLLEN num_attr = -1;
  SQLRETURN ret = SQLColAttribute(stmt.getHandle(), 1, SQL_DESC_UPDATABLE, NULL, 0, NULL, &num_attr);

  // Then The call should succeed and return a valid updatability value
  REQUIRE_THAT(OdbcResult(ret, stmt), OdbcMatchers::IsSuccess());
  CHECK((num_attr == SQL_ATTR_READONLY || num_attr == SQL_ATTR_WRITE || num_attr == SQL_ATTR_READWRITE_UNKNOWN));
}

// =============================================================================
// SQL_DESC_AUTO_UNIQUE_VALUE
// =============================================================================

TEST_CASE("SQLColAttribute returns SQL_FALSE for auto-unique-value on regular column.", "[query][col_attribute]") {
  // Given A table with an INT column is queried
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);
  conn.execute("CREATE TABLE t (val INT)");
  auto stmt = conn.execute("SELECT val FROM t");

  // When SQLColAttribute is called with SQL_DESC_AUTO_UNIQUE_VALUE
  SQLLEN num_attr = -1;
  SQLRETURN ret = SQLColAttribute(stmt.getHandle(), 1, SQL_DESC_AUTO_UNIQUE_VALUE, NULL, 0, NULL, &num_attr);

  // Then The call should succeed and return SQL_FALSE
  REQUIRE_THAT(OdbcResult(ret, stmt), OdbcMatchers::IsSuccess());
  CHECK(num_attr == SQL_FALSE);
}

// =============================================================================
// SQL_DESC_CASE_SENSITIVE
// =============================================================================

TEST_CASE("SQLColAttribute returns case sensitivity for VARCHAR.", "[query][col_attribute]") {
  // Given A table with a VARCHAR column is queried
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);
  conn.execute("CREATE TABLE t (val VARCHAR(50))");
  auto stmt = conn.execute("SELECT val FROM t");

  // When SQLColAttribute is called with SQL_DESC_CASE_SENSITIVE
  SQLLEN num_attr = -1;
  SQLRETURN ret = SQLColAttribute(stmt.getHandle(), 1, SQL_DESC_CASE_SENSITIVE, NULL, 0, NULL, &num_attr);

  // Then The call should succeed and return SQL_TRUE
  REQUIRE_THAT(OdbcResult(ret, stmt), OdbcMatchers::IsSuccess());
  CHECK(num_attr == SQL_TRUE);
}

// =============================================================================
// SQL_DESC_FIXED_PREC_SCALE
// =============================================================================

TEST_CASE("SQLColAttribute returns SQL_FALSE for fixed-prec-scale on VARCHAR.", "[query][col_attribute]") {
  // Given A table with a VARCHAR column is queried
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);
  conn.execute("CREATE TABLE t (val VARCHAR(50))");
  auto stmt = conn.execute("SELECT val FROM t");

  // When SQLColAttribute is called with SQL_DESC_FIXED_PREC_SCALE
  SQLLEN num_attr = -1;
  SQLRETURN ret = SQLColAttribute(stmt.getHandle(), 1, SQL_DESC_FIXED_PREC_SCALE, NULL, 0, NULL, &num_attr);

  // Then The call should succeed and return SQL_FALSE
  REQUIRE_THAT(OdbcResult(ret, stmt), OdbcMatchers::IsSuccess());
  CHECK(num_attr == SQL_FALSE);
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
  auto random_schema = Schema::use_random_schema(conn);
  conn.execute("CREATE TABLE t (val VARCHAR(50))");
  auto stmt = conn.execute("SELECT val FROM t");

  SQLCHAR buf_2x[256] = {0};
  SQLCHAR buf_3x[256] = {0};
  SQLSMALLINT len_2x = 0;
  SQLSMALLINT len_3x = 0;
  SQLLEN num_2x = 0;
  SQLLEN num_3x = 0;

  OLD_DRIVER_ONLY("BD#22") {
    SQLRETURN ret = SQLColAttribute(stmt.getHandle(), 1, SQL_COLUMN_NAME, buf_2x, sizeof(buf_2x), &len_2x, NULL);
    REQUIRE_THAT(OdbcResult(ret, stmt), OdbcMatchers::IsSuccess());
    CHECK(std::string(reinterpret_cast<char*>(buf_2x)).empty());

    ret = SQLColAttribute(stmt.getHandle(), 1, SQL_COLUMN_NULLABLE, NULL, 0, NULL, &num_2x);
    REQUIRE_THAT(OdbcResult(ret, stmt), OdbcMatchers::IsError() && OdbcMatchers::HasSqlState("HY091"));
  }

  NEW_DRIVER_ONLY("BD#22") {
    // When SQLColAttribute is called with SQL_COLUMN_NAME and SQL_DESC_NAME
    SQLRETURN ret = SQLColAttribute(stmt.getHandle(), 1, SQL_COLUMN_NAME, buf_2x, sizeof(buf_2x), &len_2x, NULL);
    REQUIRE_THAT(OdbcResult(ret, stmt), OdbcMatchers::IsSuccess());

    ret = SQLColAttribute(stmt.getHandle(), 1, SQL_DESC_NAME, buf_3x, sizeof(buf_3x), &len_3x, NULL);
    REQUIRE_THAT(OdbcResult(ret, stmt), OdbcMatchers::IsSuccess());

    // Then Both should return the same column name
    CHECK(std::string(reinterpret_cast<char*>(buf_2x)) == std::string(reinterpret_cast<char*>(buf_3x)));

    // When SQLColAttribute is called with SQL_COLUMN_NULLABLE and SQL_DESC_NULLABLE
    ret = SQLColAttribute(stmt.getHandle(), 1, SQL_COLUMN_NULLABLE, NULL, 0, NULL, &num_2x);
    REQUIRE_THAT(OdbcResult(ret, stmt), OdbcMatchers::IsSuccess());

    ret = SQLColAttribute(stmt.getHandle(), 1, SQL_DESC_NULLABLE, NULL, 0, NULL, &num_3x);
    REQUIRE_THAT(OdbcResult(ret, stmt), OdbcMatchers::IsSuccess());

    // Then Both should return the same nullable value
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

  // Then The call should return SQL_SUCCESS_WITH_INFO with SQLSTATE 01004
  REQUIRE_THAT(OdbcResult(ret, stmt), OdbcMatchers::IsSuccessWithInfo() && OdbcMatchers::HasSqlState("01004"));

  // And StringLengthPtr should contain the full untruncated length
  CHECK(str_len == 9);
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
// Multi-type coverage
// =============================================================================

TEST_CASE("SQLColAttribute returns correct metadata for multiple data types.", "[query][col_attribute]") {
  // Given A table with VARCHAR, NUMBER, and BOOLEAN columns is queried
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);
  conn.execute(
      "CREATE TABLE t ("
      "  str_col VARCHAR(50),"
      "  num_col NUMBER(8,2),"
      "  bool_col BOOLEAN"
      ")");
  auto stmt = conn.execute("SELECT str_col, num_col, bool_col FROM t");

  // When SQLColAttribute is called for column 1 (VARCHAR) name
  std::string col1_name = get_string_attr(stmt, 1, SQL_DESC_NAME);

  // Then The column name should be STR_COL
  CHECK(col1_name == "STR_COL");

  // When SQLColAttribute is called for column 1 (VARCHAR) concise type
  SQLLEN col1_type = get_numeric_attr(stmt, 1, SQL_DESC_CONCISE_TYPE);

  // Then The type should be SQL_VARCHAR
  CHECK(col1_type == SQL_VARCHAR);

  // When SQLColAttribute is called for column 1 (VARCHAR) length
  SQLLEN col1_length = get_numeric_attr(stmt, 1, SQL_DESC_LENGTH);

  // Then The length should be 50
  CHECK(col1_length == 50);

  // When SQLColAttribute is called for column 1 (VARCHAR) nullable
  SQLLEN col1_nullable = get_numeric_attr(stmt, 1, SQL_DESC_NULLABLE);

  // Then The column should be nullable
  CHECK(col1_nullable == SQL_NULLABLE);

  // When SQLColAttribute is called for column 2 (NUMBER) name
  std::string col2_name = get_string_attr(stmt, 2, SQL_DESC_NAME);

  // Then The column name should be NUM_COL
  CHECK(col2_name == "NUM_COL");

  // When SQLColAttribute is called for column 2 (NUMBER) concise type
  SQLLEN col2_type = get_numeric_attr(stmt, 2, SQL_DESC_CONCISE_TYPE);

  // Then The type should be SQL_DECIMAL
  CHECK(col2_type == SQL_DECIMAL);

  // When SQLColAttribute is called for column 2 (NUMBER) precision
  SQLLEN col2_precision = get_numeric_attr(stmt, 2, SQL_DESC_PRECISION);

  // Then The precision should be 8
  CHECK(col2_precision == 8);

  // When SQLColAttribute is called for column 2 (NUMBER) scale
  SQLLEN col2_scale = get_numeric_attr(stmt, 2, SQL_DESC_SCALE);

  // Then The scale should be 2
  CHECK(col2_scale == 2);

  // When SQLColAttribute is called for column 2 (NUMBER) unsigned
  SQLLEN col2_unsigned = get_numeric_attr(stmt, 2, SQL_DESC_UNSIGNED);

  // Then The column should be signed
  CHECK(col2_unsigned == SQL_FALSE);

  // When SQLColAttribute is called for column 3 (BOOLEAN) name
  std::string col3_name = get_string_attr(stmt, 3, SQL_DESC_NAME);

  // Then The column name should be BOOL_COL
  CHECK(col3_name == "BOOL_COL");

  // When SQLColAttribute is called for column 3 (BOOLEAN) concise type
  SQLLEN col3_type = get_numeric_attr(stmt, 3, SQL_DESC_CONCISE_TYPE);

  // Then The type should be SQL_BIT
  CHECK(col3_type == SQL_BIT);

  // When SQLColAttribute is called for column count
  SQLLEN count = get_numeric_attr(stmt, 1, SQL_DESC_COUNT);

  // Then The count should be 3
  CHECK(count == 3);
}
