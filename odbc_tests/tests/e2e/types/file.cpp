#include <sql.h>
#include <sqlext.h>
#include <sqltypes.h>

#include <optional>
#include <string>

#include <catch2/catch_test_macros.hpp>

#include "Connection.hpp"
#include "SchemaFixtures.hpp"
#include "get_data.hpp"
#include "odbc_cast.hpp"
#include "odbc_matchers.hpp"

namespace {

constexpr const char* kFileExpr =
    "TO_FILE(PARSE_JSON('{\"RELATIVE_PATH\":\"some_new_file.jpeg\",\"STAGE\":\"@myStage\","
    "\"STAGE_FILE_URL\":\"some_new_file.jpeg\",\"SIZE\":123,\"ETAG\":\"xxx\","
    "\"CONTENT_TYPE\":\"image/jpeg\",\"LAST_MODIFIED\":\"2025-01-01\"}'))";

constexpr const char* kOtherFileExpr =
    "TO_FILE(PARSE_JSON('{\"RELATIVE_PATH\":\"quarterly_report.pdf\",\"STAGE\":\"@otherStage\","
    "\"STAGE_FILE_URL\":\"reports/quarterly_report.pdf\",\"SIZE\":45678,\"ETAG\":\"yyy\","
    "\"CONTENT_TYPE\":\"application/pdf\",\"LAST_MODIFIED\":\"2025-06-30\"}'))";

void require_sql_varchar(const StatementHandleWrapper& stmt, SQLUSMALLINT col) {
  SQLSMALLINT data_type = 0;
  SQLRETURN ret = SQLDescribeCol(stmt.getHandle(), col, nullptr, 0, nullptr, &data_type, nullptr, nullptr, nullptr);
  REQUIRE_ODBC(ret, stmt);
  REQUIRE(data_type == SQL_VARCHAR);
}

void require_file_doc(const std::string& json, const char* relative_path, const char* stage) {
  REQUIRE(json.find(relative_path) != std::string::npos);
  REQUIRE(json.find(stage) != std::string::npos);
}

}  // namespace

TEST_CASE_METHOD(ConnSchemaFixture, "should cast FILE values to appropriate type", "[file]") {
  // Given Snowflake client is logged in

  // When A FILE column is populated via TO_FILE and queried
  conn.execute(std::string("CREATE TEMPORARY TABLE file_cast AS SELECT ") + kFileExpr + " AS file_col");
  const auto stmt = conn.execute_fetch("SELECT file_col FROM file_cast");

  // Then the FILE column should be returned as appropriate type with the expected JSON document
  require_sql_varchar(stmt, 1);
  require_file_doc(get_data<SQL_C_CHAR>(stmt, 1), "some_new_file.jpeg", "@myStage");
}

TEST_CASE("should select a FILE value built by TO_FILE without a table", "[file]") {
  // Given Snowflake client is logged in
  Connection conn;

  // When Query "SELECT TO_FILE(PARSE_JSON('{"RELATIVE_PATH": "some_new_file.jpeg", ...}'))" is executed
  const auto stmt = conn.execute_fetch(std::string("SELECT ") + kFileExpr);

  // Then the result should contain the expected FILE JSON document
  require_file_doc(get_data<SQL_C_CHAR>(stmt, 1), "some_new_file.jpeg", "@myStage");
}

TEST_CASE("should handle NULL FILE values from literals", "[file]") {
  // Given Snowflake client is logged in
  Connection conn;

  // When Query "SELECT TO_FILE(PARSE_JSON('...')), TO_FILE(NULL)" is executed
  const auto stmt = conn.execute_fetch(std::string("SELECT ") + kFileExpr + ", TO_FILE(NULL)");

  // Then the result should contain the expected FILE JSON document and NULL
  require_file_doc(get_data<SQL_C_CHAR>(stmt, 1), "some_new_file.jpeg", "@myStage");
  REQUIRE(get_data_optional<SQL_C_CHAR>(stmt, 2) == std::nullopt);
}

TEST_CASE_METHOD(ConnSchemaFixture, "should select FILE values from table", "[file]") {
  // Given Snowflake client is logged in

  // And A temporary table with an ID and a FILE column is created
  conn.execute("CREATE TEMPORARY TABLE file_table (id INT, file_col FILE)");

  // And The table is populated with two different FILE values
  conn.execute(std::string("INSERT INTO file_table SELECT 1, ") + kFileExpr + " UNION ALL SELECT 2, " + kOtherFileExpr);

  // When Query "SELECT * FROM {table} ORDER BY ID" is executed
  const auto stmt = conn.createStatement();
  SQLRETURN ret = SQLExecDirect(stmt.getHandle(), sqlchar("SELECT * FROM file_table ORDER BY id"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);

  // Then the result should contain the inserted FILE JSON documents in order
  ret = SQLFetch(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);
  REQUIRE(get_data<SQL_C_SLONG>(stmt, 1) == 1);
  require_file_doc(get_data<SQL_C_CHAR>(stmt, 2), "some_new_file.jpeg", "@myStage");
  ret = SQLFetch(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);
  REQUIRE(get_data<SQL_C_SLONG>(stmt, 1) == 2);
  require_file_doc(get_data<SQL_C_CHAR>(stmt, 2), "quarterly_report.pdf", "@otherStage");
  ret = SQLFetch(stmt.getHandle());
  REQUIRE(ret == SQL_NO_DATA);
}

TEST_CASE_METHOD(ConnSchemaFixture, "should handle NULL FILE values from table", "[file]") {
  // Given Snowflake client is logged in

  // And A temporary table with an ID and a FILE column is created
  conn.execute("CREATE TEMPORARY TABLE file_null (id INT, file_col FILE)");

  // And The table is populated with a FILE value, a NULL and another FILE value
  conn.execute(std::string("INSERT INTO file_null SELECT 1, ") + kFileExpr +
               " UNION ALL SELECT 2, TO_FILE(NULL) UNION ALL SELECT 3, " + kOtherFileExpr);

  // When Query "SELECT * FROM {table} ORDER BY ID" is executed
  const auto stmt = conn.createStatement();
  SQLRETURN ret = SQLExecDirect(stmt.getHandle(), sqlchar("SELECT * FROM file_null ORDER BY id"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);

  // Then the result should contain the inserted FILE JSON documents and NULL in order
  ret = SQLFetch(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);
  require_file_doc(get_data<SQL_C_CHAR>(stmt, 2), "some_new_file.jpeg", "@myStage");
  ret = SQLFetch(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);
  REQUIRE(get_data_optional<SQL_C_CHAR>(stmt, 2) == std::nullopt);
  ret = SQLFetch(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);
  require_file_doc(get_data<SQL_C_CHAR>(stmt, 2), "quarterly_report.pdf", "@otherStage");
  ret = SQLFetch(stmt.getHandle());
  REQUIRE(ret == SQL_NO_DATA);
}

TEST_CASE("should download FILE data in multiple chunks", "[file]") {
  // Given Snowflake client is logged in
  Connection conn;

  // When Query generating 20000 FILE values is executed
  const auto stmt = conn.createStatement();
  const std::string sql = std::string("SELECT id, ") + kFileExpr +
                          " FROM (SELECT (ROW_NUMBER() OVER (ORDER BY seq8()) - 1) AS id "
                          "FROM TABLE(GENERATOR(ROWCOUNT => 20000))) ORDER BY id";
  SQLRETURN ret = SQLExecDirect(stmt.getHandle(), sqlchar(sql.c_str()), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);

  // Then All 20000 rows should be fetched with the expected FILE JSON documents
  int row_count = 0;
  while (true) {
    ret = SQLFetch(stmt.getHandle());
    if (ret == SQL_NO_DATA) {
      break;
    }
    REQUIRE_ODBC(ret, stmt);
    INFO("row=" << row_count);
    REQUIRE(get_data<SQL_C_SLONG>(stmt, 1) == row_count);
    require_file_doc(get_data<SQL_C_CHAR>(stmt, 2), "some_new_file.jpeg", "@myStage");
    row_count++;
  }
  REQUIRE(row_count == 20000);
}
