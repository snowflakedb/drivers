#include <sql.h>
#include <sqlext.h>
#include <sqltypes.h>

#include <cstring>
#include <optional>
#include <random>
#include <string>

#include <catch2/catch_test_macros.hpp>

#include "Connection.hpp"
#include "ReadOnlyDbFixture.hpp"
#include "Schema.hpp"
#include "SchemaFixtures.hpp"
#include "WideString.hpp"
#include "compatibility.hpp"
#include "conversion_checks.hpp"
#include "geo_test_helpers.hpp"
#include "get_data.hpp"
#include "odbc_cast.hpp"
#include "odbc_matchers.hpp"
#include "query_helpers.hpp"
#include "sf_odbc.h"
#include "test_setup.hpp"

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

// Schema names must not contain '_' because SQLProcedureColumns matches
// procedure_schema with LIKE, and '_' is a single-character wildcard. The
// shared Schema helper always generates TEMP_TEST_SCHEMA_<n>, so this RAII
// type owns a no-underscore name and drops it at the end of the test.
class ProcedureSchema {
 public:
  explicit ProcedureSchema(Connection& conn) : conn_(conn) {
    std::random_device rd;
    std::mt19937_64 gen(rd());
    name_ = "FILEPROCSCHEMA" + std::to_string(gen());
    conn_.execute("CREATE SCHEMA IF NOT EXISTS " + name_);
    conn_.execute("USE SCHEMA " + name_);
  }

  ~ProcedureSchema() {
    try {
      conn_.execute("DROP SCHEMA IF EXISTS " + name_ + " CASCADE");
    } catch (...) {
      // A failed drop must not escape the destructor.
    }
  }

  ProcedureSchema(const ProcedureSchema&) = delete;
  ProcedureSchema& operator=(const ProcedureSchema&) = delete;

  const std::string& name() const { return name_; }

 private:
  Connection& conn_;
  std::string name_;
};

struct CatalogSize {
  std::string type_name;
  SQLINTEGER column_size = -1;
  SQLINTEGER buffer_length = -1;
};

CatalogSize read_catalog_size(const StatementHandleWrapper& stmt, SQLUSMALLINT type_col, SQLUSMALLINT size_col,
                              SQLUSMALLINT buffer_col, const char* expected_name) {
  SQLRETURN ret = SQLFetch(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);

  char name[64];
  std::memset(name, 0xFF, sizeof(name));
  SQLLEN name_ind = -1;
  ret = SQLGetData(stmt.getHandle(), 4, SQL_C_CHAR, name, sizeof(name), &name_ind);
  REQUIRE_ODBC(ret, stmt);
  REQUIRE(name_ind > 0);
  REQUIRE(std::string(name, static_cast<size_t>(name_ind)) == expected_name);

  char type_name[64];
  std::memset(type_name, 0xFF, sizeof(type_name));
  SQLLEN type_ind = -1;
  ret = SQLGetData(stmt.getHandle(), type_col, SQL_C_CHAR, type_name, sizeof(type_name), &type_ind);
  REQUIRE_ODBC(ret, stmt);
  REQUIRE(type_ind > 0);

  CatalogSize meta;
  meta.type_name.assign(type_name, static_cast<size_t>(type_ind));
  SQLLEN size_ind = -1;
  ret = SQLGetData(stmt.getHandle(), size_col, SQL_C_SLONG, &meta.column_size, 0, &size_ind);
  REQUIRE_ODBC(ret, stmt);
  REQUIRE(size_ind == sizeof(SQLINTEGER));
  SQLLEN buffer_ind = -1;
  ret = SQLGetData(stmt.getHandle(), buffer_col, SQL_C_SLONG, &meta.buffer_length, 0, &buffer_ind);
  REQUIRE_ODBC(ret, stmt);
  REQUIRE(buffer_ind == sizeof(SQLINTEGER));

  ret = SQLFetch(stmt.getHandle());
  REQUIRE_ODBC_NO_DATA(ret, stmt);
  ret = SQLFreeStmt(stmt.getHandle(), SQL_CLOSE);
  REQUIRE_ODBC(ret, stmt);
  return meta;
}

}  // namespace

TEST_CASE_METHOD(ConnSchemaFixture, "should cast FILE values to appropriate type", "[e2e][types][file]") {
  // Given Snowflake client is logged in

  // When A FILE column is populated via TO_FILE and queried
  conn.execute(std::string("CREATE TEMPORARY TABLE file_cast AS SELECT ") + kFileExpr + " AS file_col");
  const auto stmt = conn.execute_fetch("SELECT file_col FROM file_cast");

  // Then the FILE column should be returned as appropriate type with the expected JSON document
  require_sql_varchar(stmt, 1);
  require_sql_type_name(stmt, 1, "FILE");
  require_file_doc(get_data<SQL_C_CHAR>(stmt, 1), "some_new_file.jpeg", "@myStage");
}

TEST_CASE_METHOD(ConnSchemaFixture, "should report FILE as the column type name", "[e2e][types][file]") {
  // Given Snowflake client is logged in

  // When A FILE column is populated via TO_FILE and queried
  conn.execute(std::string("CREATE TEMPORARY TABLE file_type_name AS SELECT ") + kFileExpr + " AS file_col");
  const auto stmt = conn.execute_fetch("SELECT file_col FROM file_type_name");

  // Then SQLDescribeCol reports SQL_VARCHAR and SQL_DESC_TYPE_NAME reports FILE
  require_sql_varchar(stmt, 1);
  require_sql_type_name(stmt, 1, "FILE");
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
    require_sql_type_name(stmt, 1, "FILE");
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
  const SQLLEN wide_bytes = static_cast<SQLLEN>(full.size() * sizeof(SQLWCHAR));
  NEW_DRIVER_ONLY("BD#23") { CHECK(indicator == wide_bytes); }
  OLD_DRIVER_ONLY("BD#23") { CHECK((indicator == SQL_NO_TOTAL || indicator == wide_bytes)); }
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

TEST_CASE("should report the session maximum for a FILE column in SQLColumns when DEFAULT_VARCHAR_SIZE is set",
          "[e2e][types][file]") {
  // Given Snowflake client is logged in
  const std::string connection = get_connection_string();

  // And DEFAULT_VARCHAR_SIZE is set
  Connection conn(connection + "DEFAULT_VARCHAR_SIZE=100;");
  Schema::use_temp_session_schema(conn);
  conn.execute("CREATE TEMPORARY TABLE file_sqlcols (file_col FILE, variant_col VARIANT)");

  // When SQLColumns is called for a table with FILE and VARIANT columns
  const auto stmt = conn.createStatement();
  SQLRETURN ret = SQLSetStmtAttr(stmt.getHandle(), SQL_ATTR_METADATA_ID, sqlptr_value(SQL_TRUE), 0);
  REQUIRE_ODBC(ret, stmt);
  const std::string catalog = get_current_database(conn.handleWrapper().getHandle());
  auto column_meta = [&](const char* column) {
    ret = SQLColumns(stmt.getHandle(), sqlchar(catalog.c_str()), SQL_NTS, sqlchar(Schema::name().c_str()), SQL_NTS,
                     sqlchar("FILE_SQLCOLS"), SQL_NTS, sqlchar(column), SQL_NTS);
    REQUIRE_ODBC(ret, stmt);
    return read_catalog_size(stmt, 6, 7, 8, column);
  };
  const CatalogSize file_col = column_meta("FILE_COL");
  const CatalogSize variant_col = column_meta("VARIANT_COL");

  // Then TYPE_NAME is FILE and COLUMN_SIZE matches the VARIANT column
  CHECK(file_col.type_name == "FILE");
  CHECK(file_col.column_size == variant_col.column_size);
  CHECK(file_col.buffer_length == variant_col.buffer_length);
  NEW_DRIVER_ONLY("BD#file-sqlcolumns-ignores-default-varchar-size") {
    CHECK((file_col.column_size == 16777216 || file_col.column_size == 134217728));
  }
  OLD_DRIVER_ONLY("BD#file-sqlcolumns-ignores-default-varchar-size") { CHECK(file_col.column_size == 100); }
}

TEST_CASE("should report the session maximum for a FILE procedure argument when DEFAULT_VARCHAR_SIZE is set",
          "[e2e][types][file]") {
  // Given Snowflake client is logged in
  const std::string connection = get_scratch_db_connection_string();

  // And DEFAULT_VARCHAR_SIZE is set
  Connection conn(connection + "DEFAULT_VARCHAR_SIZE=100;");
  const ProcedureSchema proc_schema(conn);
  conn.execute(
      "CREATE OR REPLACE PROCEDURE fileproc(filearg FILE, variantarg VARIANT) "
      "RETURNS VARCHAR LANGUAGE SQL AS 'BEGIN RETURN ''ok''; END'");

  // When SQLProcedureColumns is called for a procedure with FILE and VARIANT arguments
  const auto stmt = conn.createStatement();
  SQLRETURN ret = SQLSetStmtAttr(stmt.getHandle(), SQL_ATTR_METADATA_ID, sqlptr_value(SQL_TRUE), 0);
  REQUIRE_ODBC(ret, stmt);
  const std::string catalog = get_current_database(conn.handleWrapper().getHandle());
  auto argument_meta = [&](const char* column) {
    ret = SQLProcedureColumns(stmt.getHandle(), sqlchar(catalog.c_str()), SQL_NTS, sqlchar(proc_schema.name().c_str()),
                              SQL_NTS, sqlchar("FILEPROC"), SQL_NTS, sqlchar(column), SQL_NTS);
    REQUIRE_ODBC(ret, stmt);
    return read_catalog_size(stmt, 7, 8, 9, column);
  };
  const CatalogSize file_arg = argument_meta("FILEARG");
  const CatalogSize variant_arg = argument_meta("VARIANTARG");

  // Then TYPE_NAME is FILE and COLUMN_SIZE matches the VARIANT argument
  CHECK(file_arg.type_name == "FILE");
  CHECK(file_arg.column_size == variant_arg.column_size);
  CHECK(file_arg.buffer_length == variant_arg.buffer_length);
  NEW_DRIVER_ONLY("BD#file-procedure-arguments-use-session-varchar-max") {
    CHECK((file_arg.column_size == 16777216 || file_arg.column_size == 134217728));
    CHECK(file_arg.buffer_length == file_arg.column_size);
  }
  OLD_DRIVER_ONLY("BD#file-procedure-arguments-use-session-varchar-max") {
    CHECK(file_arg.column_size == 0);
    CHECK(file_arg.buffer_length == 0);
  }
}
