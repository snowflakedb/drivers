#include <sql.h>
#include <sqlext.h>
#include <sqltypes.h>

#include <cstring>
#include <vector>

#include <catch2/catch_test_macros.hpp>

#include "ODBCConfig.hpp"
#include "ODBCFixtures.hpp"
#include "compatibility.hpp"
#include "odbc_cast.hpp"
#include "test_macros.hpp"

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLSetStmtAttr: HY010 during SQL_NEED_DATA",
                 "[odbc-api][setstmtattr][driver_attributes][error]") {
  SQLRETURN ret = SQLPrepare(stmt_handle(), sqlchar("SELECT ?"), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);

  SQLLEN dae_ind = SQL_DATA_AT_EXEC;
  ret = SQLBindParameter(stmt_handle(), 1, SQL_PARAM_INPUT, SQL_C_CHAR, SQL_VARCHAR, 100, 0,
                         reinterpret_cast<SQLPOINTER>(1), 0, &dae_ind);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLExecute(stmt_handle());
  REQUIRE(ret == SQL_NEED_DATA);

  ret = SQLSetStmtAttr(stmt_handle(), SQL_ATTR_MAX_LENGTH, reinterpret_cast<SQLPOINTER>(1024), SQL_IS_UINTEGER);
  REQUIRE_EXPECTED_ERROR(ret, "HY010", stmt_handle(), SQL_HANDLE_STMT);

  SQLCancel(stmt_handle());
}

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLSetStmtAttr: SQL_ROWSET_SIZE = 0 returns SQL_ERROR",
                 "[odbc-api][setstmtattr][driver_attributes][error]") {
  // SQL_ROWSET_SIZE = 0 is invalid. unixODBC rejects it in the DM before the
  // driver; iODBC and the Windows DM forward it. 4.x returns
  // SQL_ERROR (HY024) either way. 3.x stores 0 when the DM forwards.
  SQLRETURN ret = SQLSetStmtAttr(stmt_handle(), SQL_ROWSET_SIZE, reinterpret_cast<SQLPOINTER>(0), 0);

  NEW_DRIVER_ONLY("BD#102") { REQUIRE_EXPECTED_ERROR(ret, "HY024", stmt_handle(), SQL_HANDLE_STMT); }
  OLD_DRIVER_ONLY("BD#102") {
    if (ret == SQL_SUCCESS || ret == SQL_SUCCESS_WITH_INFO) {
      SQLULEN rowset_size = 0;
      ret = SQLGetStmtAttr(stmt_handle(), SQL_ROWSET_SIZE, &rowset_size, SQL_IS_UINTEGER, nullptr);
      REQUIRE(ret == SQL_SUCCESS);
      REQUIRE(rowset_size == 0);
    } else {
      REQUIRE(ret == SQL_ERROR);
    }
  }
}

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLSetStmtAttr: SQL_ATTR_ROW_BIND_TYPE aligned stride round-trips",
                 "[odbc-api][setstmtattr][driver_attributes]") {
  struct AlignedRow {
    SQLLEN indicator;
    SQLCHAR payload[8];
  };
  static_assert(sizeof(AlignedRow) % alignof(SQLLEN) == 0);

  SQLRETURN ret =
      SQLSetStmtAttr(stmt_handle(), SQL_ATTR_ROW_BIND_TYPE, reinterpret_cast<SQLPOINTER>(sizeof(AlignedRow)), 0);
  REQUIRE(ret == SQL_SUCCESS);

  auto bind_type = static_cast<SQLULEN>(-1);
  ret = SQLGetStmtAttr(stmt_handle(), SQL_ATTR_ROW_BIND_TYPE, &bind_type, 0, nullptr);
  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(bind_type == sizeof(AlignedRow));
}

namespace {

constexpr SQLULEN kMisalignedRowBindType = 65540;
constexpr SQLULEN kTextValueOffset = 9;
constexpr SQLULEN kNumberIndicatorOffset = 24;
constexpr SQLULEN kNumberValueOffset = 32;

SQLRETURN set_misaligned_row_bind_type(SQLHSTMT stmt, const char* old_unix_sqlstate) {
  static_assert(kMisalignedRowBindType % 8 == 4);
  SQLRETURN ret = SQLSetStmtAttr(stmt, SQL_ATTR_ROW_BIND_TYPE, reinterpret_cast<SQLPOINTER>(kMisalignedRowBindType), 0);

  NEW_DRIVER_ONLY("BD#147") { REQUIRE(ret == SQL_SUCCESS); }
  OLD_DRIVER_ONLY("BD#147") {
    if (get_platform() == PLATFORM::PLATFORM_WINDOWS ||
        (get_platform() == PLATFORM::PLATFORM_LINUX && get_arch() == ARCH::ARCH_X86_64)) {
      REQUIRE(ret == SQL_SUCCESS);
    } else if (get_platform() == PLATFORM::PLATFORM_LINUX || get_platform() == PLATFORM::PLATFORM_MACOS) {
      REQUIRE_EXPECTED_ERROR(ret, old_unix_sqlstate, stmt, SQL_HANDLE_STMT);
    } else {
      FAIL("Unsupported platform");
    }
  }
  return ret;
}

}  // namespace

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLSetStmtAttr: SQL_ATTR_ROW_BIND_TYPE 65540 matches driver-specific behavior",
                 "[odbc-api][setstmtattr][driver_attributes]") {
  SQLRETURN ret = set_misaligned_row_bind_type(stmt_handle(), "HY000");

  if (ret != SQL_SUCCESS) {
    return;
  }

  SQLULEN bind_type = static_cast<SQLULEN>(-1);
  ret = SQLGetStmtAttr(stmt_handle(), SQL_ATTR_ROW_BIND_TYPE, &bind_type, 0, nullptr);
  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(bind_type == kMisalignedRowBindType);

  ret = SQLSetStmtAttr(stmt_handle(), SQL_ATTR_ROW_ARRAY_SIZE, reinterpret_cast<SQLPOINTER>(2), 0);
  REQUIRE(ret == SQL_SUCCESS);

  SQLULEN rows_fetched = 0;
  ret = SQLSetStmtAttr(stmt_handle(), SQL_ATTR_ROWS_FETCHED_PTR, &rows_fetched, 0);
  REQUIRE(ret == SQL_SUCCESS);

  // Row 1 is 4-mod-8. Offset 9 is 1-mod-2 so wchar stores are unaligned;
  // offset 32 is 4-mod-8 so the bigint store is unaligned.
  static_assert(kTextValueOffset % 2 == 1);
  static_assert((kMisalignedRowBindType + kTextValueOffset) % 2 == 1);
  static_assert((kMisalignedRowBindType + kNumberValueOffset) % alignof(SQLBIGINT) != 0);

  std::vector<unsigned char> row_buf(2 * kMisalignedRowBindType, 0xFF);
  ret = SQLBindCol(stmt_handle(), 1, SQL_C_WCHAR, row_buf.data() + kTextValueOffset, 2 * sizeof(SQLWCHAR),
                   reinterpret_cast<SQLLEN*>(row_buf.data()));
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLBindCol(stmt_handle(), 2, SQL_C_SBIGINT, row_buf.data() + kNumberValueOffset, sizeof(SQLBIGINT),
                   reinterpret_cast<SQLLEN*>(row_buf.data() + kNumberIndicatorOffset));
  REQUIRE(ret == SQL_SUCCESS);

  char query[] = "SELECT c, n FROM (SELECT 'x' AS c, 11 AS n UNION ALL SELECT 'y' AS c, 22 AS n) ORDER BY n";
  ret = SQLExecDirect(stmt_handle(), reinterpret_cast<SQLCHAR*>(query), SQL_NTS);
  REQUIRE((ret == SQL_SUCCESS || ret == SQL_SUCCESS_WITH_INFO));

  ret = SQLFetch(stmt_handle());
  REQUIRE((ret == SQL_SUCCESS || ret == SQL_SUCCESS_WITH_INFO));
  REQUIRE(rows_fetched == 2);

  for (SQLULEN row = 0; row < rows_fetched; ++row) {
    const unsigned char* row_base = row_buf.data() + row * kMisalignedRowBindType;

    SQLLEN textIndicator = 0;
    std::memcpy(&textIndicator, row_base, sizeof(textIndicator));
    REQUIRE(textIndicator != SQL_NULL_DATA);
    REQUIRE(textIndicator >= static_cast<SQLLEN>(sizeof(SQLWCHAR)));

    SQLWCHAR text = 0;
    std::memcpy(&text, row_base + kTextValueOffset, sizeof(text));
    REQUIRE(text == static_cast<SQLWCHAR>('x' + row));

    SQLLEN numberIndicator = 0;
    std::memcpy(&numberIndicator, row_base + kNumberIndicatorOffset, sizeof(numberIndicator));
    REQUIRE(numberIndicator == static_cast<SQLLEN>(sizeof(SQLBIGINT)));

    SQLBIGINT number = 0;
    std::memcpy(&number, row_base + kNumberValueOffset, sizeof(number));
    REQUIRE(number == static_cast<SQLBIGINT>(11 * (row + 1)));
  }
}

TEST_CASE_METHOD(StmtDefaultDSNOdbc2Fixture,
                 "SQLSetStmtAttr: SQL_ATTR_ROW_BIND_TYPE 65540 under SQL_OV_ODBC2 matches driver-specific behavior",
                 "[odbc-api][setstmtattr][driver_attributes]") {
  set_misaligned_row_bind_type(stmt_handle(), "S1000");
}
