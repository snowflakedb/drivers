// INTERVAL conversion to SQL_C_DEFAULT E2E tests
// SQL_C_DEFAULT for SQL_INTERVAL_YEAR_TO_MONTH / SQL_INTERVAL_DAY_TO_SECOND resolves to the
// SQL_C_INTERVAL_* type with the same code (ODBC Appendix D, "Default C Data Types").

#include <sql.h>
#include <sqlext.h>
#include <sqltypes.h>

#include <cstring>
#include <string>

#include <catch2/catch_test_macros.hpp>

#include "Connection.hpp"
#include "HandleWrapper.hpp"
#include "compatibility.hpp"
#include "odbc_cast.hpp"
#include "odbc_matchers.hpp"

namespace {

SQL_INTERVAL_STRUCT filled_interval() {
  SQL_INTERVAL_STRUCT iv;
  std::memset(&iv, 0xFF, sizeof(iv));
  return iv;
}

void check_year_month(const SQL_INTERVAL_STRUCT& iv, SQLLEN ind, SQLSMALLINT sign, SQLUINTEGER year,
                      SQLUINTEGER month) {
  CHECK(ind == static_cast<SQLLEN>(sizeof(SQL_INTERVAL_STRUCT)));
  CHECK(iv.interval_type == SQL_IS_YEAR_TO_MONTH);
  CHECK(iv.interval_sign == sign);
  CHECK(iv.intval.year_month.year == year);
  CHECK(iv.intval.year_month.month == month);
}

void check_day_to_second(const SQL_INTERVAL_STRUCT& iv, SQLLEN ind, SQLSMALLINT sign) {
  CHECK(ind == static_cast<SQLLEN>(sizeof(SQL_INTERVAL_STRUCT)));
  CHECK(iv.interval_type == SQL_IS_DAY_TO_SECOND);
  CHECK(iv.interval_sign == sign);
  CHECK(iv.intval.day_second.day == 12);
  CHECK(iv.intval.day_second.hour == 3);
  CHECK(iv.intval.day_second.minute == 4);
  CHECK(iv.intval.day_second.second == 5);
  CHECK(iv.intval.day_second.fraction == 678000);
}

std::string bound_string(const char* buf, SQLLEN ind, size_t buf_size) {
  REQUIRE(ind >= 0);
  REQUIRE(ind < static_cast<SQLLEN>(buf_size));
  return std::string(buf, ind);
}

constexpr char kUnwritten = 0x7F;

void check_unwritten(const char* buf, size_t size) {
  for (size_t i = 0; i < size; ++i) {
    INFO("byte " << i);
    CHECK(buf[i] == kUnwritten);
  }
}

std::string get_default_string(const StatementHandleWrapper& stmt, SQLUSMALLINT col) {
  char buf[64] = {};
  SQLLEN ind = 0;
  REQUIRE_ODBC(SQLGetData(stmt.getHandle(), col, SQL_C_DEFAULT, buf, sizeof(buf), &ind), stmt);
  return bound_string(buf, ind, sizeof(buf));
}

}  // namespace

TEST_CASE("should fetch INTERVAL columns with SQL_C_DEFAULT as the C type matching the column SQL type",
          "[e2e][types][interval][conversion][c_default]") {
  // Given Snowflake client is logged in
  Connection conn;

  // When Query "SELECT '-1-2'::INTERVAL YEAR TO MONTH, '-12 3:4:5.678'::INTERVAL DAY TO SECOND,
  //   '1-2'::INTERVAL YEAR TO MONTH, '12 3:4:5.678'::INTERVAL DAY TO SECOND" is executed
  auto stmt = conn.createStatement();
  SQLRETURN ret =
      SQLExecDirect(stmt.getHandle(),
                    sqlchar("SELECT '-1-2'::INTERVAL YEAR TO MONTH, '-12 3:4:5.678'::INTERVAL DAY TO SECOND, "
                            "'1-2'::INTERVAL YEAR TO MONTH, '12 3:4:5.678'::INTERVAL DAY TO SECOND"),
                    SQL_NTS);
  REQUIRE_ODBC(ret, stmt);

  // Then columns 1-2 via SQLBindCol and columns 3-4 via SQLGetData with SQL_C_DEFAULT return SQL_INTERVAL_STRUCT
  // on the new driver and the numeric string on the old driver
  NEW_DRIVER_ONLY("BD#sql-c-default-interval-fetch") {
    SQL_INTERVAL_STRUCT bound_ym = filled_interval();
    SQL_INTERVAL_STRUCT bound_ds = filled_interval();
    SQLLEN bound_ym_ind = 0;
    SQLLEN bound_ds_ind = 0;
    REQUIRE_ODBC(SQLBindCol(stmt.getHandle(), 1, SQL_C_DEFAULT, &bound_ym, sizeof(bound_ym), &bound_ym_ind), stmt);
    REQUIRE_ODBC(SQLBindCol(stmt.getHandle(), 2, SQL_C_DEFAULT, &bound_ds, sizeof(bound_ds), &bound_ds_ind), stmt);
    REQUIRE_ODBC(SQLFetch(stmt.getHandle()), stmt);
    check_year_month(bound_ym, bound_ym_ind, SQL_TRUE, 1, 2);
    check_day_to_second(bound_ds, bound_ds_ind, SQL_TRUE);

    SQL_INTERVAL_STRUCT got_ym = filled_interval();
    SQL_INTERVAL_STRUCT got_ds = filled_interval();
    SQLLEN got_ym_ind = 0;
    SQLLEN got_ds_ind = 0;
    REQUIRE_ODBC(SQLGetData(stmt.getHandle(), 3, SQL_C_DEFAULT, &got_ym, sizeof(got_ym), &got_ym_ind), stmt);
    REQUIRE_ODBC(SQLGetData(stmt.getHandle(), 4, SQL_C_DEFAULT, &got_ds, sizeof(got_ds), &got_ds_ind), stmt);
    check_year_month(got_ym, got_ym_ind, SQL_FALSE, 1, 2);
    check_day_to_second(got_ds, got_ds_ind, SQL_FALSE);
  }
  OLD_DRIVER_ONLY("BD#sql-c-default-interval-fetch") {
    char bound_ym[64] = {};
    char bound_ds[64] = {};
    SQLLEN bound_ym_ind = 0;
    SQLLEN bound_ds_ind = 0;
    REQUIRE_ODBC(SQLBindCol(stmt.getHandle(), 1, SQL_C_DEFAULT, bound_ym, sizeof(bound_ym), &bound_ym_ind), stmt);
    REQUIRE_ODBC(SQLBindCol(stmt.getHandle(), 2, SQL_C_DEFAULT, bound_ds, sizeof(bound_ds), &bound_ds_ind), stmt);
    REQUIRE_ODBC(SQLFetch(stmt.getHandle()), stmt);
    CHECK(bound_string(bound_ym, bound_ym_ind, sizeof(bound_ym)) == "-14");
    CHECK(bound_string(bound_ds, bound_ds_ind, sizeof(bound_ds)) == "-1047845678000.000");

    CHECK(get_default_string(stmt, 3) == "14");
    CHECK(get_default_string(stmt, 4) == "1047845678000.000");
  }
}

TEST_CASE("should report 22015 when SQL_C_DEFAULT fetch of INTERVAL exceeds the default leading precision",
          "[e2e][types][interval][conversion][c_default]") {
  // Given Snowflake client is logged in
  Connection conn;

  // When Query "SELECT '100-0'::INTERVAL YEAR TO MONTH" is executed
  auto stmt = conn.execute_fetch("SELECT '100-0'::INTERVAL YEAR TO MONTH");

  // Then SQLGetData with SQL_C_DEFAULT fails with 22015 on the new driver, whose default interval leading precision
  // is two digits, and returns the numeric string on the old driver
  NEW_DRIVER_ONLY("BD#sql-c-default-interval-fetch") {
    SQL_INTERVAL_STRUCT iv = filled_interval();
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt.getHandle(), 1, SQL_C_DEFAULT, &iv, sizeof(iv), &ind);
    REQUIRE_EXPECTED_ERROR(ret, "22015", stmt.getHandle(), SQL_HANDLE_STMT);
  }
  OLD_DRIVER_ONLY("BD#sql-c-default-interval-fetch") { CHECK(get_default_string(stmt, 1) == "1200"); }
}

TEST_CASE("should report 22015 when a SQL_C_DEFAULT bound INTERVAL column exceeds the default leading precision",
          "[e2e][types][interval][conversion][c_default]") {
  // Given Snowflake client is logged in
  Connection conn;

  // When Query "SELECT '100-0'::INTERVAL YEAR TO MONTH" is executed and column 1 is bound with SQL_C_DEFAULT
  auto stmt = conn.createStatement();
  REQUIRE_ODBC(SQLExecDirect(stmt.getHandle(), sqlchar("SELECT '100-0'::INTERVAL YEAR TO MONTH"), SQL_NTS), stmt);

  // Then SQLFetch fails with 22015 on the new driver and returns the numeric string on the old driver
  NEW_DRIVER_ONLY("BD#sql-c-default-interval-fetch") {
    SQL_INTERVAL_STRUCT iv = filled_interval();
    SQLLEN ind = 0;
    REQUIRE_ODBC(SQLBindCol(stmt.getHandle(), 1, SQL_C_DEFAULT, &iv, sizeof(iv), &ind), stmt);
    SQLRETURN ret = SQLFetch(stmt.getHandle());
    REQUIRE_EXPECTED_ERROR(ret, "22015", stmt.getHandle(), SQL_HANDLE_STMT);
  }
  OLD_DRIVER_ONLY("BD#sql-c-default-interval-fetch") {
    char buf[64] = {};
    SQLLEN ind = 0;
    REQUIRE_ODBC(SQLBindCol(stmt.getHandle(), 1, SQL_C_DEFAULT, buf, sizeof(buf), &ind), stmt);
    REQUIRE_ODBC(SQLFetch(stmt.getHandle()), stmt);
    CHECK(bound_string(buf, ind, sizeof(buf)) == "1200");
  }
}

TEST_CASE("should report HY090 when a SQL_C_DEFAULT INTERVAL SQLGetData buffer is smaller than SQL_INTERVAL_STRUCT",
          "[e2e][types][interval][conversion][c_default]") {
  // Given Snowflake client is logged in
  Connection conn;

  // When Query "SELECT '-1-2'::INTERVAL YEAR TO MONTH" is executed and SQLGetData reads it with SQL_C_DEFAULT into a
  // 16-byte character buffer
  auto stmt = conn.execute_fetch("SELECT '-1-2'::INTERVAL YEAR TO MONTH");
  char buf[16];
  std::memset(buf, kUnwritten, sizeof(buf));
  SQLLEN ind = 0;
  SQLRETURN ret = SQLGetData(stmt.getHandle(), 1, SQL_C_DEFAULT, buf, sizeof(buf), &ind);

  // Then the new driver fails with HY090 and leaves the buffer unwritten, and the old driver returns the numeric string
  NEW_DRIVER_ONLY("BD#sql-c-default-interval-fetch") {
    REQUIRE_EXPECTED_ERROR(ret, "HY090", stmt.getHandle(), SQL_HANDLE_STMT);
    check_unwritten(buf, sizeof(buf));
  }
  OLD_DRIVER_ONLY("BD#sql-c-default-interval-fetch") {
    REQUIRE_ODBC(ret, stmt);
    CHECK(bound_string(buf, ind, sizeof(buf)) == "-14");
  }
}

TEST_CASE("should report HY090 when a SQL_C_DEFAULT bound INTERVAL column buffer is smaller than SQL_INTERVAL_STRUCT",
          "[e2e][types][interval][conversion][c_default]") {
  // Given Snowflake client is logged in
  Connection conn;

  // When Query "SELECT '-1-2'::INTERVAL YEAR TO MONTH" is executed, column 1 is bound with SQL_C_DEFAULT to a 16-byte
  // character buffer, and the row is fetched
  auto stmt = conn.createStatement();
  REQUIRE_ODBC(SQLExecDirect(stmt.getHandle(), sqlchar("SELECT '-1-2'::INTERVAL YEAR TO MONTH"), SQL_NTS), stmt);
  char buf[16];
  std::memset(buf, kUnwritten, sizeof(buf));
  SQLLEN ind = 0;
  REQUIRE_ODBC(SQLBindCol(stmt.getHandle(), 1, SQL_C_DEFAULT, buf, sizeof(buf), &ind), stmt);
  SQLRETURN ret = SQLFetch(stmt.getHandle());

  // Then the new driver fails with HY090 and leaves the buffer unwritten, and the old driver returns the numeric string
  NEW_DRIVER_ONLY("BD#sql-c-default-interval-fetch") {
    REQUIRE_EXPECTED_ERROR(ret, "HY090", stmt.getHandle(), SQL_HANDLE_STMT);
    check_unwritten(buf, sizeof(buf));
  }
  OLD_DRIVER_ONLY("BD#sql-c-default-interval-fetch") {
    REQUIRE_ODBC(ret, stmt);
    CHECK(bound_string(buf, ind, sizeof(buf)) == "-14");
  }
}
