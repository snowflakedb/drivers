// REAL explicit SQL_C_DOUBLE and SQL_C_FLOAT conversion E2E tests

#include <sql.h>
#include <sqlext.h>
#include <sqltypes.h>

#include <optional>
#include <vector>

#include <catch2/catch_test_macros.hpp>
#include <catch2/matchers/catch_matchers_floating_point.hpp>

#include "Connection.hpp"
#include "Schema.hpp"
#include "TestTable.hpp"
#include "conversion_checks.hpp"
#include "get_diag_rec.hpp"

// ============================================================================
// Explicit C type conversions from FLOAT columns
// ============================================================================

TEST_CASE("REAL explicit SQL_C_DOUBLE", "[e2e][types][real]") {
  // Given A Snowflake connection
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);

  // When FLOAT value is fetched as SQL_C_DOUBLE
  auto stmt = conn.execute_fetch("SELECT 123.456::FLOAT");

  double val = check_no_truncation<SQL_C_DOUBLE>(stmt, 1);

  // Then The value matches within relative tolerance
  CHECK_THAT(val, Catch::Matchers::WithinRel(123.456));
}

TEST_CASE("REAL explicit SQL_C_FLOAT", "[e2e][types][real]") {
  // Given A Snowflake connection
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);

  // When FLOAT value is fetched as SQL_C_FLOAT
  auto stmt = conn.execute_fetch("SELECT 123.5::FLOAT");

  float val = check_no_truncation<SQL_C_FLOAT>(stmt, 1);

  // Then The value matches within relative tolerance
  CHECK_THAT(val, Catch::Matchers::WithinRel(123.5f));
}

TEST_CASE("REAL precision - Snowflake FLOAT has ~15 significant digits", "[e2e][types][real]") {
  // Given A Snowflake connection
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);

  // When FLOAT value with 15 significant digits is fetched as SQL_C_DOUBLE
  auto stmt = conn.execute_fetch("SELECT 1.23456789012345::FLOAT");
  double val = check_no_truncation<SQL_C_DOUBLE>(stmt, 1);

  // Then The value matches within relative tolerance
  CHECK_THAT(val, Catch::Matchers::WithinRel(1.23456789012345));
}

TEST_CASE("REAL negative zero", "[e2e][types][real]") {
  // Given A Snowflake connection
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);

  // When Negative zero FLOAT is fetched as SQL_C_DOUBLE
  auto stmt = conn.execute_fetch("SELECT -0.0::FLOAT");
  double val = check_no_truncation<SQL_C_DOUBLE>(stmt, 1);

  // Then The value is exactly zero
  CHECK(val == 0.0);
}

TEST_CASE("REAL SQL_C_FLOAT precision loss", "[e2e][types][real]") {
  // Given A Snowflake connection
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);

  // When Values representable in f32 are fetched as SQL_C_FLOAT
  auto stmt1 = conn.execute_fetch("SELECT 0.5::FLOAT");
  auto stmt2 = conn.execute_fetch("SELECT 1000000.0::FLOAT");

  // Then They match without truncation
  CHECK(check_no_truncation<SQL_C_FLOAT>(stmt1, 1) == 0.5f);

  // And Large value representable in f32 succeeds
  CHECK(check_no_truncation<SQL_C_FLOAT>(stmt2, 1) == 1000000.0f);
}

TEST_CASE("REAL SQL_C_FLOAT overflow returns 22003", "[e2e][types][real]") {
  // Given A Snowflake connection
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);

  // When Values exceeding f32 range are fetched as SQL_C_FLOAT
  auto stmt_pos = conn.execute_fetch("SELECT 1e300::FLOAT");
  auto stmt_neg = conn.execute_fetch("SELECT -1e300::FLOAT");
  auto stmt_ok = conn.execute_fetch("SELECT 1e38::FLOAT");

  // Then Positive overflow returns 22003
  check_numeric_out_of_range<SQL_C_FLOAT>(stmt_pos, 1);

  // And Negative overflow returns 22003
  check_numeric_out_of_range<SQL_C_FLOAT>(stmt_neg, 1);

  // And Large value within f32 range succeeds
  CHECK(check_no_truncation<SQL_C_FLOAT>(stmt_ok, 1) == 1e38f);
}

TEST_CASE("REAL NULL to SQL_C_FLOAT and SQL_C_DOUBLE", "[real][conversion][c_real][null]") {
  // Given A Snowflake connection
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);

  // When NULL FLOAT values are queried
  const auto query = "SELECT NULL::FLOAT";
  // Then NULL FLOAT values return SQL_NULL_DATA
  check_null_via_get_data(conn.execute_fetch(query), 1, SQL_C_DOUBLE);
  check_null_via_get_data(conn.execute_fetch("SELECT NULL::DOUBLE"), 1, SQL_C_FLOAT);
}

TEST_CASE("REAL NULL mixed with non-NULL in multiple rows", "[real][conversion][c_real][null]") {
  // Given A Snowflake connection
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);

  // When A table with mixed NULL and non-NULL FLOAT rows is queried
  TestTable table(conn, "test_real_null", "val FLOAT", "(1.5), (NULL), (-2.75), (NULL), (0.0)");
  auto stmt = conn.execute_fetch("SELECT * FROM " + table.name());

  std::vector<std::optional<double>> expected = {1.5, std::nullopt, -2.75, std::nullopt, 0.0};
  SQLDOUBLE value = 0;
  SQLLEN indicator = 0;

  // Then NULLs return SQL_NULL_DATA and non-NULLs return correct values
  for (size_t row = 0; row < expected.size(); ++row) {
    if (row > 0) {
      SQLRETURN ret = SQLFetch(stmt.getHandle());
      CHECK(ret == SQL_SUCCESS);
    }
    INFO("Row " << row);
    SQLRETURN ret = SQLGetData(stmt.getHandle(), 1, SQL_C_DOUBLE, &value, sizeof(value), &indicator);
    CHECK(ret == SQL_SUCCESS);
    if (expected[row].has_value()) {
      CHECK(indicator != SQL_NULL_DATA);
      CHECK(value == expected[row].value());
    } else {
      CHECK(indicator == SQL_NULL_DATA);
    }
  }
}

TEST_CASE("REAL SQLGetData NULL without indicator returns 22002", "[real][conversion][c_real][null][22002]") {
  // Given A Snowflake connection
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);

  // When A NULL FLOAT value is fetched without providing an indicator pointer
  auto stmt = conn.execute_fetch("SELECT NULL::FLOAT");
  SQLDOUBLE value = 0;
  SQLRETURN ret = SQLGetData(stmt.getHandle(), 1, SQL_C_DOUBLE, &value, sizeof(value), nullptr);

  // Then SQL_ERROR is returned with SQLSTATE 22002
  CHECK(ret == SQL_ERROR);
  CHECK(get_sqlstate(stmt) == "22002");
}
