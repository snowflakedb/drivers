#include <vector>

#include <catch2/catch_test_macros.hpp>

#include "ODBCConfig.hpp"
#include "compatibility.hpp"
#include "odbc_cast.hpp"
#include "odbc_matchers.hpp"

TEST_CASE("Replay: exec_direct_getdata_13col_111rows", "[dtm]") {
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
    SQLRETURN ret = SQLExecDirect(stmt0, sqlchar("desc table \"DTMREPLAYTESTDB\".\"PUBLIC\".\"sss1\""), SQL_NTS);
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
    CHECK(std::string(buf.data()) == "extract_source_log_sys");
    CHECK(ind == 22);
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
    CHECK(std::string(buf.data()) == "orig_record_number");
    CHECK(ind == 18);
  }

  // SQLGetData col 2
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 2, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "VARCHAR(18)");
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
    CHECK(std::string(buf.data()) == "rownum");
    CHECK(ind == 6);
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
    CHECK(std::string(buf.data()) == "subrec");
    CHECK(ind == 6);
  }

  // SQLGetData col 2
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 2, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "VARCHAR(14)");
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
    CHECK(std::string(buf.data()) == "new_record_number");
    CHECK(ind == 17);
  }

  // SQLGetData col 2
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 2, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "VARCHAR(18)");
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
    CHECK(std::string(buf.data()) == "age_bucket");
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
    CHECK(std::string(buf.data()) == "age_in_days");
    CHECK(ind == 11);
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
    CHECK(std::string(buf.data()) == "applicant");
    CHECK(ind == 9);
  }

  // SQLGetData col 2
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 2, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "VARCHAR(25)");
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
    CHECK(std::string(buf.data()) == "base_unit_of_measure_type");
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
    CHECK(std::string(buf.data()) == "company_code");
    CHECK(ind == 12);
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
    CHECK(std::string(buf.data()) == "currency_code");
    CHECK(ind == 13);
  }

  // SQLGetData col 2
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 2, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "VARCHAR(5)");
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
    CHECK(std::string(buf.data()) == "customer_code_2digit");
    CHECK(ind == 20);
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
    CHECK(std::string(buf.data()) == "customer_group");
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
    CHECK(std::string(buf.data()) == "customer_po_number");
    CHECK(ind == 18);
  }

  // SQLGetData col 2
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 2, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "VARCHAR(35)");
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
    CHECK(std::string(buf.data()) == "document_date_vbak");
    CHECK(ind == 18);
  }

  // SQLGetData col 2
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 2, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "DATE");
    CHECK(ind == 4);
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
    CHECK(std::string(buf.data()) == "document_line_number");
    CHECK(ind == 20);
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
    CHECK(std::string(buf.data()) == "document_number");
    CHECK(ind == 15);
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
    CHECK(std::string(buf.data()) == "document_type");
    CHECK(ind == 13);
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
    CHECK(std::string(buf.data()) == "dw_customer_code");
    CHECK(ind == 16);
  }

  // SQLGetData col 2
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 2, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "VARCHAR(20)");
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
    CHECK(std::string(buf.data()) == "dw_customer_type");
    CHECK(ind == 16);
  }

  // SQLGetData col 2
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 2, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "VARCHAR(20)");
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
    CHECK(std::string(buf.data()) == "old_product_code");
    CHECK(ind == 16);
  }

  // SQLGetData col 2
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 2, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "VARCHAR(20)");
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
    CHECK(std::string(buf.data()) == "old_product_type");
    CHECK(ind == 16);
  }

  // SQLGetData col 2
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 2, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "VARCHAR(20)");
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
    CHECK(std::string(buf.data()) == "new_product_code");
    CHECK(ind == 16);
  }

  // SQLGetData col 2
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 2, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "VARCHAR(20)");
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
    CHECK(std::string(buf.data()) == "new_product_type");
    CHECK(ind == 16);
  }

  // SQLGetData col 2
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 2, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "VARCHAR(20)");
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
    CHECK(std::string(buf.data()) == "final_bill_flag");
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
    CHECK(std::string(buf.data()) == "fiscal_year");
    CHECK(ind == 11);
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
    CHECK(std::string(buf.data()) == "forecast_finish_date");
    CHECK(ind == 20);
  }

  // SQLGetData col 2
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 2, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "DATE");
    CHECK(ind == 4);
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
    CHECK(std::string(buf.data()) == "forecast_start_date");
    CHECK(ind == 19);
  }

  // SQLGetData col 2
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 2, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "DATE");
    CHECK(ind == 4);
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
    CHECK(std::string(buf.data()) == "formatted_wbs_element");
    CHECK(ind == 21);
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
    CHECK(std::string(buf.data()) == "gl_account_number");
    CHECK(ind == 17);
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
    CHECK(std::string(buf.data()) == "rate");
    CHECK(ind == 4);
  }

  // SQLGetData col 2
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 2, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "FLOAT");
    CHECK(ind == 5);
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
    CHECK(std::string(buf.data()) == "new_group_currency_amt");
    CHECK(ind == 22);
  }

  // SQLGetData col 2
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 2, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "NUMBER(18,3)");
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
    CHECK(std::string(buf.data()) == "orig_group_currency_amt");
    CHECK(ind == 23);
  }

  // SQLGetData col 2
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 2, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "NUMBER(18,3)");
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
    CHECK(std::string(buf.data()) == "hrc_code");
    CHECK(ind == 8);
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
    CHECK(std::string(buf.data()) == "installation_complete_flag");
    CHECK(ind == 26);
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
    CHECK(std::string(buf.data()) == "inventory_location");
    CHECK(ind == 18);
  }

  // SQLGetData col 2
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 2, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "VARCHAR(35)");
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
    CHECK(std::string(buf.data()) == "investment_category");
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
    CHECK(std::string(buf.data()) == "item_committed_on_job_date");
    CHECK(ind == 26);
  }

  // SQLGetData col 2
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 2, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "DATE");
    CHECK(ind == 4);
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
    CHECK(std::string(buf.data()) == "item_delivery_block");
    CHECK(ind == 19);
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
    CHECK(std::string(buf.data()) == "orig_local_currency_amt");
    CHECK(ind == 23);
  }

  // SQLGetData col 2
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 2, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "NUMBER(18,3)");
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
    CHECK(std::string(buf.data()) == "new_local_currency_amt");
    CHECK(ind == 22);
  }

  // SQLGetData col 2
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 2, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "NUMBER(18,3)");
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
    CHECK(std::string(buf.data()) == "material_acct_assign_group");
    CHECK(ind == 26);
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
    CHECK(std::string(buf.data()) == "material_code");
    CHECK(ind == 13);
  }

  // SQLGetData col 2
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 2, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "VARCHAR(18)");
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
    CHECK(std::string(buf.data()) == "material_description");
    CHECK(ind == 20);
  }

  // SQLGetData col 2
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 2, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "VARCHAR(40)");
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
    CHECK(std::string(buf.data()) == "material_document_item");
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
    CHECK(std::string(buf.data()) == "material_document_number");
    CHECK(ind == 24);
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
    CHECK(std::string(buf.data()) == "material_document_year");
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
    CHECK(std::string(buf.data()) == "material_to_cust_cmpl_date");
    CHECK(ind == 26);
  }

  // SQLGetData col 2
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 2, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "DATE");
    CHECK(ind == 4);
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
    CHECK(std::string(buf.data()) == "material_to_lsc_cmpl_date");
    CHECK(ind == 25);
  }

  // SQLGetData col 2
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 2, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "DATE");
    CHECK(ind == 4);
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
    CHECK(std::string(buf.data()) == "merchandise_class");
    CHECK(ind == 17);
  }

  // SQLGetData col 2
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 2, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "VARCHAR(18)");
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
    CHECK(std::string(buf.data()) == "movement_type");
    CHECK(ind == 13);
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
    CHECK(std::string(buf.data()) == "order_actual_eng_cmpl_date");
    CHECK(ind == 26);
  }

  // SQLGetData col 2
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 2, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "DATE");
    CHECK(ind == 4);
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
    CHECK(std::string(buf.data()) == "order_actual_eng_start_date");
    CHECK(ind == 27);
  }

  // SQLGetData col 2
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 2, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "DATE");
    CHECK(ind == 4);
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
    CHECK(std::string(buf.data()) == "order_actual_inst_cmpl_date");
    CHECK(ind == 27);
  }

  // SQLGetData col 2
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 2, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "DATE");
    CHECK(ind == 4);
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
    CHECK(std::string(buf.data()) == "order_actual_inst_start_date");
    CHECK(ind == 28);
  }

  // SQLGetData col 2
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 2, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "DATE");
    CHECK(ind == 4);
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
    CHECK(std::string(buf.data()) == "order_change_note");
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
    CHECK(std::string(buf.data()) == "order_cust_request_cmpl_dt");
    CHECK(ind == 26);
  }

  // SQLGetData col 2
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 2, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "DATE");
    CHECK(ind == 4);
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
    CHECK(std::string(buf.data()) == "order_cust_request_onjob_dt");
    CHECK(ind == 27);
  }

  // SQLGetData col 2
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 2, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "DATE");
    CHECK(ind == 4);
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
    CHECK(std::string(buf.data()) == "order_cust_request_ship_dt");
    CHECK(ind == 26);
  }

  // SQLGetData col 2
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 2, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "DATE");
    CHECK(ind == 4);
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
    CHECK(std::string(buf.data()) == "order_delivery_block");
    CHECK(ind == 20);
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
    CHECK(std::string(buf.data()) == "order_final_bill_date");
    CHECK(ind == 21);
  }

  // SQLGetData col 2
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 2, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "DATE");
    CHECK(ind == 4);
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
    CHECK(std::string(buf.data()) == "order_item_note");
    CHECK(ind == 15);
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
    CHECK(std::string(buf.data()) == "order_main_ship_date");
    CHECK(ind == 20);
  }

  // SQLGetData col 2
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 2, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "DATE");
    CHECK(ind == 4);
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
    CHECK(std::string(buf.data()) == "order_sched_eng_cmpl_date");
    CHECK(ind == 25);
  }

  // SQLGetData col 2
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 2, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "DATE");
    CHECK(ind == 4);
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
    CHECK(std::string(buf.data()) == "order_sched_eng_start_date");
    CHECK(ind == 26);
  }

  // SQLGetData col 2
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 2, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "DATE");
    CHECK(ind == 4);
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
    CHECK(std::string(buf.data()) == "order_sched_inst_cmpl_date");
    CHECK(ind == 26);
  }

  // SQLGetData col 2
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 2, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "DATE");
    CHECK(ind == 4);
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
    CHECK(std::string(buf.data()) == "order_sched_inst_start_date");
    CHECK(ind == 27);
  }

  // SQLGetData col 2
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 2, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "DATE");
    CHECK(ind == 4);
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
    CHECK(std::string(buf.data()) == "order_sched_onjob_date");
    CHECK(ind == 22);
  }

  // SQLGetData col 2
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 2, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "DATE");
    CHECK(ind == 4);
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
    CHECK(std::string(buf.data()) == "order_ship_complete_date");
    CHECK(ind == 24);
  }

  // SQLGetData col 2
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 2, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "DATE");
    CHECK(ind == 4);
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
    CHECK(std::string(buf.data()) == "orig_fiscal_year");
    CHECK(ind == 16);
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
    CHECK(std::string(buf.data()) == "orig_posting_period");
    CHECK(ind == 19);
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
    CHECK(std::string(buf.data()) == "planned_end_date");
    CHECK(ind == 16);
  }

  // SQLGetData col 2
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 2, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "DATE");
    CHECK(ind == 4);
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
    CHECK(std::string(buf.data()) == "planned_start_date");
    CHECK(ind == 18);
  }

  // SQLGetData col 2
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 2, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "DATE");
    CHECK(ind == 4);
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
    CHECK(std::string(buf.data()) == "plant_code");
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
    CHECK(std::string(buf.data()) == "po_document_number");
    CHECK(ind == 18);
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
    CHECK(std::string(buf.data()) == "po_line_number");
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
    CHECK(std::string(buf.data()) == "poc_flag");
    CHECK(ind == 8);
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
    CHECK(std::string(buf.data()) == "posting_date");
    CHECK(ind == 12);
  }

  // SQLGetData col 2
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 2, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "DATE");
    CHECK(ind == 4);
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
    CHECK(std::string(buf.data()) == "posting_period");
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
    CHECK(std::string(buf.data()) == "profile_identifier");
    CHECK(ind == 18);
  }

  // SQLGetData col 2
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 2, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "VARCHAR(7)");
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
    CHECK(std::string(buf.data()) == "profit_center_code");
    CHECK(ind == 18);
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
    CHECK(std::string(buf.data()) == "program_field");
    CHECK(ind == 13);
  }

  // SQLGetData col 2
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 2, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "VARCHAR(20)");
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
    CHECK(std::string(buf.data()) == "project_category");
    CHECK(ind == 16);
  }

  // SQLGetData col 2
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 2, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "VARCHAR(20)");
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
    CHECK(std::string(buf.data()) == "project_category_key");
    CHECK(ind == 20);
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
    CHECK(std::string(buf.data()) == "project_group");
    CHECK(ind == 13);
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
    CHECK(std::string(buf.data()) == "project_manager_id");
    CHECK(ind == 18);
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
    CHECK(std::string(buf.data()) == "project_manager_name");
    CHECK(ind == 20);
  }

  // SQLGetData col 2
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 2, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "VARCHAR(25)");
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
    CHECK(std::string(buf.data()) == "rejection_reason");
    CHECK(ind == 16);
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
    CHECK(std::string(buf.data()) == "requested_delvry_date");
    CHECK(ind == 21);
  }

  // SQLGetData col 2
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 2, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "DATE");
    CHECK(ind == 4);
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
    CHECK(std::string(buf.data()) == "ret_sales_order_item_number");
    CHECK(ind == 27);
  }

  // SQLGetData col 2
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 2, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "VARCHAR(6)");
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
    CHECK(std::string(buf.data()) == "ret_sales_order_number");
    CHECK(ind == 22);
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
    CHECK(std::string(buf.data()) == "sales_comments");
    CHECK(ind == 14);
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
    CHECK(std::string(buf.data()) == "sales_document_item_number");
    CHECK(ind == 26);
  }

  // SQLGetData col 2
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 2, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "VARCHAR(6)");
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
    CHECK(std::string(buf.data()) == "sales_document_number");
    CHECK(ind == 21);
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
    CHECK(std::string(buf.data()) == "sales_document_type");
    CHECK(ind == 19);
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
    CHECK(std::string(buf.data()) == "sales_organization_code");
    CHECK(ind == 23);
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
    CHECK(std::string(buf.data()) == "ship_complete_flag");
    CHECK(ind == 18);
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
    CHECK(std::string(buf.data()) == "sold_to_customer_number");
    CHECK(ind == 23);
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
    CHECK(std::string(buf.data()) == "storage_location");
    CHECK(ind == 16);
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
    CHECK(std::string(buf.data()) == "system_code");
    CHECK(ind == 11);
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
    CHECK(std::string(buf.data()) == "orig_total_movement_qty");
    CHECK(ind == 23);
  }

  // SQLGetData col 2
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 2, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "NUMBER(18,3)");
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
    CHECK(std::string(buf.data()) == "new_total_movement_qty");
    CHECK(ind == 22);
  }

  // SQLGetData col 2
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 2, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "NUMBER(18,3)");
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
    CHECK(std::string(buf.data()) == "trading_partner_id");
    CHECK(ind == 18);
  }

  // SQLGetData col 2
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 2, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "VARCHAR(6)");
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
    CHECK(std::string(buf.data()) == "vendor_account_number");
    CHECK(ind == 21);
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
    CHECK(std::string(buf.data()) == "vendor_name");
    CHECK(ind == 11);
  }

  // SQLGetData col 2
  {
    std::vector<char> buf(4097, 0);
    SQLLEN ind = 0;
    SQLRETURN ret = SQLGetData(stmt0, 2, SQL_CHAR, buf.data(), 4096, &ind);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf.data()) == "VARCHAR(35)");
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
    CHECK(std::string(buf.data()) == "wbs_element_number");
    CHECK(ind == 18);
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
    CHECK(std::string(buf.data()) == "wbs_id");
    CHECK(ind == 6);
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
    CHECK(std::string(buf.data()) == "wbs_status");
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
    CHECK(std::string(buf.data()) == "wbs_status_desc");
    CHECK(ind == 15);
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
    CHECK(std::string(buf.data()) == "wbs_status_type");
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
    CHECK(std::string(buf.data()) == "datetime_entered");
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
