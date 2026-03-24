#include <sql.h>
#include <sqlext.h>
#include <sqltypes.h>

#include <string>
#include <vector>

#include <catch2/catch_test_macros.hpp>

#include "Connection.hpp"
#include "Schema.hpp"
#include "TestTable.hpp"
#include "compatibility.hpp"
#include "conversion_checks.hpp"
#include "get_data.hpp"
#include "get_diag_rec.hpp"
#include "odbc_matchers.hpp"

TEST_CASE("Test decimal to SQL_C_CHAR and SQL_C_DEFAULT conversion", "[fixed][conversion][c_char]") {
  // Given A Snowflake connection is established
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);

  // When A table with various DECIMAL/NUMBER/INT columns is queried
  TestTable table(conn, "test_number",
                  "num0 NUMBER, num10 NUMBER(10,1), dec20 DECIMAL(20,2), "
                  "numeric30 NUMERIC(30,3), int1 INT, int2 INTEGER",
                  "(123, 123.4, 123.45, 123.456, 123, 123)");

  auto stmt = conn.execute_fetch("SELECT * FROM " + table.name());

  std::vector<std::string> expected_str = {"123", "123.4", "123.45", "123.456", "123", "123"};

  // Then SQL_C_CHAR and SQL_C_DEFAULT return correct string representations
  for (int i = 1; i <= 6; ++i) {
    INFO("Testing column " << i << " with SQL_C_CHAR");
    CHECK(check_char_success(stmt, i) == expected_str[i - 1]);
  }

  for (int i = 1; i <= 6; ++i) {
    INFO("Testing column " << i << " with SQL_C_DEFAULT");
    CHECK(get_data_default_as_string(stmt, i) == expected_str[i - 1]);
  }
}

TEST_CASE("SQL_DECIMAL default conversion", "[fixed][conversion][c_char][default]") {
  // Given A Snowflake connection is established
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);

  {
    INFO("basic values from table");
    // When A table with DECIMAL columns is queried
    TestTable table(conn, "test_decimal_default",
                    "d1 DECIMAL(10,0), d2 DECIMAL(10,1), d3 DECIMAL(10,2), d4 DECIMAL(10,3)",
                    "(123, 123.4, 123.45, 123.456)");

    auto stmt = conn.execute_fetch("SELECT * FROM " + table.name());

    // Then SQL_C_DEFAULT returns matching string values
    std::vector<std::string> expected = {"123", "123.4", "123.45", "123.456"};
    for (int i = 1; i <= 4; ++i) {
      INFO("Column " << i);
      CHECK(get_data_default_as_string(stmt, i) == expected[i - 1]);
    }
  }

  {
    INFO("negative values");
    // When Negative DECIMAL values are queried
    auto stmt = conn.execute_fetch(
        "SELECT -123::DECIMAL(10,0), -123.4::DECIMAL(10,1), "
        "-123.45::DECIMAL(10,2), -123.456::DECIMAL(10,3)");

    // Then SQL_C_DEFAULT returns matching negative string values
    std::vector<std::string> expected = {"-123", "-123.4", "-123.45", "-123.456"};
    for (int i = 1; i <= 4; ++i) {
      INFO("Column " << i);
      CHECK(get_data_default_as_string(stmt, i) == expected[i - 1]);
    }
  }

  {
    INFO("zero with varying scale");
    // When Zero values with different scales are queried
    auto stmt = conn.execute_fetch("SELECT 0::DECIMAL(10,0), 0::DECIMAL(10,2), 0::DECIMAL(10,5)");

    // Then SQL_C_DEFAULT returns zero with correct scale
    CHECK(get_data_default_as_string(stmt, 1) == "0");
    CHECK(get_data_default_as_string(stmt, 2) == "0.00");
    CHECK(get_data_default_as_string(stmt, 3) == "0.00000");
  }

  {
    INFO("small values with large scale");
    // When Small fractional DECIMAL values are queried
    auto stmt = conn.execute_fetch(
        "SELECT 0.05::DECIMAL(10,2), 0.001::DECIMAL(10,3), "
        "0.00001::DECIMAL(10,5)");

    // Then SQL_C_DEFAULT returns matching small fractional values
    CHECK(get_data_default_as_string(stmt, 1) == "0.05");
    CHECK(get_data_default_as_string(stmt, 2) == "0.001");
    CHECK(get_data_default_as_string(stmt, 3) == "0.00001");
  }

  {
    INFO("negative small fractional values");
    // When Negative small fractional values are queried
    auto stmt = conn.execute_fetch("SELECT -0.05::DECIMAL(10,2), -0.001::DECIMAL(10,3), -0.5::DECIMAL(10,1)");

    // Then Both SQL_C_CHAR and SQL_C_DEFAULT return matching values
    CHECK(check_char_success(stmt, 1) == "-0.05");
    CHECK(check_char_success(stmt, 2) == "-0.001");
    CHECK(check_char_success(stmt, 3) == "-0.5");
    CHECK(get_data_default_as_string(stmt, 1) == "-0.05");
    CHECK(get_data_default_as_string(stmt, 2) == "-0.001");
    CHECK(get_data_default_as_string(stmt, 3) == "-0.5");
  }

  {
    INFO("various SQL numeric type synonyms");
    // When NUMBER, DECIMAL, and NUMERIC synonyms are queried
    auto stmt = conn.execute_fetch("SELECT 42::NUMBER(10,2), 42::DECIMAL(10,2), 42::NUMERIC(10,2)");
    std::string expected = "42.00";

    // Then All synonyms return the same string
    for (int i = 1; i <= 3; ++i) {
      INFO("Column " << i << " with SQL_C_CHAR");
      CHECK(check_char_success(stmt, i) == expected);
    }
    for (int i = 1; i <= 3; ++i) {
      INFO("Column " << i << " with SQL_C_DEFAULT");
      CHECK(get_data_default_as_string(stmt, i) == expected);
    }
  }

  {
    INFO("INT column resolves to SQL_C_CHAR");
    // When INT/INTEGER/BIGINT columns are queried with SQL_C_DEFAULT
    auto stmt = conn.execute_fetch("SELECT 42::INT, -7::INTEGER, 0::BIGINT");

    // Then SQL_C_DEFAULT returns matching integer strings
    CHECK(get_data_default_as_string(stmt, 1) == "42");
    CHECK(get_data_default_as_string(stmt, 2) == "-7");
    CHECK(get_data_default_as_string(stmt, 3) == "0");
  }

  {
    INFO("matches explicit SQL_C_CHAR via table");
    // When A table with DECIMAL columns is queried twice
    TestTable table(conn, "test_decimal_default_vs_char", "d1 DECIMAL(10,1), d2 DECIMAL(20,2), d3 DECIMAL(30,3)",
                    "(123.4, 123.45, 123.456)");

    auto stmt_char = conn.execute_fetch("SELECT * FROM " + table.name());
    auto stmt_default = conn.execute_fetch("SELECT * FROM " + table.name());

    // Then SQL_C_CHAR and SQL_C_DEFAULT return identical values
    for (int i = 1; i <= 3; ++i) {
      INFO("Column " << i);
      CHECK(check_char_success(stmt_char, i) == get_data_default_as_string(stmt_default, i));
    }
  }
}

TEST_CASE("SQL_DECIMAL default conversion - large precision", "[fixed][conversion][c_char][default]") {
  // Given A Snowflake connection is established
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);

  {
    INFO("large values");
    // When A table with NUMBER(38) columns is queried
    TestTable table(conn, "test_decimal_large", "a NUMBER(38,0), b NUMBER(38,37)",
                    "(10000000000000000000000000000000000000, "
                    " 1.0000000000000000000000000000000000000)");

    auto stmt = conn.execute_fetch("SELECT * FROM " + table.name());

    // Then SQL_C_DEFAULT preserves full 38-digit precision
    CHECK(get_data_default_as_string(stmt, 1) == "10000000000000000000000000000000000000");
    CHECK(get_data_default_as_string(stmt, 2) == "1.0000000000000000000000000000000000000");
  }

  {
    INFO("max positive and negative values");
    // When A table with max/min NUMBER(38) values is queried
    TestTable table(conn, "test_decimal_max", "a NUMBER(38,0), b NUMBER(38,37)",
                    "(99999999999999999999999999999999999999, "
                    " 9.9999999999999999999999999999999999999), "
                    "(-99999999999999999999999999999999999999, "
                    " -9.9999999999999999999999999999999999999)");

    auto stmt = conn.execute_fetch("SELECT * FROM " + table.name());

    // Then SQL_C_DEFAULT returns correct boundary strings for both rows
    CHECK(get_data_default_as_string(stmt, 1) == "99999999999999999999999999999999999999");
    CHECK(get_data_default_as_string(stmt, 2) == "9.9999999999999999999999999999999999999");

    SQLRETURN ret = SQLFetch(stmt.getHandle());
    CHECK(ret == SQL_SUCCESS);
    CHECK(get_data_default_as_string(stmt, 1) == "-99999999999999999999999999999999999999");
    CHECK(get_data_default_as_string(stmt, 2) == "-9.9999999999999999999999999999999999999");
  }
}

TEST_CASE("SQL_DECIMAL to SQL_C_WCHAR", "[fixed][conversion][c_wchar]") {
  // Given A Snowflake connection is established
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);

  {
    INFO("basic values");
    // When Various NUMBER values are queried as SQL_C_WCHAR
    auto stmt = conn.execute_fetch(
        "SELECT 42::NUMBER(10,0), -7::NUMBER(10,0), 0::NUMBER(10,0), "
        "123.45::NUMBER(10,2), -0.05::NUMBER(10,2)");

    // Then SQL_C_WCHAR returns matching wide strings
    CHECK(check_wchar_success(stmt, 1) == u"42");
    CHECK(check_wchar_success(stmt, 2) == u"-7");
    CHECK(check_wchar_success(stmt, 3) == u"0");
    CHECK(check_wchar_success(stmt, 4) == u"123.45");
    CHECK(check_wchar_success(stmt, 5) == u"-0.05");
  }

  {
    INFO("large precision values");
    // When A 38-digit NUMBER is queried as SQL_C_WCHAR
    auto stmt = conn.execute_fetch("SELECT 99999999999999999999999999999999999999::NUMBER(38,0)");

    // Then SQL_C_WCHAR preserves full precision
    CHECK(check_wchar_success(stmt, 1) == u"99999999999999999999999999999999999999");
  }

  {
    INFO("matches SQL_C_CHAR");
    // When The same value is queried as both SQL_C_CHAR and SQL_C_WCHAR
    const std::string query = "SELECT 123.456::DECIMAL(10,3)";
    auto char_str = check_char_success(conn.execute_fetch(query), 1);
    auto wchar_str = check_wchar_success(conn.execute_fetch(query), 1);

    // Then SQL_C_WCHAR matches SQL_C_CHAR
    std::u16string expected_wchar(char_str.begin(), char_str.end());
    CHECK(wchar_str == expected_wchar);
  }
}

TEST_CASE("SQL_DECIMAL SQL_C_CHAR buffer handling", "[fixed][conversion][c_char][buffer]") {
  // Given A Snowflake connection is established
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);

  // When NUMBER values are fetched into various buffer sizes
  (void)0;  // SECTIONs below perform the fetch
  // Then The driver returns appropriate SQLSTATE codes for buffer overflow, truncation, and exact fit
  SECTION("whole digits do not fit returns 22003") {
    SKIP_OLD_DRIVER("BD#11",
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
    SKIP_OLD_DRIVER("BD#11",
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
    auto stmt = conn.execute_fetch("SELECT -12.345::DECIMAL(10,3)");

    char small_buffer[5];
    SQLLEN indicator = 0;
    SQLRETURN ret = SQLGetData(stmt.getHandle(), 1, SQL_C_CHAR, small_buffer, sizeof(small_buffer), &indicator);

    CHECK(ret == SQL_SUCCESS_WITH_INFO);
    CHECK(get_sqlstate(stmt) == "01004");
  }
}

TEST_CASE("Without TREAT_DECIMAL_AS_INT default is SQL_C_CHAR for scale=0", "[fixed][conversion][c_char][default]") {
  // Given A Snowflake connection is established
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);

  // When An integer DECIMAL value is queried with SQL_C_DEFAULT
  auto stmt = conn.execute_fetch("SELECT 42::DECIMAL(10,0)");

  char buffer[100];
  SQLLEN indicator = -999;
  SQLRETURN ret = SQLGetData(stmt.getHandle(), 1, SQL_C_DEFAULT, buffer, sizeof(buffer), &indicator);

  // Then SQL_C_DEFAULT resolves to SQL_C_CHAR and returns string "42"
  CHECK(ret == SQL_SUCCESS);
  CHECK(indicator > 0);
  CHECK(std::string(buffer, indicator) == "42");
}

TEST_CASE("Test string at limits", "[fixed][conversion][c_char][limits]") {
  // Given A Snowflake connection is established
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);

  // When Max and min 37-digit NUMBER values are fetched as SQL_C_CHAR and SQL_C_DEFAULT
  std::string max_val = std::string(37, '9');
  std::string min_val = "-" + std::string(37, '9');
  auto stmt = conn.execute_fetch("SELECT " + max_val + " AS max, " + min_val + " AS min");

  // Then Both SQL_C_CHAR and SQL_C_DEFAULT return correct extreme values
  CHECK(check_char_success(stmt, 1) == max_val);
  CHECK(check_char_success(stmt, 2) == min_val);
  CHECK(get_data_default_as_string(stmt, 1) == max_val);
  CHECK(get_data_default_as_string(stmt, 2) == min_val);
}

TEST_CASE("DECIMAL multiple rows as SQL_C_CHAR", "[fixed][conversion][c_char][multirow]") {
  // Given A Snowflake connection is established
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);

  // When A table with various DECIMAL(10,2) values is queried
  TestTable table(conn, "test_number_multi", "val DECIMAL(10,2)",
                  "(0.00), (1.00), (-1.00), (999.99), (-999.99), (0.01), (-0.01)");

  auto stmt = conn.execute_fetch("SELECT * FROM " + table.name());
  std::vector<std::string> expected = {"0.00", "1.00", "-1.00", "999.99", "-999.99", "0.01", "-0.01"};

  // Then Each row returns the correct string value
  for (size_t row = 0; row < expected.size(); ++row) {
    if (row > 0) {
      SQLRETURN ret = SQLFetch(stmt.getHandle());
      CHECK(ret == SQL_SUCCESS);
    }
    INFO("Row " << row << " expected: " << expected[row]);
    CHECK(check_char_success(stmt, 1) == expected[row]);
  }
}

TEST_CASE("NUMBER NULL to SQL_C_CHAR types", "[fixed][conversion][c_char][null]") {
  // Given A Snowflake connection is established
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);

  // When A NULL NUMBER value is queried
  const auto query = "SELECT NULL::NUMBER(10,0)";
  // Then Indicator returns SQL_NULL_DATA
  check_null_via_get_data(conn.execute_fetch("SELECT NULL::DECIMAL(20,5)"), 1, SQL_C_CHAR);
  check_null_via_get_data(conn.execute_fetch(query), 1, SQL_C_WCHAR);
  check_null_via_get_data(conn.execute_fetch("SELECT NULL::DECIMAL(10,2)"), 1, SQL_C_DEFAULT);
}
