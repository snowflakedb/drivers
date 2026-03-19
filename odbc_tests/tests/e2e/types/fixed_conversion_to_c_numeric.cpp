#include <sql.h>
#include <sqlext.h>
#include <sqltypes.h>

#include <catch2/catch_test_macros.hpp>

#include "Connection.hpp"
#include "Schema.hpp"
#include "compatibility.hpp"
#include "conversion_checks.hpp"
#include "get_data.hpp"
#include "get_diag_rec.hpp"
#include "macros.hpp"

TEST_CASE("SQL_DECIMAL to SQL_C_NUMERIC", "[fixed][conversion][c_numeric]") {
  // Given A Snowflake connection is established
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);

  // When NUMBER/DECIMAL values are fetched as SQL_C_NUMERIC
  (void)0;
  // Then SQL_NUMERIC_STRUCT fields match expected sign, precision, scale, and val
  {
    INFO("positive integer");
    auto numeric = check_no_truncation<SQL_C_NUMERIC>(conn.execute_fetch("SELECT 42::NUMBER(10,0)"), 1);
    CHECK(numeric.sign == 1);
    CHECK(numeric.val[0] == 42);
    check_numeric_val_zero_from(numeric, 1);
  }

  {
    INFO("negative value");
    auto numeric = check_no_truncation<SQL_C_NUMERIC>(conn.execute_fetch("SELECT -123::NUMBER(10,0)"), 1);
    CHECK(numeric.sign == 0);
    CHECK(numeric.val[0] == 123);
    check_numeric_val_zero_from(numeric, 1);
  }

  {
    INFO("zero");
    auto numeric = check_no_truncation<SQL_C_NUMERIC>(conn.execute_fetch("SELECT 0::NUMBER(10,0)"), 1);
    CHECK(numeric.sign == 1);
    check_numeric_val_zero_from(numeric, 0);
  }

  {
    INFO("with scale defaults to scale=0 truncation");
    auto numeric = check_fractional_truncation<SQL_C_NUMERIC>(conn.execute_fetch("SELECT 123.45::NUMBER(10,2)"), 1);
    CHECK(numeric_val_to_ull(numeric) == 123);
  }
}

TEST_CASE("SQL_DECIMAL to SQL_C_NUMERIC with SQL_DESC_PRECISION and SQL_DESC_SCALE",
          "[fixed][conversion][c_numeric][descriptor]") {
  SKIP_OLD_DRIVER("BD#13", "Old driver ignores SQL_DESC_PRECISION and SQL_DESC_SCALE set via SQLSetDescField");
  // Given A Snowflake connection is established
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);

  {
    INFO("target scale matches source scale - no truncation");
    // When SQL_DESC_PRECISION and SQL_DESC_SCALE are set via SQLSetDescField
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
    REQUIRE(ret == SQL_SUCCESS);
    // Then SQL_NUMERIC_STRUCT respects the custom precision and scale settings
    CHECK(numeric.precision == 10);
    CHECK(numeric.scale == 2);
    CHECK(numeric.sign == 1);
    CHECK(numeric_val_to_ull(numeric) == 12345);
  }

  {
    INFO("target scale=0 truncates fractional part - 01S07");
    // When SQL_DESC_SCALE is set to 0
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
    // Then SQL_SUCCESS_WITH_INFO with 01S07 and truncated value
    CHECK(ret == SQL_SUCCESS_WITH_INFO);
    CHECK(get_sqlstate(stmt) == "01S07");
    CHECK(numeric_val_to_ull(numeric) == 123);
  }

  {
    INFO("target scale > source scale upscales value");
    // When SQL_DESC_SCALE is greater than source scale
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
    REQUIRE(ret == SQL_SUCCESS);
    // Then Value is upscaled
    CHECK(numeric_val_to_ull(numeric) == 42000);
  }

  {
    INFO("target scale < source scale with exact division - no truncation");
    // When Target scale divides source scale exactly
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
    REQUIRE(ret == SQL_SUCCESS);
    CHECK(numeric_val_to_ull(numeric) == 123);
  }

  {
    INFO("target scale < source scale with remainder - 01S07");
    // When Target scale causes fractional truncation
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
    CHECK(numeric_val_to_ull(numeric) == 19);
  }

  {
    INFO("custom precision is reflected in output struct");
    // When SQL_DESC_PRECISION is set to a custom value
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
    REQUIRE(ret == SQL_SUCCESS);
    CHECK(numeric.precision == 5);
  }

  {
    INFO("negative value with upscale");
    // When Negative value is upscaled
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
    REQUIRE(ret == SQL_SUCCESS);
    CHECK(numeric.sign == 0);
    CHECK(numeric_val_to_ull(numeric) == 700);
  }

  {
    INFO("zero with non-zero target scale");
    // When Zero is fetched with non-zero target scale
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
    REQUIRE(ret == SQL_SUCCESS);
    CHECK(numeric_val_to_ull(numeric) == 0);
  }
}

TEST_CASE("NUMBER NULL to SQL_C_NUMERIC", "[fixed][conversion][c_numeric][null]") {
  // Given A Snowflake connection is established
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);

  // When A NULL NUMBER value is queried
  auto stmt = conn.execute_fetch("SELECT NULL::NUMBER(10,2)");
  // Then Indicator returns SQL_NULL_DATA
  check_null_via_get_data(stmt, 1, SQL_C_NUMERIC);
}
