#include <vector>

#include <catch2/catch_test_macros.hpp>

#include "ODBCConfig.hpp"
#include "compatibility.hpp"
#include "odbc_cast.hpp"
#include "odbc_matchers.hpp"

TEST_CASE("Replay: exec_direct_getdata_13col_91rows", "[dtm]") {
  auto config = DataSourceConfig::Snowflake().install();
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  SQLHENV env0 = SQL_NULL_HENV;
  // SQLAllocHandle - SQLHENV
  {
    SQLRETURN ret = SQLAllocHandle(SQL_HANDLE_ENV, SQL_NULL_HANDLE, &env0);
    REQUIRE(ret == SQL_SUCCESS);
    REQUIRE(env0 != SQL_NULL_HENV);
  }

  // SQLSetEnvAttr - SQL_ATTR_ODBC_VERSION
  {
    SQLRETURN ret = SQLSetEnvAttr(env0, SQL_ATTR_ODBC_VERSION, (SQLPOINTER)SQL_OV_ODBC3_80, 0);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_ENV, env0), OdbcMatchers::IsSuccess());
  }

  SQLHDBC dbc0 = SQL_NULL_HDBC;
  // SQLAllocHandle - SQLHDBC
  {
    SQLRETURN ret = SQLAllocHandle(SQL_HANDLE_DBC, env0, &dbc0);
    REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_ENV, env0), OdbcMatchers::IsSuccess());
    REQUIRE(dbc0 != SQL_NULL_HDBC);
  }

  // SQLSetConnectAttr - SQL_ATTR_CONNECTION_TIMEOUT
  {
    SQLRETURN ret = SQLSetConnectAttr(dbc0, SQL_ATTR_CONNECTION_TIMEOUT, nullptr, 0);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc0), OdbcMatchers::IsSuccess());
  }

  // SQLSetConnectAttr - SQL_ATTR_AUTOCOMMIT
  {
    SQLRETURN ret = SQLSetConnectAttr(dbc0, SQL_ATTR_AUTOCOMMIT, (SQLPOINTER)SQL_AUTOCOMMIT_ON, 0);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc0), OdbcMatchers::IsSuccess());
  }

  // SQLDriverConnect
  {
    SQLRETURN ret = SQLDriverConnect(dbc0, nullptr, sqlchar(config.connection_string().c_str()), SQL_NTS, nullptr, 0,
                                     nullptr, SQL_DRIVER_NOPROMPT);
    REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc0), OdbcMatchers::Succeeded());
  }

  // SQLSetConnectAttr - SQL_ATTR_AUTOCOMMIT
  {
    SQLRETURN ret = SQLSetConnectAttr(dbc0, SQL_ATTR_AUTOCOMMIT, (SQLPOINTER)SQL_AUTOCOMMIT_ON, 0);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc0), OdbcMatchers::IsSuccess());
  }

  SQLHSTMT stmt0 = SQL_NULL_HSTMT;
  // SQLAllocHandle - SQLHSTMT
  {
    SQLRETURN ret = SQLAllocHandle(SQL_HANDLE_STMT, dbc0, &stmt0);
    REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc0), OdbcMatchers::IsSuccess());
    REQUIRE(stmt0 != SQL_NULL_HSTMT);
  }

  // SQLExecDirect
  {
    SQLRETURN ret = SQLExecDirect(stmt0, sqlchar("desc table \"DTMREPLAYTESTDB\".\"PUBLIC\".\"bpamain\""), SQL_NTS);
    REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
  }

  // SQLNumResultCols
  {
    SQLSMALLINT numCols = 0;
    SQLRETURN ret = SQLNumResultCols(stmt0, &numCols);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(numCols == 13);
  }

  // SQLDescribeCol col 1
  {
    char colName[256] = {};
    SQLSMALLINT dataType = 0, scale = 0, nullable = 0;
    SQLULEN colSize = 0;
    SQLRETURN ret = SQLDescribeCol(stmt0, 1, reinterpret_cast<SQLCHAR*>(colName), 255, nullptr, &dataType, &colSize,
                                   &scale, &nullable);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(colName) == "name");
    CHECK(dataType == 12);
    CHECK(colSize == 16777216);
    CHECK(scale == 0);
    CHECK(nullable == 1);
  }

  // SQLDescribeCol col 2
  {
    char colName[256] = {};
    SQLSMALLINT dataType = 0, scale = 0, nullable = 0;
    SQLULEN colSize = 0;
    SQLRETURN ret = SQLDescribeCol(stmt0, 2, reinterpret_cast<SQLCHAR*>(colName), 255, nullptr, &dataType, &colSize,
                                   &scale, &nullable);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(colName) == "type");
    CHECK(dataType == 12);
    CHECK(colSize == 16777216);
    CHECK(scale == 0);
    CHECK(nullable == 1);
  }

  // SQLDescribeCol col 3
  {
    char colName[256] = {};
    SQLSMALLINT dataType = 0, scale = 0, nullable = 0;
    SQLULEN colSize = 0;
    SQLRETURN ret = SQLDescribeCol(stmt0, 3, reinterpret_cast<SQLCHAR*>(colName), 255, nullptr, &dataType, &colSize,
                                   &scale, &nullable);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(colName) == "kind");
    CHECK(dataType == 12);
    CHECK(colSize == 16777216);
    CHECK(scale == 0);
    CHECK(nullable == 1);
  }

  // SQLDescribeCol col 4
  {
    char colName[256] = {};
    SQLSMALLINT dataType = 0, scale = 0, nullable = 0;
    SQLULEN colSize = 0;
    SQLRETURN ret = SQLDescribeCol(stmt0, 4, reinterpret_cast<SQLCHAR*>(colName), 255, nullptr, &dataType, &colSize,
                                   &scale, &nullable);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(colName) == "null?");
    CHECK(dataType == 12);
    CHECK(colSize == 16777216);
    CHECK(scale == 0);
    CHECK(nullable == 1);
  }

  // SQLDescribeCol col 5
  {
    char colName[256] = {};
    SQLSMALLINT dataType = 0, scale = 0, nullable = 0;
    SQLULEN colSize = 0;
    SQLRETURN ret = SQLDescribeCol(stmt0, 5, reinterpret_cast<SQLCHAR*>(colName), 255, nullptr, &dataType, &colSize,
                                   &scale, &nullable);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(colName) == "default");
    CHECK(dataType == 12);
    CHECK(colSize == 16777216);
    CHECK(scale == 0);
    CHECK(nullable == 1);
  }

  // SQLDescribeCol col 6
  {
    char colName[256] = {};
    SQLSMALLINT dataType = 0, scale = 0, nullable = 0;
    SQLULEN colSize = 0;
    SQLRETURN ret = SQLDescribeCol(stmt0, 6, reinterpret_cast<SQLCHAR*>(colName), 255, nullptr, &dataType, &colSize,
                                   &scale, &nullable);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(colName) == "primary key");
    CHECK(dataType == 12);
    CHECK(colSize == 16777216);
    CHECK(scale == 0);
    CHECK(nullable == 1);
  }

  // SQLDescribeCol col 7
  {
    char colName[256] = {};
    SQLSMALLINT dataType = 0, scale = 0, nullable = 0;
    SQLULEN colSize = 0;
    SQLRETURN ret = SQLDescribeCol(stmt0, 7, reinterpret_cast<SQLCHAR*>(colName), 255, nullptr, &dataType, &colSize,
                                   &scale, &nullable);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(colName) == "unique key");
    CHECK(dataType == 12);
    CHECK(colSize == 16777216);
    CHECK(scale == 0);
    CHECK(nullable == 1);
  }

  // SQLDescribeCol col 8
  {
    char colName[256] = {};
    SQLSMALLINT dataType = 0, scale = 0, nullable = 0;
    SQLULEN colSize = 0;
    SQLRETURN ret = SQLDescribeCol(stmt0, 8, reinterpret_cast<SQLCHAR*>(colName), 255, nullptr, &dataType, &colSize,
                                   &scale, &nullable);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(colName) == "check");
    CHECK(dataType == 12);
    CHECK(colSize == 16777216);
    CHECK(scale == 0);
    CHECK(nullable == 1);
  }

  // SQLDescribeCol col 9
  {
    char colName[256] = {};
    SQLSMALLINT dataType = 0, scale = 0, nullable = 0;
    SQLULEN colSize = 0;
    SQLRETURN ret = SQLDescribeCol(stmt0, 9, reinterpret_cast<SQLCHAR*>(colName), 255, nullptr, &dataType, &colSize,
                                   &scale, &nullable);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(colName) == "expression");
    CHECK(dataType == 12);
    CHECK(colSize == 16777216);
    CHECK(scale == 0);
    CHECK(nullable == 1);
  }

  // SQLDescribeCol col 10
  {
    char colName[256] = {};
    SQLSMALLINT dataType = 0, scale = 0, nullable = 0;
    SQLULEN colSize = 0;
    SQLRETURN ret = SQLDescribeCol(stmt0, 10, reinterpret_cast<SQLCHAR*>(colName), 255, nullptr, &dataType, &colSize,
                                   &scale, &nullable);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(colName) == "comment");
    CHECK(dataType == 12);
    CHECK(colSize == 16777216);
    CHECK(scale == 0);
    CHECK(nullable == 1);
  }

  // SQLDescribeCol col 11
  {
    char colName[256] = {};
    SQLSMALLINT dataType = 0, scale = 0, nullable = 0;
    SQLULEN colSize = 0;
    SQLRETURN ret = SQLDescribeCol(stmt0, 11, reinterpret_cast<SQLCHAR*>(colName), 255, nullptr, &dataType, &colSize,
                                   &scale, &nullable);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(colName) == "policy name");
    CHECK(dataType == 12);
    CHECK(colSize == 16777216);
    CHECK(scale == 0);
    CHECK(nullable == 1);
  }

  // SQLDescribeCol col 12
  {
    char colName[256] = {};
    SQLSMALLINT dataType = 0, scale = 0, nullable = 0;
    SQLULEN colSize = 0;
    SQLRETURN ret = SQLDescribeCol(stmt0, 12, reinterpret_cast<SQLCHAR*>(colName), 255, nullptr, &dataType, &colSize,
                                   &scale, &nullable);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(colName) == "privacy domain");
    CHECK(dataType == 12);
    CHECK(colSize == 134217728);
    CHECK(scale == 0);
    CHECK(nullable == 1);
  }

  // SQLDescribeCol col 13
  {
    char colName[256] = {};
    SQLSMALLINT dataType = 0, scale = 0, nullable = 0;
    SQLULEN colSize = 0;
    SQLRETURN ret = SQLDescribeCol(stmt0, 13, reinterpret_cast<SQLCHAR*>(colName), 255, nullptr, &dataType, &colSize,
                                   &scale, &nullable);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(colName) == "write default");
    CHECK(dataType == 12);
    CHECK(colSize == 16777216);
    CHECK(scale == 0);
    CHECK(nullable == 1);
  }

  // SQLFetch
  {
    SQLRETURN ret = SQLFetch(stmt0);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
  }

  // SQLGetData col 1
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 1, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "Status");
    CHECK(ind == 6);
  }

  // SQLGetData col 2
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 2, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "VARCHAR(1)");
    CHECK(ind == 10);
  }

  // SQLGetData col 3
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 3, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "COLUMN");
    CHECK(ind == 6);
  }

  // SQLGetData col 4
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 4, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "Y");
    CHECK(ind == 1);
  }

  // SQLGetData col 5
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 5, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 6
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 6, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "N");
    CHECK(ind == 1);
  }

  // SQLGetData col 7
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 7, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "N");
    CHECK(ind == 1);
  }

  // SQLGetData col 8
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 8, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 9
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 9, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 10
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 10, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 11
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 11, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 12
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 12, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 13
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 13, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLFetch
  {
    SQLRETURN ret = SQLFetch(stmt0);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
  }

  // SQLGetData col 1
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 1, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "Dfu");
    CHECK(ind == 3);
  }

  // SQLGetData col 2
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 2, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "VARCHAR(1)");
    CHECK(ind == 10);
  }

  // SQLGetData col 3
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 3, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "COLUMN");
    CHECK(ind == 6);
  }

  // SQLGetData col 4
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 4, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "Y");
    CHECK(ind == 1);
  }

  // SQLGetData col 5
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 5, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 6
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 6, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "N");
    CHECK(ind == 1);
  }

  // SQLGetData col 7
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 7, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "N");
    CHECK(ind == 1);
  }

  // SQLGetData col 8
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 8, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 9
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 9, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 10
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 10, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 11
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 11, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 12
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 12, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 13
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 13, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLFetch
  {
    SQLRETURN ret = SQLFetch(stmt0);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
  }

  // SQLGetData col 1
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 1, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "FlWork");
    CHECK(ind == 6);
  }

  // SQLGetData col 2
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 2, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "VARCHAR(1)");
    CHECK(ind == 10);
  }

  // SQLGetData col 3
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 3, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "COLUMN");
    CHECK(ind == 6);
  }

  // SQLGetData col 4
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 4, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "Y");
    CHECK(ind == 1);
  }

  // SQLGetData col 5
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 5, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 6
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 6, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "N");
    CHECK(ind == 1);
  }

  // SQLGetData col 7
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 7, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "N");
    CHECK(ind == 1);
  }

  // SQLGetData col 8
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 8, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 9
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 9, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 10
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 10, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 11
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 11, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 12
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 12, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 13
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 13, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLFetch
  {
    SQLRETURN ret = SQLFetch(stmt0);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
  }

  // SQLGetData col 1
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 1, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "Au");
    CHECK(ind == 2);
  }

  // SQLGetData col 2
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 2, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "VARCHAR(8)");
    CHECK(ind == 10);
  }

  // SQLGetData col 3
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 3, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "COLUMN");
    CHECK(ind == 6);
  }

  // SQLGetData col 4
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 4, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "Y");
    CHECK(ind == 1);
  }

  // SQLGetData col 5
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 5, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 6
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 6, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "N");
    CHECK(ind == 1);
  }

  // SQLGetData col 7
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 7, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "N");
    CHECK(ind == 1);
  }

  // SQLGetData col 8
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 8, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 9
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 9, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 10
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 10, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 11
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 11, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 12
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 12, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 13
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 13, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLFetch
  {
    SQLRETURN ret = SQLFetch(stmt0);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
  }

  // SQLGetData col 1
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 1, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "modified_cd");
    CHECK(ind == 11);
  }

  // SQLGetData col 2
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 2, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "VARCHAR(8)");
    CHECK(ind == 10);
  }

  // SQLGetData col 3
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 3, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "COLUMN");
    CHECK(ind == 6);
  }

  // SQLGetData col 4
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 4, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "Y");
    CHECK(ind == 1);
  }

  // SQLGetData col 5
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 5, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 6
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 6, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "N");
    CHECK(ind == 1);
  }

  // SQLGetData col 7
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 7, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "N");
    CHECK(ind == 1);
  }

  // SQLGetData col 8
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 8, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 9
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 9, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 10
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 10, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 11
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 11, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 12
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 12, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 13
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 13, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLFetch
  {
    SQLRETURN ret = SQLFetch(stmt0);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
  }

  // SQLGetData col 1
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 1, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "PKey");
    CHECK(ind == 4);
  }

  // SQLGetData col 2
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 2, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "VARCHAR(16)");
    CHECK(ind == 11);
  }

  // SQLGetData col 3
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 3, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "COLUMN");
    CHECK(ind == 6);
  }

  // SQLGetData col 4
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 4, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "N");
    CHECK(ind == 1);
  }

  // SQLGetData col 5
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 5, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 6
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 6, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "N");
    CHECK(ind == 1);
  }

  // SQLGetData col 7
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 7, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "N");
    CHECK(ind == 1);
  }

  // SQLGetData col 8
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 8, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 9
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 9, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 10
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 10, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 11
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 11, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 12
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 12, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 13
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 13, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLFetch
  {
    SQLRETURN ret = SQLFetch(stmt0);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
  }

  // SQLGetData col 1
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 1, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "HKey");
    CHECK(ind == 4);
  }

  // SQLGetData col 2
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 2, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "VARCHAR(64)");
    CHECK(ind == 11);
  }

  // SQLGetData col 3
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 3, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "COLUMN");
    CHECK(ind == 6);
  }

  // SQLGetData col 4
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 4, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "Y");
    CHECK(ind == 1);
  }

  // SQLGetData col 5
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 5, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 6
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 6, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "N");
    CHECK(ind == 1);
  }

  // SQLGetData col 7
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 7, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "N");
    CHECK(ind == 1);
  }

  // SQLGetData col 8
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 8, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 9
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 9, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 10
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 10, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 11
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 11, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 12
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 12, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 13
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 13, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLFetch
  {
    SQLRETURN ret = SQLFetch(stmt0);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
  }

  // SQLGetData col 1
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 1, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "BusinessModified");
    CHECK(ind == 16);
  }

  // SQLGetData col 2
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 2, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "TIMESTAMP_NTZ(6)");
    CHECK(ind == 16);
  }

  // SQLGetData col 3
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 3, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "COLUMN");
    CHECK(ind == 6);
  }

  // SQLGetData col 4
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 4, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "Y");
    CHECK(ind == 1);
  }

  // SQLGetData col 5
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 5, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 6
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 6, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "N");
    CHECK(ind == 1);
  }

  // SQLGetData col 7
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 7, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "N");
    CHECK(ind == 1);
  }

  // SQLGetData col 8
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 8, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 9
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 9, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 10
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 10, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 11
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 11, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 12
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 12, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 13
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 13, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLFetch
  {
    SQLRETURN ret = SQLFetch(stmt0);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
  }

  // SQLGetData col 1
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 1, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "BusinessAu");
    CHECK(ind == 10);
  }

  // SQLGetData col 2
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 2, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "VARCHAR(8)");
    CHECK(ind == 10);
  }

  // SQLGetData col 3
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 3, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "COLUMN");
    CHECK(ind == 6);
  }

  // SQLGetData col 4
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 4, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "Y");
    CHECK(ind == 1);
  }

  // SQLGetData col 5
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 5, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 6
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 6, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "N");
    CHECK(ind == 1);
  }

  // SQLGetData col 7
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 7, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "N");
    CHECK(ind == 1);
  }

  // SQLGetData col 8
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 8, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 9
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 9, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 10
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 10, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 11
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 11, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 12
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 12, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 13
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 13, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLFetch
  {
    SQLRETURN ret = SQLFetch(stmt0);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
  }

  // SQLGetData col 1
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 1, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "HostSource");
    CHECK(ind == 10);
  }

  // SQLGetData col 2
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 2, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "VARCHAR(50)");
    CHECK(ind == 11);
  }

  // SQLGetData col 3
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 3, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "COLUMN");
    CHECK(ind == 6);
  }

  // SQLGetData col 4
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 4, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "Y");
    CHECK(ind == 1);
  }

  // SQLGetData col 5
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 5, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 6
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 6, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "N");
    CHECK(ind == 1);
  }

  // SQLGetData col 7
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 7, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "N");
    CHECK(ind == 1);
  }

  // SQLGetData col 8
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 8, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 9
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 9, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 10
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 10, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 11
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 11, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 12
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 12, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 13
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 13, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLFetch
  {
    SQLRETURN ret = SQLFetch(stmt0);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
  }

  // SQLGetData col 1
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 1, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "Client");
    CHECK(ind == 6);
  }

  // SQLGetData col 2
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 2, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "VARCHAR(3)");
    CHECK(ind == 10);
  }

  // SQLGetData col 3
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 3, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "COLUMN");
    CHECK(ind == 6);
  }

  // SQLGetData col 4
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 4, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "Y");
    CHECK(ind == 1);
  }

  // SQLGetData col 5
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 5, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 6
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 6, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "N");
    CHECK(ind == 1);
  }

  // SQLGetData col 7
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 7, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "N");
    CHECK(ind == 1);
  }

  // SQLGetData col 8
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 8, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 9
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 9, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 10
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 10, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 11
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 11, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 12
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 12, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 13
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 13, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLFetch
  {
    SQLRETURN ret = SQLFetch(stmt0);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
  }

  // SQLGetData col 1
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 1, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "Phone1");
    CHECK(ind == 6);
  }

  // SQLGetData col 2
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 2, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "VARCHAR(30)");
    CHECK(ind == 11);
  }

  // SQLGetData col 3
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 3, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "COLUMN");
    CHECK(ind == 6);
  }

  // SQLGetData col 4
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 4, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "Y");
    CHECK(ind == 1);
  }

  // SQLGetData col 5
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 5, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 6
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 6, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "N");
    CHECK(ind == 1);
  }

  // SQLGetData col 7
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 7, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "N");
    CHECK(ind == 1);
  }

  // SQLGetData col 8
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 8, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 9
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 9, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 10
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 10, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 11
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 11, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 12
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 12, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 13
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 13, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLFetch
  {
    SQLRETURN ret = SQLFetch(stmt0);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
  }

  // SQLGetData col 1
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 1, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "Phone2");
    CHECK(ind == 6);
  }

  // SQLGetData col 2
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 2, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "VARCHAR(30)");
    CHECK(ind == 11);
  }

  // SQLGetData col 3
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 3, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "COLUMN");
    CHECK(ind == 6);
  }

  // SQLGetData col 4
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 4, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "Y");
    CHECK(ind == 1);
  }

  // SQLGetData col 5
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 5, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 6
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 6, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "N");
    CHECK(ind == 1);
  }

  // SQLGetData col 7
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 7, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "N");
    CHECK(ind == 1);
  }

  // SQLGetData col 8
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 8, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 9
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 9, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 10
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 10, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 11
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 11, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 12
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 12, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 13
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 13, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLFetch
  {
    SQLRETURN ret = SQLFetch(stmt0);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
  }

  // SQLGetData col 1
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 1, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "Phone3");
    CHECK(ind == 6);
  }

  // SQLGetData col 2
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 2, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "VARCHAR(30)");
    CHECK(ind == 11);
  }

  // SQLGetData col 3
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 3, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "COLUMN");
    CHECK(ind == 6);
  }

  // SQLGetData col 4
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 4, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "Y");
    CHECK(ind == 1);
  }

  // SQLGetData col 5
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 5, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 6
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 6, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "N");
    CHECK(ind == 1);
  }

  // SQLGetData col 7
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 7, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "N");
    CHECK(ind == 1);
  }

  // SQLGetData col 8
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 8, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 9
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 9, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 10
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 10, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 11
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 11, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 12
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 12, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 13
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 13, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLFetch
  {
    SQLRETURN ret = SQLFetch(stmt0);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
  }

  // SQLGetData col 1
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 1, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "Fax1");
    CHECK(ind == 4);
  }

  // SQLGetData col 2
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 2, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "VARCHAR(30)");
    CHECK(ind == 11);
  }

  // SQLGetData col 3
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 3, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "COLUMN");
    CHECK(ind == 6);
  }

  // SQLGetData col 4
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 4, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "Y");
    CHECK(ind == 1);
  }

  // SQLGetData col 5
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 5, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 6
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 6, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "N");
    CHECK(ind == 1);
  }

  // SQLGetData col 7
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 7, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "N");
    CHECK(ind == 1);
  }

  // SQLGetData col 8
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 8, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 9
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 9, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 10
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 10, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 11
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 11, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 12
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 12, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 13
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 13, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLFetch
  {
    SQLRETURN ret = SQLFetch(stmt0);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
  }

  // SQLGetData col 1
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 1, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "Fax2");
    CHECK(ind == 4);
  }

  // SQLGetData col 2
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 2, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "VARCHAR(30)");
    CHECK(ind == 11);
  }

  // SQLGetData col 3
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 3, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "COLUMN");
    CHECK(ind == 6);
  }

  // SQLGetData col 4
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 4, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "Y");
    CHECK(ind == 1);
  }

  // SQLGetData col 5
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 5, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 6
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 6, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "N");
    CHECK(ind == 1);
  }

  // SQLGetData col 7
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 7, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "N");
    CHECK(ind == 1);
  }

  // SQLGetData col 8
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 8, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 9
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 9, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 10
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 10, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 11
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 11, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 12
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 12, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 13
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 13, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLFetch
  {
    SQLRETURN ret = SQLFetch(stmt0);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
  }

  // SQLGetData col 1
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 1, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "Fax3");
    CHECK(ind == 4);
  }

  // SQLGetData col 2
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 2, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "VARCHAR(30)");
    CHECK(ind == 11);
  }

  // SQLGetData col 3
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 3, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "COLUMN");
    CHECK(ind == 6);
  }

  // SQLGetData col 4
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 4, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "Y");
    CHECK(ind == 1);
  }

  // SQLGetData col 5
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 5, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 6
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 6, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "N");
    CHECK(ind == 1);
  }

  // SQLGetData col 7
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 7, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "N");
    CHECK(ind == 1);
  }

  // SQLGetData col 8
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 8, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 9
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 9, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 10
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 10, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 11
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 11, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 12
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 12, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 13
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 13, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLFetch
  {
    SQLRETURN ret = SQLFetch(stmt0);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
  }

  // SQLGetData col 1
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 1, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "EMail1");
    CHECK(ind == 6);
  }

  // SQLGetData col 2
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 2, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "VARCHAR(60)");
    CHECK(ind == 11);
  }

  // SQLGetData col 3
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 3, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "COLUMN");
    CHECK(ind == 6);
  }

  // SQLGetData col 4
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 4, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "Y");
    CHECK(ind == 1);
  }

  // SQLGetData col 5
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 5, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 6
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 6, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "N");
    CHECK(ind == 1);
  }

  // SQLGetData col 7
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 7, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "N");
    CHECK(ind == 1);
  }

  // SQLGetData col 8
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 8, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 9
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 9, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 10
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 10, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 11
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 11, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 12
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 12, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 13
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 13, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLFetch
  {
    SQLRETURN ret = SQLFetch(stmt0);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
  }

  // SQLGetData col 1
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 1, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "EMail2");
    CHECK(ind == 6);
  }

  // SQLGetData col 2
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 2, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "VARCHAR(60)");
    CHECK(ind == 11);
  }

  // SQLGetData col 3
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 3, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "COLUMN");
    CHECK(ind == 6);
  }

  // SQLGetData col 4
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 4, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "Y");
    CHECK(ind == 1);
  }

  // SQLGetData col 5
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 5, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 6
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 6, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "N");
    CHECK(ind == 1);
  }

  // SQLGetData col 7
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 7, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "N");
    CHECK(ind == 1);
  }

  // SQLGetData col 8
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 8, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 9
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 9, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 10
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 10, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 11
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 11, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 12
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 12, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 13
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 13, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLFetch
  {
    SQLRETURN ret = SQLFetch(stmt0);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
  }

  // SQLGetData col 1
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 1, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "EMail3");
    CHECK(ind == 6);
  }

  // SQLGetData col 2
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 2, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "VARCHAR(60)");
    CHECK(ind == 11);
  }

  // SQLGetData col 3
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 3, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "COLUMN");
    CHECK(ind == 6);
  }

  // SQLGetData col 4
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 4, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "Y");
    CHECK(ind == 1);
  }

  // SQLGetData col 5
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 5, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 6
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 6, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "N");
    CHECK(ind == 1);
  }

  // SQLGetData col 7
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 7, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "N");
    CHECK(ind == 1);
  }

  // SQLGetData col 8
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 8, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 9
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 9, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 10
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 10, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 11
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 11, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 12
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 12, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 13
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 13, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLFetch
  {
    SQLRETURN ret = SQLFetch(stmt0);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
  }

  // SQLGetData col 1
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 1, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "URL1");
    CHECK(ind == 4);
  }

  // SQLGetData col 2
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 2, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "VARCHAR(256)");
    CHECK(ind == 12);
  }

  // SQLGetData col 3
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 3, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "COLUMN");
    CHECK(ind == 6);
  }

  // SQLGetData col 4
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 4, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "Y");
    CHECK(ind == 1);
  }

  // SQLGetData col 5
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 5, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 6
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 6, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "N");
    CHECK(ind == 1);
  }

  // SQLGetData col 7
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 7, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "N");
    CHECK(ind == 1);
  }

  // SQLGetData col 8
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 8, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 9
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 9, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 10
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 10, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 11
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 11, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 12
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 12, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 13
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 13, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLFetch
  {
    SQLRETURN ret = SQLFetch(stmt0);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
  }

  // SQLGetData col 1
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 1, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "URL2");
    CHECK(ind == 4);
  }

  // SQLGetData col 2
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 2, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "VARCHAR(256)");
    CHECK(ind == 12);
  }

  // SQLGetData col 3
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 3, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "COLUMN");
    CHECK(ind == 6);
  }

  // SQLGetData col 4
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 4, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "Y");
    CHECK(ind == 1);
  }

  // SQLGetData col 5
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 5, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 6
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 6, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "N");
    CHECK(ind == 1);
  }

  // SQLGetData col 7
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 7, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "N");
    CHECK(ind == 1);
  }

  // SQLGetData col 8
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 8, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 9
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 9, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 10
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 10, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 11
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 11, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 12
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 12, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 13
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 13, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLFetch
  {
    SQLRETURN ret = SQLFetch(stmt0);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
  }

  // SQLGetData col 1
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 1, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "URL3");
    CHECK(ind == 4);
  }

  // SQLGetData col 2
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 2, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "VARCHAR(256)");
    CHECK(ind == 12);
  }

  // SQLGetData col 3
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 3, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "COLUMN");
    CHECK(ind == 6);
  }

  // SQLGetData col 4
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 4, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "Y");
    CHECK(ind == 1);
  }

  // SQLGetData col 5
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 5, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 6
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 6, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "N");
    CHECK(ind == 1);
  }

  // SQLGetData col 7
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 7, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "N");
    CHECK(ind == 1);
  }

  // SQLGetData col 8
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 8, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 9
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 9, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 10
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 10, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 11
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 11, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 12
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 12, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 13
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 13, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLFetch
  {
    SQLRETURN ret = SQLFetch(stmt0);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
  }

  // SQLGetData col 1
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 1, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "Id");
    CHECK(ind == 2);
  }

  // SQLGetData col 2
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 2, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "VARCHAR(50)");
    CHECK(ind == 11);
  }

  // SQLGetData col 3
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 3, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "COLUMN");
    CHECK(ind == 6);
  }

  // SQLGetData col 4
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 4, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "Y");
    CHECK(ind == 1);
  }

  // SQLGetData col 5
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 5, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 6
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 6, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "N");
    CHECK(ind == 1);
  }

  // SQLGetData col 7
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 7, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "N");
    CHECK(ind == 1);
  }

  // SQLGetData col 8
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 8, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 9
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 9, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 10
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 10, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 11
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 11, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 12
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 12, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 13
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 13, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLFetch
  {
    SQLRETURN ret = SQLFetch(stmt0);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
  }

  // SQLGetData col 1
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 1, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "Name");
    CHECK(ind == 4);
  }

  // SQLGetData col 2
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 2, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "VARCHAR(256)");
    CHECK(ind == 12);
  }

  // SQLGetData col 3
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 3, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "COLUMN");
    CHECK(ind == 6);
  }

  // SQLGetData col 4
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 4, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "Y");
    CHECK(ind == 1);
  }

  // SQLGetData col 5
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 5, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 6
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 6, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "N");
    CHECK(ind == 1);
  }

  // SQLGetData col 7
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 7, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "N");
    CHECK(ind == 1);
  }

  // SQLGetData col 8
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 8, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 9
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 9, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 10
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 10, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 11
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 11, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 12
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 12, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 13
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 13, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLFetch
  {
    SQLRETURN ret = SQLFetch(stmt0);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
  }

  // SQLGetData col 1
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 1, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "Matchcode");
    CHECK(ind == 9);
  }

  // SQLGetData col 2
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 2, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "VARCHAR(256)");
    CHECK(ind == 12);
  }

  // SQLGetData col 3
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 3, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "COLUMN");
    CHECK(ind == 6);
  }

  // SQLGetData col 4
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 4, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "Y");
    CHECK(ind == 1);
  }

  // SQLGetData col 5
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 5, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 6
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 6, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "N");
    CHECK(ind == 1);
  }

  // SQLGetData col 7
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 7, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "N");
    CHECK(ind == 1);
  }

  // SQLGetData col 8
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 8, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 9
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 9, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 10
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 10, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 11
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 11, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 12
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 12, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 13
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 13, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLFetch
  {
    SQLRETURN ret = SQLFetch(stmt0);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
  }

  // SQLGetData col 1
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 1, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "BpaType");
    CHECK(ind == 7);
  }

  // SQLGetData col 2
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 2, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "VARCHAR(3)");
    CHECK(ind == 10);
  }

  // SQLGetData col 3
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 3, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "COLUMN");
    CHECK(ind == 6);
  }

  // SQLGetData col 4
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 4, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "Y");
    CHECK(ind == 1);
  }

  // SQLGetData col 5
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 5, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 6
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 6, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "N");
    CHECK(ind == 1);
  }

  // SQLGetData col 7
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 7, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "N");
    CHECK(ind == 1);
  }

  // SQLGetData col 8
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 8, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 9
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 9, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 10
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 10, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 11
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 11, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 12
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 12, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 13
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 13, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLFetch
  {
    SQLRETURN ret = SQLFetch(stmt0);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
  }

  // SQLGetData col 1
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 1, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "SalesRelevant");
    CHECK(ind == 13);
  }

  // SQLGetData col 2
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 2, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "VARCHAR(1)");
    CHECK(ind == 10);
  }

  // SQLGetData col 3
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 3, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "COLUMN");
    CHECK(ind == 6);
  }

  // SQLGetData col 4
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 4, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "Y");
    CHECK(ind == 1);
  }

  // SQLGetData col 5
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 5, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 6
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 6, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "N");
    CHECK(ind == 1);
  }

  // SQLGetData col 7
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 7, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "N");
    CHECK(ind == 1);
  }

  // SQLGetData col 8
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 8, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 9
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 9, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 10
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 10, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 11
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 11, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 12
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 12, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 13
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 13, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLFetch
  {
    SQLRETURN ret = SQLFetch(stmt0);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
  }

  // SQLGetData col 1
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 1, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "LanguageSpoken");
    CHECK(ind == 14);
  }

  // SQLGetData col 2
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 2, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "VARCHAR(2)");
    CHECK(ind == 10);
  }

  // SQLGetData col 3
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 3, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "COLUMN");
    CHECK(ind == 6);
  }

  // SQLGetData col 4
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 4, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "Y");
    CHECK(ind == 1);
  }

  // SQLGetData col 5
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 5, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 6
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 6, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "N");
    CHECK(ind == 1);
  }

  // SQLGetData col 7
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 7, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "N");
    CHECK(ind == 1);
  }

  // SQLGetData col 8
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 8, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 9
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 9, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 10
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 10, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 11
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 11, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 12
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 12, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 13
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 13, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLFetch
  {
    SQLRETURN ret = SQLFetch(stmt0);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
  }

  // SQLGetData col 1
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 1, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "Taxable");
    CHECK(ind == 7);
  }

  // SQLGetData col 2
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 2, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "VARCHAR(1)");
    CHECK(ind == 10);
  }

  // SQLGetData col 3
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 3, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "COLUMN");
    CHECK(ind == 6);
  }

  // SQLGetData col 4
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 4, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "Y");
    CHECK(ind == 1);
  }

  // SQLGetData col 5
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 5, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 6
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 6, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "N");
    CHECK(ind == 1);
  }

  // SQLGetData col 7
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 7, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "N");
    CHECK(ind == 1);
  }

  // SQLGetData col 8
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 8, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 9
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 9, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 10
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 10, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 11
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 11, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 12
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 12, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 13
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 13, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLFetch
  {
    SQLRETURN ret = SQLFetch(stmt0);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
  }

  // SQLGetData col 1
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 1, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "TaxJurisdictionCode");
    CHECK(ind == 19);
  }

  // SQLGetData col 2
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 2, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "VARCHAR(15)");
    CHECK(ind == 11);
  }

  // SQLGetData col 3
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 3, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "COLUMN");
    CHECK(ind == 6);
  }

  // SQLGetData col 4
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 4, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "Y");
    CHECK(ind == 1);
  }

  // SQLGetData col 5
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 5, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 6
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 6, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "N");
    CHECK(ind == 1);
  }

  // SQLGetData col 7
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 7, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "N");
    CHECK(ind == 1);
  }

  // SQLGetData col 8
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 8, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 9
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 9, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 10
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 10, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 11
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 11, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 12
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 12, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 13
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 13, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLFetch
  {
    SQLRETURN ret = SQLFetch(stmt0);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
  }

  // SQLGetData col 1
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 1, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "DataSource");
    CHECK(ind == 10);
  }

  // SQLGetData col 2
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 2, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "VARCHAR(30)");
    CHECK(ind == 11);
  }

  // SQLGetData col 3
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 3, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "COLUMN");
    CHECK(ind == 6);
  }

  // SQLGetData col 4
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 4, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "Y");
    CHECK(ind == 1);
  }

  // SQLGetData col 5
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 5, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 6
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 6, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "N");
    CHECK(ind == 1);
  }

  // SQLGetData col 7
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 7, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "N");
    CHECK(ind == 1);
  }

  // SQLGetData col 8
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 8, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 9
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 9, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 10
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 10, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 11
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 11, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 12
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 12, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 13
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 13, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLFetch
  {
    SQLRETURN ret = SQLFetch(stmt0);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
  }

  // SQLGetData col 1
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 1, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "Phase");
    CHECK(ind == 5);
  }

  // SQLGetData col 2
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 2, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "VARCHAR(3)");
    CHECK(ind == 10);
  }

  // SQLGetData col 3
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 3, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "COLUMN");
    CHECK(ind == 6);
  }

  // SQLGetData col 4
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 4, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "Y");
    CHECK(ind == 1);
  }

  // SQLGetData col 5
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 5, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 6
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 6, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "N");
    CHECK(ind == 1);
  }

  // SQLGetData col 7
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 7, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "N");
    CHECK(ind == 1);
  }

  // SQLGetData col 8
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 8, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 9
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 9, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 10
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 10, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 11
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 11, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 12
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 12, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 13
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 13, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLFetch
  {
    SQLRETURN ret = SQLFetch(stmt0);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
  }

  // SQLGetData col 1
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 1, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "BpaState");
    CHECK(ind == 8);
  }

  // SQLGetData col 2
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 2, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "VARCHAR(3)");
    CHECK(ind == 10);
  }

  // SQLGetData col 3
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 3, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "COLUMN");
    CHECK(ind == 6);
  }

  // SQLGetData col 4
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 4, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "Y");
    CHECK(ind == 1);
  }

  // SQLGetData col 5
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 5, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 6
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 6, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "N");
    CHECK(ind == 1);
  }

  // SQLGetData col 7
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 7, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "N");
    CHECK(ind == 1);
  }

  // SQLGetData col 8
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 8, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 9
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 9, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 10
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 10, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 11
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 11, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 12
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 12, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 13
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 13, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLFetch
  {
    SQLRETURN ret = SQLFetch(stmt0);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
  }

  // SQLGetData col 1
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 1, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "BpaMetaPKey");
    CHECK(ind == 11);
  }

  // SQLGetData col 2
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 2, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "VARCHAR(16)");
    CHECK(ind == 11);
  }

  // SQLGetData col 3
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 3, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "COLUMN");
    CHECK(ind == 6);
  }

  // SQLGetData col 4
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 4, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "Y");
    CHECK(ind == 1);
  }

  // SQLGetData col 5
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 5, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 6
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 6, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "N");
    CHECK(ind == 1);
  }

  // SQLGetData col 7
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 7, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "N");
    CHECK(ind == 1);
  }

  // SQLGetData col 8
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 8, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 9
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 9, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 10
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 10, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 11
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 11, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 12
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 12, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 13
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 13, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLFetch
  {
    SQLRETURN ret = SQLFetch(stmt0);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
  }

  // SQLGetData col 1
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 1, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "BpaMetaHKey");
    CHECK(ind == 11);
  }

  // SQLGetData col 2
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 2, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "VARCHAR(64)");
    CHECK(ind == 11);
  }

  // SQLGetData col 3
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 3, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "COLUMN");
    CHECK(ind == 6);
  }

  // SQLGetData col 4
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 4, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "Y");
    CHECK(ind == 1);
  }

  // SQLGetData col 5
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 5, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 6
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 6, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "N");
    CHECK(ind == 1);
  }

  // SQLGetData col 7
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 7, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "N");
    CHECK(ind == 1);
  }

  // SQLGetData col 8
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 8, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 9
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 9, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 10
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 10, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 11
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 11, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 12
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 12, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 13
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 13, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLFetch
  {
    SQLRETURN ret = SQLFetch(stmt0);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
  }

  // SQLGetData col 1
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 1, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "Deleted");
    CHECK(ind == 7);
  }

  // SQLGetData col 2
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 2, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "VARCHAR(1)");
    CHECK(ind == 10);
  }

  // SQLGetData col 3
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 3, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "COLUMN");
    CHECK(ind == 6);
  }

  // SQLGetData col 4
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 4, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "Y");
    CHECK(ind == 1);
  }

  // SQLGetData col 5
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 5, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 6
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 6, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "N");
    CHECK(ind == 1);
  }

  // SQLGetData col 7
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 7, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "N");
    CHECK(ind == 1);
  }

  // SQLGetData col 8
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 8, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 9
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 9, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 10
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 10, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 11
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 11, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 12
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 12, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 13
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 13, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLFetch
  {
    SQLRETURN ret = SQLFetch(stmt0);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
  }

  // SQLGetData col 1
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 1, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "OneTimeCustomer");
    CHECK(ind == 15);
  }

  // SQLGetData col 2
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 2, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "VARCHAR(1)");
    CHECK(ind == 10);
  }

  // SQLGetData col 3
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 3, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "COLUMN");
    CHECK(ind == 6);
  }

  // SQLGetData col 4
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 4, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "Y");
    CHECK(ind == 1);
  }

  // SQLGetData col 5
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 5, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 6
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 6, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "N");
    CHECK(ind == 1);
  }

  // SQLGetData col 7
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 7, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "N");
    CHECK(ind == 1);
  }

  // SQLGetData col 8
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 8, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 9
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 9, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 10
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 10, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 11
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 11, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 12
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 12, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 13
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 13, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLFetch
  {
    SQLRETURN ret = SQLFetch(stmt0);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
  }

  // SQLGetData col 1
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 1, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "DeleteReservationDate");
    CHECK(ind == 21);
  }

  // SQLGetData col 2
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 2, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "TIMESTAMP_NTZ(6)");
    CHECK(ind == 16);
  }

  // SQLGetData col 3
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 3, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "COLUMN");
    CHECK(ind == 6);
  }

  // SQLGetData col 4
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 4, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "Y");
    CHECK(ind == 1);
  }

  // SQLGetData col 5
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 5, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 6
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 6, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "N");
    CHECK(ind == 1);
  }

  // SQLGetData col 7
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 7, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "N");
    CHECK(ind == 1);
  }

  // SQLGetData col 8
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 8, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 9
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 9, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 10
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 10, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 11
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 11, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 12
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 12, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 13
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 13, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLFetch
  {
    SQLRETURN ret = SQLFetch(stmt0);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
  }

  // SQLGetData col 1
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 1, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "MyPriceListSAP");
    CHECK(ind == 14);
  }

  // SQLGetData col 2
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 2, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "VARCHAR(10)");
    CHECK(ind == 11);
  }

  // SQLGetData col 3
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 3, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "COLUMN");
    CHECK(ind == 6);
  }

  // SQLGetData col 4
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 4, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "Y");
    CHECK(ind == 1);
  }

  // SQLGetData col 5
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 5, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 6
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 6, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "N");
    CHECK(ind == 1);
  }

  // SQLGetData col 7
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 7, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "N");
    CHECK(ind == 1);
  }

  // SQLGetData col 8
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 8, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 9
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 9, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 10
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 10, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 11
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 11, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 12
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 12, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 13
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 13, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLFetch
  {
    SQLRETURN ret = SQLFetch(stmt0);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
  }

  // SQLGetData col 1
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 1, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "MyCustomerType");
    CHECK(ind == 14);
  }

  // SQLGetData col 2
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 2, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "VARCHAR(1)");
    CHECK(ind == 10);
  }

  // SQLGetData col 3
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 3, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "COLUMN");
    CHECK(ind == 6);
  }

  // SQLGetData col 4
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 4, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "Y");
    CHECK(ind == 1);
  }

  // SQLGetData col 5
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 5, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 6
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 6, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "N");
    CHECK(ind == 1);
  }

  // SQLGetData col 7
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 7, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "N");
    CHECK(ind == 1);
  }

  // SQLGetData col 8
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 8, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 9
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 9, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 10
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 10, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 11
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 11, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 12
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 12, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 13
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 13, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLFetch
  {
    SQLRETURN ret = SQLFetch(stmt0);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
  }

  // SQLGetData col 1
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 1, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "MySecondaryContact");
    CHECK(ind == 18);
  }

  // SQLGetData col 2
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 2, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "VARCHAR(30)");
    CHECK(ind == 11);
  }

  // SQLGetData col 3
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 3, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "COLUMN");
    CHECK(ind == 6);
  }

  // SQLGetData col 4
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 4, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "Y");
    CHECK(ind == 1);
  }

  // SQLGetData col 5
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 5, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 6
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 6, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "N");
    CHECK(ind == 1);
  }

  // SQLGetData col 7
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 7, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "N");
    CHECK(ind == 1);
  }

  // SQLGetData col 8
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 8, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 9
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 9, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 10
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 10, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 11
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 11, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 12
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 12, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 13
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 13, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLFetch
  {
    SQLRETURN ret = SQLFetch(stmt0);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
  }

  // SQLGetData col 1
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 1, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "MyPriceList");
    CHECK(ind == 11);
  }

  // SQLGetData col 2
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 2, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "NUMBER(9,0)");
    CHECK(ind == 11);
  }

  // SQLGetData col 3
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 3, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "COLUMN");
    CHECK(ind == 6);
  }

  // SQLGetData col 4
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 4, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "Y");
    CHECK(ind == 1);
  }

  // SQLGetData col 5
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 5, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 6
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 6, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "N");
    CHECK(ind == 1);
  }

  // SQLGetData col 7
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 7, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "N");
    CHECK(ind == 1);
  }

  // SQLGetData col 8
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 8, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 9
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 9, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 10
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 10, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 11
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 11, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 12
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 12, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 13
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 13, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLFetch
  {
    SQLRETURN ret = SQLFetch(stmt0);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
  }

  // SQLGetData col 1
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 1, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "MyGeoCode");
    CHECK(ind == 9);
  }

  // SQLGetData col 2
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 2, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "VARCHAR(4)");
    CHECK(ind == 10);
  }

  // SQLGetData col 3
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 3, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "COLUMN");
    CHECK(ind == 6);
  }

  // SQLGetData col 4
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 4, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "Y");
    CHECK(ind == 1);
  }

  // SQLGetData col 5
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 5, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 6
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 6, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "N");
    CHECK(ind == 1);
  }

  // SQLGetData col 7
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 7, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "N");
    CHECK(ind == 1);
  }

  // SQLGetData col 8
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 8, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 9
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 9, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 10
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 10, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 11
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 11, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 12
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 12, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 13
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 13, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLFetch
  {
    SQLRETURN ret = SQLFetch(stmt0);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
  }

  // SQLGetData col 1
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 1, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "MyOrgCode1");
    CHECK(ind == 10);
  }

  // SQLGetData col 2
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 2, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "VARCHAR(4)");
    CHECK(ind == 10);
  }

  // SQLGetData col 3
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 3, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "COLUMN");
    CHECK(ind == 6);
  }

  // SQLGetData col 4
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 4, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "Y");
    CHECK(ind == 1);
  }

  // SQLGetData col 5
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 5, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 6
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 6, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "N");
    CHECK(ind == 1);
  }

  // SQLGetData col 7
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 7, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "N");
    CHECK(ind == 1);
  }

  // SQLGetData col 8
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 8, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 9
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 9, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 10
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 10, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 11
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 11, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 12
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 12, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 13
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 13, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLFetch
  {
    SQLRETURN ret = SQLFetch(stmt0);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
  }

  // SQLGetData col 1
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 1, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "MyOrgCode2");
    CHECK(ind == 10);
  }

  // SQLGetData col 2
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 2, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "VARCHAR(4)");
    CHECK(ind == 10);
  }

  // SQLGetData col 3
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 3, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "COLUMN");
    CHECK(ind == 6);
  }

  // SQLGetData col 4
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 4, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "Y");
    CHECK(ind == 1);
  }

  // SQLGetData col 5
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 5, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 6
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 6, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "N");
    CHECK(ind == 1);
  }

  // SQLGetData col 7
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 7, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "N");
    CHECK(ind == 1);
  }

  // SQLGetData col 8
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 8, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 9
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 9, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 10
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 10, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 11
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 11, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 12
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 12, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 13
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 13, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLFetch
  {
    SQLRETURN ret = SQLFetch(stmt0);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
  }

  // SQLGetData col 1
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 1, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "MyLiquorLicenseNo");
    CHECK(ind == 17);
  }

  // SQLGetData col 2
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 2, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "VARCHAR(30)");
    CHECK(ind == 11);
  }

  // SQLGetData col 3
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 3, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "COLUMN");
    CHECK(ind == 6);
  }

  // SQLGetData col 4
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 4, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "Y");
    CHECK(ind == 1);
  }

  // SQLGetData col 5
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 5, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 6
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 6, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "N");
    CHECK(ind == 1);
  }

  // SQLGetData col 7
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 7, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "N");
    CHECK(ind == 1);
  }

  // SQLGetData col 8
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 8, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 9
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 9, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 10
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 10, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 11
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 11, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 12
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 12, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 13
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 13, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLFetch
  {
    SQLRETURN ret = SQLFetch(stmt0);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
  }

  // SQLGetData col 1
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 1, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "MyHoldOrderIndicator");
    CHECK(ind == 20);
  }

  // SQLGetData col 2
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 2, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "VARCHAR(1)");
    CHECK(ind == 10);
  }

  // SQLGetData col 3
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 3, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "COLUMN");
    CHECK(ind == 6);
  }

  // SQLGetData col 4
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 4, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "Y");
    CHECK(ind == 1);
  }

  // SQLGetData col 5
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 5, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 6
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 6, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "N");
    CHECK(ind == 1);
  }

  // SQLGetData col 7
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 7, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "N");
    CHECK(ind == 1);
  }

  // SQLGetData col 8
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 8, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 9
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 9, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 10
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 10, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 11
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 11, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 12
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 12, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 13
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 13, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLFetch
  {
    SQLRETURN ret = SQLFetch(stmt0);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
  }

  // SQLGetData col 1
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 1, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "MyIsUrgent");
    CHECK(ind == 10);
  }

  // SQLGetData col 2
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 2, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "VARCHAR(1)");
    CHECK(ind == 10);
  }

  // SQLGetData col 3
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 3, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "COLUMN");
    CHECK(ind == 6);
  }

  // SQLGetData col 4
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 4, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "Y");
    CHECK(ind == 1);
  }

  // SQLGetData col 5
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 5, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 6
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 6, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "N");
    CHECK(ind == 1);
  }

  // SQLGetData col 7
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 7, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "N");
    CHECK(ind == 1);
  }

  // SQLGetData col 8
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 8, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 9
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 9, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 10
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 10, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 11
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 11, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 12
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 12, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 13
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 13, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLFetch
  {
    SQLRETURN ret = SQLFetch(stmt0);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
  }

  // SQLGetData col 1
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 1, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "MyCreateNow");
    CHECK(ind == 11);
  }

  // SQLGetData col 2
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 2, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "VARCHAR(1)");
    CHECK(ind == 10);
  }

  // SQLGetData col 3
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 3, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "COLUMN");
    CHECK(ind == 6);
  }

  // SQLGetData col 4
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 4, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "Y");
    CHECK(ind == 1);
  }

  // SQLGetData col 5
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 5, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 6
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 6, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "N");
    CHECK(ind == 1);
  }

  // SQLGetData col 7
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 7, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "N");
    CHECK(ind == 1);
  }

  // SQLGetData col 8
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 8, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 9
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 9, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 10
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 10, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 11
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 11, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 12
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 12, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 13
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 13, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLFetch
  {
    SQLRETURN ret = SQLFetch(stmt0);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
  }

  // SQLGetData col 1
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 1, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "MyOrderTakingCustomer");
    CHECK(ind == 21);
  }

  // SQLGetData col 2
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 2, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "VARCHAR(1)");
    CHECK(ind == 10);
  }

  // SQLGetData col 3
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 3, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "COLUMN");
    CHECK(ind == 6);
  }

  // SQLGetData col 4
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 4, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "Y");
    CHECK(ind == 1);
  }

  // SQLGetData col 5
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 5, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 6
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 6, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "N");
    CHECK(ind == 1);
  }

  // SQLGetData col 7
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 7, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "N");
    CHECK(ind == 1);
  }

  // SQLGetData col 8
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 8, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 9
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 9, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 10
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 10, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 11
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 11, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 12
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 12, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 13
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 13, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLFetch
  {
    SQLRETURN ret = SQLFetch(stmt0);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
  }

  // SQLGetData col 1
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 1, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "MyTaxClass1");
    CHECK(ind == 11);
  }

  // SQLGetData col 2
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 2, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "VARCHAR(1)");
    CHECK(ind == 10);
  }

  // SQLGetData col 3
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 3, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "COLUMN");
    CHECK(ind == 6);
  }

  // SQLGetData col 4
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 4, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "Y");
    CHECK(ind == 1);
  }

  // SQLGetData col 5
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 5, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 6
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 6, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "N");
    CHECK(ind == 1);
  }

  // SQLGetData col 7
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 7, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "N");
    CHECK(ind == 1);
  }

  // SQLGetData col 8
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 8, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 9
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 9, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 10
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 10, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 11
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 11, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 12
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 12, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 13
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 13, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLFetch
  {
    SQLRETURN ret = SQLFetch(stmt0);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
  }

  // SQLGetData col 1
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 1, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "MyTaxClass2");
    CHECK(ind == 11);
  }

  // SQLGetData col 2
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 2, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "VARCHAR(1)");
    CHECK(ind == 10);
  }

  // SQLGetData col 3
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 3, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "COLUMN");
    CHECK(ind == 6);
  }

  // SQLGetData col 4
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 4, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "Y");
    CHECK(ind == 1);
  }

  // SQLGetData col 5
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 5, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 6
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 6, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "N");
    CHECK(ind == 1);
  }

  // SQLGetData col 7
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 7, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "N");
    CHECK(ind == 1);
  }

  // SQLGetData col 8
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 8, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 9
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 9, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 10
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 10, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 11
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 11, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 12
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 12, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 13
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 13, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLFetch
  {
    SQLRETURN ret = SQLFetch(stmt0);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
  }

  // SQLGetData col 1
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 1, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "MyTaxClass3");
    CHECK(ind == 11);
  }

  // SQLGetData col 2
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 2, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "VARCHAR(1)");
    CHECK(ind == 10);
  }

  // SQLGetData col 3
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 3, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "COLUMN");
    CHECK(ind == 6);
  }

  // SQLGetData col 4
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 4, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "Y");
    CHECK(ind == 1);
  }

  // SQLGetData col 5
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 5, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 6
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 6, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "N");
    CHECK(ind == 1);
  }

  // SQLGetData col 7
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 7, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "N");
    CHECK(ind == 1);
  }

  // SQLGetData col 8
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 8, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 9
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 9, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 10
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 10, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 11
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 11, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 12
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 12, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 13
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 13, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLFetch
  {
    SQLRETURN ret = SQLFetch(stmt0);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
  }

  // SQLGetData col 1
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 1, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "MyTaxClass4");
    CHECK(ind == 11);
  }

  // SQLGetData col 2
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 2, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "VARCHAR(1)");
    CHECK(ind == 10);
  }

  // SQLGetData col 3
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 3, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "COLUMN");
    CHECK(ind == 6);
  }

  // SQLGetData col 4
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 4, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "Y");
    CHECK(ind == 1);
  }

  // SQLGetData col 5
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 5, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 6
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 6, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "N");
    CHECK(ind == 1);
  }

  // SQLGetData col 7
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 7, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "N");
    CHECK(ind == 1);
  }

  // SQLGetData col 8
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 8, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 9
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 9, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 10
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 10, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 11
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 11, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 12
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 12, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 13
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 13, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLFetch
  {
    SQLRETURN ret = SQLFetch(stmt0);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
  }

  // SQLGetData col 1
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 1, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "MyTaxClass5");
    CHECK(ind == 11);
  }

  // SQLGetData col 2
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 2, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "VARCHAR(1)");
    CHECK(ind == 10);
  }

  // SQLGetData col 3
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 3, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "COLUMN");
    CHECK(ind == 6);
  }

  // SQLGetData col 4
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 4, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "Y");
    CHECK(ind == 1);
  }

  // SQLGetData col 5
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 5, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 6
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 6, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "N");
    CHECK(ind == 1);
  }

  // SQLGetData col 7
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 7, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "N");
    CHECK(ind == 1);
  }

  // SQLGetData col 8
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 8, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 9
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 9, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 10
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 10, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 11
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 11, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 12
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 12, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 13
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 13, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLFetch
  {
    SQLRETURN ret = SQLFetch(stmt0);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
  }

  // SQLGetData col 1
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 1, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "MyConsumerTradeChannel");
    CHECK(ind == 22);
  }

  // SQLGetData col 2
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 2, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "VARCHAR(4)");
    CHECK(ind == 10);
  }

  // SQLGetData col 3
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 3, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "COLUMN");
    CHECK(ind == 6);
  }

  // SQLGetData col 4
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 4, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "Y");
    CHECK(ind == 1);
  }

  // SQLGetData col 5
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 5, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 6
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 6, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "N");
    CHECK(ind == 1);
  }

  // SQLGetData col 7
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 7, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "N");
    CHECK(ind == 1);
  }

  // SQLGetData col 8
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 8, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 9
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 9, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 10
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 10, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 11
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 11, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 12
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 12, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 13
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 13, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLFetch
  {
    SQLRETURN ret = SQLFetch(stmt0);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
  }

  // SQLGetData col 1
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 1, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "MyConsumerSubTradeChannel");
    CHECK(ind == 25);
  }

  // SQLGetData col 2
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 2, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "VARCHAR(4)");
    CHECK(ind == 10);
  }

  // SQLGetData col 3
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 3, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "COLUMN");
    CHECK(ind == 6);
  }

  // SQLGetData col 4
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 4, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "Y");
    CHECK(ind == 1);
  }

  // SQLGetData col 5
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 5, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 6
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 6, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "N");
    CHECK(ind == 1);
  }

  // SQLGetData col 7
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 7, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "N");
    CHECK(ind == 1);
  }

  // SQLGetData col 8
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 8, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 9
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 9, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 10
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 10, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 11
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 11, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 12
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 12, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 13
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 13, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLFetch
  {
    SQLRETURN ret = SQLFetch(stmt0);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
  }

  // SQLGetData col 1
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 1, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "MyDTC");
    CHECK(ind == 5);
  }

  // SQLGetData col 2
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 2, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "VARCHAR(3)");
    CHECK(ind == 10);
  }

  // SQLGetData col 3
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 3, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "COLUMN");
    CHECK(ind == 6);
  }

  // SQLGetData col 4
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 4, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "Y");
    CHECK(ind == 1);
  }

  // SQLGetData col 5
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 5, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 6
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 6, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "N");
    CHECK(ind == 1);
  }

  // SQLGetData col 7
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 7, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "N");
    CHECK(ind == 1);
  }

  // SQLGetData col 8
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 8, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 9
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 9, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 10
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 10, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 11
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 11, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 12
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 12, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 13
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 13, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLFetch
  {
    SQLRETURN ret = SQLFetch(stmt0);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
  }

  // SQLGetData col 1
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 1, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "MyTradingChain");
    CHECK(ind == 14);
  }

  // SQLGetData col 2
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 2, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "VARCHAR(30)");
    CHECK(ind == 11);
  }

  // SQLGetData col 3
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 3, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "COLUMN");
    CHECK(ind == 6);
  }

  // SQLGetData col 4
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 4, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "Y");
    CHECK(ind == 1);
  }

  // SQLGetData col 5
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 5, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 6
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 6, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "N");
    CHECK(ind == 1);
  }

  // SQLGetData col 7
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 7, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "N");
    CHECK(ind == 1);
  }

  // SQLGetData col 8
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 8, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 9
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 9, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 10
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 10, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 11
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 11, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 12
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 12, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 13
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 13, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLFetch
  {
    SQLRETURN ret = SQLFetch(stmt0);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
  }

  // SQLGetData col 1
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 1, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "MyRedScore");
    CHECK(ind == 10);
  }

  // SQLGetData col 2
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 2, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "NUMBER(15,6)");
    CHECK(ind == 12);
  }

  // SQLGetData col 3
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 3, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "COLUMN");
    CHECK(ind == 6);
  }

  // SQLGetData col 4
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 4, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "Y");
    CHECK(ind == 1);
  }

  // SQLGetData col 5
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 5, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 6
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 6, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "N");
    CHECK(ind == 1);
  }

  // SQLGetData col 7
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 7, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "N");
    CHECK(ind == 1);
  }

  // SQLGetData col 8
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 8, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 9
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 9, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 10
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 10, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 11
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 11, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 12
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 12, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 13
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 13, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLFetch
  {
    SQLRETURN ret = SQLFetch(stmt0);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
  }

  // SQLGetData col 1
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 1, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "MyCAC");
    CHECK(ind == 5);
  }

  // SQLGetData col 2
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 2, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "VARCHAR(4)");
    CHECK(ind == 10);
  }

  // SQLGetData col 3
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 3, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "COLUMN");
    CHECK(ind == 6);
  }

  // SQLGetData col 4
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 4, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "Y");
    CHECK(ind == 1);
  }

  // SQLGetData col 5
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 5, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 6
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 6, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "N");
    CHECK(ind == 1);
  }

  // SQLGetData col 7
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 7, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "N");
    CHECK(ind == 1);
  }

  // SQLGetData col 8
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 8, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 9
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 9, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 10
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 10, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 11
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 11, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 12
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 12, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 13
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 13, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLFetch
  {
    SQLRETURN ret = SQLFetch(stmt0);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
  }

  // SQLGetData col 1
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 1, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "MyTradeChannel");
    CHECK(ind == 14);
  }

  // SQLGetData col 2
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 2, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "VARCHAR(3)");
    CHECK(ind == 10);
  }

  // SQLGetData col 3
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 3, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "COLUMN");
    CHECK(ind == 6);
  }

  // SQLGetData col 4
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 4, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "Y");
    CHECK(ind == 1);
  }

  // SQLGetData col 5
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 5, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 6
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 6, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "N");
    CHECK(ind == 1);
  }

  // SQLGetData col 7
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 7, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "N");
    CHECK(ind == 1);
  }

  // SQLGetData col 8
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 8, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 9
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 9, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 10
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 10, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 11
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 11, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 12
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 12, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 13
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 13, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLFetch
  {
    SQLRETURN ret = SQLFetch(stmt0);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
  }

  // SQLGetData col 1
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 1, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "MySubTradeChannel");
    CHECK(ind == 17);
  }

  // SQLGetData col 2
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 2, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "VARCHAR(100)");
    CHECK(ind == 12);
  }

  // SQLGetData col 3
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 3, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "COLUMN");
    CHECK(ind == 6);
  }

  // SQLGetData col 4
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 4, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "Y");
    CHECK(ind == 1);
  }

  // SQLGetData col 5
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 5, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 6
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 6, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "N");
    CHECK(ind == 1);
  }

  // SQLGetData col 7
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 7, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "N");
    CHECK(ind == 1);
  }

  // SQLGetData col 8
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 8, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 9
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 9, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 10
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 10, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 11
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 11, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 12
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 12, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 13
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 13, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLFetch
  {
    SQLRETURN ret = SQLFetch(stmt0);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
  }

  // SQLGetData col 1
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 1, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "MySegment");
    CHECK(ind == 9);
  }

  // SQLGetData col 2
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 2, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "VARCHAR(3)");
    CHECK(ind == 10);
  }

  // SQLGetData col 3
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 3, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "COLUMN");
    CHECK(ind == 6);
  }

  // SQLGetData col 4
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 4, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "Y");
    CHECK(ind == 1);
  }

  // SQLGetData col 5
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 5, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 6
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 6, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "N");
    CHECK(ind == 1);
  }

  // SQLGetData col 7
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 7, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "N");
    CHECK(ind == 1);
  }

  // SQLGetData col 8
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 8, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 9
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 9, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 10
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 10, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 11
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 11, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 12
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 12, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 13
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 13, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLFetch
  {
    SQLRETURN ret = SQLFetch(stmt0);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
  }

  // SQLGetData col 1
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 1, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "MyPrimaryContact");
    CHECK(ind == 16);
  }

  // SQLGetData col 2
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 2, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "VARCHAR(30)");
    CHECK(ind == 11);
  }

  // SQLGetData col 3
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 3, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "COLUMN");
    CHECK(ind == 6);
  }

  // SQLGetData col 4
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 4, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "Y");
    CHECK(ind == 1);
  }

  // SQLGetData col 5
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 5, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 6
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 6, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "N");
    CHECK(ind == 1);
  }

  // SQLGetData col 7
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 7, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "N");
    CHECK(ind == 1);
  }

  // SQLGetData col 8
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 8, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 9
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 9, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 10
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 10, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 11
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 11, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 12
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 12, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 13
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 13, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLFetch
  {
    SQLRETURN ret = SQLFetch(stmt0);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
  }

  // SQLGetData col 1
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 1, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "MyApprovalStatus");
    CHECK(ind == 16);
  }

  // SQLGetData col 2
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 2, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "VARCHAR(10)");
    CHECK(ind == 11);
  }

  // SQLGetData col 3
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 3, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "COLUMN");
    CHECK(ind == 6);
  }

  // SQLGetData col 4
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 4, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "Y");
    CHECK(ind == 1);
  }

  // SQLGetData col 5
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 5, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 6
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 6, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "N");
    CHECK(ind == 1);
  }

  // SQLGetData col 7
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 7, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "N");
    CHECK(ind == 1);
  }

  // SQLGetData col 8
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 8, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 9
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 9, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 10
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 10, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 11
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 11, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 12
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 12, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 13
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 13, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLFetch
  {
    SQLRETURN ret = SQLFetch(stmt0);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
  }

  // SQLGetData col 1
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 1, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "MyDeliveryRecipient");
    CHECK(ind == 19);
  }

  // SQLGetData col 2
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 2, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "VARCHAR(1)");
    CHECK(ind == 10);
  }

  // SQLGetData col 3
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 3, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "COLUMN");
    CHECK(ind == 6);
  }

  // SQLGetData col 4
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 4, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "Y");
    CHECK(ind == 1);
  }

  // SQLGetData col 5
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 5, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 6
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 6, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "N");
    CHECK(ind == 1);
  }

  // SQLGetData col 7
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 7, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "N");
    CHECK(ind == 1);
  }

  // SQLGetData col 8
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 8, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 9
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 9, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 10
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 10, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 11
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 11, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 12
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 12, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 13
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 13, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLFetch
  {
    SQLRETURN ret = SQLFetch(stmt0);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
  }

  // SQLGetData col 1
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 1, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "MyCustomerLocation");
    CHECK(ind == 18);
  }

  // SQLGetData col 2
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 2, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "VARCHAR(4)");
    CHECK(ind == 10);
  }

  // SQLGetData col 3
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 3, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "COLUMN");
    CHECK(ind == 6);
  }

  // SQLGetData col 4
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 4, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "Y");
    CHECK(ind == 1);
  }

  // SQLGetData col 5
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 5, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 6
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 6, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "N");
    CHECK(ind == 1);
  }

  // SQLGetData col 7
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 7, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "N");
    CHECK(ind == 1);
  }

  // SQLGetData col 8
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 8, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 9
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 9, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 10
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 10, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 11
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 11, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 12
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 12, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 13
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 13, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLFetch
  {
    SQLRETURN ret = SQLFetch(stmt0);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
  }

  // SQLGetData col 1
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 1, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "MyMATPhysicalCases");
    CHECK(ind == 18);
  }

  // SQLGetData col 2
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 2, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "NUMBER(9,0)");
    CHECK(ind == 11);
  }

  // SQLGetData col 3
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 3, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "COLUMN");
    CHECK(ind == 6);
  }

  // SQLGetData col 4
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 4, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "Y");
    CHECK(ind == 1);
  }

  // SQLGetData col 5
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 5, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 6
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 6, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "N");
    CHECK(ind == 1);
  }

  // SQLGetData col 7
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 7, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "N");
    CHECK(ind == 1);
  }

  // SQLGetData col 8
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 8, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 9
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 9, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 10
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 10, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 11
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 11, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 12
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 12, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 13
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 13, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLFetch
  {
    SQLRETURN ret = SQLFetch(stmt0);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
  }

  // SQLGetData col 1
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 1, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "MyTradingChainInterfaced");
    CHECK(ind == 24);
  }

  // SQLGetData col 2
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 2, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "VARCHAR(30)");
    CHECK(ind == 11);
  }

  // SQLGetData col 3
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 3, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "COLUMN");
    CHECK(ind == 6);
  }

  // SQLGetData col 4
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 4, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "Y");
    CHECK(ind == 1);
  }

  // SQLGetData col 5
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 5, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 6
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 6, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "N");
    CHECK(ind == 1);
  }

  // SQLGetData col 7
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 7, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "N");
    CHECK(ind == 1);
  }

  // SQLGetData col 8
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 8, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 9
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 9, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 10
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 10, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 11
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 11, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 12
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 12, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 13
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 13, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLFetch
  {
    SQLRETURN ret = SQLFetch(stmt0);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
  }

  // SQLGetData col 1
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 1, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "MyRedSurveyDate");
    CHECK(ind == 15);
  }

  // SQLGetData col 2
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 2, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "TIMESTAMP_NTZ(6)");
    CHECK(ind == 16);
  }

  // SQLGetData col 3
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 3, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "COLUMN");
    CHECK(ind == 6);
  }

  // SQLGetData col 4
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 4, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "Y");
    CHECK(ind == 1);
  }

  // SQLGetData col 5
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 5, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 6
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 6, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "N");
    CHECK(ind == 1);
  }

  // SQLGetData col 7
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 7, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "N");
    CHECK(ind == 1);
  }

  // SQLGetData col 8
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 8, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 9
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 9, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 10
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 10, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 11
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 11, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 12
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 12, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 13
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 13, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLFetch
  {
    SQLRETURN ret = SQLFetch(stmt0);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
  }

  // SQLGetData col 1
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 1, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "MyRedIndicator");
    CHECK(ind == 14);
  }

  // SQLGetData col 2
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 2, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "VARCHAR(1)");
    CHECK(ind == 10);
  }

  // SQLGetData col 3
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 3, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "COLUMN");
    CHECK(ind == 6);
  }

  // SQLGetData col 4
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 4, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "Y");
    CHECK(ind == 1);
  }

  // SQLGetData col 5
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 5, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 6
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 6, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "N");
    CHECK(ind == 1);
  }

  // SQLGetData col 7
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 7, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "N");
    CHECK(ind == 1);
  }

  // SQLGetData col 8
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 8, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 9
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 9, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 10
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 10, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 11
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 11, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 12
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 12, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 13
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 13, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLFetch
  {
    SQLRETURN ret = SQLFetch(stmt0);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
  }

  // SQLGetData col 1
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 1, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "MyIsREDAudited");
    CHECK(ind == 14);
  }

  // SQLGetData col 2
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 2, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "VARCHAR(1)");
    CHECK(ind == 10);
  }

  // SQLGetData col 3
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 3, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "COLUMN");
    CHECK(ind == 6);
  }

  // SQLGetData col 4
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 4, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "Y");
    CHECK(ind == 1);
  }

  // SQLGetData col 5
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 5, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 6
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 6, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "N");
    CHECK(ind == 1);
  }

  // SQLGetData col 7
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 7, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "N");
    CHECK(ind == 1);
  }

  // SQLGetData col 8
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 8, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 9
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 9, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 10
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 10, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 11
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 11, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 12
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 12, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 13
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 13, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLFetch
  {
    SQLRETURN ret = SQLFetch(stmt0);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
  }

  // SQLGetData col 1
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 1, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "MySurveyorScore");
    CHECK(ind == 15);
  }

  // SQLGetData col 2
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 2, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "NUMBER(15,6)");
    CHECK(ind == 12);
  }

  // SQLGetData col 3
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 3, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "COLUMN");
    CHECK(ind == 6);
  }

  // SQLGetData col 4
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 4, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "Y");
    CHECK(ind == 1);
  }

  // SQLGetData col 5
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 5, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 6
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 6, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "N");
    CHECK(ind == 1);
  }

  // SQLGetData col 7
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 7, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "N");
    CHECK(ind == 1);
  }

  // SQLGetData col 8
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 8, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 9
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 9, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 10
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 10, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 11
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 11, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 12
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 12, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 13
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 13, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLFetch
  {
    SQLRETURN ret = SQLFetch(stmt0);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
  }

  // SQLGetData col 1
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 1, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "MySelfAssessmentScore");
    CHECK(ind == 21);
  }

  // SQLGetData col 2
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 2, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "NUMBER(15,6)");
    CHECK(ind == 12);
  }

  // SQLGetData col 3
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 3, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "COLUMN");
    CHECK(ind == 6);
  }

  // SQLGetData col 4
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 4, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "Y");
    CHECK(ind == 1);
  }

  // SQLGetData col 5
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 5, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 6
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 6, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "N");
    CHECK(ind == 1);
  }

  // SQLGetData col 7
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 7, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "N");
    CHECK(ind == 1);
  }

  // SQLGetData col 8
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 8, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 9
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 9, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 10
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 10, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 11
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 11, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 12
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 12, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 13
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 13, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLFetch
  {
    SQLRETURN ret = SQLFetch(stmt0);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
  }

  // SQLGetData col 1
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 1, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "MySelfAssessmentDate");
    CHECK(ind == 20);
  }

  // SQLGetData col 2
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 2, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "TIMESTAMP_NTZ(6)");
    CHECK(ind == 16);
  }

  // SQLGetData col 3
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 3, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "COLUMN");
    CHECK(ind == 6);
  }

  // SQLGetData col 4
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 4, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "Y");
    CHECK(ind == 1);
  }

  // SQLGetData col 5
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 5, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 6
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 6, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "N");
    CHECK(ind == 1);
  }

  // SQLGetData col 7
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 7, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "N");
    CHECK(ind == 1);
  }

  // SQLGetData col 8
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 8, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 9
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 9, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 10
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 10, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 11
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 11, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 12
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 12, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 13
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 13, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLFetch
  {
    SQLRETURN ret = SQLFetch(stmt0);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
  }

  // SQLGetData col 1
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 1, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "MyLastShelfShareDate");
    CHECK(ind == 20);
  }

  // SQLGetData col 2
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 2, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "TIMESTAMP_NTZ(6)");
    CHECK(ind == 16);
  }

  // SQLGetData col 3
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 3, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "COLUMN");
    CHECK(ind == 6);
  }

  // SQLGetData col 4
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 4, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "Y");
    CHECK(ind == 1);
  }

  // SQLGetData col 5
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 5, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 6
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 6, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "N");
    CHECK(ind == 1);
  }

  // SQLGetData col 7
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 7, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "N");
    CHECK(ind == 1);
  }

  // SQLGetData col 8
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 8, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 9
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 9, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 10
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 10, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 11
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 11, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 12
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 12, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 13
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 13, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLFetch
  {
    SQLRETURN ret = SQLFetch(stmt0);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
  }

  // SQLGetData col 1
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 1, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "MyLastShelfShareValue");
    CHECK(ind == 21);
  }

  // SQLGetData col 2
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 2, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "NUMBER(15,6)");
    CHECK(ind == 12);
  }

  // SQLGetData col 3
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 3, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "COLUMN");
    CHECK(ind == 6);
  }

  // SQLGetData col 4
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 4, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "Y");
    CHECK(ind == 1);
  }

  // SQLGetData col 5
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 5, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 6
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 6, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "N");
    CHECK(ind == 1);
  }

  // SQLGetData col 7
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 7, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "N");
    CHECK(ind == 1);
  }

  // SQLGetData col 8
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 8, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 9
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 9, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 10
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 10, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 11
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 11, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 12
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 12, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 13
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 13, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLFetch
  {
    SQLRETURN ret = SQLFetch(stmt0);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
  }

  // SQLGetData col 1
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 1, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "MyLastFridgeShareDate");
    CHECK(ind == 21);
  }

  // SQLGetData col 2
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 2, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "TIMESTAMP_NTZ(6)");
    CHECK(ind == 16);
  }

  // SQLGetData col 3
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 3, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "COLUMN");
    CHECK(ind == 6);
  }

  // SQLGetData col 4
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 4, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "Y");
    CHECK(ind == 1);
  }

  // SQLGetData col 5
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 5, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 6
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 6, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "N");
    CHECK(ind == 1);
  }

  // SQLGetData col 7
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 7, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "N");
    CHECK(ind == 1);
  }

  // SQLGetData col 8
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 8, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 9
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 9, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 10
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 10, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 11
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 11, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 12
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 12, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 13
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 13, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLFetch
  {
    SQLRETURN ret = SQLFetch(stmt0);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
  }

  // SQLGetData col 1
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 1, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "MyLastFridgeShareValue");
    CHECK(ind == 22);
  }

  // SQLGetData col 2
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 2, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "NUMBER(15,6)");
    CHECK(ind == 12);
  }

  // SQLGetData col 3
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 3, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "COLUMN");
    CHECK(ind == 6);
  }

  // SQLGetData col 4
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 4, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "Y");
    CHECK(ind == 1);
  }

  // SQLGetData col 5
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 5, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 6
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 6, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "N");
    CHECK(ind == 1);
  }

  // SQLGetData col 7
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 7, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "N");
    CHECK(ind == 1);
  }

  // SQLGetData col 8
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 8, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 9
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 9, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 10
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 10, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 11
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 11, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 12
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 12, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 13
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 13, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLFetch
  {
    SQLRETURN ret = SQLFetch(stmt0);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
  }

  // SQLGetData col 1
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 1, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "MyOperationalTradeChannel");
    CHECK(ind == 25);
  }

  // SQLGetData col 2
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 2, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "VARCHAR(3)");
    CHECK(ind == 10);
  }

  // SQLGetData col 3
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 3, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "COLUMN");
    CHECK(ind == 6);
  }

  // SQLGetData col 4
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 4, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "Y");
    CHECK(ind == 1);
  }

  // SQLGetData col 5
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 5, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 6
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 6, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "N");
    CHECK(ind == 1);
  }

  // SQLGetData col 7
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 7, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "N");
    CHECK(ind == 1);
  }

  // SQLGetData col 8
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 8, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 9
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 9, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 10
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 10, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 11
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 11, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 12
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 12, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 13
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 13, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLFetch
  {
    SQLRETURN ret = SQLFetch(stmt0);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
  }

  // SQLGetData col 1
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 1, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "MyOperationalMarketType");
    CHECK(ind == 23);
  }

  // SQLGetData col 2
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 2, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "VARCHAR(2)");
    CHECK(ind == 10);
  }

  // SQLGetData col 3
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 3, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "COLUMN");
    CHECK(ind == 6);
  }

  // SQLGetData col 4
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 4, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "Y");
    CHECK(ind == 1);
  }

  // SQLGetData col 5
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 5, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 6
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 6, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "N");
    CHECK(ind == 1);
  }

  // SQLGetData col 7
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 7, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "N");
    CHECK(ind == 1);
  }

  // SQLGetData col 8
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 8, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 9
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 9, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 10
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 10, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 11
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 11, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 12
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 12, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 13
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 13, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLFetch
  {
    SQLRETURN ret = SQLFetch(stmt0);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
  }

  // SQLGetData col 1
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 1, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "MyIsSMO");
    CHECK(ind == 7);
  }

  // SQLGetData col 2
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 2, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "VARCHAR(1)");
    CHECK(ind == 10);
  }

  // SQLGetData col 3
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 3, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "COLUMN");
    CHECK(ind == 6);
  }

  // SQLGetData col 4
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 4, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "Y");
    CHECK(ind == 1);
  }

  // SQLGetData col 5
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 5, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 6
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 6, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "N");
    CHECK(ind == 1);
  }

  // SQLGetData col 7
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 7, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "N");
    CHECK(ind == 1);
  }

  // SQLGetData col 8
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 8, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 9
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 9, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 10
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 10, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 11
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 11, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 12
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 12, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 13
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 13, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLFetch
  {
    SQLRETURN ret = SQLFetch(stmt0);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
  }

  // SQLGetData col 1
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 1, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "MyCreatedManuallyInCAS");
    CHECK(ind == 22);
  }

  // SQLGetData col 2
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 2, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "VARCHAR(1)");
    CHECK(ind == 10);
  }

  // SQLGetData col 3
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 3, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "COLUMN");
    CHECK(ind == 6);
  }

  // SQLGetData col 4
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 4, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "Y");
    CHECK(ind == 1);
  }

  // SQLGetData col 5
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 5, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 6
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 6, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "N");
    CHECK(ind == 1);
  }

  // SQLGetData col 7
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 7, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "N");
    CHECK(ind == 1);
  }

  // SQLGetData col 8
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 8, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 9
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 9, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 10
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 10, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 11
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 11, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 12
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 12, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 13
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 13, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLFetch
  {
    SQLRETURN ret = SQLFetch(stmt0);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
  }

  // SQLGetData col 1
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 1, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "MySuperTradeChannel");
    CHECK(ind == 19);
  }

  // SQLGetData col 2
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 2, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "VARCHAR(30)");
    CHECK(ind == 11);
  }

  // SQLGetData col 3
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 3, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "COLUMN");
    CHECK(ind == 6);
  }

  // SQLGetData col 4
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 4, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "Y");
    CHECK(ind == 1);
  }

  // SQLGetData col 5
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 5, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 6
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 6, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "N");
    CHECK(ind == 1);
  }

  // SQLGetData col 7
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 7, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "N");
    CHECK(ind == 1);
  }

  // SQLGetData col 8
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 8, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 9
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 9, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 10
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 10, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 11
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 11, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 12
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 12, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 13
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 13, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLFetch
  {
    SQLRETURN ret = SQLFetch(stmt0);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
  }

  // SQLGetData col 1
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 1, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "MySalesChannel");
    CHECK(ind == 14);
  }

  // SQLGetData col 2
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 2, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "VARCHAR(4)");
    CHECK(ind == 10);
  }

  // SQLGetData col 3
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 3, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "COLUMN");
    CHECK(ind == 6);
  }

  // SQLGetData col 4
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 4, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "Y");
    CHECK(ind == 1);
  }

  // SQLGetData col 5
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 5, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 6
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 6, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "N");
    CHECK(ind == 1);
  }

  // SQLGetData col 7
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 7, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "N");
    CHECK(ind == 1);
  }

  // SQLGetData col 8
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 8, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 9
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 9, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 10
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 10, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 11
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 11, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 12
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 12, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 13
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 13, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLFetch
  {
    SQLRETURN ret = SQLFetch(stmt0);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
  }

  // SQLGetData col 1
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 1, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "MyTerritoryID1");
    CHECK(ind == 14);
  }

  // SQLGetData col 2
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 2, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "VARCHAR(30)");
    CHECK(ind == 11);
  }

  // SQLGetData col 3
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 3, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "COLUMN");
    CHECK(ind == 6);
  }

  // SQLGetData col 4
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 4, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "Y");
    CHECK(ind == 1);
  }

  // SQLGetData col 5
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 5, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 6
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 6, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "N");
    CHECK(ind == 1);
  }

  // SQLGetData col 7
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 7, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "N");
    CHECK(ind == 1);
  }

  // SQLGetData col 8
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 8, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 9
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 9, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 10
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 10, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 11
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 11, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 12
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 12, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 13
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 13, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLFetch
  {
    SQLRETURN ret = SQLFetch(stmt0);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
  }

  // SQLGetData col 1
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 1, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "MyTerritoryID2");
    CHECK(ind == 14);
  }

  // SQLGetData col 2
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 2, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "VARCHAR(30)");
    CHECK(ind == 11);
  }

  // SQLGetData col 3
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 3, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "COLUMN");
    CHECK(ind == 6);
  }

  // SQLGetData col 4
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 4, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "Y");
    CHECK(ind == 1);
  }

  // SQLGetData col 5
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 5, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 6
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 6, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "N");
    CHECK(ind == 1);
  }

  // SQLGetData col 7
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 7, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "N");
    CHECK(ind == 1);
  }

  // SQLGetData col 8
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 8, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 9
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 9, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 10
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 10, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 11
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 11, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 12
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 12, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 13
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 13, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLFetch
  {
    SQLRETURN ret = SQLFetch(stmt0);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
  }

  // SQLGetData col 1
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 1, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "MyCallStartTime");
    CHECK(ind == 15);
  }

  // SQLGetData col 2
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 2, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "TIMESTAMP_NTZ(6)");
    CHECK(ind == 16);
  }

  // SQLGetData col 3
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 3, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "COLUMN");
    CHECK(ind == 6);
  }

  // SQLGetData col 4
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 4, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "Y");
    CHECK(ind == 1);
  }

  // SQLGetData col 5
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 5, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 6
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 6, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "N");
    CHECK(ind == 1);
  }

  // SQLGetData col 7
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 7, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "N");
    CHECK(ind == 1);
  }

  // SQLGetData col 8
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 8, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 9
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 9, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 10
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 10, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 11
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 11, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 12
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 12, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 13
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 13, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLFetch
  {
    SQLRETURN ret = SQLFetch(stmt0);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
  }

  // SQLGetData col 1
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 1, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "MyDerived");
    CHECK(ind == 9);
  }

  // SQLGetData col 2
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 2, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "VARCHAR(1)");
    CHECK(ind == 10);
  }

  // SQLGetData col 3
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 3, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "COLUMN");
    CHECK(ind == 6);
  }

  // SQLGetData col 4
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 4, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "Y");
    CHECK(ind == 1);
  }

  // SQLGetData col 5
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 5, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 6
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 6, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "N");
    CHECK(ind == 1);
  }

  // SQLGetData col 7
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 7, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "N");
    CHECK(ind == 1);
  }

  // SQLGetData col 8
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 8, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 9
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 9, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 10
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 10, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 11
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 11, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 12
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 12, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLGetData col 13
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 13, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(ind == SQL_NULL_DATA);
  }

  // SQLFetch
  {
    SQLRETURN ret = SQLFetch(stmt0);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsNoData());
  }

  // SQLMoreResults
  {
    SQLRETURN ret = SQLMoreResults(stmt0);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsNoData());
  }

  // SQLFreeHandle - SQLHSTMT
  {
    SQLRETURN ret = SQLFreeHandle(SQL_HANDLE_STMT, stmt0);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
  }

  // SQLDisconnect
  {
    SQLRETURN ret = SQLDisconnect(dbc0);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc0), OdbcMatchers::Succeeded());
  }

  // SQLFreeHandle - SQLHDBC
  {
    SQLRETURN ret = SQLFreeHandle(SQL_HANDLE_DBC, dbc0);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc0), OdbcMatchers::IsSuccess());
  }
}
