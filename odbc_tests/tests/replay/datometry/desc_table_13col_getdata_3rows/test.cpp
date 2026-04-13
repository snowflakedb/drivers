#include <vector>

#include <catch2/catch_test_macros.hpp>

#include "ODBCConfig.hpp"
#include "compatibility.hpp"
#include "odbc_cast.hpp"
#include "odbc_matchers.hpp"

TEST_CASE("Replay: desc_table_13col_getdata_3rows", "[dtm]") {
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
    SQLRETURN ret = SQLExecDirect(stmt0, sqlchar("desc table \"DTMREPLAYTESTDB\".\"PUBLIC\".\"cv_test1\""), SQL_NTS);
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
    CHECK(std::string(buf.data()) == "i");
    CHECK(ind == 1);
  }

  // SQLGetData col 2
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 2, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "NUMBER(38,0)");
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
    CHECK(std::string(buf.data()) == "j");
    CHECK(ind == 1);
  }

  // SQLGetData col 2
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 2, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "NUMBER(38,0)");
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
    CHECK(std::string(buf.data()) == "k");
    CHECK(ind == 1);
  }

  // SQLGetData col 2
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 2, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "NUMBER(38,0)");
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
