#include <cstring>
#include <string>

#include <catch2/catch_test_macros.hpp>

#include "Connection.hpp"
#include "Schema.hpp"
#include "get_diag_rec.hpp"
#include "odbc_matchers.hpp"

// =============================================================================
// Tests for SQLColAttributes (ODBC 2.x) based on ODBC specification:
// https://learn.microsoft.com/en-us/sql/odbc/reference/syntax/sqlcolattributes-function
// https://learn.microsoft.com/en-us/sql/odbc/reference/appendixes/sqlcolattributes-mapping
//
// SQLColAttributes is the ODBC 2.x predecessor of SQLColAttribute.
// It uses SQL_COLUMN_* field identifiers which are mapped to SQL_DESC_*
// equivalents internally. Three identifiers have different semantics:
//   SQL_COLUMN_LENGTH    -> transfer octet length (not column size)
//   SQL_COLUMN_PRECISION -> column size (char length for chars, precision for numerics)
//   SQL_COLUMN_SCALE     -> decimal digits
// =============================================================================

// =============================================================================
// SQL_COLUMN_NAME
// =============================================================================

TEST_CASE("SQLColAttributes returns correct column name via SQL_COLUMN_NAME.", "[query][col_attributes]") {
  // Given A query with a named column is executed
  Connection conn;
  auto stmt = conn.execute("SELECT 42 AS MY_COLUMN");

  // When SQLColAttributes is called with SQL_COLUMN_NAME
  SQLCHAR buffer[128] = {0};
  SQLSMALLINT str_len = 0;
  SQLLEN num_attr = 0;
  SQLRETURN ret = SQLColAttributes(stmt.getHandle(), 1, SQL_COLUMN_NAME, buffer, sizeof(buffer), &str_len, &num_attr);

  // Then The call should succeed and return the column name
  REQUIRE_THAT(OdbcResult(ret, stmt), OdbcMatchers::IsSuccess());
  CHECK(std::string(reinterpret_cast<char*>(buffer)) == "MY_COLUMN");
  CHECK(str_len == 9);
}

// =============================================================================
// SQL_COLUMN_TYPE
// =============================================================================

TEST_CASE("SQLColAttributes returns SQL_DECIMAL for numeric literal via SQL_COLUMN_TYPE.", "[query][col_attributes]") {
  // Given A query returning a numeric literal is executed
  Connection conn;
  auto stmt = conn.execute("SELECT 42");

  // When SQLColAttributes is called with SQL_COLUMN_TYPE
  SQLLEN num_attr = 0;
  SQLRETURN ret = SQLColAttributes(stmt.getHandle(), 1, SQL_COLUMN_TYPE, NULL, 0, NULL, &num_attr);

  // Then The call should succeed and return SQL_DECIMAL
  REQUIRE_THAT(OdbcResult(ret, stmt), OdbcMatchers::IsSuccess());
  CHECK(num_attr == SQL_DECIMAL);
}

TEST_CASE("SQLColAttributes returns SQL_VARCHAR for VARCHAR column via SQL_COLUMN_TYPE.", "[query][col_attributes]") {
  // Given A table with a VARCHAR column is queried
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);
  conn.execute("CREATE TABLE t (val VARCHAR(100))");
  auto stmt = conn.execute("SELECT val FROM t");

  // When SQLColAttributes is called with SQL_COLUMN_TYPE
  SQLLEN num_attr = 0;
  SQLRETURN ret = SQLColAttributes(stmt.getHandle(), 1, SQL_COLUMN_TYPE, NULL, 0, NULL, &num_attr);

  // Then The call should succeed and return SQL_VARCHAR
  REQUIRE_THAT(OdbcResult(ret, stmt), OdbcMatchers::IsSuccess());
  CHECK(num_attr == SQL_VARCHAR);
}

// =============================================================================
// SQL_COLUMN_LENGTH (transfer octet length — ODBC 2.x specific)
// =============================================================================

TEST_CASE("SQLColAttributes returns transfer octet length for VARCHAR via SQL_COLUMN_LENGTH.",
          "[query][col_attributes]") {
  // Given A table with a VARCHAR(100) column is queried
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);
  conn.execute("CREATE TABLE t (val VARCHAR(100))");
  auto stmt = conn.execute("SELECT val FROM t");

  // When SQLColAttributes is called with SQL_COLUMN_LENGTH
  SQLLEN num_attr = 0;
  SQLRETURN ret = SQLColAttributes(stmt.getHandle(), 1, SQL_COLUMN_LENGTH, NULL, 0, NULL, &num_attr);

  // Then The call should succeed and return the transfer octet length
  REQUIRE_THAT(OdbcResult(ret, stmt), OdbcMatchers::IsSuccess());
  CHECK(num_attr >= 100);
}

TEST_CASE("SQLColAttributes returns transfer octet length for NUMBER via SQL_COLUMN_LENGTH.",
          "[query][col_attributes]") {
  // Given A table with a NUMBER(10,2) column is queried
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);
  conn.execute("CREATE TABLE t (val NUMBER(10,2))");
  auto stmt = conn.execute("SELECT val FROM t");

  // When SQLColAttributes is called with SQL_COLUMN_LENGTH
  SQLLEN num_attr = 0;
  SQLRETURN ret = SQLColAttributes(stmt.getHandle(), 1, SQL_COLUMN_LENGTH, NULL, 0, NULL, &num_attr);

  // Then The call should succeed and return a positive transfer octet length
  REQUIRE_THAT(OdbcResult(ret, stmt), OdbcMatchers::IsSuccess());
  CHECK(num_attr > 0);
}

// =============================================================================
// SQL_COLUMN_NULLABLE
// =============================================================================

TEST_CASE("SQLColAttributes returns SQL_NULLABLE for nullable column.", "[query][col_attributes]") {
  // Given A table with a nullable column is queried
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);
  conn.execute("CREATE TABLE t (val VARCHAR(50))");
  auto stmt = conn.execute("SELECT val FROM t");

  // When SQLColAttributes is called with SQL_COLUMN_NULLABLE
  SQLLEN num_attr = -1;
  SQLRETURN ret = SQLColAttributes(stmt.getHandle(), 1, SQL_COLUMN_NULLABLE, NULL, 0, NULL, &num_attr);

  // Then The call should succeed and return SQL_NULLABLE
  REQUIRE_THAT(OdbcResult(ret, stmt), OdbcMatchers::IsSuccess());
  CHECK(num_attr == SQL_NULLABLE);
}

TEST_CASE("SQLColAttributes returns SQL_NO_NULLS for NOT NULL column.", "[query][col_attributes]") {
  // Given A table with a NOT NULL column is queried
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);
  conn.execute("CREATE TABLE t (val VARCHAR(50) NOT NULL)");
  auto stmt = conn.execute("SELECT val FROM t");

  // When SQLColAttributes is called with SQL_COLUMN_NULLABLE
  SQLLEN num_attr = -1;
  SQLRETURN ret = SQLColAttributes(stmt.getHandle(), 1, SQL_COLUMN_NULLABLE, NULL, 0, NULL, &num_attr);

  // Then The call should succeed and return SQL_NO_NULLS
  REQUIRE_THAT(OdbcResult(ret, stmt), OdbcMatchers::IsSuccess());
  CHECK(num_attr == SQL_NO_NULLS);
}

// =============================================================================
// SQL_COLUMN_PRECISION (column size — ODBC 2.x specific)
// =============================================================================

TEST_CASE("SQLColAttributes returns character length for VARCHAR via SQL_COLUMN_PRECISION.",
          "[query][col_attributes]") {
  // Given A table with a VARCHAR(200) column is queried
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);
  conn.execute("CREATE TABLE t (val VARCHAR(200))");
  auto stmt = conn.execute("SELECT val FROM t");

  // When SQLColAttributes is called with SQL_COLUMN_PRECISION
  SQLLEN num_attr = 0;
  SQLRETURN ret = SQLColAttributes(stmt.getHandle(), 1, SQL_COLUMN_PRECISION, NULL, 0, NULL, &num_attr);

  // Then The call should succeed and return 200
  REQUIRE_THAT(OdbcResult(ret, stmt), OdbcMatchers::IsSuccess());
  CHECK(num_attr == 200);
}

TEST_CASE("SQLColAttributes returns numeric precision for NUMBER via SQL_COLUMN_PRECISION.",
          "[query][col_attributes]") {
  // Given A table with a NUMBER(10,2) column is queried
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);
  conn.execute("CREATE TABLE t (val NUMBER(10,2))");
  auto stmt = conn.execute("SELECT val FROM t");

  // When SQLColAttributes is called with SQL_COLUMN_PRECISION
  SQLLEN num_attr = 0;
  SQLRETURN ret = SQLColAttributes(stmt.getHandle(), 1, SQL_COLUMN_PRECISION, NULL, 0, NULL, &num_attr);

  // Then The call should succeed and return 10
  REQUIRE_THAT(OdbcResult(ret, stmt), OdbcMatchers::IsSuccess());
  CHECK(num_attr == 10);
}

// =============================================================================
// SQL_COLUMN_SCALE (decimal digits — ODBC 2.x specific)
// =============================================================================

TEST_CASE("SQLColAttributes returns scale for NUMBER via SQL_COLUMN_SCALE.", "[query][col_attributes]") {
  // Given A table with a NUMBER(10,4) column is queried
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);
  conn.execute("CREATE TABLE t (val NUMBER(10,4))");
  auto stmt = conn.execute("SELECT val FROM t");

  // When SQLColAttributes is called with SQL_COLUMN_SCALE
  SQLLEN num_attr = 0;
  SQLRETURN ret = SQLColAttributes(stmt.getHandle(), 1, SQL_COLUMN_SCALE, NULL, 0, NULL, &num_attr);

  // Then The call should succeed and return 4
  REQUIRE_THAT(OdbcResult(ret, stmt), OdbcMatchers::IsSuccess());
  CHECK(num_attr == 4);
}

TEST_CASE("SQLColAttributes returns 0 scale for VARCHAR via SQL_COLUMN_SCALE.", "[query][col_attributes]") {
  // Given A table with a VARCHAR column is queried
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);
  conn.execute("CREATE TABLE t (val VARCHAR(50))");
  auto stmt = conn.execute("SELECT val FROM t");

  // When SQLColAttributes is called with SQL_COLUMN_SCALE
  SQLLEN num_attr = -1;
  SQLRETURN ret = SQLColAttributes(stmt.getHandle(), 1, SQL_COLUMN_SCALE, NULL, 0, NULL, &num_attr);

  // Then The call should succeed and return 0
  REQUIRE_THAT(OdbcResult(ret, stmt), OdbcMatchers::IsSuccess());
  CHECK(num_attr == 0);
}

// =============================================================================
// SQL_COLUMN_COUNT
// =============================================================================

TEST_CASE("SQLColAttributes returns column count via SQL_COLUMN_COUNT.", "[query][col_attributes]") {
  // Given A multi-column query is executed
  Connection conn;
  auto stmt = conn.execute("SELECT 1 AS a, 2 AS b, 3 AS c");

  // When SQLColAttributes is called with SQL_COLUMN_COUNT
  SQLLEN num_attr = 0;
  SQLRETURN ret = SQLColAttributes(stmt.getHandle(), 1, SQL_COLUMN_COUNT, NULL, 0, NULL, &num_attr);

  // Then The call should succeed and return 3
  REQUIRE_THAT(OdbcResult(ret, stmt), OdbcMatchers::IsSuccess());
  CHECK(num_attr == 3);
}

// =============================================================================
// Multiple data types
// =============================================================================

TEST_CASE("SQLColAttributes returns correct metadata for each column in a multi-column result.",
          "[query][col_attributes]") {
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

  SQLCHAR buffer[128] = {0};
  SQLSMALLINT str_len = 0;
  SQLLEN num_attr = 0;

  // When SQLColAttributes is called for column 1 (VARCHAR) name
  SQLRETURN ret = SQLColAttributes(stmt.getHandle(), 1, SQL_COLUMN_NAME, buffer, sizeof(buffer), &str_len, &num_attr);
  REQUIRE_THAT(OdbcResult(ret, stmt), OdbcMatchers::IsSuccess());

  // Then The column name should be STR_COL
  CHECK(std::string(reinterpret_cast<char*>(buffer)) == "STR_COL");

  // When SQLColAttributes is called for column 1 (VARCHAR) type
  num_attr = 0;
  ret = SQLColAttributes(stmt.getHandle(), 1, SQL_COLUMN_TYPE, NULL, 0, NULL, &num_attr);
  REQUIRE_THAT(OdbcResult(ret, stmt), OdbcMatchers::IsSuccess());

  // Then The type should be SQL_VARCHAR
  CHECK(num_attr == SQL_VARCHAR);

  // When SQLColAttributes is called for column 1 (VARCHAR) precision
  num_attr = 0;
  ret = SQLColAttributes(stmt.getHandle(), 1, SQL_COLUMN_PRECISION, NULL, 0, NULL, &num_attr);
  REQUIRE_THAT(OdbcResult(ret, stmt), OdbcMatchers::IsSuccess());

  // Then The precision (column size) should be 50
  CHECK(num_attr == 50);

  // When SQLColAttributes is called for column 2 (NUMBER) name
  memset(buffer, 0, sizeof(buffer));
  ret = SQLColAttributes(stmt.getHandle(), 2, SQL_COLUMN_NAME, buffer, sizeof(buffer), &str_len, &num_attr);
  REQUIRE_THAT(OdbcResult(ret, stmt), OdbcMatchers::IsSuccess());

  // Then The column name should be NUM_COL
  CHECK(std::string(reinterpret_cast<char*>(buffer)) == "NUM_COL");

  // When SQLColAttributes is called for column 2 (NUMBER) type
  num_attr = 0;
  ret = SQLColAttributes(stmt.getHandle(), 2, SQL_COLUMN_TYPE, NULL, 0, NULL, &num_attr);
  REQUIRE_THAT(OdbcResult(ret, stmt), OdbcMatchers::IsSuccess());

  // Then The type should be SQL_DECIMAL
  CHECK(num_attr == SQL_DECIMAL);

  // When SQLColAttributes is called for column 2 (NUMBER) precision
  num_attr = 0;
  ret = SQLColAttributes(stmt.getHandle(), 2, SQL_COLUMN_PRECISION, NULL, 0, NULL, &num_attr);
  REQUIRE_THAT(OdbcResult(ret, stmt), OdbcMatchers::IsSuccess());

  // Then The precision should be 8
  CHECK(num_attr == 8);

  // When SQLColAttributes is called for column 2 (NUMBER) scale
  num_attr = 0;
  ret = SQLColAttributes(stmt.getHandle(), 2, SQL_COLUMN_SCALE, NULL, 0, NULL, &num_attr);
  REQUIRE_THAT(OdbcResult(ret, stmt), OdbcMatchers::IsSuccess());

  // Then The scale should be 2
  CHECK(num_attr == 2);

  // When SQLColAttributes is called for column 3 (BOOLEAN) name
  memset(buffer, 0, sizeof(buffer));
  ret = SQLColAttributes(stmt.getHandle(), 3, SQL_COLUMN_NAME, buffer, sizeof(buffer), &str_len, &num_attr);
  REQUIRE_THAT(OdbcResult(ret, stmt), OdbcMatchers::IsSuccess());

  // Then The column name should be BOOL_COL
  CHECK(std::string(reinterpret_cast<char*>(buffer)) == "BOOL_COL");

  // When SQLColAttributes is called for column 3 (BOOLEAN) type
  num_attr = 0;
  ret = SQLColAttributes(stmt.getHandle(), 3, SQL_COLUMN_TYPE, NULL, 0, NULL, &num_attr);
  REQUIRE_THAT(OdbcResult(ret, stmt), OdbcMatchers::IsSuccess());

  // Then The type should be SQL_BIT
  CHECK(num_attr == SQL_BIT);
}

// =============================================================================
// Error Cases
// =============================================================================

TEST_CASE("SQLColAttributes returns 07009 for column number 0 without bookmarks.", "[query][col_attributes]") {
  // Given A query is executed
  Connection conn;
  auto stmt = conn.execute("SELECT 42 AS value");

  // When SQLColAttributes is called with column number 0 for a non-count attribute
  SQLLEN num_attr = 0;
  SQLRETURN ret = SQLColAttributes(stmt.getHandle(), 0, SQL_COLUMN_NAME, NULL, 0, NULL, &num_attr);

  // Then The call should return SQL_ERROR with SQLSTATE 07009
  REQUIRE_THAT(OdbcResult(ret, stmt), OdbcMatchers::IsError() && OdbcMatchers::HasSqlState("07009"));
}

TEST_CASE("SQLColAttributes returns 07009 for out-of-range column number.", "[query][col_attributes]") {
  // Given A single-column query is executed
  Connection conn;
  auto stmt = conn.execute("SELECT 42 AS value");

  // When SQLColAttributes is called with a column number beyond the result set
  SQLLEN num_attr = 0;
  SQLRETURN ret = SQLColAttributes(stmt.getHandle(), 100, SQL_COLUMN_TYPE, NULL, 0, NULL, &num_attr);

  // Then The call should return SQL_ERROR with SQLSTATE 07009
  REQUIRE_THAT(OdbcResult(ret, stmt), OdbcMatchers::IsError() && OdbcMatchers::HasSqlState("07009"));
}

TEST_CASE("SQLColAttributes returns HY010 when called before prepare or execute.", "[query][col_attributes]") {
  // Given A statement handle exists but no query has been prepared or executed
  Connection conn;
  auto stmt = conn.createStatement();

  // When SQLColAttributes is called without any prepare or execute
  SQLLEN num_attr = 0;
  SQLRETURN ret = SQLColAttributes(stmt.getHandle(), 1, SQL_COLUMN_TYPE, NULL, 0, NULL, &num_attr);

  // Then The call should return SQL_ERROR with SQLSTATE HY010
  REQUIRE_THAT(OdbcResult(ret, stmt), OdbcMatchers::IsError() && OdbcMatchers::HasSqlState("HY010"));
}
