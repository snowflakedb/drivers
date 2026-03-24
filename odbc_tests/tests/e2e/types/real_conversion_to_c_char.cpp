#include <sql.h>
#include <sqlext.h>
#include <sqltypes.h>

#include <string>

#include <catch2/catch_test_macros.hpp>
#include <catch2/matchers/catch_matchers_floating_point.hpp>

#include "Connection.hpp"
#include "Schema.hpp"
#include "compatibility.hpp"
#include "conversion_checks.hpp"
#include "get_diag_rec.hpp"

TEST_CASE("REAL explicit SQL_C_CHAR", "[e2e][types][real]") {
  // Given A Snowflake connection
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);

  // When FLOAT values are fetched as SQL_C_CHAR
  auto stmt = conn.execute_fetch("SELECT 123.456::FLOAT, -99.5::FLOAT, 0::FLOAT");

  std::string s1 = check_char_success(stmt, 1);
  std::string s2 = check_char_success(stmt, 2);
  std::string s3 = check_char_success(stmt, 3);

  // Then The string representations match the expected numeric values
  CHECK_THAT(std::stod(s1), Catch::Matchers::WithinRel(123.456));
  CHECK_THAT(std::stod(s2), Catch::Matchers::WithinRel(-99.5));
  CHECK_THAT(std::stod(s3), Catch::Matchers::WithinAbs(0.0, 1e-15));
}

TEST_CASE("REAL to SQL_C_WCHAR", "[e2e][types][real]") {
  // Given A Snowflake connection
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);

  // When FLOAT values are fetched as SQL_C_WCHAR
  {
    auto stmt = conn.execute_fetch("SELECT 123.456::FLOAT, -99.5::FLOAT, 0.0::FLOAT");
    auto s1 = check_wchar_success(stmt, 1);
    auto s2 = check_wchar_success(stmt, 2);
    auto s3 = check_wchar_success(stmt, 3);

    // Then The wide string representations are non-empty
    CHECK(!s1.empty());
    CHECK(!s2.empty());
    CHECK(!s3.empty());
  }

  // And SQL_C_WCHAR matches SQL_C_CHAR for the same value
  {
    const std::string query = "SELECT 3.125::FLOAT";
    auto char_str = check_char_success(conn.execute_fetch(query), 1);
    auto wchar_str = check_wchar_success(conn.execute_fetch(query), 1);
    std::u16string expected_wchar(char_str.begin(), char_str.end());
    CHECK(wchar_str == expected_wchar);
  }
}

TEST_CASE("REAL SQL_C_CHAR buffer handling", "[e2e][types][real]") {
  // Given A Snowflake connection
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);

  // When FLOAT values are fetched into various buffer sizes as SQL_C_CHAR
  (void)0;  // SECTIONs below perform the fetch
  // Then The driver returns appropriate SQLSTATE codes for buffer overflow, truncation, and exact fit
  SECTION("large buffer succeeds") {
    auto stmt = conn.execute_fetch("SELECT 42.5::FLOAT");

    char buffer[100];
    SQLLEN indicator = 0;
    SQLRETURN ret = SQLGetData(stmt.getHandle(), 1, SQL_C_CHAR, buffer, sizeof(buffer), &indicator);
    CHECK(ret == SQL_SUCCESS);
    CHECK(indicator > 0);
    CHECK_THAT(std::stod(std::string(buffer, indicator)), Catch::Matchers::WithinRel(42.5));
  }

  SECTION("fractional-only truncation returns 01004") {
    SKIP_OLD_DRIVER(
        "BD#15",
        "Old driver returns SQL_ERROR instead of SQL_SUCCESS_WITH_INFO for small SQL_C_CHAR buffer on FLOAT columns");

    auto stmt = conn.execute_fetch("SELECT 3.14159::FLOAT");

    char small_buffer[4];
    SQLLEN indicator = 0;
    SQLRETURN ret = SQLGetData(stmt.getHandle(), 1, SQL_C_CHAR, small_buffer, sizeof(small_buffer), &indicator);

    CHECK(ret == SQL_SUCCESS_WITH_INFO);
    CHECK(get_sqlstate(stmt) == "01004");
  }

  SECTION("whole digits lost returns 22003") {
    auto stmt = conn.execute_fetch("SELECT 123456.789::FLOAT");

    char small_buffer[4];
    SQLLEN indicator = 0;
    SQLRETURN ret = SQLGetData(stmt.getHandle(), 1, SQL_C_CHAR, small_buffer, sizeof(small_buffer), &indicator);

    CHECK(ret == SQL_ERROR);
    CHECK(get_sqlstate(stmt) == "22003");
  }
}

TEST_CASE("REAL SQL_C_WCHAR buffer handling", "[e2e][types][real]") {
  // Given A Snowflake connection
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);

  // When FLOAT values are fetched into various buffer sizes as SQL_C_WCHAR
  (void)0;  // SECTIONs below perform the fetch
  // Then The driver returns appropriate SQLSTATE codes for buffer overflow, truncation, and exact fit
  SECTION("large buffer succeeds") {
    auto stmt = conn.execute_fetch("SELECT 42.5::FLOAT");
    auto result = check_wchar_success(stmt, 1);
    CHECK(!result.empty());
  }

  SECTION("fractional-only truncation returns 01004") {
    SKIP_OLD_DRIVER(
        "BD#15",
        "Old driver returns SQL_ERROR instead of SQL_SUCCESS_WITH_INFO for small SQL_C_WCHAR buffer on FLOAT columns");

    auto stmt = conn.execute_fetch("SELECT 3.14159::FLOAT");

    char16_t small_buffer[4];
    SQLLEN indicator = 0;
    SQLRETURN ret = SQLGetData(stmt.getHandle(), 1, SQL_C_WCHAR, small_buffer, sizeof(small_buffer), &indicator);

    CHECK(ret == SQL_SUCCESS_WITH_INFO);
    CHECK(get_sqlstate(stmt) == "01004");
  }

  SECTION("whole digits lost returns 22003") {
    // TODO: Enable when wide functions are implemented
    WINDOWS_ONLY { SKIP("Windows manager binds to longer SQL_C_CHAR buffer and performs the conversion"); }
    auto stmt = conn.execute_fetch("SELECT 123456.789::FLOAT");

    char16_t small_buffer[4];
    SQLLEN indicator = 0;
    SQLRETURN ret = SQLGetData(stmt.getHandle(), 1, SQL_C_WCHAR, small_buffer, sizeof(small_buffer), &indicator);

    CHECK(ret == SQL_ERROR);
    CHECK(get_sqlstate(stmt) == "22003");
  }
}

TEST_CASE("REAL explicit SQL_C_CHAR for special values", "[e2e][types][real]") {
  // Given A Snowflake connection
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);

  // When Special FLOAT values (integer-valued, negative, very small, very large) are fetched as SQL_C_CHAR
  {
    std::string s = check_char_success(conn.execute_fetch("SELECT 42.0::FLOAT"), 1);
    // Then The string representations correctly represent each value
    CHECK_THAT(std::stod(s), Catch::Matchers::WithinAbs(42.0, 1e-15));
  }

  {
    std::string s = check_char_success(conn.execute_fetch("SELECT -0.001::FLOAT"), 1);
    CHECK_THAT(std::stod(s), Catch::Matchers::WithinRel(-0.001));
  }

  {
    std::string s = check_char_success(conn.execute_fetch("SELECT 1e-300::FLOAT"), 1);
    double parsed = std::stod(s);
    CHECK(parsed > 0.0);
    CHECK(parsed < 1e-299);
  }

  {
    std::string s = check_char_success(conn.execute_fetch("SELECT 1e300::FLOAT"), 1);
    double parsed = std::stod(s);
    CHECK(parsed > 9e299);
  }
}

TEST_CASE("REAL NaN to CHAR produces NaN string", "[e2e][types][real]") {
  // Given A Snowflake connection
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);

  // When NaN is fetched as SQL_C_CHAR
  auto stmt = conn.execute_fetch("SELECT 'NaN'::FLOAT");
  char buf[64] = {};
  SQLLEN indicator = -999;
  SQLRETURN ret = SQLGetData(stmt.getHandle(), 1, SQL_C_CHAR, buf, sizeof(buf), &indicator);

  // Then The result is the string "NaN"
  CHECK(ret == SQL_SUCCESS);
  std::string result(buf);
  CHECK(result == "NaN");
}

TEST_CASE("REAL NULL to SQL_C_CHAR", "[real][conversion][c_char][null]") {
  // Given A Snowflake connection
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);

  // When A NULL FLOAT value is queried
  auto stmt = conn.execute_fetch("SELECT NULL::FLOAT");
  // Then NULL FLOAT values return SQL_NULL_DATA
  check_null_via_get_data(stmt, 1, SQL_C_CHAR);
}
