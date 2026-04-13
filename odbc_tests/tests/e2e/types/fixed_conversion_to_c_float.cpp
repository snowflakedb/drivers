#include <sql.h>
#include <sqlext.h>
#include <sqltypes.h>

#include <string>
#include <vector>

#include <catch2/catch_test_macros.hpp>

#include "Connection.hpp"
#include "Schema.hpp"
#include "TestTable.hpp"
#include "conversion_checks.hpp"
#include "get_data.hpp"
#include "odbc_matchers.hpp"

TEST_CASE("Test decimal to floating point conversion", "[fixed][conversion][c_real]") {
  // Given A Snowflake connection is established
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);

  // When A table with DECIMAL/NUMBER/INT columns containing value 123 is queried
  TestTable table(conn, "test_number",
                  "num0 NUMBER, num10 NUMBER(10,1), dec20 DECIMAL(20,2), "
                  "numeric30 NUMERIC(30,3), int1 INT, int2 INTEGER",
                  "(123, 123.4, 123.45, 123.456, 123, 123)");

  auto stmt = conn.execute_fetch("SELECT * FROM " + table.name());

  // Then SQL_C_FLOAT and SQL_C_DOUBLE return correct values without truncation
  std::vector<float> expected_float = {123.0f, 123.4f, 123.45f, 123.456f, 123.0f, 123.0f};
  std::vector<double> expected_double = {123.0, 123.4, 123.45, 123.456, 123.0, 123.0};

  for (int i = 1; i <= 6; ++i) {
    INFO("Testing column " << i << " with SQL_C_FLOAT");
    CHECK(check_no_truncation<SQL_C_FLOAT>(stmt, i) == expected_float[i - 1]);
  }

  for (int i = 1; i <= 6; ++i) {
    INFO("Testing column " << i << " with SQL_C_DOUBLE");
    CHECK(check_no_truncation<SQL_C_DOUBLE>(stmt, i) == expected_double[i - 1]);
  }
}

TEST_CASE("SQL_DECIMAL explicit floating point conversions preserve fraction", "[fixed][conversion][c_real]") {
  // Given A Snowflake connection is established
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);

  // When A fractional DECIMAL value 123.789 is fetched as float and double
  const std::string query = "SELECT 123.789::DECIMAL(10,3)";

  float float_val = check_no_truncation<SQL_C_FLOAT>(conn.execute_fetch(query), 1);
  double double_val = check_no_truncation<SQL_C_DOUBLE>(conn.execute_fetch(query), 1);

  // Then SQL_C_FLOAT and SQL_C_DOUBLE preserve the fractional part
  CHECK(float_val > 123.78f);
  CHECK(float_val < 123.80f);
  CHECK(double_val > 123.788);
  CHECK(double_val < 123.790);
}

TEST_CASE("DECIMAL to floating point precision", "[fixed][conversion][c_real][precision]") {
  // Given A Snowflake connection is established
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);

  // When NUMBER values with varying significant digits are fetched as float and double
  (void)0;
  // Then Precision is preserved within the limits of each floating point type
  {
    INFO("SQL_C_DOUBLE with 15 significant digits");
    auto stmt = conn.execute_fetch("SELECT 123456789012345::NUMBER(15,0)");

    CHECK(check_no_truncation<SQL_C_DOUBLE>(stmt, 1) == 123456789012345.0);
  }

  {
    INFO("SQL_C_FLOAT with 6 significant digits");
    auto stmt = conn.execute_fetch("SELECT 123456::NUMBER(10,0)");

    CHECK(check_no_truncation<SQL_C_FLOAT>(stmt, 1) == 123456.0f);
  }
}

TEST_CASE("DECIMAL multiple rows as SQL_C_DOUBLE", "[fixed][conversion][c_real][multirow]") {
  // Given A Snowflake connection is established
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);

  // When A table with various DECIMAL(10,3) values is queried
  TestTable table(conn, "test_number_double_multi", "val DECIMAL(10,3)", "(0.000), (1.500), (-2.750), (100.125)");

  auto stmt = conn.execute_fetch("SELECT * FROM " + table.name());
  std::vector<double> expected = {0.0, 1.5, -2.75, 100.125};

  // Then Each row returns the correct double value
  for (size_t row = 0; row < expected.size(); ++row) {
    if (row > 0) {
      SQLRETURN ret = SQLFetch(stmt.getHandle());
      CHECK(ret == SQL_SUCCESS);
    }
    INFO("Row " << row << " expected double: " << expected[row]);
    CHECK(check_no_truncation<SQL_C_DOUBLE>(stmt, 1) == expected[row]);
  }
}

TEST_CASE("NUMBER NULL to SQL_C_FLOAT and SQL_C_DOUBLE", "[fixed][conversion][c_real][null]") {
  // Given A Snowflake connection is established
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);

  // When A NULL NUMBER value is queried
  const auto query = "SELECT NULL::NUMBER(10,0)";
  // Then Indicator returns SQL_NULL_DATA
  check_null_via_get_data(conn.execute_fetch(query), 1, SQL_C_FLOAT);
  check_null_via_get_data(conn.execute_fetch(query), 1, SQL_C_DOUBLE);
}
