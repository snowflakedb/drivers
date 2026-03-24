#include <catch2/catch_test_macros.hpp>

#include "ODBCFixtures.hpp"
#include "odbc_cast.hpp"
#include "odbc_matchers.hpp"

TEST_CASE_METHOD(DbcDefaultDSNFixture, "Replay: SELECT 1", "[replay]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();
  // SQLDriverConnect
  {
    SQLRETURN ret = SQLDriverConnect(dbc_handle(), nullptr, sqlchar(connection_string().c_str()), SQL_NTS, nullptr, 0,
                                     nullptr, SQL_DRIVER_NOPROMPT);
    REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc_handle()), OdbcMatchers::Succeeded());
  }

  // SQLGetInfo - SQL_DRIVER_VER
  {
    char buf[256] = {};
    SQLSMALLINT len = 0;
    SQLRETURN ret = SQLGetInfo(dbc_handle(), SQL_DRIVER_VER, buf, 255, &len);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc_handle()), OdbcMatchers::Succeeded());
  }

  // SQLGetInfo - SQL_DRIVER_NAME
  {
    char buf[256] = {};
    SQLSMALLINT len = 0;
    SQLRETURN ret = SQLGetInfo(dbc_handle(), SQL_DRIVER_NAME, buf, 255, &len);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc_handle()), OdbcMatchers::Succeeded());
  }

  SQLHSTMT stmt0 = SQL_NULL_HSTMT;
  // SQLAllocHandle - SQLHSTMT
  {
    SQLRETURN ret = SQLAllocHandle(SQL_HANDLE_STMT, dbc_handle(), &stmt0);
    REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc_handle()), OdbcMatchers::IsSuccess());
  }

  // SQLPrepare
  {
    SQLRETURN ret = SQLPrepare(stmt0, sqlchar("SELECT 1"), SQL_NTS);
    REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
  }

  // SQLExecute
  {
    SQLRETURN ret = SQLExecute(stmt0);
    REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
  }

  // SQLNumResultCols
  {
    SQLSMALLINT numCols = 0;
    SQLRETURN ret = SQLNumResultCols(stmt0, &numCols);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(numCols == 1);
  }

  // SQLDescribeCol col 1
  {
    char colName[51] = {};
    SQLSMALLINT dataType = 0, scale = 0, nullable = 0;
    SQLULEN colSize = 0;
    SQLRETURN ret = SQLDescribeCol(stmt0, 1, reinterpret_cast<SQLCHAR*>(colName), 50, nullptr, &dataType, &colSize,
                                   &scale, &nullable);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(colName) == "1");
    CHECK(dataType == SQL_DECIMAL);
    CHECK(colSize == 1);
    CHECK(scale == 0);
    CHECK(nullable == SQL_NO_NULLS);
  }

  // SQLFetchScroll
  {
    SQLRETURN ret = SQLFetchScroll(stmt0, SQL_FETCH_NEXT, 1);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
  }

  // SQLGetData col 1
  {
    char buf[1025] = {};
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 1, SQL_C_CHAR, buf, 1024, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf) == "1");
    CHECK(ind == 1);
  }

  // SQLFetchScroll
  {
    SQLRETURN ret = SQLFetchScroll(stmt0, SQL_FETCH_NEXT, 1);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsNoData());
  }

  // SQLMoreResults
  {
    SQLRETURN ret = SQLMoreResults(stmt0);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsNoData());
  }

  // SQLCloseCursor
  {
    SQLRETURN ret = SQLCloseCursor(stmt0);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsError());
  }

  // SQLCloseCursor
  {
    SQLRETURN ret = SQLCloseCursor(stmt0);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsError());
  }

  SQLFreeHandle(SQL_HANDLE_STMT, stmt0);
  SQLDisconnect(dbc_handle());
}
