#include <sql.h>
#include <sqlext.h>
#include <sqltypes.h>

#include <optional>
#include <sstream>
#include <string>
#include <vector>

#include <catch2/catch_test_macros.hpp>

#include "Connection.hpp"
#include "HandleWrapper.hpp"
#include "Schema.hpp"
#include "TestTable.hpp"
#include "compatibility.hpp"
#include "conversion_checks.hpp"
#include "get_data.hpp"
#include "get_diag_rec.hpp"
#include "macros.hpp"
#include "test_setup.hpp"

// ============================================================================
// Helpers
// ============================================================================

inline std::string get_data_default_as_string(const StatementHandleWrapper& stmt, SQLUSMALLINT col) {
  char buffer[1000];
  SQLLEN indicator;
  SQLRETURN ret = SQLGetData(stmt.getHandle(), col, SQL_C_DEFAULT, buffer, sizeof(buffer), &indicator);
  CHECK(ret == SQL_SUCCESS);
  return std::string(buffer, indicator);
}

inline SQL_NUMERIC_STRUCT get_binary_as_numeric(const StatementHandleWrapper& stmt, SQLUSMALLINT col) {
  char buffer[100] = {};
  SQLLEN indicator = 0;
  SQLRETURN ret = SQLGetData(stmt.getHandle(), col, SQL_C_BINARY, buffer, sizeof(buffer), &indicator);
  CHECK(ret == SQL_SUCCESS);
  CHECK(indicator == sizeof(SQL_NUMERIC_STRUCT));
  return *reinterpret_cast<SQL_NUMERIC_STRUCT*>(buffer);
}

inline void check_numeric_val_zero_from(const SQL_NUMERIC_STRUCT& numeric, int start) {
  for (int i = start; i < 16; ++i) {
    CHECK(numeric.val[i] == 0);
  }
}

template <int SQL_C_TYPE>
void check_integer_columns(const StatementHandleWrapper& stmt, const std::vector<int>& exact_cols,
                           const std::vector<int>& truncated_cols, typename MetaOfSqlCType<SQL_C_TYPE>::type expected) {
  for (int col : exact_cols) {
    INFO("Column " << col << " with " << MetaOfSqlCType<SQL_C_TYPE>().name() << " (exact)");
    CHECK(check_no_truncation<SQL_C_TYPE>(stmt, col) == expected);
  }
  for (int col : truncated_cols) {
    INFO("Column " << col << " with " << MetaOfSqlCType<SQL_C_TYPE>().name() << " (truncated)");
    CHECK(check_fractional_truncation<SQL_C_TYPE>(stmt, col) == expected);
  }
}

// ============================================================================
// Basic decimal conversion across all C integer types
// ============================================================================

TEST_CASE("Test decimal conversion", "[datatype][number]") {
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);
  TestTable table(conn, "test_number",
                  "num0 NUMBER, num10 NUMBER(10,1), dec20 DECIMAL(20,2), "
                  "numeric30 NUMERIC(30,3), int1 INT, int2 INTEGER",
                  "(123, 123.4, 123.45, 123.456, 123, 123)");

  auto stmt = conn.execute_fetch("SELECT * FROM " + table.name());

  std::vector<int> exact_cols = {1, 5, 6};
  std::vector<int> truncated_cols = {2, 3, 4};

  check_integer_columns<SQL_C_LONG>(stmt, exact_cols, truncated_cols, 123);
  check_integer_columns<SQL_C_SLONG>(stmt, exact_cols, truncated_cols, 123);
  check_integer_columns<SQL_C_ULONG>(stmt, exact_cols, truncated_cols, 123);
  check_integer_columns<SQL_C_SHORT>(stmt, exact_cols, truncated_cols, 123);
  check_integer_columns<SQL_C_SSHORT>(stmt, exact_cols, truncated_cols, 123);
  check_integer_columns<SQL_C_USHORT>(stmt, exact_cols, truncated_cols, 123);
  check_integer_columns<SQL_C_TINYINT>(stmt, exact_cols, truncated_cols, 123);
  check_integer_columns<SQL_C_STINYINT>(stmt, exact_cols, truncated_cols, 123);
  check_integer_columns<SQL_C_UTINYINT>(stmt, exact_cols, truncated_cols, 123);
  check_integer_columns<SQL_C_SBIGINT>(stmt, exact_cols, truncated_cols, 123);
  check_integer_columns<SQL_C_UBIGINT>(stmt, exact_cols, truncated_cols, 123);

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

  std::vector<std::string> expected_str = {"123", "123.4", "123.45", "123.456", "123", "123"};

  for (int i = 1; i <= 6; ++i) {
    INFO("Testing column " << i << " with SQL_C_CHAR");
    CHECK(check_char_success(stmt, i) == expected_str[i - 1]);
  }

  for (int i = 1; i <= 6; ++i) {
    INFO("Testing column " << i << " with SQL_C_DEFAULT");
    CHECK(get_data_default_as_string(stmt, i) == expected_str[i - 1]);
  }
}

// ============================================================================
// Test at type limits
// ============================================================================

template <int SQL_C_TYPE>
void test_at_limits(Connection& conn) {
  std::stringstream queryBuilder;
  queryBuilder << "SELECT ";
  queryBuilder << +std::numeric_limits<typename MetaOfSqlCType<SQL_C_TYPE>::type>::max() << " AS max, ";
  queryBuilder << +std::numeric_limits<typename MetaOfSqlCType<SQL_C_TYPE>::type>::min() << " AS min";
  auto query = queryBuilder.str();
  INFO("Executing query: " << query);
  auto stmt = conn.execute_fetch(query);
  CHECK(check_no_truncation<SQL_C_TYPE>(stmt, 1) ==
        std::numeric_limits<typename MetaOfSqlCType<SQL_C_TYPE>::type>::max());
  CHECK(check_no_truncation<SQL_C_TYPE>(stmt, 2) ==
        std::numeric_limits<typename MetaOfSqlCType<SQL_C_TYPE>::type>::min());
}

void test_string_at_limits(Connection& conn) {
  std::string max = std::string(37, '9');
  std::string min = "-" + std::string(37, '9');
  std::stringstream queryBuilder;
  queryBuilder << "SELECT " << max << " AS max, " << min << " AS min";
  auto query = queryBuilder.str();
  INFO("Executing query: " << query);
  auto stmt = conn.execute_fetch(query);
  CHECK(check_char_success(stmt, 1) == max);
  CHECK(check_char_success(stmt, 2) == min);
  CHECK(get_data_default_as_string(stmt, 1) == max);
  CHECK(get_data_default_as_string(stmt, 2) == min);
}

TEST_CASE("Test at limits", "[datatype][number]") {
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);
  test_at_limits<SQL_C_LONG>(conn);
  test_at_limits<SQL_C_SLONG>(conn);
  test_at_limits<SQL_C_ULONG>(conn);
  test_at_limits<SQL_C_SHORT>(conn);
  test_at_limits<SQL_C_SSHORT>(conn);
  test_at_limits<SQL_C_USHORT>(conn);
  test_at_limits<SQL_C_TINYINT>(conn);
  test_at_limits<SQL_C_STINYINT>(conn);
  test_at_limits<SQL_C_UTINYINT>(conn);
  test_at_limits<SQL_C_SBIGINT>(conn);
  test_at_limits<SQL_C_UBIGINT>(conn);
  test_string_at_limits(conn);
}

// ============================================================================
// SQL_DECIMAL default conversion tests (SQL_C_DEFAULT)
// Per ODBC spec, SQL_DECIMAL's default C type is SQL_C_CHAR.
// ============================================================================

TEST_CASE("SQL_DECIMAL default conversion", "[datatype][number][decimal][default]") {
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);

  SECTION("basic values from table") {
    TestTable table(conn, "test_decimal_default",
                    "d1 DECIMAL(10,0), d2 DECIMAL(10,1), d3 DECIMAL(10,2), d4 DECIMAL(10,3)",
                    "(123, 123.4, 123.45, 123.456)");

    auto stmt = conn.execute_fetch("SELECT * FROM " + table.name());
    std::vector<std::string> expected = {"123", "123.4", "123.45", "123.456"};
    for (int i = 1; i <= 4; ++i) {
      INFO("Column " << i);
      CHECK(get_data_default_as_string(stmt, i) == expected[i - 1]);
    }
  }

  SECTION("negative values") {
    auto stmt = conn.execute_fetch(
        "SELECT -123::DECIMAL(10,0), -123.4::DECIMAL(10,1), "
        "-123.45::DECIMAL(10,2), -123.456::DECIMAL(10,3)");
    std::vector<std::string> expected = {"-123", "-123.4", "-123.45", "-123.456"};
    for (int i = 1; i <= 4; ++i) {
      INFO("Column " << i);
      CHECK(get_data_default_as_string(stmt, i) == expected[i - 1]);
    }
  }

  SECTION("zero with varying scale") {
    auto stmt = conn.execute_fetch("SELECT 0::DECIMAL(10,0), 0::DECIMAL(10,2), 0::DECIMAL(10,5)");
    CHECK(get_data_default_as_string(stmt, 1) == "0");
    CHECK(get_data_default_as_string(stmt, 2) == "0.00");
    CHECK(get_data_default_as_string(stmt, 3) == "0.00000");
  }

  SECTION("small values with large scale") {
    auto stmt = conn.execute_fetch(
        "SELECT 0.05::DECIMAL(10,2), 0.001::DECIMAL(10,3), "
        "0.00001::DECIMAL(10,5)");
    CHECK(get_data_default_as_string(stmt, 1) == "0.05");
    CHECK(get_data_default_as_string(stmt, 2) == "0.001");
    CHECK(get_data_default_as_string(stmt, 3) == "0.00001");
  }

  SECTION("negative small fractional values") {
    auto stmt = conn.execute_fetch("SELECT -0.05::DECIMAL(10,2), -0.001::DECIMAL(10,3), -0.5::DECIMAL(10,1)");
    CHECK(check_char_success(stmt, 1) == "-0.05");
    CHECK(check_char_success(stmt, 2) == "-0.001");
    CHECK(check_char_success(stmt, 3) == "-0.5");
    CHECK(get_data_default_as_string(stmt, 1) == "-0.05");
    CHECK(get_data_default_as_string(stmt, 2) == "-0.001");
    CHECK(get_data_default_as_string(stmt, 3) == "-0.5");
  }

  SECTION("various SQL numeric type synonyms") {
    auto stmt = conn.execute_fetch("SELECT 42::NUMBER(10,2), 42::DECIMAL(10,2), 42::NUMERIC(10,2)");
    std::string expected = "42.00";
    for (int i = 1; i <= 3; ++i) {
      INFO("Column " << i << " with SQL_C_CHAR");
      CHECK(check_char_success(stmt, i) == expected);
    }
    for (int i = 1; i <= 3; ++i) {
      INFO("Column " << i << " with SQL_C_DEFAULT");
      CHECK(get_data_default_as_string(stmt, i) == expected);
    }
  }

  SECTION("INT column resolves to SQL_C_CHAR") {
    auto stmt = conn.execute_fetch("SELECT 42::INT, -7::INTEGER, 0::BIGINT");
    CHECK(get_data_default_as_string(stmt, 1) == "42");
    CHECK(get_data_default_as_string(stmt, 2) == "-7");
    CHECK(get_data_default_as_string(stmt, 3) == "0");
  }

  SECTION("matches explicit SQL_C_CHAR via table") {
    TestTable table(conn, "test_decimal_default_vs_char", "d1 DECIMAL(10,1), d2 DECIMAL(20,2), d3 DECIMAL(30,3)",
                    "(123.4, 123.45, 123.456)");

    auto stmt_char = conn.execute_fetch("SELECT * FROM " + table.name());
    auto stmt_default = conn.execute_fetch("SELECT * FROM " + table.name());
    for (int i = 1; i <= 3; ++i) {
      INFO("Column " << i);
      CHECK(check_char_success(stmt_char, i) == get_data_default_as_string(stmt_default, i));
    }
  }
}

TEST_CASE("SQL_DECIMAL default conversion - large precision", "[datatype][number][decimal][default]") {
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);

  SECTION("large values") {
    TestTable table(conn, "test_decimal_large", "a NUMBER(38,0), b NUMBER(38,37)",
                    "(10000000000000000000000000000000000000, "
                    " 1.0000000000000000000000000000000000000)");

    auto stmt = conn.execute_fetch("SELECT * FROM " + table.name());
    CHECK(get_data_default_as_string(stmt, 1) == "10000000000000000000000000000000000000");
    CHECK(get_data_default_as_string(stmt, 2) == "1.0000000000000000000000000000000000000");
  }

  SECTION("max positive and negative values") {
    TestTable table(conn, "test_decimal_max", "a NUMBER(38,0), b NUMBER(38,37)",
                    "(99999999999999999999999999999999999999, "
                    " 9.9999999999999999999999999999999999999), "
                    "(-99999999999999999999999999999999999999, "
                    " -9.9999999999999999999999999999999999999)");

    auto stmt = conn.execute_fetch("SELECT * FROM " + table.name());
    CHECK(get_data_default_as_string(stmt, 1) == "99999999999999999999999999999999999999");
    CHECK(get_data_default_as_string(stmt, 2) == "9.9999999999999999999999999999999999999");

    SQLRETURN ret = SQLFetch(stmt.getHandle());
    CHECK(ret == SQL_SUCCESS);
    CHECK(get_data_default_as_string(stmt, 1) == "-99999999999999999999999999999999999999");
    CHECK(get_data_default_as_string(stmt, 2) == "-9.9999999999999999999999999999999999999");
  }
}

// ============================================================================
// Explicit conversions - integers truncate, floats preserve
// ============================================================================

TEST_CASE("SQL_DECIMAL explicit conversions", "[datatype][number][decimal]") {
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);
  const std::string query = "SELECT 123.789::DECIMAL(10,3)";

  SECTION("integers truncate fractional part") {
    CHECK(check_fractional_truncation<SQL_C_LONG>(conn.execute_fetch(query), 1) == 123);
    CHECK(check_fractional_truncation<SQL_C_SLONG>(conn.execute_fetch(query), 1) == 123);
    CHECK(check_fractional_truncation<SQL_C_ULONG>(conn.execute_fetch(query), 1) == 123);
    CHECK(check_fractional_truncation<SQL_C_SHORT>(conn.execute_fetch(query), 1) == 123);
    CHECK(check_fractional_truncation<SQL_C_SSHORT>(conn.execute_fetch(query), 1) == 123);
    CHECK(check_fractional_truncation<SQL_C_USHORT>(conn.execute_fetch(query), 1) == 123);
    CHECK(check_fractional_truncation<SQL_C_TINYINT>(conn.execute_fetch(query), 1) == 123);
    CHECK(check_fractional_truncation<SQL_C_STINYINT>(conn.execute_fetch(query), 1) == 123);
    CHECK(check_fractional_truncation<SQL_C_UTINYINT>(conn.execute_fetch(query), 1) == 123);
    CHECK(check_fractional_truncation<SQL_C_SBIGINT>(conn.execute_fetch(query), 1) == 123);
    CHECK(check_fractional_truncation<SQL_C_UBIGINT>(conn.execute_fetch(query), 1) == 123);
  }

  SECTION("floating point preserves fractional part") {
    float float_val = check_no_truncation<SQL_C_FLOAT>(conn.execute_fetch(query), 1);
    CHECK(float_val > 123.78f);
    CHECK(float_val < 123.80f);

    double double_val = check_no_truncation<SQL_C_DOUBLE>(conn.execute_fetch(query), 1);
    CHECK(double_val > 123.788);
    CHECK(double_val < 123.790);
  }
}

// ============================================================================
// NULL handling tests
// ============================================================================

TEST_CASE("NUMBER NULL handling", "[datatype][number][null]") {
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);

  SECTION("SQL_C_LONG indicator returns SQL_NULL_DATA") {
    auto stmt = conn.execute_fetch("SELECT NULL::NUMBER(10,0), NULL::DECIMAL(10,2), NULL::NUMERIC(20,5)");
    for (int i = 1; i <= 3; ++i) {
      INFO("Column " << i << " should be NULL");
      check_null_via_get_data(stmt, i, SQL_C_LONG);
    }
  }

  SECTION("SQL_C_CHAR returns SQL_NULL_DATA") {
    auto stmt = conn.execute_fetch("SELECT NULL::DECIMAL(20,5)");
    check_null_via_get_data(stmt, 1, SQL_C_CHAR);
  }

  SECTION("SQL_C_DEFAULT returns SQL_NULL_DATA") {
    auto stmt = conn.execute_fetch("SELECT NULL::DECIMAL(10,2)");
    check_null_via_get_data(stmt, 1, SQL_C_DEFAULT);
  }

  SECTION("SQL_C_WCHAR returns SQL_NULL_DATA") {
    auto stmt = conn.execute_fetch("SELECT NULL::NUMBER(10,0)");
    check_null_via_get_data(stmt, 1, SQL_C_WCHAR);
  }

  SECTION("SQL_C_NUMERIC returns SQL_NULL_DATA") {
    auto stmt = conn.execute_fetch("SELECT NULL::NUMBER(10,2)");
    check_null_via_get_data(stmt, 1, SQL_C_NUMERIC);
  }

  SECTION("SQL_C_BINARY returns SQL_NULL_DATA") {
    auto stmt = conn.execute_fetch("SELECT NULL::NUMBER(10,0)");
    check_null_via_get_data(stmt, 1, SQL_C_BINARY);
  }
}

TEST_CASE("NUMBER NULL mixed with non-NULL in multiple rows", "[datatype][number][null]") {
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);

  TestTable table(conn, "test_number_null", "val NUMBER(10,0)", "(42), (NULL), (-7), (NULL), (0)");

  auto stmt = conn.execute_fetch("SELECT * FROM " + table.name());

  std::vector<std::optional<SQLINTEGER>> expected = {42, std::nullopt, -7, std::nullopt, 0};
  SQLINTEGER value = 0;
  SQLLEN indicator = 0;

  for (size_t row = 0; row < expected.size(); ++row) {
    if (row > 0) {
      SQLRETURN ret = SQLFetch(stmt.getHandle());
      CHECK(ret == SQL_SUCCESS);
    }
    INFO("Row " << row);
    SQLRETURN ret = SQLGetData(stmt.getHandle(), 1, SQL_C_LONG, &value, sizeof(value), &indicator);
    CHECK(ret == SQL_SUCCESS);
    if (expected[row].has_value()) {
      CHECK(indicator != SQL_NULL_DATA);
      CHECK(value == expected[row].value());
    } else {
      CHECK(indicator == SQL_NULL_DATA);
    }
  }
}

// ============================================================================
// Truncation and scale tests
// ============================================================================

TEST_CASE("SQL_DECIMAL truncation and scale", "[datatype][number][truncation]") {
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);

  SECTION("zero with high scale to SQL_C_LONG") {
    auto stmt = conn.execute_fetch("SELECT 0::NUMBER(38,10), 0::NUMBER(38,37), 0::NUMBER(20,15)");
    CHECK(check_no_truncation<SQL_C_LONG>(stmt, 1) == 0);
    CHECK(check_no_truncation<SQL_C_LONG>(stmt, 2) == 0);
    CHECK(check_no_truncation<SQL_C_LONG>(stmt, 3) == 0);
  }

  SECTION("nonzero with high scale to SQL_C_LONG") {
    CHECK(check_no_truncation<SQL_C_LONG>(conn.execute_fetch("SELECT 5.0000000000::NUMBER(38,10)"), 1) == 5);
    CHECK(check_no_truncation<SQL_C_LONG>(conn.execute_fetch("SELECT -3.0000000000::NUMBER(38,10)"), 1) == -3);
  }

  SECTION("fractional values truncate toward zero") {
    auto stmt = conn.execute_fetch(
        "SELECT 0.9::DECIMAL(3,1), -0.9::DECIMAL(3,1), "
        "0.1::DECIMAL(3,1), -0.1::DECIMAL(3,1), "
        "0.5::DECIMAL(3,1), -0.5::DECIMAL(3,1)");
    for (int i = 1; i <= 6; ++i) {
      INFO("Column " << i);
      CHECK(check_fractional_truncation<SQL_C_LONG>(stmt, i) == 0);
    }
  }

  SECTION("values just below type boundary") {
    auto stmt = conn.execute_fetch(
        "SELECT 1.99::DECIMAL(5,2), -1.99::DECIMAL(5,2), "
        "127.99::DECIMAL(5,2), -128.99::DECIMAL(6,2)");
    CHECK(check_fractional_truncation<SQL_C_LONG>(stmt, 1) == 1);
    CHECK(check_fractional_truncation<SQL_C_LONG>(stmt, 2) == -1);
    CHECK(check_fractional_truncation<SQL_C_STINYINT>(stmt, 3) == 127);
    CHECK(check_fractional_truncation<SQL_C_STINYINT>(stmt, 4) == -128);
  }

  SECTION("exact scale division - no fractional remainder") {
    auto stmt = conn.execute_fetch(
        "SELECT 100.00::DECIMAL(10,2), 0.50::DECIMAL(10,2), "
        "-25.00::DECIMAL(10,2), 1.00::DECIMAL(10,2)");
    CHECK(check_no_truncation<SQL_C_LONG>(stmt, 1) == 100);
    CHECK(check_fractional_truncation<SQL_C_LONG>(stmt, 2) == 0);
    CHECK(check_no_truncation<SQL_C_LONG>(stmt, 3) == -25);
    CHECK(check_no_truncation<SQL_C_LONG>(stmt, 4) == 1);
  }
}

// ============================================================================
// Scale=0 pure integer tests
// ============================================================================

TEST_CASE("NUMBER scale=0 - INT and INTEGER types", "[datatype][number][integer]") {
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);

  TestTable table(conn, "test_int_types", "a INT, b INTEGER, c BIGINT, d SMALLINT, e TINYINT",
                  "(100, -200, 9223372036854775807, -32000, 120)");

  auto stmt = conn.execute_fetch("SELECT * FROM " + table.name());

  CHECK(check_no_truncation<SQL_C_LONG>(stmt, 1) == 100);
  CHECK(check_no_truncation<SQL_C_LONG>(stmt, 2) == -200);
  CHECK(check_no_truncation<SQL_C_SBIGINT>(stmt, 3) == 9223372036854775807LL);
  CHECK(check_no_truncation<SQL_C_SHORT>(stmt, 4) == -32000);
  CHECK(check_no_truncation<SQL_C_TINYINT>(stmt, 5) == 120);

  CHECK(check_char_success(stmt, 1) == "100");
  CHECK(check_char_success(stmt, 2) == "-200");
  CHECK(check_char_success(stmt, 3) == "9223372036854775807");
  CHECK(check_char_success(stmt, 4) == "-32000");
  CHECK(check_char_success(stmt, 5) == "120");
}

// ============================================================================
// SQL_C_CHAR buffer truncation tests
// ============================================================================

TEST_CASE("SQL_DECIMAL SQL_C_CHAR buffer handling", "[datatype][number][char][buffer]") {
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);

  SECTION("whole digits do not fit returns 22003") {
    SKIP_OLD_DRIVER("BD#13",
                    "Old driver returns SQL_SUCCESS instead of SQL_ERROR (22003) when whole digits do not fit in "
                    "SQL_C_CHAR buffer");

    auto stmt = conn.execute_fetch("SELECT 123456::NUMBER(10,0)");

    char small_buffer[4];
    SQLLEN indicator = 0;
    SQLRETURN ret = SQLGetData(stmt.getHandle(), 1, SQL_C_CHAR, small_buffer, sizeof(small_buffer), &indicator);

    CHECK(ret == SQL_ERROR);
    CHECK(get_sqlstate(stmt) == "22003");
  }

  SECTION("fractional-only truncation returns 01004") {
    SKIP_OLD_DRIVER("BD#11",
                    "SNOW-3120035: Old driver returns SQL_SUCCESS instead of SQL_SUCCESS_WITH_INFO for truncation");

    auto stmt = conn.execute_fetch("SELECT 12.345::DECIMAL(10,3)");

    char small_buffer[4];
    SQLLEN indicator = 0;
    SQLRETURN ret = SQLGetData(stmt.getHandle(), 1, SQL_C_CHAR, small_buffer, sizeof(small_buffer), &indicator);

    CHECK(ret == SQL_SUCCESS_WITH_INFO);
    CHECK(get_sqlstate(stmt) == "01004");
  }

  SECTION("exact buffer fits") {
    auto stmt = conn.execute_fetch("SELECT 42::NUMBER(10,0)");

    char exact_buffer[3];
    SQLLEN indicator = 0;
    SQLRETURN ret = SQLGetData(stmt.getHandle(), 1, SQL_C_CHAR, exact_buffer, sizeof(exact_buffer), &indicator);
    CHECK(ret == SQL_SUCCESS);
    CHECK(indicator == 2);
    CHECK(std::string(exact_buffer) == "42");
  }

  SECTION("negative whole digits do not fit returns 22003") {
    SKIP_OLD_DRIVER("BD#13",
                    "Old driver returns SQL_SUCCESS instead of SQL_ERROR (22003) when whole digits do not fit in "
                    "SQL_C_CHAR buffer");

    auto stmt = conn.execute_fetch("SELECT -123::NUMBER(10,0)");

    char small_buffer[4];
    SQLLEN indicator = 0;
    SQLRETURN ret = SQLGetData(stmt.getHandle(), 1, SQL_C_CHAR, small_buffer, sizeof(small_buffer), &indicator);

    CHECK(ret == SQL_ERROR);
    CHECK(get_sqlstate(stmt) == "22003");
  }

  SECTION("negative fractional-only truncation returns 01004") {
    SKIP_OLD_DRIVER("BD#11",
                    "SNOW-3120035: Old driver returns SQL_SUCCESS instead of SQL_SUCCESS_WITH_INFO for truncation");

    auto stmt = conn.execute_fetch("SELECT -12.345::DECIMAL(10,3)");

    char small_buffer[5];
    SQLLEN indicator = 0;
    SQLRETURN ret = SQLGetData(stmt.getHandle(), 1, SQL_C_CHAR, small_buffer, sizeof(small_buffer), &indicator);

    CHECK(ret == SQL_SUCCESS_WITH_INFO);
    CHECK(get_sqlstate(stmt) == "01004");
  }
}

// ============================================================================
// Multiple rows with varying scales and mixed positive/negative
// ============================================================================

TEST_CASE("DECIMAL multiple rows", "[datatype][number][multirow]") {
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);

  SECTION("various values as SQL_C_CHAR") {
    TestTable table(conn, "test_number_multi", "val DECIMAL(10,2)",
                    "(0.00), (1.00), (-1.00), (999.99), (-999.99), (0.01), (-0.01)");

    auto stmt = conn.execute_fetch("SELECT * FROM " + table.name());
    std::vector<std::string> expected = {"0.00", "1.00", "-1.00", "999.99", "-999.99", "0.01", "-0.01"};

    for (size_t row = 0; row < expected.size(); ++row) {
      if (row > 0) {
        SQLRETURN ret = SQLFetch(stmt.getHandle());
        CHECK(ret == SQL_SUCCESS);
      }
      INFO("Row " << row << " expected: " << expected[row]);
      CHECK(check_char_success(stmt, 1) == expected[row]);
    }
  }

  SECTION("various values as SQL_C_DOUBLE") {
    TestTable table(conn, "test_number_double_multi", "val DECIMAL(10,3)", "(0.000), (1.500), (-2.750), (100.125)");

    auto stmt = conn.execute_fetch("SELECT * FROM " + table.name());
    std::vector<double> expected = {0.0, 1.5, -2.75, 100.125};

    for (size_t row = 0; row < expected.size(); ++row) {
      if (row > 0) {
        SQLRETURN ret = SQLFetch(stmt.getHandle());
        CHECK(ret == SQL_SUCCESS);
      }
      INFO("Row " << row << " expected double: " << expected[row]);
      CHECK(check_no_truncation<SQL_C_DOUBLE>(stmt, 1) == expected[row]);
    }
  }
}

// ============================================================================
// SQL_C_DOUBLE/SQL_C_FLOAT precision checks
// ============================================================================

TEST_CASE("DECIMAL to floating point precision", "[datatype][number][precision]") {
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);

  SECTION("SQL_C_DOUBLE with 15 significant digits") {
    auto stmt = conn.execute_fetch("SELECT 123456789012345::NUMBER(15,0)");
    CHECK(check_no_truncation<SQL_C_DOUBLE>(stmt, 1) == 123456789012345.0);
  }

  SECTION("SQL_C_FLOAT with 6 significant digits") {
    auto stmt = conn.execute_fetch("SELECT 123456::NUMBER(10,0)");
    CHECK(check_no_truncation<SQL_C_FLOAT>(stmt, 1) == 123456.0f);
  }
}

// ============================================================================
// SQL_C_WCHAR (wide character) conversion tests
// ============================================================================

TEST_CASE("SQL_DECIMAL to SQL_C_WCHAR", "[datatype][number][wchar]") {
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);

  SECTION("basic values") {
    auto stmt = conn.execute_fetch(
        "SELECT 42::NUMBER(10,0), -7::NUMBER(10,0), 0::NUMBER(10,0), "
        "123.45::NUMBER(10,2), -0.05::NUMBER(10,2)");
    CHECK(check_wchar_success(stmt, 1) == u"42");
    CHECK(check_wchar_success(stmt, 2) == u"-7");
    CHECK(check_wchar_success(stmt, 3) == u"0");
    CHECK(check_wchar_success(stmt, 4) == u"123.45");
    CHECK(check_wchar_success(stmt, 5) == u"-0.05");
  }

  SECTION("large precision values") {
    auto stmt = conn.execute_fetch("SELECT 99999999999999999999999999999999999999::NUMBER(38,0)");
    CHECK(check_wchar_success(stmt, 1) == u"99999999999999999999999999999999999999");
  }

  SECTION("matches SQL_C_CHAR") {
    const std::string query = "SELECT 123.456::DECIMAL(10,3)";
    auto char_str = check_char_success(conn.execute_fetch(query), 1);
    auto wchar_str = check_wchar_success(conn.execute_fetch(query), 1);
    std::u16string expected_wchar(char_str.begin(), char_str.end());
    CHECK(wchar_str == expected_wchar);
  }
}

// ============================================================================
// SQL_C_NUMERIC (SQL_NUMERIC_STRUCT) conversion tests
// ============================================================================

inline unsigned long long numeric_val_to_ull(const SQL_NUMERIC_STRUCT& n) {
  unsigned long long result = 0;
  for (int i = 7; i >= 0; --i) {
    result = (result << 8) | n.val[i];
  }
  return result;
}

TEST_CASE("SQL_DECIMAL to SQL_C_NUMERIC", "[datatype][number][numeric]") {
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);

  SECTION("positive integer") {
    auto numeric = check_no_truncation<SQL_C_NUMERIC>(conn.execute_fetch("SELECT 42::NUMBER(10,0)"), 1);
    CHECK(numeric.sign == 1);
    CHECK(numeric.val[0] == 42);
    check_numeric_val_zero_from(numeric, 1);
  }

  SECTION("negative value") {
    auto numeric = check_no_truncation<SQL_C_NUMERIC>(conn.execute_fetch("SELECT -123::NUMBER(10,0)"), 1);
    CHECK(numeric.sign == 0);
    CHECK(numeric.val[0] == 123);
    check_numeric_val_zero_from(numeric, 1);
  }

  SECTION("zero") {
    auto numeric = check_no_truncation<SQL_C_NUMERIC>(conn.execute_fetch("SELECT 0::NUMBER(10,0)"), 1);
    CHECK(numeric.sign == 1);
    check_numeric_val_zero_from(numeric, 0);
  }

  SECTION("with scale defaults to scale=0 truncation") {
    auto numeric = check_fractional_truncation<SQL_C_NUMERIC>(conn.execute_fetch("SELECT 123.45::NUMBER(10,2)"), 1);
    CHECK(numeric.sign == 1);
    CHECK(numeric_val_to_ull(numeric) == 123);
  }
}

// ============================================================================
// SQL_C_NUMERIC with explicit SQL_DESC_PRECISION / SQL_DESC_SCALE
// Tests the interaction between source DECIMAL(p,s)
// and target descriptor precision/scale.
// ============================================================================

TEST_CASE("SQL_DECIMAL to SQL_C_NUMERIC with SQL_DESC_PRECISION and SQL_DESC_SCALE",
          "[datatype][number][numeric][descriptor]") {
  SKIP_OLD_DRIVER("BD#15", "Old driver ignores SQL_DESC_PRECISION and SQL_DESC_SCALE set via SQLSetDescField");
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);

  SECTION("target scale matches source scale - no truncation") {
    auto stmt = conn.execute_fetch("SELECT 123.45::DECIMAL(10,2)");

    SQLHDESC ard = SQL_NULL_HDESC;
    SQLRETURN ret = SQLGetStmtAttr(stmt.getHandle(), SQL_ATTR_APP_ROW_DESC, &ard, 0, NULL);
    REQUIRE(ret == SQL_SUCCESS);
    ret = SQLSetDescField(ard, 1, SQL_DESC_PRECISION, (SQLPOINTER)10, 0);
    REQUIRE(ret == SQL_SUCCESS);
    ret = SQLSetDescField(ard, 1, SQL_DESC_SCALE, (SQLPOINTER)2, 0);
    REQUIRE(ret == SQL_SUCCESS);

    SQL_NUMERIC_STRUCT numeric = {};
    SQLLEN indicator = 0;
    ret = SQLGetData(stmt.getHandle(), 1, SQL_C_NUMERIC, &numeric, sizeof(numeric), &indicator);

    CHECK(ret == SQL_SUCCESS);
    CHECK(numeric.precision == 10);
    CHECK(numeric.scale == 2);
    CHECK(numeric.sign == 1);
    CHECK(numeric_val_to_ull(numeric) == 12345);
  }

  SECTION("target scale=0 truncates fractional part - 01S07") {
    auto stmt = conn.execute_fetch("SELECT 123.45::DECIMAL(10,2)");

    SQLHDESC ard = SQL_NULL_HDESC;
    SQLRETURN ret = SQLGetStmtAttr(stmt.getHandle(), SQL_ATTR_APP_ROW_DESC, &ard, 0, NULL);
    REQUIRE(ret == SQL_SUCCESS);
    ret = SQLSetDescField(ard, 1, SQL_DESC_PRECISION, (SQLPOINTER)10, 0);
    REQUIRE(ret == SQL_SUCCESS);
    ret = SQLSetDescField(ard, 1, SQL_DESC_SCALE, (SQLPOINTER)0, 0);
    REQUIRE(ret == SQL_SUCCESS);

    SQL_NUMERIC_STRUCT numeric = {};
    SQLLEN indicator = 0;
    ret = SQLGetData(stmt.getHandle(), 1, SQL_C_NUMERIC, &numeric, sizeof(numeric), &indicator);

    CHECK(ret == SQL_SUCCESS_WITH_INFO);
    CHECK(get_sqlstate(stmt) == "01S07");
    CHECK(numeric.precision == 10);
    CHECK(numeric.scale == 0);
    CHECK(numeric.sign == 1);
    CHECK(numeric_val_to_ull(numeric) == 123);
  }

  SECTION("target scale > source scale upscales value") {
    auto stmt = conn.execute_fetch("SELECT 42::NUMBER(10,0)");

    SQLHDESC ard = SQL_NULL_HDESC;
    SQLRETURN ret = SQLGetStmtAttr(stmt.getHandle(), SQL_ATTR_APP_ROW_DESC, &ard, 0, NULL);
    REQUIRE(ret == SQL_SUCCESS);
    ret = SQLSetDescField(ard, 1, SQL_DESC_PRECISION, (SQLPOINTER)10, 0);
    REQUIRE(ret == SQL_SUCCESS);
    ret = SQLSetDescField(ard, 1, SQL_DESC_SCALE, (SQLPOINTER)3, 0);
    REQUIRE(ret == SQL_SUCCESS);

    SQL_NUMERIC_STRUCT numeric = {};
    SQLLEN indicator = 0;
    ret = SQLGetData(stmt.getHandle(), 1, SQL_C_NUMERIC, &numeric, sizeof(numeric), &indicator);

    CHECK(ret == SQL_SUCCESS);
    CHECK(numeric.precision == 10);
    CHECK(numeric.scale == 3);
    CHECK(numeric.sign == 1);
    CHECK(numeric_val_to_ull(numeric) == 42000);
  }

  SECTION("target scale < source scale with exact division - no truncation") {
    auto stmt = conn.execute_fetch("SELECT 12.300::DECIMAL(10,3)");

    SQLHDESC ard = SQL_NULL_HDESC;
    SQLRETURN ret = SQLGetStmtAttr(stmt.getHandle(), SQL_ATTR_APP_ROW_DESC, &ard, 0, NULL);
    REQUIRE(ret == SQL_SUCCESS);
    ret = SQLSetDescField(ard, 1, SQL_DESC_PRECISION, (SQLPOINTER)10, 0);
    REQUIRE(ret == SQL_SUCCESS);
    ret = SQLSetDescField(ard, 1, SQL_DESC_SCALE, (SQLPOINTER)1, 0);
    REQUIRE(ret == SQL_SUCCESS);

    SQL_NUMERIC_STRUCT numeric = {};
    SQLLEN indicator = 0;
    ret = SQLGetData(stmt.getHandle(), 1, SQL_C_NUMERIC, &numeric, sizeof(numeric), &indicator);

    CHECK(ret == SQL_SUCCESS);
    CHECK(numeric.precision == 10);
    CHECK(numeric.scale == 1);
    CHECK(numeric.sign == 1);
    CHECK(numeric_val_to_ull(numeric) == 123);
  }

  SECTION("target scale < source scale with remainder - 01S07") {
    auto stmt = conn.execute_fetch("SELECT 1.999::DECIMAL(10,3)");

    SQLHDESC ard = SQL_NULL_HDESC;
    SQLRETURN ret = SQLGetStmtAttr(stmt.getHandle(), SQL_ATTR_APP_ROW_DESC, &ard, 0, NULL);
    REQUIRE(ret == SQL_SUCCESS);
    ret = SQLSetDescField(ard, 1, SQL_DESC_PRECISION, (SQLPOINTER)10, 0);
    REQUIRE(ret == SQL_SUCCESS);
    ret = SQLSetDescField(ard, 1, SQL_DESC_SCALE, (SQLPOINTER)1, 0);
    REQUIRE(ret == SQL_SUCCESS);

    SQL_NUMERIC_STRUCT numeric = {};
    SQLLEN indicator = 0;
    ret = SQLGetData(stmt.getHandle(), 1, SQL_C_NUMERIC, &numeric, sizeof(numeric), &indicator);

    CHECK(ret == SQL_SUCCESS_WITH_INFO);
    CHECK(get_sqlstate(stmt) == "01S07");
    CHECK(numeric.precision == 10);
    CHECK(numeric.scale == 1);
    CHECK(numeric.sign == 1);
    CHECK(numeric_val_to_ull(numeric) == 19);
  }

  SECTION("custom precision is reflected in output struct") {
    auto stmt = conn.execute_fetch("SELECT 42::NUMBER(38,0)");

    SQLHDESC ard = SQL_NULL_HDESC;
    SQLRETURN ret = SQLGetStmtAttr(stmt.getHandle(), SQL_ATTR_APP_ROW_DESC, &ard, 0, NULL);
    REQUIRE(ret == SQL_SUCCESS);
    ret = SQLSetDescField(ard, 1, SQL_DESC_PRECISION, (SQLPOINTER)5, 0);
    REQUIRE(ret == SQL_SUCCESS);
    ret = SQLSetDescField(ard, 1, SQL_DESC_SCALE, (SQLPOINTER)0, 0);
    REQUIRE(ret == SQL_SUCCESS);

    SQL_NUMERIC_STRUCT numeric = {};
    SQLLEN indicator = 0;
    ret = SQLGetData(stmt.getHandle(), 1, SQL_C_NUMERIC, &numeric, sizeof(numeric), &indicator);

    CHECK(ret == SQL_SUCCESS);
    CHECK(numeric.precision == 5);
    CHECK(numeric.scale == 0);
    CHECK(numeric_val_to_ull(numeric) == 42);
  }

  SECTION("negative value with upscale") {
    auto stmt = conn.execute_fetch("SELECT -7::NUMBER(10,0)");

    SQLHDESC ard = SQL_NULL_HDESC;
    SQLRETURN ret = SQLGetStmtAttr(stmt.getHandle(), SQL_ATTR_APP_ROW_DESC, &ard, 0, NULL);
    REQUIRE(ret == SQL_SUCCESS);
    ret = SQLSetDescField(ard, 1, SQL_DESC_PRECISION, (SQLPOINTER)10, 0);
    REQUIRE(ret == SQL_SUCCESS);
    ret = SQLSetDescField(ard, 1, SQL_DESC_SCALE, (SQLPOINTER)2, 0);
    REQUIRE(ret == SQL_SUCCESS);

    SQL_NUMERIC_STRUCT numeric = {};
    SQLLEN indicator = 0;
    ret = SQLGetData(stmt.getHandle(), 1, SQL_C_NUMERIC, &numeric, sizeof(numeric), &indicator);

    CHECK(ret == SQL_SUCCESS);
    CHECK(numeric.sign == 0);
    CHECK(numeric.scale == 2);
    CHECK(numeric_val_to_ull(numeric) == 700);
  }

  SECTION("zero with non-zero target scale") {
    auto stmt = conn.execute_fetch("SELECT 0::NUMBER(10,0)");

    SQLHDESC ard = SQL_NULL_HDESC;
    SQLRETURN ret = SQLGetStmtAttr(stmt.getHandle(), SQL_ATTR_APP_ROW_DESC, &ard, 0, NULL);
    REQUIRE(ret == SQL_SUCCESS);
    ret = SQLSetDescField(ard, 1, SQL_DESC_PRECISION, (SQLPOINTER)10, 0);
    REQUIRE(ret == SQL_SUCCESS);
    ret = SQLSetDescField(ard, 1, SQL_DESC_SCALE, (SQLPOINTER)5, 0);
    REQUIRE(ret == SQL_SUCCESS);

    SQL_NUMERIC_STRUCT numeric = {};
    SQLLEN indicator = 0;
    ret = SQLGetData(stmt.getHandle(), 1, SQL_C_NUMERIC, &numeric, sizeof(numeric), &indicator);

    CHECK(ret == SQL_SUCCESS);
    CHECK(numeric.scale == 5);
    CHECK(numeric_val_to_ull(numeric) == 0);
  }
}

// ============================================================================
// SQL_C_BINARY conversion tests
// ============================================================================

TEST_CASE("SQL_DECIMAL to SQL_C_BINARY", "[datatype][number][binary]") {
  SKIP_OLD_DRIVER("BD#12", "SNOW-3127864: Old driver fix to be merged to return the proper size for SQL_NUMERIC");
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);

  SECTION("integer value") {
    auto num = get_binary_as_numeric(conn.execute_fetch("SELECT 42::NUMBER(10,0)"), 1);
    CHECK(num.sign == 1);
    CHECK(num.val[0] == 42);
    check_numeric_val_zero_from(num, 1);
  }

  SECTION("scaled value") {
    auto num = get_binary_as_numeric(conn.execute_fetch("SELECT 123.45::NUMBER(10,2)"), 1);
    CHECK(num.sign == 1);
    CHECK(num.scale == 0);
    CHECK(num.val[0] == 123);
    check_numeric_val_zero_from(num, 1);
  }

  SECTION("negative value") {
    auto num = get_binary_as_numeric(conn.execute_fetch("SELECT -7::NUMBER(10,0)"), 1);
    CHECK(num.sign == 0);
    CHECK(num.val[0] == 7);
    check_numeric_val_zero_from(num, 1);
  }
}

// ============================================================================
// SQL_C_BINARY buffer truncation
// Per ODBC spec: when byte length of data > BufferLength → 22003
// ============================================================================

TEST_CASE("SQL_DECIMAL SQL_C_BINARY buffer too small returns 22003", "[datatype][number][binary][22003]") {
  SKIP_OLD_DRIVER(
      "BD#14",
      "Old driver does not return SQL_ERROR (22003) when SQL_C_BINARY buffer is too small for SQL_NUMERIC_STRUCT");
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);

  auto stmt = conn.execute_fetch("SELECT 42::NUMBER(10,0)");

  char tiny_buffer[4];
  SQLLEN indicator = 0;
  SQLRETURN ret = SQLGetData(stmt.getHandle(), 1, SQL_C_BINARY, tiny_buffer, sizeof(tiny_buffer), &indicator);

  CHECK(ret == SQL_ERROR);
  CHECK(get_sqlstate(stmt) == "22003");
}

// ============================================================================
// ODBC Spec: SQLSTATE 01S07 — Fractional truncation warning
// Per ODBC spec, converting a numeric SQL value with fractional digits to an
// integer C type should return SQL_SUCCESS_WITH_INFO with SQLSTATE 01S07.
// ============================================================================

TEST_CASE("SQL_DECIMAL fractional truncation returns 01S07", "[datatype][number][01S07]") {
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);

  SECTION("SQL_C_LONG") {
    auto stmt = conn.execute_fetch("SELECT 123.45::DECIMAL(10,2)");
    auto value = check_fractional_truncation<SQL_C_LONG>(stmt, 1);
    CHECK(value == 123);
  }

  SECTION("SQL_C_SHORT") {
    auto stmt = conn.execute_fetch("SELECT 9.99::DECIMAL(5,2)");
    auto value = check_fractional_truncation<SQL_C_SHORT>(stmt, 1);
    CHECK(value == 9);
  }

  SECTION("SQL_C_STINYINT") {
    auto stmt = conn.execute_fetch("SELECT 1.5::DECIMAL(3,1)");
    auto value = check_fractional_truncation<SQL_C_STINYINT>(stmt, 1);
    CHECK(value == 1);
  }

  SECTION("SQL_C_SBIGINT") {
    auto stmt = conn.execute_fetch("SELECT 999.001::DECIMAL(10,3)");
    auto value = check_fractional_truncation<SQL_C_SBIGINT>(stmt, 1);
    CHECK(value == 999);
  }

  SECTION("exact integer does NOT produce 01S07") {
    auto stmt = conn.execute_fetch("SELECT 100.00::DECIMAL(10,2)");
    auto value = check_no_truncation<SQL_C_LONG>(stmt, 1);
    CHECK(value == 100);
  }
}

// ============================================================================
// ODBC Spec: SQLSTATE 22003 — Numeric value out of range (integer overflow)
// Per ODBC spec, when conversion would result in loss of whole (not fractional)
// digits, the driver should return SQL_ERROR with SQLSTATE 22003.
// ============================================================================

TEST_CASE("SQL_DECIMAL overflow returns 22003", "[datatype][number][22003]") {
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);

  SECTION("SQL_C_LONG - above i32 max") {
    auto stmt = conn.execute_fetch("SELECT 2147483648::NUMBER(10,0)");
    check_numeric_out_of_range<SQL_C_LONG>(stmt, 1);
  }

  SECTION("SQL_C_LONG - below i32 min") {
    auto stmt = conn.execute_fetch("SELECT -2147483649::NUMBER(10,0)");
    check_numeric_out_of_range<SQL_C_LONG>(stmt, 1);
  }

  SECTION("SQL_C_SHORT - above i16 max") {
    auto stmt = conn.execute_fetch("SELECT 32768::NUMBER(5,0)");
    check_numeric_out_of_range<SQL_C_SHORT>(stmt, 1);
  }

  SECTION("SQL_C_STINYINT - above i8 max") {
    auto stmt = conn.execute_fetch("SELECT 128::NUMBER(3,0)");
    check_numeric_out_of_range<SQL_C_STINYINT>(stmt, 1);
  }

  SECTION("SQL_C_STINYINT - below i8 min") {
    auto stmt = conn.execute_fetch("SELECT -129::NUMBER(3,0)");
    check_numeric_out_of_range<SQL_C_STINYINT>(stmt, 1);
  }

  SECTION("SQL_C_UTINYINT - negative") {
    auto stmt = conn.execute_fetch("SELECT -1::NUMBER(1,0)");
    check_numeric_out_of_range<SQL_C_UTINYINT>(stmt, 1);
  }

  SECTION("SQL_C_UTINYINT - above u8 max") {
    auto stmt = conn.execute_fetch("SELECT 256::NUMBER(3,0)");
    check_numeric_out_of_range<SQL_C_UTINYINT>(stmt, 1);
  }

  SECTION("SQL_C_USHORT - negative") {
    auto stmt = conn.execute_fetch("SELECT -1::NUMBER(1,0)");
    check_numeric_out_of_range<SQL_C_USHORT>(stmt, 1);
  }

  SECTION("SQL_C_ULONG - negative") {
    auto stmt = conn.execute_fetch("SELECT -1::NUMBER(1,0)");
    check_numeric_out_of_range<SQL_C_ULONG>(stmt, 1);
  }
}

// ============================================================================
// ODBC Spec: SQL_C_BIT — spec-compliant behavior
// Per ODBC spec for SQL_C_BIT:
//   Exact 0 or 1                    -> SQL_SUCCESS
//   Value > 0, < 2, != 1 (fraction) -> SQL_SUCCESS_WITH_INFO, 01S07
//   Value < 0 or >= 2               -> SQL_ERROR, 22003
// ============================================================================

TEST_CASE("SQL_C_BIT spec compliance", "[datatype][number][bit]") {
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);

  SECTION("0 and 1 succeed") {
    auto stmt = conn.execute_fetch("SELECT 0::NUMBER(10,0), 1::NUMBER(10,0), 0.00::NUMBER(10,2)");
    CHECK(check_no_truncation<SQL_C_BIT>(stmt, 1) == 0);
    CHECK(check_no_truncation<SQL_C_BIT>(stmt, 2) == 1);
    CHECK(check_no_truncation<SQL_C_BIT>(stmt, 3) == 0);
  }

  SECTION("negative integer is out of range (22003)") {
    auto stmt = conn.execute_fetch("SELECT -1::NUMBER(10,0)");
    check_numeric_out_of_range<SQL_C_BIT>(stmt, 1);
  }

  SECTION("value 2 returns 22003") {
    auto stmt = conn.execute_fetch("SELECT 2::NUMBER(1,0)");
    check_numeric_out_of_range<SQL_C_BIT>(stmt, 1);
  }

  SECTION("negative fractional value returns 22003") {
    auto stmt = conn.execute_fetch("SELECT -0.5::DECIMAL(3,1)");
    check_numeric_out_of_range<SQL_C_BIT>(stmt, 1);
  }

  SECTION("fractional 0.5 truncates to 0 with 01S07") {
    auto stmt = conn.execute_fetch("SELECT 0.5::DECIMAL(3,1)");
    auto value = check_fractional_truncation<SQL_C_BIT>(stmt, 1);
    CHECK(value == 0);
  }

  SECTION("fractional 1.5 truncates to 1 with 01S07") {
    auto stmt = conn.execute_fetch("SELECT 1.5::DECIMAL(3,1)");
    auto value = check_fractional_truncation<SQL_C_BIT>(stmt, 1);
    CHECK(value == 1);
  }

  SECTION("exact 1.0 does NOT produce warning") {
    auto stmt = conn.execute_fetch("SELECT 1.00::DECIMAL(5,2)");
    auto value = check_no_truncation<SQL_C_BIT>(stmt, 1);
    CHECK(value == 1);
  }
}

TEST_CASE("SQL_DECIMAL SQLGetData NULL without indicator returns 22002", "[datatype][number][null]") {
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);

  auto stmt = conn.execute_fetch("SELECT NULL::DECIMAL(10,2)");

  SQLINTEGER value = 0;
  SQLRETURN ret = SQLGetData(stmt.getHandle(), 1, SQL_C_LONG, &value, sizeof(value), nullptr);
  CHECK(ret == SQL_ERROR);
  CHECK(get_sqlstate(stmt) == "22002");
}

// ============================================================================
// TREAT_DECIMAL_AS_INT — FIXED with scale=0 reported as SQL_BIGINT
// ============================================================================

TEST_CASE("TREAT_DECIMAL_AS_INT SQL_C_DEFAULT resolves to SBIGINT for scale=0",
          "[datatype][number][treat_decimal_as_int]") {
  Connection conn;
  conn.execute("ALTER SESSION SET ODBC_TREAT_DECIMAL_AS_INT=true");
  auto random_schema = Schema::use_random_schema(conn);

  SECTION("positive integer") {
    auto stmt = conn.execute_fetch("SELECT 42::DECIMAL(10,0)");

    SQLBIGINT value = 0;
    SQLLEN indicator = -999;
    SQLRETURN ret = SQLGetData(stmt.getHandle(), 1, SQL_C_DEFAULT, &value, sizeof(value), &indicator);
    CHECK(ret == SQL_SUCCESS);
    CHECK(indicator == sizeof(SQLBIGINT));
    CHECK(value == 42);
  }

  SECTION("negative integer") {
    auto stmt = conn.execute_fetch("SELECT -123::DECIMAL(10,0)");

    SQLBIGINT value = 0;
    SQLLEN indicator = -999;
    SQLRETURN ret = SQLGetData(stmt.getHandle(), 1, SQL_C_DEFAULT, &value, sizeof(value), &indicator);
    CHECK(ret == SQL_SUCCESS);
    CHECK(indicator == sizeof(SQLBIGINT));
    CHECK(value == -123);
  }

  SECTION("zero") {
    auto stmt = conn.execute_fetch("SELECT 0::DECIMAL(10,0)");

    SQLBIGINT value = -1;
    SQLLEN indicator = -999;
    SQLRETURN ret = SQLGetData(stmt.getHandle(), 1, SQL_C_DEFAULT, &value, sizeof(value), &indicator);
    CHECK(ret == SQL_SUCCESS);
    CHECK(indicator == sizeof(SQLBIGINT));
    CHECK(value == 0);
  }

  SECTION("max precision 18") {
    auto stmt = conn.execute_fetch("SELECT 999999999999999999::DECIMAL(18,0)");

    SQLBIGINT value = 0;
    SQLLEN indicator = -999;
    SQLRETURN ret = SQLGetData(stmt.getHandle(), 1, SQL_C_DEFAULT, &value, sizeof(value), &indicator);
    CHECK(ret == SQL_SUCCESS);
    CHECK(indicator == sizeof(SQLBIGINT));
    CHECK(value == 999999999999999999LL);
  }
}

TEST_CASE("TREAT_DECIMAL_AS_INT does not affect scale > 0", "[datatype][number][treat_decimal_as_int]") {
  Connection conn;
  conn.execute("ALTER SESSION SET ODBC_TREAT_DECIMAL_AS_INT=true");
  auto random_schema = Schema::use_random_schema(conn);

  auto stmt = conn.execute_fetch("SELECT 123.45::DECIMAL(10,2)");

  char buffer[100];
  SQLLEN indicator = -999;
  SQLRETURN ret = SQLGetData(stmt.getHandle(), 1, SQL_C_DEFAULT, buffer, sizeof(buffer), &indicator);
  CHECK(ret == SQL_SUCCESS);
  CHECK(indicator > 0);
  CHECK(std::string(buffer, indicator) == "123.45");
}

TEST_CASE("TREAT_DECIMAL_AS_INT applies to precision > 18 too", "[datatype][number][treat_decimal_as_int]") {
  Connection conn;
  conn.execute("ALTER SESSION SET ODBC_TREAT_DECIMAL_AS_INT=true");
  auto random_schema = Schema::use_random_schema(conn);

  auto stmt = conn.execute_fetch("SELECT 42::NUMBER(38,0)");

  SQLBIGINT value = 0;
  SQLLEN indicator = -999;
  SQLRETURN ret = SQLGetData(stmt.getHandle(), 1, SQL_C_DEFAULT, &value, sizeof(value), &indicator);
  CHECK(ret == SQL_SUCCESS);
  CHECK(indicator == sizeof(SQLBIGINT));
  CHECK(value == 42);
}

TEST_CASE("TREAT_BIG_NUMBER_AS_STRING overrides TREAT_DECIMAL_AS_INT for precision > 18",
          "[datatype][number][treat_big_number_as_string]") {
  Connection conn;
  conn.execute("ALTER SESSION SET ODBC_TREAT_DECIMAL_AS_INT=true");
  conn.execute("ALTER SESSION SET ODBC_TREAT_BIG_NUMBER_AS_STRING=true");
  auto random_schema = Schema::use_random_schema(conn);

  SECTION("NUMBER(38,0) resolves to SQL_C_CHAR when both settings are true") {
    auto stmt = conn.execute_fetch("SELECT 42::NUMBER(38,0)");

    char buffer[100];
    SQLLEN indicator = -999;
    SQLRETURN ret = SQLGetData(stmt.getHandle(), 1, SQL_C_DEFAULT, buffer, sizeof(buffer), &indicator);
    CHECK(ret == SQL_SUCCESS);
    CHECK(indicator > 0);
    CHECK(std::string(buffer, indicator) == "42");
  }

  SECTION("DECIMAL(10,0) still resolves to SBIGINT (precision <= 18)") {
    auto stmt = conn.execute_fetch("SELECT 42::DECIMAL(10,0)");

    SQLBIGINT value = 0;
    SQLLEN indicator = -999;
    SQLRETURN ret = SQLGetData(stmt.getHandle(), 1, SQL_C_DEFAULT, &value, sizeof(value), &indicator);
    CHECK(ret == SQL_SUCCESS);
    CHECK(indicator == sizeof(SQLBIGINT));
    CHECK(value == 42);
  }
}

// ============================================================================
// Interval conversions from FIXED numeric columns
// Per ODBC spec "SQL to C: Numeric" table for interval C types:
//   Single-field: Data not truncated -> success
//                 Fractional truncation -> 01S07
//                 Overflow -> 22015
//   Multi-field:  Always 22015
// ============================================================================

TEST_CASE("NUMBER to single-field interval types", "[datatype][number][interval]") {
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);

  // SQL_C_INTERVAL_YEAR - positive integer
  {
    auto stmt = conn.execute_fetch("SELECT 5::NUMBER(10,0)");
    auto interval = check_no_truncation<SQL_C_INTERVAL_YEAR>(stmt, 1);
    CHECK(interval.interval_type == SQL_IS_YEAR);
    CHECK(interval.interval_sign == SQL_FALSE);
    CHECK(interval.intval.year_month.year == 5);
  }

  // SQL_C_INTERVAL_YEAR - negative value
  {
    auto stmt = conn.execute_fetch("SELECT -3::NUMBER(10,0)");
    auto interval = check_no_truncation<SQL_C_INTERVAL_YEAR>(stmt, 1);
    CHECK(interval.interval_type == SQL_IS_YEAR);
    CHECK(interval.interval_sign == SQL_TRUE);
    CHECK(interval.intval.year_month.year == 3);
  }

  // SQL_C_INTERVAL_YEAR - zero
  {
    auto stmt = conn.execute_fetch("SELECT 0::NUMBER(10,0)");
    auto interval = check_no_truncation<SQL_C_INTERVAL_YEAR>(stmt, 1);
    CHECK(interval.interval_type == SQL_IS_YEAR);
    CHECK(interval.interval_sign == SQL_FALSE);
    CHECK(interval.intval.year_month.year == 0);
  }

  // SQL_C_INTERVAL_MONTH - positive integer
  {
    auto stmt = conn.execute_fetch("SELECT 10::NUMBER(10,0)");
    auto interval = check_no_truncation<SQL_C_INTERVAL_MONTH>(stmt, 1);
    CHECK(interval.interval_type == SQL_IS_MONTH);
    CHECK(interval.interval_sign == SQL_FALSE);
    CHECK(interval.intval.year_month.month == 10);
  }

  // SQL_C_INTERVAL_DAY - positive integer
  {
    auto stmt = conn.execute_fetch("SELECT 15::NUMBER(10,0)");
    auto interval = check_no_truncation<SQL_C_INTERVAL_DAY>(stmt, 1);
    CHECK(interval.interval_type == SQL_IS_DAY);
    CHECK(interval.interval_sign == SQL_FALSE);
    CHECK(interval.intval.day_second.day == 15);
  }

  // SQL_C_INTERVAL_HOUR - positive integer
  {
    auto stmt = conn.execute_fetch("SELECT 8::NUMBER(10,0)");
    auto interval = check_no_truncation<SQL_C_INTERVAL_HOUR>(stmt, 1);
    CHECK(interval.interval_type == SQL_IS_HOUR);
    CHECK(interval.interval_sign == SQL_FALSE);
    CHECK(interval.intval.day_second.hour == 8);
  }

  // SQL_C_INTERVAL_MINUTE - positive integer
  {
    auto stmt = conn.execute_fetch("SELECT 30::NUMBER(10,0)");
    auto interval = check_no_truncation<SQL_C_INTERVAL_MINUTE>(stmt, 1);
    CHECK(interval.interval_type == SQL_IS_MINUTE);
    CHECK(interval.interval_sign == SQL_FALSE);
    CHECK(interval.intval.day_second.minute == 30);
  }

  // SQL_C_INTERVAL_SECOND - integer, no fraction
  {
    auto stmt = conn.execute_fetch("SELECT 45::NUMBER(10,0)");
    auto interval = check_no_truncation<SQL_C_INTERVAL_SECOND>(stmt, 1);
    CHECK(interval.interval_type == SQL_IS_SECOND);
    CHECK(interval.interval_sign == SQL_FALSE);
    CHECK(interval.intval.day_second.second == 45);
    CHECK(interval.intval.day_second.fraction == 0);
  }

  // SQL_C_INTERVAL_SECOND - with fractional part
  {
    auto stmt = conn.execute_fetch("SELECT 45.500::DECIMAL(10,3)");
    auto interval = check_no_truncation<SQL_C_INTERVAL_SECOND>(stmt, 1);
    CHECK(interval.interval_type == SQL_IS_SECOND);
    CHECK(interval.intval.day_second.second == 45);
    CHECK(interval.intval.day_second.fraction == 500000);
  }

  // SQL_C_INTERVAL_SECOND - negative with fraction
  {
    auto stmt = conn.execute_fetch("SELECT -10.25::DECIMAL(10,2)");
    auto interval = check_no_truncation<SQL_C_INTERVAL_SECOND>(stmt, 1);
    CHECK(interval.interval_type == SQL_IS_SECOND);
    CHECK(interval.interval_sign == SQL_TRUE);
    CHECK(interval.intval.day_second.second == 10);
    CHECK(interval.intval.day_second.fraction == 250000);
  }
}

TEST_CASE("NUMBER to interval - fractional truncation returns 01S07", "[datatype][number][interval][01S07]") {
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);

  // SQL_C_INTERVAL_YEAR with fractional
  {
    auto stmt = conn.execute_fetch("SELECT 5.7::DECIMAL(10,1)");
    auto interval = check_fractional_truncation<SQL_C_INTERVAL_YEAR>(stmt, 1);
    CHECK(interval.interval_type == SQL_IS_YEAR);
    CHECK(interval.intval.year_month.year == 5);
  }

  // SQL_C_INTERVAL_MONTH with fractional
  {
    auto stmt = conn.execute_fetch("SELECT 10.3::DECIMAL(10,1)");
    auto interval = check_fractional_truncation<SQL_C_INTERVAL_MONTH>(stmt, 1);
    CHECK(interval.interval_type == SQL_IS_MONTH);
    CHECK(interval.intval.year_month.month == 10);
  }

  // SQL_C_INTERVAL_DAY with fractional
  {
    auto stmt = conn.execute_fetch("SELECT 15.9::DECIMAL(10,1)");
    auto interval = check_fractional_truncation<SQL_C_INTERVAL_DAY>(stmt, 1);
    CHECK(interval.interval_type == SQL_IS_DAY);
    CHECK(interval.intval.day_second.day == 15);
  }

  // SQL_C_INTERVAL_HOUR with fractional
  {
    auto stmt = conn.execute_fetch("SELECT 8.5::DECIMAL(10,1)");
    auto interval = check_fractional_truncation<SQL_C_INTERVAL_HOUR>(stmt, 1);
    CHECK(interval.interval_type == SQL_IS_HOUR);
    CHECK(interval.intval.day_second.hour == 8);
  }

  // SQL_C_INTERVAL_MINUTE with fractional
  {
    auto stmt = conn.execute_fetch("SELECT 30.1::DECIMAL(10,1)");
    auto interval = check_fractional_truncation<SQL_C_INTERVAL_MINUTE>(stmt, 1);
    CHECK(interval.interval_type == SQL_IS_MINUTE);
    CHECK(interval.intval.day_second.minute == 30);
  }
}

TEST_CASE("NUMBER to interval - sub-microsecond truncation returns 01S07", "[datatype][number][interval][01S07]") {
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);

  // SQL_C_INTERVAL_SECOND - scale 9, sub-microsecond digits truncated
  {
    auto stmt = conn.execute_fetch("SELECT 45.123456789::DECIMAL(12,9)");
    auto interval = check_fractional_truncation<SQL_C_INTERVAL_SECOND>(stmt, 1);
    CHECK(interval.interval_type == SQL_IS_SECOND);
    CHECK(interval.intval.day_second.second == 45);
    CHECK(interval.intval.day_second.fraction == 123456);
  }

  // SQL_C_INTERVAL_SECOND - scale 9, exact microseconds, no warning
  {
    auto stmt = conn.execute_fetch("SELECT 45.123456000::DECIMAL(12,9)");
    auto interval = check_no_truncation<SQL_C_INTERVAL_SECOND>(stmt, 1);
    CHECK(interval.interval_type == SQL_IS_SECOND);
    CHECK(interval.intval.day_second.second == 45);
    CHECK(interval.intval.day_second.fraction == 123456);
  }
}

TEST_CASE("NUMBER to interval - no negative zero", "[datatype][number][interval][edge]") {
  SKIP_OLD_DRIVER("BD#19", "Old driver produces negative zero for interval types");
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);

  // SQL_C_INTERVAL_YEAR - negative fraction truncated to zero
  {
    auto stmt = conn.execute_fetch("SELECT -0.5::DECIMAL(10,1)");
    auto interval = check_fractional_truncation<SQL_C_INTERVAL_YEAR>(stmt, 1);
    CHECK(interval.interval_type == SQL_IS_YEAR);
    CHECK(interval.interval_sign == SQL_FALSE);
    CHECK(interval.intval.year_month.year == 0);
  }

  // SQL_C_INTERVAL_MONTH - negative fraction truncated to zero
  {
    auto stmt = conn.execute_fetch("SELECT -0.3::DECIMAL(10,1)");
    auto interval = check_fractional_truncation<SQL_C_INTERVAL_MONTH>(stmt, 1);
    CHECK(interval.interval_type == SQL_IS_MONTH);
    CHECK(interval.interval_sign == SQL_FALSE);
    CHECK(interval.intval.year_month.month == 0);
  }

  // SQL_C_INTERVAL_DAY - negative fraction truncated to zero
  {
    auto stmt = conn.execute_fetch("SELECT -0.9::DECIMAL(10,1)");
    auto interval = check_fractional_truncation<SQL_C_INTERVAL_DAY>(stmt, 1);
    CHECK(interval.interval_type == SQL_IS_DAY);
    CHECK(interval.interval_sign == SQL_FALSE);
    CHECK(interval.intval.day_second.day == 0);
  }

  // SQL_C_INTERVAL_SECOND - negative fraction keeps sign when fraction nonzero
  {
    auto stmt = conn.execute_fetch("SELECT -0.5::DECIMAL(10,1)");
    auto interval = check_no_truncation<SQL_C_INTERVAL_SECOND>(stmt, 1);
    CHECK(interval.interval_type == SQL_IS_SECOND);
    CHECK(interval.interval_sign == SQL_TRUE);
    CHECK(interval.intval.day_second.second == 0);
    CHECK(interval.intval.day_second.fraction == 500000);
  }
}

TEST_CASE("NUMBER to multi-field interval returns 22015", "[datatype][number][interval][22015]") {
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);

  // SQL_C_INTERVAL_YEAR_TO_MONTH
  check_interval_precision_lost<SQL_C_INTERVAL_YEAR_TO_MONTH>(conn.execute_fetch("SELECT 42::NUMBER(10,0)"), 1);

  // SQL_C_INTERVAL_DAY_TO_HOUR
  check_interval_precision_lost<SQL_C_INTERVAL_DAY_TO_HOUR>(conn.execute_fetch("SELECT 42::NUMBER(10,0)"), 1);

  // SQL_C_INTERVAL_DAY_TO_MINUTE
  check_interval_precision_lost<SQL_C_INTERVAL_DAY_TO_MINUTE>(conn.execute_fetch("SELECT 42::NUMBER(10,0)"), 1);

  // SQL_C_INTERVAL_DAY_TO_SECOND
  check_interval_precision_lost<SQL_C_INTERVAL_DAY_TO_SECOND>(conn.execute_fetch("SELECT 42::NUMBER(10,0)"), 1);

  // SQL_C_INTERVAL_HOUR_TO_MINUTE
  check_interval_precision_lost<SQL_C_INTERVAL_HOUR_TO_MINUTE>(conn.execute_fetch("SELECT 42::NUMBER(10,0)"), 1);

  // SQL_C_INTERVAL_HOUR_TO_SECOND
  check_interval_precision_lost<SQL_C_INTERVAL_HOUR_TO_SECOND>(conn.execute_fetch("SELECT 42::NUMBER(10,0)"), 1);

  // SQL_C_INTERVAL_MINUTE_TO_SECOND
  check_interval_precision_lost<SQL_C_INTERVAL_MINUTE_TO_SECOND>(conn.execute_fetch("SELECT 42::NUMBER(10,0)"), 1);
}

TEST_CASE("NUMBER to interval - NULL returns SQL_NULL_DATA", "[datatype][number][interval][null]") {
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);

  // SQL_C_INTERVAL_YEAR
  check_null_via_get_data(conn.execute_fetch("SELECT NULL::NUMBER(10,0)"), 1, SQL_C_INTERVAL_YEAR);

  // SQL_C_INTERVAL_MONTH
  check_null_via_get_data(conn.execute_fetch("SELECT NULL::NUMBER(10,0)"), 1, SQL_C_INTERVAL_MONTH);

  // SQL_C_INTERVAL_DAY
  check_null_via_get_data(conn.execute_fetch("SELECT NULL::NUMBER(10,0)"), 1, SQL_C_INTERVAL_DAY);

  // SQL_C_INTERVAL_HOUR
  check_null_via_get_data(conn.execute_fetch("SELECT NULL::NUMBER(10,0)"), 1, SQL_C_INTERVAL_HOUR);

  // SQL_C_INTERVAL_MINUTE
  check_null_via_get_data(conn.execute_fetch("SELECT NULL::NUMBER(10,0)"), 1, SQL_C_INTERVAL_MINUTE);

  // SQL_C_INTERVAL_SECOND
  check_null_via_get_data(conn.execute_fetch("SELECT NULL::NUMBER(10,0)"), 1, SQL_C_INTERVAL_SECOND);
}

// ============================================================================
// Interval leading precision via SQL_DESC_DATETIME_INTERVAL_PRECISION
// Per ODBC spec the default leading precision is 2 digits, so values >= 100
// must fail with 22015. Applications can increase precision via SQLSetDescField.
// ============================================================================

TEST_CASE("NUMBER to interval - default precision rejects values >= 100", "[datatype][number][interval][precision]") {
  SKIP_OLD_DRIVER("BD#22", "Old driver does not enforce interval leading precision");
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);

  // value 99 succeeds with default precision 2
  {
    auto stmt = conn.execute_fetch("SELECT 99::NUMBER(10,0)");
    auto interval = check_no_truncation<SQL_C_INTERVAL_YEAR>(stmt, 1);
    CHECK(interval.interval_type == SQL_IS_YEAR);
    CHECK(interval.intval.year_month.year == 99);
  }

  // value 100 fails with default precision 2
  {
    auto stmt = conn.execute_fetch("SELECT 100::NUMBER(10,0)");
    check_interval_precision_lost<SQL_C_INTERVAL_YEAR>(stmt, 1);
  }

  // value -100 fails with default precision 2
  {
    auto stmt = conn.execute_fetch("SELECT -100::NUMBER(10,0)");
    check_interval_precision_lost<SQL_C_INTERVAL_DAY>(stmt, 1);
  }

  // SQL_C_INTERVAL_SECOND with value 100 fails
  {
    auto stmt = conn.execute_fetch("SELECT 100::NUMBER(10,0)");
    check_interval_precision_lost<SQL_C_INTERVAL_SECOND>(stmt, 1);
  }
}

TEST_CASE("NUMBER to interval - custom precision via SQLSetDescField",
          "[datatype][number][interval][precision][descriptor]") {
  SKIP_OLD_DRIVER("BD#22", "Old driver does not support SQL_DESC_DATETIME_INTERVAL_PRECISION");
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);

  // precision 5 allows values up to 99999
  {
    auto stmt = conn.execute_fetch("SELECT 99999::NUMBER(10,0)");

    SQLHDESC ard = SQL_NULL_HDESC;
    SQLRETURN ret = SQLGetStmtAttr(stmt.getHandle(), SQL_ATTR_APP_ROW_DESC, &ard, 0, NULL);
    REQUIRE(ret == SQL_SUCCESS);
    ret = SQLSetDescField(ard, 1, SQL_DESC_DATETIME_INTERVAL_PRECISION, (SQLPOINTER)5, 0);
    REQUIRE(ret == SQL_SUCCESS);

    auto interval = check_no_truncation<SQL_C_INTERVAL_YEAR>(stmt, 1);
    CHECK(interval.interval_type == SQL_IS_YEAR);
    CHECK(interval.intval.year_month.year == 99999);
  }

  // precision 5 rejects value 100000
  {
    auto stmt = conn.execute_fetch("SELECT 100000::NUMBER(10,0)");

    SQLHDESC ard = SQL_NULL_HDESC;
    SQLRETURN ret = SQLGetStmtAttr(stmt.getHandle(), SQL_ATTR_APP_ROW_DESC, &ard, 0, NULL);
    REQUIRE(ret == SQL_SUCCESS);
    ret = SQLSetDescField(ard, 1, SQL_DESC_DATETIME_INTERVAL_PRECISION, (SQLPOINTER)5, 0);
    REQUIRE(ret == SQL_SUCCESS);

    check_interval_precision_lost<SQL_C_INTERVAL_YEAR>(stmt, 1);
  }

  // precision 1 allows single digit
  {
    auto stmt = conn.execute_fetch("SELECT 9::NUMBER(10,0)");

    SQLHDESC ard = SQL_NULL_HDESC;
    SQLRETURN ret = SQLGetStmtAttr(stmt.getHandle(), SQL_ATTR_APP_ROW_DESC, &ard, 0, NULL);
    REQUIRE(ret == SQL_SUCCESS);
    ret = SQLSetDescField(ard, 1, SQL_DESC_DATETIME_INTERVAL_PRECISION, (SQLPOINTER)1, 0);
    REQUIRE(ret == SQL_SUCCESS);

    auto interval = check_no_truncation<SQL_C_INTERVAL_HOUR>(stmt, 1);
    CHECK(interval.interval_type == SQL_IS_HOUR);
    CHECK(interval.intval.day_second.hour == 9);
  }

  // precision 1 rejects two digits
  {
    auto stmt = conn.execute_fetch("SELECT 10::NUMBER(10,0)");

    SQLHDESC ard = SQL_NULL_HDESC;
    SQLRETURN ret = SQLGetStmtAttr(stmt.getHandle(), SQL_ATTR_APP_ROW_DESC, &ard, 0, NULL);
    REQUIRE(ret == SQL_SUCCESS);
    ret = SQLSetDescField(ard, 1, SQL_DESC_DATETIME_INTERVAL_PRECISION, (SQLPOINTER)1, 0);
    REQUIRE(ret == SQL_SUCCESS);

    check_interval_precision_lost<SQL_C_INTERVAL_HOUR>(stmt, 1);
  }

  // precision 9 allows large values for SQL_C_INTERVAL_SECOND
  {
    auto stmt = conn.execute_fetch("SELECT 999999999::NUMBER(10,0)");

    SQLHDESC ard = SQL_NULL_HDESC;
    SQLRETURN ret = SQLGetStmtAttr(stmt.getHandle(), SQL_ATTR_APP_ROW_DESC, &ard, 0, NULL);
    REQUIRE(ret == SQL_SUCCESS);
    ret = SQLSetDescField(ard, 1, SQL_DESC_DATETIME_INTERVAL_PRECISION, (SQLPOINTER)9, 0);
    REQUIRE(ret == SQL_SUCCESS);

    auto interval = check_no_truncation<SQL_C_INTERVAL_SECOND>(stmt, 1);
    CHECK(interval.interval_type == SQL_IS_SECOND);
    CHECK(interval.intval.day_second.second == 999999999);
    CHECK(interval.intval.day_second.fraction == 0);
  }
}

TEST_CASE("Without TREAT_DECIMAL_AS_INT default is SQL_C_CHAR for scale=0",
          "[datatype][number][treat_decimal_as_int]") {
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);

  auto stmt = conn.execute_fetch("SELECT 42::DECIMAL(10,0)");

  char buffer[100];
  SQLLEN indicator = -999;
  SQLRETURN ret = SQLGetData(stmt.getHandle(), 1, SQL_C_DEFAULT, buffer, sizeof(buffer), &indicator);
  CHECK(ret == SQL_SUCCESS);
  CHECK(indicator > 0);
  CHECK(std::string(buffer, indicator) == "42");
}

TEST_CASE("TREAT_DECIMAL_AS_INT with table columns", "[datatype][number][treat_decimal_as_int]") {
  Connection conn;
  conn.execute("ALTER SESSION SET ODBC_TREAT_DECIMAL_AS_INT=true");
  auto random_schema = Schema::use_random_schema(conn);

  std::string table_name = "test_decimal_as_int";
  TestTable table(conn, table_name, "d_int DECIMAL(10,0), d_frac DECIMAL(10,2), d_big NUMBER(38,0)",
                  "(42, 123.45, 42)");

  auto stmt = conn.execute_fetch("SELECT * FROM " + table.name());

  SECTION("DECIMAL(10,0) column returns SBIGINT via SQL_C_DEFAULT") {
    SQLBIGINT value = 0;
    SQLLEN indicator = -999;
    SQLRETURN ret = SQLGetData(stmt.getHandle(), 1, SQL_C_DEFAULT, &value, sizeof(value), &indicator);
    CHECK(ret == SQL_SUCCESS);
    CHECK(indicator == sizeof(SQLBIGINT));
    CHECK(value == 42);
  }

  SECTION("DECIMAL(10,2) column returns CHAR via SQL_C_DEFAULT") {
    char buffer[100];
    SQLLEN indicator = -999;
    SQLRETURN ret = SQLGetData(stmt.getHandle(), 2, SQL_C_DEFAULT, buffer, sizeof(buffer), &indicator);
    CHECK(ret == SQL_SUCCESS);
    CHECK(indicator > 0);
    CHECK(std::string(buffer, indicator) == "123.45");
  }

  SECTION(
      "NUMBER(38,0) column returns SBIGINT via SQL_C_DEFAULT (BigInt, precision > 18 but no big_number_as_string)") {
    SQLBIGINT value = 0;
    SQLLEN indicator = -999;
    SQLRETURN ret = SQLGetData(stmt.getHandle(), 3, SQL_C_DEFAULT, &value, sizeof(value), &indicator);
    CHECK(ret == SQL_SUCCESS);
    CHECK(indicator == sizeof(SQLBIGINT));
    CHECK(value == 42);
  }
}
