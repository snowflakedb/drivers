#include <sql.h>
#include <sqlext.h>
#include <sqltypes.h>

#include <cstring>
#include <optional>
#include <string>

#include <catch2/catch_test_macros.hpp>

#include "Connection.hpp"
#include "SchemaFixtures.hpp"
#include "WideString.hpp"
#include "compatibility.hpp"
#include "conversion_checks.hpp"
#include "geo_test_helpers.hpp"
#include "get_data.hpp"
#include "odbc_cast.hpp"
#include "odbc_matchers.hpp"
#include "sf_odbc.h"

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

void require_type_name(const StatementHandleWrapper& stmt, SQLUSMALLINT col, const char* expected) {
  SQLCHAR type_name[128] = {};
  SQLSMALLINT name_len = 0;
  SQLRETURN ret =
      SQLColAttribute(stmt.getHandle(), col, SQL_DESC_TYPE_NAME, type_name, sizeof(type_name), &name_len, nullptr);
  REQUIRE_ODBC(ret, stmt);
  REQUIRE(std::string(reinterpret_cast<char*>(type_name), static_cast<std::size_t>(name_len)) == expected);
}

void require_varchar_descriptor(const StatementHandleWrapper& stmt, SQLUSMALLINT col) {
  require_sql_varchar(stmt, col);

  SQLLEN concise_type = 0;
  SQLRETURN ret = SQLColAttribute(stmt.getHandle(), col, SQL_DESC_CONCISE_TYPE, nullptr, 0, nullptr, &concise_type);
  REQUIRE_ODBC(ret, stmt);
  REQUIRE(concise_type == static_cast<SQLLEN>(SQL_VARCHAR));

  SQLLEN desc_type = 0;
  ret = SQLColAttribute(stmt.getHandle(), col, SQL_DESC_TYPE, nullptr, 0, nullptr, &desc_type);
  REQUIRE_ODBC(ret, stmt);
  REQUIRE(desc_type == static_cast<SQLLEN>(SQL_VARCHAR));
}

void require_file_doc(const std::string& json, const char* relative_path, const char* stage) {
  REQUIRE(json.find(relative_path) != std::string::npos);
  REQUIRE(json.find(stage) != std::string::npos);
}

void require_wide_file_doc(const std::u32string& json, const char* relative_path, const char* stage) {
  const auto contains = [&json](const char* needle) {
    std::u32string wide;
    wide.reserve(std::strlen(needle));
    for (const unsigned char* byte = reinterpret_cast<const unsigned char*>(needle); *byte != 0; ++byte) {
      wide.push_back(static_cast<char32_t>(*byte));
    }
    return json.find(wide) != std::u32string::npos;
  };
  REQUIRE(contains(relative_path));
  REQUIRE(contains(stage));
}

}  // namespace

TEST_CASE_METHOD(ConnSchemaFixture, "should cast FILE values to appropriate type", "[e2e][types][file]") {
  // Given Snowflake client is logged in

  // When A FILE column is populated via TO_FILE and queried
  conn.execute(std::string("CREATE TEMPORARY TABLE file_cast AS SELECT ") + kFileExpr + " AS file_col");
  const auto stmt = conn.execute_fetch("SELECT file_col FROM file_cast");

  // Then the FILE column should be returned as appropriate type with the expected JSON document
  require_sql_varchar(stmt, 1);
  require_type_name(stmt, 1, "FILE");
  require_file_doc(get_data<SQL_C_CHAR>(stmt, 1), "some_new_file.jpeg", "@myStage");
}

TEST_CASE_METHOD(ConnSchemaFixture, "should report FILE as the column type name", "[e2e][types][file]") {
  // Given Snowflake client is logged in

  // When A FILE column is populated via TO_FILE and queried
  conn.execute(std::string("CREATE TEMPORARY TABLE file_type_name AS SELECT ") + kFileExpr + " AS file_col");
  const auto stmt = conn.execute_fetch("SELECT file_col FROM file_type_name");

  // Then SQLDescribeCol reports SQL_VARCHAR and SQL_DESC_TYPE_NAME reports FILE
  require_sql_varchar(stmt, 1);
  require_type_name(stmt, 1, "FILE");
}

TEST_CASE("should keep SQL_VARCHAR for FILE when ODBC_USE_CUSTOM_SQL_DATA_TYPES is flag_state", "[e2e][types][file]") {
  // Given Snowflake client is logged in
  Connection conn;

  for (const char* flag_state : {"FALSE", "TRUE"}) {
    INFO("ODBC_USE_CUSTOM_SQL_DATA_TYPES=" << flag_state);

    // And Session parameter ODBC_USE_CUSTOM_SQL_DATA_TYPES is <flag_state>
    conn.execute(std::string("ALTER SESSION SET ODBC_USE_CUSTOM_SQL_DATA_TYPES = ") + flag_state);

    // When A FILE column is populated via TO_FILE and queried
    const auto stmt = conn.execute_fetch(std::string("SELECT ") + kFileExpr);

    // Then SQLDescribeCol reports SQL_VARCHAR and SQL_DESC_TYPE_NAME reports FILE
    if (std::string(flag_state) == "TRUE") {
      NEW_DRIVER_ONLY("BD#file-stays-varchar-with-custom-sql-types") { require_varchar_descriptor(stmt, 1); }
      OLD_DRIVER_ONLY("BD#file-stays-varchar-with-custom-sql-types") { require_sql_type(stmt, 1, SQL_SF_OBJECT); }
    } else {
      require_varchar_descriptor(stmt, 1);
    }
    require_type_name(stmt, 1, "FILE");
  }
}

TEST_CASE("should retrieve FILE data as SQL_C_WCHAR", "[e2e][types][file][conversion][wchar]") {
  // Given Snowflake client is logged in
  Connection conn;

  // When A FILE column is populated via TO_FILE and queried
  const auto stmt = conn.execute_fetch(std::string("SELECT ") + kFileExpr);

  // Then the FILE column should be returned as SQL_C_WCHAR with the expected JSON document
  require_wide_file_doc(check_wchar_success(stmt, 1), "some_new_file.jpeg", "@myStage");
}

TEST_CASE("should retrieve FILE data as SQL_C_BINARY", "[e2e][types][file][conversion][binary]") {
  // Given Snowflake client is logged in
  Connection conn;

  // When A FILE column is populated via TO_FILE and queried
  const auto stmt = conn.execute_fetch(std::string("SELECT ") + kFileExpr);

  // Then the FILE column should be returned as SQL_C_BINARY with the expected JSON document
  SQLCHAR buffer[512];
  std::memset(buffer, 0xFF, sizeof(buffer));
  SQLLEN indicator = -1;
  SQLRETURN ret = SQLGetData(stmt.getHandle(), 1, SQL_C_BINARY, buffer, sizeof(buffer), &indicator);
  REQUIRE_ODBC(ret, stmt);
  REQUIRE(indicator > 0);
  REQUIRE(static_cast<std::size_t>(indicator) < sizeof(buffer));
  require_file_doc(std::string(reinterpret_cast<char*>(buffer), static_cast<std::size_t>(indicator)),
                   "some_new_file.jpeg", "@myStage");
}

TEST_CASE("should return SQL_NULL_DATA for NULL FILE as SQL_C_BINARY", "[e2e][types][file][conversion][binary]") {
  // Given Snowflake client is logged in
  Connection conn;

  // When Query "SELECT TO_FILE(NULL)" is executed
  const auto stmt = conn.execute_fetch("SELECT TO_FILE(NULL)");

  // Then the indicator should be SQL_NULL_DATA
  SQLCHAR buffer[64];
  std::memset(buffer, 0xFF, sizeof(buffer));
  SQLLEN indicator = 0;
  SQLRETURN ret = SQLGetData(stmt.getHandle(), 1, SQL_C_BINARY, buffer, sizeof(buffer), &indicator);
  REQUIRE_ODBC(ret, stmt);
  CHECK(indicator == SQL_NULL_DATA);
  CHECK(buffer[0] == 0xFF);
}

TEST_CASE("should truncate FILE data when the character buffer is too short", "[e2e][types][file][conversion][char]") {
  // Given Snowflake client is logged in
  Connection conn;

  // When A FILE column is populated via TO_FILE and queried
  const auto stmt = conn.execute_fetch(std::string("SELECT ") + kFileExpr + ", " + kFileExpr);

  // Then SQLGetData should return SQL_SUCCESS_WITH_INFO with SQLSTATE 01004
  char buffer[10];
  std::memset(buffer, 0xFF, sizeof(buffer));
  SQLLEN indicator = -1;
  SQLRETURN ret = SQLGetData(stmt.getHandle(), 1, SQL_C_CHAR, buffer, sizeof(buffer), &indicator);
  REQUIRE_THAT(OdbcResult(ret, stmt), OdbcMatchers::IsSuccessWithInfo() && OdbcMatchers::HasSqlState("01004"));
  const std::string full = get_data<SQL_C_CHAR>(stmt, 2);
  CHECK(buffer[sizeof(buffer) - 1] == '\0');
  CHECK(std::string(buffer) == full.substr(0, sizeof(buffer) - 1));
  NEW_DRIVER_ONLY("BD#23") { CHECK(indicator == static_cast<SQLLEN>(full.size())); }
  OLD_DRIVER_ONLY("BD#23") { CHECK((indicator == SQL_NO_TOTAL || indicator == static_cast<SQLLEN>(full.size()))); }
}

TEST_CASE("should truncate FILE data as SQL_C_WCHAR when the buffer is too short",
          "[e2e][types][file][conversion][wchar]") {
  // Given Snowflake client is logged in
  Connection conn;

  // When A FILE column is populated via TO_FILE and queried
  const auto stmt = conn.execute_fetch(std::string("SELECT ") + kFileExpr + ", " + kFileExpr);

  // Then SQLGetData should return SQL_SUCCESS_WITH_INFO with SQLSTATE 01004
  SQLWCHAR buffer[6];
  std::memset(buffer, 0xFF, sizeof(buffer));
  SQLLEN indicator = -1;
  SQLRETURN ret = SQLGetData(stmt.getHandle(), 1, SQL_C_WCHAR, buffer, sizeof(buffer), &indicator);
  REQUIRE_THAT(OdbcResult(ret, stmt), OdbcMatchers::IsSuccessWithInfo() && OdbcMatchers::HasSqlState("01004"));
  const std::u32string full = check_wchar_success(stmt, 2);
  const std::size_t units = (sizeof(buffer) / sizeof(SQLWCHAR)) - 1;
  CHECK(buffer[units] == 0);
  const std::u32string truncated = sf::wide::decode_wide(buffer, units);
  CHECK(full.compare(0, truncated.size(), truncated) == 0);
  CHECK(truncated.size() == units);
  const bool indicator_reports_full_length =
      (indicator == SQL_NO_TOTAL) || (indicator > static_cast<SQLLEN>(sizeof(buffer)));
  CHECK(indicator_reports_full_length);
}

TEST_CASE("should select a FILE value built by TO_FILE without a table", "[e2e][types][file]") {
  // Given Snowflake client is logged in
  Connection conn;

  // When Query "SELECT TO_FILE(PARSE_JSON('{"RELATIVE_PATH": "some_new_file.jpeg", ...}'))" is executed
  const auto stmt = conn.execute_fetch(std::string("SELECT ") + kFileExpr);

  // Then the result should contain the expected FILE JSON document
  require_file_doc(get_data<SQL_C_CHAR>(stmt, 1), "some_new_file.jpeg", "@myStage");
}

TEST_CASE("should handle NULL FILE values from literals", "[e2e][types][file]") {
  // Given Snowflake client is logged in
  Connection conn;

  // When Query "SELECT TO_FILE(PARSE_JSON('...')), TO_FILE(NULL)" is executed
  const auto stmt = conn.execute_fetch(std::string("SELECT ") + kFileExpr + ", TO_FILE(NULL)");

  // Then the result should contain the expected FILE JSON document and NULL
  require_file_doc(get_data<SQL_C_CHAR>(stmt, 1), "some_new_file.jpeg", "@myStage");
  REQUIRE(get_data_optional<SQL_C_CHAR>(stmt, 2) == std::nullopt);
}

TEST_CASE_METHOD(ConnSchemaFixture, "should select FILE values from table", "[e2e][types][file]") {
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

TEST_CASE_METHOD(ConnSchemaFixture, "should handle NULL FILE values from table", "[e2e][types][file]") {
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

TEST_CASE("should download FILE data in multiple chunks", "[e2e][types][file]") {
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
