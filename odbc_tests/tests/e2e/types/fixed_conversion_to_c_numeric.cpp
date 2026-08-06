#include <sql.h>
#include <sqlext.h>
#include <sqltypes.h>

#include <catch2/catch_test_macros.hpp>

#include "SchemaFixtures.hpp"
#include "compatibility.hpp"
#include "conversion_checks.hpp"
#include "get_data.hpp"
#include "get_diag_rec.hpp"
#include "odbc_matchers.hpp"

TEST_CASE_METHOD(ConnSchemaFixture, "SQL_DECIMAL to SQL_C_NUMERIC", "[fixed][conversion][c_numeric]") {
  // Given A Snowflake connection is established

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

TEST_CASE_METHOD(ConnSchemaFixture, "SQL_DECIMAL to SQL_C_NUMERIC with SQL_DESC_PRECISION and SQL_DESC_SCALE",
                 "[fixed][conversion][c_numeric][descriptor]") {
  // Given A Snowflake connection is established
  // BD#13: old driver ignores SQL_DESC_PRECISION and SQL_DESC_SCALE, always using precision=38 scale=0.
  // New driver honours the descriptor settings.

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
    // Then The conversion result and precision differ between old and new driver (BD#13)
    OLD_DRIVER_ONLY("BD#13") {
      CHECK(ret == SQL_SUCCESS_WITH_INFO);
      CHECK(get_sqlstate(stmt) == "01S07");
    }
    NEW_DRIVER_ONLY("BD#13") { REQUIRE(ret == SQL_SUCCESS); }
    CHECK(numeric.sign == 1);
    OLD_DRIVER_ONLY("BD#13") {
      CHECK(numeric.precision == 38);
      CHECK(numeric.scale == 0);
      CHECK(numeric_val_to_ull(numeric) == 123);
    }
    NEW_DRIVER_ONLY("BD#13") {
      CHECK(numeric.precision == 10);
      CHECK(numeric.scale == 2);
      CHECK(numeric_val_to_ull(numeric) == 12345);
    }
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
    // Both drivers truncate .45 → 01S07; precision differs
    CHECK(ret == SQL_SUCCESS_WITH_INFO);
    CHECK(get_sqlstate(stmt) == "01S07");
    CHECK(numeric_val_to_ull(numeric) == 123);
    OLD_DRIVER_ONLY("BD#13") { CHECK(numeric.precision == 38); }
    NEW_DRIVER_ONLY("BD#13") { CHECK(numeric.precision == 10); }
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
    // Both return SQL_SUCCESS (42 is integer, no fractional truncation either way)
    CHECK(ret == SQL_SUCCESS);
    OLD_DRIVER_ONLY("BD#13") {
      CHECK(numeric.precision == 38);
      CHECK(numeric.scale == 0);
      CHECK(numeric_val_to_ull(numeric) == 42);
    }
    NEW_DRIVER_ONLY("BD#13") {
      CHECK(numeric.precision == 10);
      CHECK(numeric.scale == 3);
      CHECK(numeric_val_to_ull(numeric) == 42000);
    }
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
    // Old uses scale=0, truncates .300 → 01S07. New uses scale=1, exact → SQL_SUCCESS.
    OLD_DRIVER_ONLY("BD#13") {
      CHECK(ret == SQL_SUCCESS_WITH_INFO);
      CHECK(get_sqlstate(stmt) == "01S07");
      CHECK(numeric.precision == 38);
      CHECK(numeric.scale == 0);
      CHECK(numeric_val_to_ull(numeric) == 12);
    }
    NEW_DRIVER_ONLY("BD#13") {
      REQUIRE(ret == SQL_SUCCESS);
      CHECK(numeric.precision == 10);
      CHECK(numeric.scale == 1);
      CHECK(numeric_val_to_ull(numeric) == 123);
    }
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
    // Both truncate fractional part → 01S07; but to different scales (0 vs 1)
    CHECK(ret == SQL_SUCCESS_WITH_INFO);
    CHECK(get_sqlstate(stmt) == "01S07");
    OLD_DRIVER_ONLY("BD#13") {
      CHECK(numeric.precision == 38);
      CHECK(numeric.scale == 0);
      CHECK(numeric_val_to_ull(numeric) == 1);
    }
    NEW_DRIVER_ONLY("BD#13") {
      CHECK(numeric.precision == 10);
      CHECK(numeric.scale == 1);
      CHECK(numeric_val_to_ull(numeric) == 19);
    }
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
    CHECK(ret == SQL_SUCCESS);
    OLD_DRIVER_ONLY("BD#13") { CHECK(numeric.precision == 38); }
    NEW_DRIVER_ONLY("BD#13") { CHECK(numeric.precision == 5); }
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
    CHECK(ret == SQL_SUCCESS);
    CHECK(numeric.sign == 0);
    OLD_DRIVER_ONLY("BD#13") {
      CHECK(numeric.precision == 38);
      CHECK(numeric.scale == 0);
      CHECK(numeric_val_to_ull(numeric) == 7);
    }
    NEW_DRIVER_ONLY("BD#13") { CHECK(numeric_val_to_ull(numeric) == 700); }
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
    CHECK(ret == SQL_SUCCESS);
    CHECK(numeric_val_to_ull(numeric) == 0);
    OLD_DRIVER_ONLY("BD#13") {
      CHECK(numeric.precision == 38);
      CHECK(numeric.scale == 0);
    }
    NEW_DRIVER_ONLY("BD#13") { CHECK(numeric.scale == 5); }
  }
}

TEST_CASE_METHOD(ConnSchemaFixture, "NUMBER NULL to SQL_C_NUMERIC", "[fixed][conversion][c_numeric][null]") {
  // Given A Snowflake connection is established

  // When A NULL NUMBER value is queried
  auto stmt = conn.execute_fetch("SELECT NULL::NUMBER(10,2)");
  // Then Indicator returns SQL_NULL_DATA
  check_null_via_get_data(stmt, 1, SQL_C_NUMERIC);
}
