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

// DEFAULT_VARCHAR_SIZE / DEFAULT_BINARY_SIZE rewrite max-length TEXT and
// BINARY COLUMN_SIZE (session max or the 16 MiB / 8 MiB workaround).
// Bounded columns and fetched values stay unchanged.

namespace {

constexpr SQLUSMALLINT kSqlColumnsColumnName = 4;
constexpr SQLUSMALLINT kSqlColumnsColumnSize = 7;
constexpr SQLUSMALLINT kSqlColumnsBufferLength = 8;
constexpr SQLUSMALLINT kSqlColumnsCharOctetLength = 16;

const char* kTable = "DEF_SIZE_TEST";

struct ColType {
  std::string name;
  SQLULEN column_size = static_cast<SQLULEN>(-1);
  SQLLEN display_size = static_cast<SQLLEN>(0x7FFFFFFFFFFFFFFF);
  SQLLEN length = static_cast<SQLLEN>(0x7FFFFFFFFFFFFFFF);
  SQLLEN octet_length = static_cast<SQLLEN>(0x7FFFFFFFFFFFFFFF);
};

struct ColumnsMeta {
  std::string name;
  SQLINTEGER column_size = static_cast<SQLINTEGER>(0x7FFFFFFF);
  SQLINTEGER buffer_length = static_cast<SQLINTEGER>(0x7FFFFFFF);
  SQLINTEGER char_octet_length = static_cast<SQLINTEGER>(0x7FFFFFFF);
};

struct SizedConn {
  Connection conn;
  explicit SizedConn(const std::string& extra) : conn(get_connection_string() + extra) {
    Schema::use_temp_session_schema(conn);
  }
};

void create_size_table(Connection& conn) {
  conn.execute(std::string("CREATE OR REPLACE TEMPORARY TABLE ") + kTable +
               " (C1 VARCHAR, C2 VARCHAR(8000), C3 BINARY, C4 BINARY(16))");
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
  SQLSMALLINT sql_type = 0;
  SQLSMALLINT decimal_digits = 0;
  SQLSMALLINT nullable = 0;
  SQLRETURN ret = SQLDescribeCol(stmt.getHandle(), col, name, sizeof(name), &name_len, &sql_type, &out.column_size,
                                 &decimal_digits, &nullable);
  REQUIRE_ODBC(ret, stmt);
  out.name = take_written(reinterpret_cast<char*>(name), sizeof(name), name_len);

  ret = SQLColAttribute(stmt.getHandle(), col, SQL_DESC_DISPLAY_SIZE, nullptr, 0, nullptr, &out.display_size);
  REQUIRE_ODBC(ret, stmt);
  ret = SQLColAttribute(stmt.getHandle(), col, SQL_DESC_LENGTH, nullptr, 0, nullptr, &out.length);
  REQUIRE_ODBC(ret, stmt);
  ret = SQLColAttribute(stmt.getHandle(), col, SQL_DESC_OCTET_LENGTH, nullptr, 0, nullptr, &out.octet_length);
  REQUIRE_ODBC(ret, stmt);
  return out;
}

std::vector<ColType> describe_size_columns(Connection& conn) {
  auto stmt = conn.execute(std::string("SELECT C1, C2, C3, C4 FROM ") + kTable);
  return {describe_column(stmt, 1), describe_column(stmt, 2), describe_column(stmt, 3), describe_column(stmt, 4)};
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

  SQLLEN column_size_ind = 0;
  ret = SQLGetData(stmt.getHandle(), kSqlColumnsColumnSize, SQL_C_SLONG, &row.column_size, 0, &column_size_ind);
  REQUIRE_ODBC(ret, stmt);
  REQUIRE(column_size_ind == sizeof(SQLINTEGER));

  SQLLEN buffer_length_ind = 0;
  ret = SQLGetData(stmt.getHandle(), kSqlColumnsBufferLength, SQL_C_SLONG, &row.buffer_length, 0, &buffer_length_ind);
  REQUIRE_ODBC(ret, stmt);
  REQUIRE(buffer_length_ind == sizeof(SQLINTEGER));

  SQLLEN char_octet_ind = 0;
  ret =
      SQLGetData(stmt.getHandle(), kSqlColumnsCharOctetLength, SQL_C_SLONG, &row.char_octet_length, 0, &char_octet_ind);
  REQUIRE_ODBC(ret, stmt);
  REQUIRE(char_octet_ind == sizeof(SQLINTEGER));

  ret = SQLFetch(stmt.getHandle());
  REQUIRE_ODBC_NO_DATA(ret, stmt);
  return row;
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

TEST_CASE("should report session-max sizes when DEFAULT_VARCHAR_SIZE and DEFAULT_BINARY_SIZE are unset",
          "[datatype][string][binary][default_varchar_binary_size]") {
  // Given a connection with both keys unset
  SizedConn sized("");
  create_size_table(sized.conn);

  // When the VARCHAR and BINARY columns are described
  const auto cols = describe_size_columns(sized.conn);
  REQUIRE(cols[0].name == "C1");
  REQUIRE(cols[1].name == "C2");
  REQUIRE(cols[2].name == "C3");
  REQUIRE(cols[3].name == "C4");
  CHECK(cols[1].column_size == 8000);
  CHECK(cols[3].column_size == 16);

  // Then unbounded columns keep the session maximum
  CHECK(cols[0].column_size >= cols[1].column_size);
  CHECK(cols[2].column_size >= cols[3].column_size);
  CHECK(cols[0].length == static_cast<SQLLEN>(cols[0].column_size));
  CHECK(cols[2].length == static_cast<SQLLEN>(cols[2].column_size));
  CHECK(cols[2].display_size == static_cast<SQLLEN>(2 * cols[2].column_size));
}

TEST_CASE("should rewrite max-length VARCHAR and BINARY metadata with DEFAULT_VARCHAR_SIZE and DEFAULT_BINARY_SIZE",
          "[datatype][string][binary][default_varchar_binary_size]") {
  // Given DEFAULT_VARCHAR_SIZE=2000 and DEFAULT_BINARY_SIZE=1000
  SizedConn sized("DEFAULT_VARCHAR_SIZE=2000;DEFAULT_BINARY_SIZE=1000;");
  create_size_table(sized.conn);
  sized.conn.execute(std::string("INSERT INTO ") + kTable +
                     " (C1, C2, C3, C4) VALUES ('unbounded', 'bounded', TO_BINARY('ABCD'), TO_BINARY('0011'))");

  // When those columns are described and fetched
  auto stmt = sized.conn.execute(std::string("SELECT C1, C2, C3, C4 FROM ") + kTable);
  const auto cols = std::vector<ColType>{describe_column(stmt, 1), describe_column(stmt, 2), describe_column(stmt, 3),
                                         describe_column(stmt, 4)};

  // Then only max-length columns rewrite; fetched values stay unchanged
  CHECK(cols[0].column_size == 2000);
  CHECK(cols[0].display_size == 2000);
  CHECK(cols[0].length == 2000);
  CHECK(cols[1].column_size == 8000);
  CHECK(cols[1].length == 8000);
  CHECK(cols[2].column_size == 1000);
  CHECK(cols[2].length == 1000);
  CHECK(cols[2].display_size == 2000);
  CHECK(cols[2].octet_length == 1000);
  CHECK(cols[3].column_size == 16);
  CHECK(cols[3].length == 16);

  const SQLRETURN fetched = SQLFetch(stmt.getHandle());
  REQUIRE_ODBC(fetched, stmt);
  CHECK(get_data<SQL_C_CHAR>(stmt, 1) == "unbounded");
  CHECK(get_data<SQL_C_CHAR>(stmt, 2) == "bounded");
}

TEST_CASE("should apply swapped DEFAULT_VARCHAR_SIZE and DEFAULT_BINARY_SIZE to SQLColumns sizes",
          "[datatype][string][binary][default_varchar_binary_size]") {
  // Given swapped sizes, DEFAULT_BINARY_SIZE=2000 and DEFAULT_VARCHAR_SIZE=1000
  SizedConn sized("DEFAULT_BINARY_SIZE=2000;DEFAULT_VARCHAR_SIZE=1000;");
  create_size_table(sized.conn);

  // When SQLColumns is called for each column
  const auto c1 = sqlcolumns_row(sized.conn, "C1");
  const auto c2 = sqlcolumns_row(sized.conn, "C2");
  const auto c3 = sqlcolumns_row(sized.conn, "C3");
  const auto c4 = sqlcolumns_row(sized.conn, "C4");
  REQUIRE(c1.name == "C1");
  REQUIRE(c2.name == "C2");
  REQUIRE(c3.name == "C3");
  REQUIRE(c4.name == "C4");

  // Then each max-length type uses its own default size
  CHECK(c1.column_size == 1000);
  CHECK(c1.buffer_length == 1000);
  CHECK(c1.char_octet_length == 1000);
  CHECK(c2.column_size == 8000);
  CHECK(c3.column_size == 2000);
  CHECK(c3.buffer_length == 2000);
  CHECK(c3.char_octet_length == 2000);
  CHECK(c4.column_size == 16);
}

TEST_CASE("should report the same DEFAULT_VARCHAR_SIZE and DEFAULT_BINARY_SIZE on SQLColumns as SQLDescribeCol",
          "[datatype][string][binary][default_varchar_binary_size]") {
  // Given DEFAULT_VARCHAR_SIZE=2000 and DEFAULT_BINARY_SIZE=1000
  SizedConn sized("DEFAULT_VARCHAR_SIZE=2000;DEFAULT_BINARY_SIZE=1000;");
  create_size_table(sized.conn);

  // When SQLDescribeCol and SQLColumns both describe the max-length columns
  const auto cols = describe_size_columns(sized.conn);
  const auto c1 = sqlcolumns_row(sized.conn, "C1");
  const auto c3 = sqlcolumns_row(sized.conn, "C3");

  // Then catalog COLUMN_SIZE matches SQLDescribeCol
  CHECK(c1.column_size == 2000);
  CHECK(c1.buffer_length == 2000);
  CHECK(c1.char_octet_length == 2000);
  CHECK(static_cast<SQLULEN>(c1.column_size) == cols[0].column_size);
  CHECK(c3.column_size == 1000);
  CHECK(c3.buffer_length == 1000);
  CHECK(c3.char_octet_length == 1000);
  CHECK(static_cast<SQLULEN>(c3.column_size) == cols[2].column_size);
}

TEST_CASE("should accept DEFAULT_VARCHAR_SIZE and DEFAULT_BINARY_SIZE and report only unrecognized keys as 01S00",
          "[datatype][string][binary][default_varchar_binary_size]") {
  // Given a connection string that pairs both keys with a key neither driver accepts
  const std::string conn_str =
      get_connection_string() + "DEFAULT_VARCHAR_SIZE=2000;DEFAULT_BINARY_SIZE=1000;INVALIDKEY=abc;";

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

  // Then only the unrecognized key is 01S00; the size keys remain usable
  IODBC_ONLY { REQUIRE_THAT(connect_result, OdbcMatchers::Succeeded()); }
  NON_IODBC {
    REQUIRE_THAT(connect_result, OdbcMatchers::IsSuccessWithInfo());
    REQUIRE_THAT(connect_result, OdbcMatchers::HasSqlState("01S00"));
    CHECK_THAT(invalid, ContainsSubstring("INVALIDKEY"));
    CHECK_THAT(invalid, !ContainsSubstring("DEFAULT_VARCHAR_SIZE"));
    CHECK_THAT(invalid, !ContainsSubstring("DEFAULT_BINARY_SIZE"));
  }

  auto stmt = StatementHandleWrapper(connected.getHandle(), SQL_HANDLE_STMT);
  std::string query = "SELECT 1";
  const SQLRETURN exec = SQLExecDirect(stmt.getHandle(), sqlchar(query.c_str()), SQL_NTS);
  REQUIRE_ODBC(exec, stmt);
  const SQLRETURN fetched = SQLFetch(stmt.getHandle());
  REQUIRE_ODBC(fetched, stmt);
  CHECK(get_data<SQL_C_CHAR>(stmt, 1) == "1");
}
