#include <sql.h>
#include <sqlext.h>
#include <sqltypes.h>

#include <algorithm>
#include <cstring>
#include <string>
#include <vector>

#include <catch2/catch_test_macros.hpp>
#include <catch2/matchers/catch_matchers_string.hpp>

#include "Connection.hpp"
#include "HandleWrapper.hpp"
#include "Schema.hpp"
#include "compatibility.hpp"
#include "get_data.hpp"
#include "get_diag_rec.hpp"
#include "odbc_cast.hpp"
#include "odbc_matchers.hpp"
#include "test_setup.hpp"

using Catch::Matchers::ContainsSubstring;

// MapToLongVarchar remaps SQL_CHAR / SQL_VARCHAR metadata when COLUMN_SIZE is
// greater than the threshold. Fetch values and COLUMN_SIZE stay the same.

namespace {

constexpr SQLUSMALLINT kSqlColumnsColumnName = 4;
constexpr SQLUSMALLINT kSqlColumnsDataType = 5;
constexpr SQLUSMALLINT kSqlColumnsTypeName = 6;
constexpr SQLUSMALLINT kSqlColumnsColumnSize = 7;
constexpr SQLUSMALLINT kSqlColumnsSqlDataType = 14;

constexpr SQLUSMALLINT kSqlTypeInfoTypeName = 1;
constexpr SQLUSMALLINT kSqlTypeInfoDataType = 2;
constexpr SQLUSMALLINT kSqlTypeInfoColumnSize = 3;

const char* kTable = "MAP_LV_TEST";

struct ColType {
  std::string name;
  SQLSMALLINT describe = static_cast<SQLSMALLINT>(0x7FFF);
  SQLLEN desc_type = static_cast<SQLLEN>(0x7FFFFFFFFFFFFFFF);
  SQLLEN concise_type = static_cast<SQLLEN>(0x7FFFFFFFFFFFFFFF);
  SQLULEN column_size = static_cast<SQLULEN>(-1);
};

struct ColumnsMeta {
  std::string name;
  std::string type_name;
  SQLSMALLINT data_type = static_cast<SQLSMALLINT>(0x7FFF);
  SQLSMALLINT sql_data_type = static_cast<SQLSMALLINT>(0x7FFF);
  SQLINTEGER column_size = static_cast<SQLINTEGER>(0x7FFFFFFF);
};

struct MappedConn {
  Connection conn;
  explicit MappedConn(const std::string& extra) : conn(get_connection_string() + extra) {
    Schema::use_temp_session_schema(conn);
  }
};

void create_string_table(Connection& conn) {
  conn.execute(std::string("CREATE OR REPLACE TEMPORARY TABLE ") + kTable +
               " (C1 VARCHAR, C2 VARCHAR(8000), C3 VARCHAR(8001))");
}

std::string take_written(const char* buf, size_t capacity, SQLLEN available) {
  REQUIRE(available > 0);
  auto n = std::min(static_cast<size_t>(available), capacity - 1);
  while (n > 0 && buf[n - 1] == '\0') {
    --n;
  }
  REQUIRE(n > 0);
  return {buf, n};
}

std::string current_database(Connection& conn) {
  char db_name[256];
  std::memset(db_name, 0xFF, sizeof(db_name));
  SQLSMALLINT db_name_len = 0;
  SQLRETURN ret =
      SQLGetInfo(conn.handleWrapper().getHandle(), SQL_DATABASE_NAME, db_name, sizeof(db_name), &db_name_len);
  REQUIRE_ODBC(ret, conn.handleWrapper());
  return take_written(db_name, sizeof(db_name), db_name_len);
}

ColType describe_column(StatementHandleWrapper& stmt, SQLUSMALLINT col) {
  ColType out;
  SQLCHAR name[64];
  std::memset(name, 0xFF, sizeof(name));
  SQLSMALLINT name_len = 0;
  SQLSMALLINT decimal_digits = 0;
  SQLSMALLINT nullable = 0;
  SQLRETURN ret = SQLDescribeCol(stmt.getHandle(), col, name, sizeof(name), &name_len, &out.describe, &out.column_size,
                                 &decimal_digits, &nullable);
  REQUIRE_ODBC(ret, stmt);
  out.name = take_written(reinterpret_cast<char*>(name), sizeof(name), name_len);

  ret = SQLColAttribute(stmt.getHandle(), col, SQL_DESC_TYPE, nullptr, 0, nullptr, &out.desc_type);
  REQUIRE_ODBC(ret, stmt);

  ret = SQLColAttribute(stmt.getHandle(), col, SQL_DESC_CONCISE_TYPE, nullptr, 0, nullptr, &out.concise_type);
  REQUIRE_ODBC(ret, stmt);
  return out;
}

std::vector<ColType> describe_varchar_columns(Connection& conn) {
  auto stmt = conn.execute(std::string("SELECT C1, C2, C3 FROM ") + kTable);
  return {describe_column(stmt, 1), describe_column(stmt, 2), describe_column(stmt, 3)};
}

void expect_sql_type(const ColType& col, SQLSMALLINT sql_type) {
  CHECK(col.describe == sql_type);
  CHECK(col.desc_type == sql_type);
  CHECK(col.concise_type == sql_type);
}

void expect_varchar_sizes(const std::vector<ColType>& cols) {
  REQUIRE(cols.size() == 3);
  REQUIRE(cols[0].name == "C1");
  REQUIRE(cols[1].name == "C2");
  REQUIRE(cols[2].name == "C3");
  CHECK(cols[1].column_size == 8000);
  CHECK(cols[2].column_size == 8001);
  CHECK(cols[0].column_size >= cols[2].column_size);
}

ColumnsMeta sqlcolumns_row(Connection& conn, const std::string& column) {
  auto stmt = conn.createStatement();
  SQLRETURN ret = SQLSetStmtAttr(stmt.getHandle(), SQL_ATTR_METADATA_ID, sqlptr_value(SQL_TRUE), 0);
  REQUIRE_ODBC(ret, stmt);

  const std::string catalog = current_database(conn);
  ret = SQLColumns(stmt.getHandle(), sqlchar(catalog.c_str()), SQL_NTS, sqlchar(Schema::name().c_str()), SQL_NTS,
                   sqlchar(kTable), SQL_NTS, sqlchar(column.c_str()), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);

  ret = SQLFetch(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);

  ColumnsMeta row;
  char name_buf[128];
  std::memset(name_buf, 0xFF, sizeof(name_buf));
  SQLLEN name_ind = 0;
  ret = SQLGetData(stmt.getHandle(), kSqlColumnsColumnName, SQL_C_CHAR, name_buf, sizeof(name_buf), &name_ind);
  REQUIRE_ODBC(ret, stmt);
  row.name = take_written(name_buf, sizeof(name_buf), name_ind);

  char type_name_buf[128];
  std::memset(type_name_buf, 0xFF, sizeof(type_name_buf));
  SQLLEN type_name_ind = 0;
  ret = SQLGetData(stmt.getHandle(), kSqlColumnsTypeName, SQL_C_CHAR, type_name_buf, sizeof(type_name_buf),
                   &type_name_ind);
  REQUIRE_ODBC(ret, stmt);
  row.type_name = take_written(type_name_buf, sizeof(type_name_buf), type_name_ind);

  SQLLEN data_type_ind = 0;
  ret = SQLGetData(stmt.getHandle(), kSqlColumnsDataType, SQL_C_SSHORT, &row.data_type, 0, &data_type_ind);
  REQUIRE_ODBC(ret, stmt);
  REQUIRE(data_type_ind == sizeof(SQLSMALLINT));

  SQLLEN sql_data_type_ind = 0;
  ret = SQLGetData(stmt.getHandle(), kSqlColumnsSqlDataType, SQL_C_SSHORT, &row.sql_data_type, 0, &sql_data_type_ind);
  REQUIRE_ODBC(ret, stmt);
  REQUIRE(sql_data_type_ind == sizeof(SQLSMALLINT));

  SQLLEN column_size_ind = 0;
  ret = SQLGetData(stmt.getHandle(), kSqlColumnsColumnSize, SQL_C_SLONG, &row.column_size, 0, &column_size_ind);
  REQUIRE_ODBC(ret, stmt);
  REQUIRE(column_size_ind == sizeof(SQLINTEGER));

  ret = SQLFetch(stmt.getHandle());
  REQUIRE_ODBC_NO_DATA(ret, stmt);
  return row;
}

struct TypeInfoRow {
  std::string type_name;
  SQLSMALLINT data_type = static_cast<SQLSMALLINT>(0x7FFF);
  SQLINTEGER column_size = static_cast<SQLINTEGER>(0x7FFFFFFF);
};

std::vector<TypeInfoRow> longvarchar_type_info(Connection& conn) {
  auto stmt = conn.createStatement();
  SQLRETURN ret = SQLGetTypeInfo(stmt.getHandle(), SQL_LONGVARCHAR);
  REQUIRE_ODBC(ret, stmt);

  std::vector<TypeInfoRow> rows;
  while (true) {
    ret = SQLFetch(stmt.getHandle());
    if (ret == SQL_NO_DATA) {
      break;
    }
    REQUIRE_ODBC(ret, stmt);

    TypeInfoRow row;
    char type_name[128];
    std::memset(type_name, 0xFF, sizeof(type_name));
    SQLLEN type_name_ind = 0;
    ret = SQLGetData(stmt.getHandle(), kSqlTypeInfoTypeName, SQL_C_CHAR, type_name, sizeof(type_name), &type_name_ind);
    REQUIRE_ODBC(ret, stmt);
    row.type_name = take_written(type_name, sizeof(type_name), type_name_ind);

    SQLLEN data_type_ind = 0;
    ret = SQLGetData(stmt.getHandle(), kSqlTypeInfoDataType, SQL_C_SSHORT, &row.data_type, 0, &data_type_ind);
    REQUIRE_ODBC(ret, stmt);
    REQUIRE(data_type_ind == sizeof(SQLSMALLINT));

    SQLLEN column_size_ind = 0;
    ret = SQLGetData(stmt.getHandle(), kSqlTypeInfoColumnSize, SQL_C_SLONG, &row.column_size, 0, &column_size_ind);
    REQUIRE_ODBC(ret, stmt);
    REQUIRE(column_size_ind == sizeof(SQLINTEGER));
    rows.push_back(row);
  }
  return rows;
}

const TypeInfoRow* find_type_name(const std::vector<TypeInfoRow>& rows, const char* name) {
  for (const auto& row : rows) {
    if (row.type_name == name) {
      return &row;
    }
  }
  return nullptr;
}

std::string joined_text_for_state(const std::vector<DiagRec>& records, const std::string& sql_state) {
  std::string text;
  for (const auto& rec : records) {
    if (rec.sqlState == sql_state) {
      text += rec.messageText;
      text += '\n';
    }
  }
  return text;
}

}  // namespace

TEST_CASE("should report SQL_VARCHAR for every string column when MapToLongVarchar is unset",
          "[datatype][string][map_to_long_varchar]") {
  // Given a connection with MapToLongVarchar unset (3.x default -1)
  MappedConn mapped("");
  create_string_table(mapped.conn);

  // When the VARCHAR columns are described
  const auto cols = describe_varchar_columns(mapped.conn);
  expect_varchar_sizes(cols);

  // Then remapping stays off and SQLGetTypeInfo(SQL_LONGVARCHAR) is empty
  expect_sql_type(cols[0], SQL_VARCHAR);
  expect_sql_type(cols[1], SQL_VARCHAR);
  expect_sql_type(cols[2], SQL_VARCHAR);
  CHECK(longvarchar_type_info(mapped.conn).empty());
}

TEST_CASE("should remap VARCHAR columns above MapToLongVarchar=8000 to SQL_LONGVARCHAR",
          "[datatype][string][map_to_long_varchar]") {
  // Given a connection with threshold 8000 and a row of VARCHAR values
  MappedConn mapped("MapToLongVarchar=8000;");
  create_string_table(mapped.conn);
  mapped.conn.execute(std::string("INSERT INTO ") + kTable +
                      " (C1, C2, C3) VALUES ('unbounded', 'at-threshold', 'over')");

  // When those columns are described and fetched
  auto stmt = mapped.conn.execute(std::string("SELECT C1, C2, C3 FROM ") + kTable);
  const auto cols = std::vector<ColType>{describe_column(stmt, 1), describe_column(stmt, 2), describe_column(stmt, 3)};
  expect_varchar_sizes(cols);
  const auto type_info = longvarchar_type_info(mapped.conn);

  // Then sizes above the threshold remap; fetched values stay VARCHAR
  expect_sql_type(cols[0], SQL_LONGVARCHAR);
  expect_sql_type(cols[1], SQL_VARCHAR);
  expect_sql_type(cols[2], SQL_LONGVARCHAR);
  REQUIRE(type_info.size() == 2);
  const auto* char_row = find_type_name(type_info, "CHAR");
  const auto* varchar_row = find_type_name(type_info, "VARCHAR");
  REQUIRE(char_row != nullptr);
  REQUIRE(varchar_row != nullptr);
  CHECK(char_row->data_type == SQL_LONGVARCHAR);
  CHECK(varchar_row->data_type == SQL_LONGVARCHAR);
  CHECK(char_row->column_size >= static_cast<SQLINTEGER>(cols[0].column_size));
  CHECK(varchar_row->column_size >= static_cast<SQLINTEGER>(cols[0].column_size));

  const SQLRETURN fetched = SQLFetch(stmt.getHandle());
  REQUIRE_ODBC(fetched, stmt);
  CHECK(get_data<SQL_C_CHAR>(stmt, 1) == "unbounded");
  CHECK(get_data<SQL_C_CHAR>(stmt, 2) == "at-threshold");
  CHECK(get_data<SQL_C_CHAR>(stmt, 3) == "over");
}

TEST_CASE("should remap SQLColumns DATA_TYPE and SQL_DATA_TYPE with the same MapToLongVarchar threshold",
          "[datatype][string][map_to_long_varchar]") {
  // Given a connection with threshold 8000
  MappedConn mapped("MapToLongVarchar=8000;");
  create_string_table(mapped.conn);

  // When SQLColumns is called for the VARCHAR columns
  const auto c1 = sqlcolumns_row(mapped.conn, "C1");
  const auto c2 = sqlcolumns_row(mapped.conn, "C2");
  const auto c3 = sqlcolumns_row(mapped.conn, "C3");
  REQUIRE(c1.name == "C1");
  REQUIRE(c2.name == "C2");
  REQUIRE(c3.name == "C3");
  CHECK(c2.column_size == 8000);
  CHECK(c3.column_size == 8001);
  CHECK(c1.column_size >= c3.column_size);
  CHECK(c1.type_name == "VARCHAR");
  CHECK(c2.type_name == "VARCHAR");
  CHECK(c3.type_name == "VARCHAR");

  // Then catalog DATA_TYPE / SQL_DATA_TYPE follow the same threshold; TYPE_NAME does not
  CHECK(c1.data_type == SQL_LONGVARCHAR);
  CHECK(c1.sql_data_type == SQL_LONGVARCHAR);
  CHECK(c2.data_type == SQL_VARCHAR);
  CHECK(c2.sql_data_type == SQL_VARCHAR);
  CHECK(c3.data_type == SQL_LONGVARCHAR);
  CHECK(c3.sql_data_type == SQL_LONGVARCHAR);
}

TEST_CASE("should apply DEFAULT_VARCHAR_SIZE before the MapToLongVarchar threshold",
          "[datatype][string][map_to_long_varchar]") {
  // Given both keys, with DEFAULT_VARCHAR_SIZE equal to the remap threshold
  MappedConn mapped("MapToLongVarchar=8000;DEFAULT_VARCHAR_SIZE=8000;");
  create_string_table(mapped.conn);

  // When the VARCHAR columns are described
  const auto cols = describe_varchar_columns(mapped.conn);
  CHECK(cols[1].column_size == 8000);
  CHECK(cols[2].column_size == 8001);

  // Then unbounded TEXT is rewritten to 8000 first, so C1 stays VARCHAR
  CHECK(cols[0].column_size == 8000);
  expect_sql_type(cols[0], SQL_VARCHAR);
  expect_sql_type(cols[1], SQL_VARCHAR);
  expect_sql_type(cols[2], SQL_LONGVARCHAR);
}

TEST_CASE("should accept MapToLongVarchar and report only unrecognized keys as 01S00",
          "[datatype][string][map_to_long_varchar]") {
  // Given a connection string that pairs MapToLongVarchar with a key neither driver accepts
  const std::string conn_str = get_connection_string() + "MapToLongVarchar=8000;INVALIDKEY=abc;";

  // When SQLDriverConnect runs
  auto env = Connection::initEnv();
  auto dbc = env.createConnectionHandle();
  const SQLRETURN ret = SQLDriverConnect(dbc.getHandle(), nullptr, sqlchar(conn_str.c_str()), SQL_NTS, nullptr, 0,
                                         nullptr, SQL_DRIVER_NOPROMPT);
  ConnectedConnectionWrapper connected(dbc.getHandle());
  dbc.release();
  const auto connect_result = OdbcResult(ret, SQL_HANDLE_DBC, connected.getHandle());
  const auto records = get_diag_rec(SQL_HANDLE_DBC, connected.getHandle());
  const std::string invalid = joined_text_for_state(records, "01S00");

  // Then MapToLongVarchar is accepted; only INVALIDKEY appears in 01S00
  IODBC_ONLY { REQUIRE_THAT(connect_result, OdbcMatchers::Succeeded()); }
  NON_IODBC {
    REQUIRE_THAT(connect_result, OdbcMatchers::IsSuccessWithInfo());
    REQUIRE_THAT(connect_result, OdbcMatchers::HasSqlState("01S00"));
    CHECK_THAT(invalid, ContainsSubstring("INVALIDKEY"));
    CHECK_THAT(invalid, !ContainsSubstring("MAPTOLONGVARCHAR"));
  }

  auto stmt = StatementHandleWrapper(connected.getHandle(), SQL_HANDLE_STMT);
  std::string query = "SELECT 1";
  const SQLRETURN exec = SQLExecDirect(stmt.getHandle(), sqlchar(query.c_str()), SQL_NTS);
  REQUIRE_ODBC(exec, stmt);
  const SQLRETURN fetched = SQLFetch(stmt.getHandle());
  REQUIRE_ODBC(fetched, stmt);
  CHECK(get_data<SQL_C_CHAR>(stmt, 1) == "1");
}
