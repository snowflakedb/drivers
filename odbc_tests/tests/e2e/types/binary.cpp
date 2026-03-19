#include <algorithm>
#include <cstdio>
#include <cstring>
#include <optional>
#include <set>
#include <string>
#include <vector>

#include <catch2/catch_test_macros.hpp>

#include "Connection.hpp"
#include "HandleWrapper.hpp"
#include "Schema.hpp"
#include "compatibility.hpp"
#include "odbc_cast.hpp"
#include "odbc_matchers.hpp"

static std::string expected_padded_hex(int seq) {
  char padded[11];
  snprintf(padded, sizeof(padded), "%010d", seq);
  std::string hex;
  for (int i = 0; i < 10; ++i) {
    char h[3];
    snprintf(h, sizeof(h), "%02X", static_cast<unsigned char>(padded[i]));
    hex += h;
  }
  return hex;
}

static std::string get_binary_hex(const StatementHandleWrapper& stmt, SQLUSMALLINT col) {
  SQLCHAR buffer[8192] = {};
  SQLLEN indicator = 0;
  SQLRETURN ret = SQLGetData(stmt.getHandle(), col, SQL_C_BINARY, buffer, sizeof(buffer), &indicator);
  REQUIRE_ODBC(ret, stmt);
  REQUIRE(indicator != SQL_NULL_DATA);
  REQUIRE(indicator <= static_cast<SQLLEN>(sizeof(buffer)));
  std::string hex;
  for (SQLLEN i = 0; i < indicator; ++i) {
    char h[3];
    snprintf(h, sizeof(h), "%02X", buffer[i]);
    hex += h;
  }
  return hex;
}

static std::optional<std::string> get_binary_hex_optional(const StatementHandleWrapper& stmt, SQLUSMALLINT col) {
  SQLCHAR buffer[8192] = {};
  SQLLEN indicator = 0;
  SQLRETURN ret = SQLGetData(stmt.getHandle(), col, SQL_C_BINARY, buffer, sizeof(buffer), &indicator);
  REQUIRE_ODBC(ret, stmt);
  if (indicator == SQL_NULL_DATA) {
    return std::nullopt;
  }
  REQUIRE(indicator <= static_cast<SQLLEN>(sizeof(buffer)));
  std::string hex;
  for (SQLLEN i = 0; i < indicator; ++i) {
    char h[3];
    snprintf(h, sizeof(h), "%02X", buffer[i]);
    hex += h;
  }
  return hex;
}

TEST_CASE("should cast binary values to appropriate type", "[datatype][binary]") {
  // Given Snowflake client is logged in
  Connection conn;

  // When Query "SELECT TO_BINARY('48656C6C6F', 'HEX')::BINARY, TO_BINARY('V29ybGQ=', 'BASE64')::BINARY" is executed
  const auto stmt =
      conn.execute_fetch("SELECT TO_BINARY('48656C6C6F', 'HEX')::BINARY, TO_BINARY('V29ybGQ=', 'BASE64')::BINARY");

  // Then All values should be returned as appropriate binary type
  auto hex1 = get_binary_hex(stmt, 1);
  auto hex2 = get_binary_hex(stmt, 2);

  // And the result should contain binary values:
  REQUIRE(hex1 == "48656C6C6F");
  REQUIRE(hex2 == "576F726C64");
}

TEST_CASE("should select binary literals", "[datatype][binary]") {
  // Given Snowflake client is logged in
  Connection conn;

  // When Queries selecting binary literals are executed:
  const auto stmt1 = conn.execute_fetch("SELECT X'48656C6C6F'::BINARY");
  const auto stmt2 = conn.execute_fetch("SELECT TO_BINARY('48656C6C6F', 'HEX')::BINARY");
  const auto stmt3 = conn.execute_fetch("SELECT TO_BINARY('ASNFZ4mrze8=', 'BASE64')::BINARY");

  // Then the results should contain expected binary values
  REQUIRE(get_binary_hex(stmt1, 1) == "48656C6C6F");
  REQUIRE(get_binary_hex(stmt2, 1) == "48656C6C6F");
  REQUIRE(get_binary_hex(stmt3, 1) == "0123456789ABCDEF");
}

TEST_CASE("should handle binary corner case values from literals", "[datatype][binary]") {
  // Given Snowflake client is logged in
  Connection conn;

  // When Query selecting corner case binary literals is executed
  const auto stmt = conn.execute_fetch(
      "SELECT X''::BINARY, X'00'::BINARY, X'FF'::BINARY, "
      "X'0000000000'::BINARY, X'FFFFFFFFFF'::BINARY, X'48006500'::BINARY");

  // Then the result should contain expected corner case binary values
  REQUIRE(get_binary_hex(stmt, 1).empty());
  REQUIRE(get_binary_hex(stmt, 2) == "00");
  REQUIRE(get_binary_hex(stmt, 3) == "FF");
  REQUIRE(get_binary_hex(stmt, 4) == "0000000000");
  REQUIRE(get_binary_hex(stmt, 5) == "FFFFFFFFFF");
  REQUIRE(get_binary_hex(stmt, 6) == "48006500");
}

TEST_CASE("should handle NULL binary values from literals", "[datatype][binary]") {
  // Given Snowflake client is logged in
  Connection conn;

  // When Query "SELECT NULL::{type}, X'ABCD', NULL::{type}" is executed
  const auto stmt = conn.execute_fetch("SELECT NULL::BINARY, X'ABCD'::BINARY, NULL::BINARY");

  // Then Result should contain [NULL, 0xABCD, NULL]
  REQUIRE(get_binary_hex_optional(stmt, 1) == std::nullopt);
  REQUIRE(get_binary_hex_optional(stmt, 2) == "ABCD");
  REQUIRE(get_binary_hex_optional(stmt, 3) == std::nullopt);
}

TEST_CASE("should select binary values from table", "[datatype][binary]") {
  // Given Snowflake client is logged in
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);

  // And A temporary table with BINARY column is created
  conn.execute("CREATE TABLE binary_table (col BINARY)");

  // And The table is populated with binary values [X'48656C6C6F', X'576F726C64', X'0123456789ABCDEF']
  conn.execute("INSERT INTO binary_table VALUES (X'48656C6C6F'), (X'576F726C64'), (X'0123456789ABCDEF')");

  // When Query "SELECT * FROM {table} ORDER BY col" is executed
  const auto stmt = conn.execute_fetch("SELECT * FROM binary_table ORDER BY col");

  // Then the result should contain binary values in order:
  REQUIRE(get_binary_hex(stmt, 1) == "0123456789ABCDEF");

  SQLRETURN ret = SQLFetch(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);
  REQUIRE(get_binary_hex(stmt, 1) == "48656C6C6F");

  ret = SQLFetch(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);
  REQUIRE(get_binary_hex(stmt, 1) == "576F726C64");

  REQUIRE(SQLFetch(stmt.getHandle()) == SQL_NO_DATA);
}

TEST_CASE("should select corner case binary values from table", "[datatype][binary]") {
  // Given Snowflake client is logged in
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);

  // And A temporary table with BINARY column is created
  conn.execute("CREATE TABLE binary_corner_table (col BINARY)");

  // And The table is populated with corner case binary values
  conn.execute(
      "INSERT INTO binary_corner_table VALUES "
      "(X''), (X'00'), (X'FF'), (X'000000'), (X'48006500')");

  // When Query "SELECT * FROM {table} ORDER BY 1" is executed
  const auto stmt = conn.execute_fetch("SELECT col FROM binary_corner_table ORDER BY 1");

  // Then the result should contain the inserted corner case binary values
  std::vector<std::string> expected = {"", "00", "000000", "48006500", "FF"};
  REQUIRE(get_binary_hex(stmt, 1) == expected[0]);
  for (size_t i = 1; i < expected.size(); ++i) {
    SQLRETURN ret = SQLFetch(stmt.getHandle());
    REQUIRE_ODBC(ret, stmt);
    REQUIRE(get_binary_hex(stmt, 1) == expected[i]);
  }
  REQUIRE(SQLFetch(stmt.getHandle()) == SQL_NO_DATA);
}

TEST_CASE("should select NULL binary values from table", "[datatype][binary]") {
  // Given Snowflake client is logged in
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);

  // And A temporary table with BINARY column is created
  conn.execute("CREATE TABLE binary_null_table (col BINARY)");

  // And The table is populated with NULL and non-NULL binary values [NULL, X'ABCD', NULL]
  conn.execute("INSERT INTO binary_null_table VALUES (NULL), (X'ABCD'), (NULL)");

  // When Query "SELECT * FROM {table}" is executed
  const auto stmt = conn.createStatement();
  SQLRETURN ret = SQLExecDirect(stmt.getHandle(), sqlchar("SELECT col FROM binary_null_table"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);

  // Then there are 3 rows returned
  int null_count = 0;
  int value_count = 0;

  while (true) {
    ret = SQLFetch(stmt.getHandle());
    if (ret == SQL_NO_DATA) {
      break;
    }
    REQUIRE_ODBC(ret, stmt);

    if (auto result = get_binary_hex_optional(stmt, 1); !result.has_value()) {
      null_count++;
    } else {
      REQUIRE(result.value() == "ABCD");
      value_count++;
    }
  }

  // And 2 rows should contain NULL values
  REQUIRE(null_count == 2);

  // And 1 row should contain 0xABCD
  REQUIRE(value_count == 1);
}

TEST_CASE("should select binary with specified length from table", "[datatype][binary]") {
  // Given Snowflake client is logged in
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);

  // And Table with columns (bin5 BINARY(5), bin10 BINARY(10), bin_default BINARY) exists
  conn.execute("CREATE TABLE binary_len_table (bin5 BINARY(5), bin10 BINARY(10), bin_default BINARY)");

  // And Row (X'0102030405', X'01020304050607080910', X'48656C6C6F') is inserted
  conn.execute("INSERT INTO binary_len_table VALUES (X'0102030405', X'01020304050607080910', X'48656C6C6F')");

  // When Query "SELECT * FROM {table}" is executed
  const auto stmt = conn.execute_fetch("SELECT * FROM binary_len_table");

  // Then Result should contain binary values with correct lengths
  auto describe_col = [&](SQLUSMALLINT col_num) {
    SQLCHAR col_name[128] = {};
    SQLSMALLINT name_len = 0, data_type = 0, decimal_digits = 0, nullable = 0;
    SQLULEN col_size = 0;
    SQLRETURN r = SQLDescribeCol(stmt.getHandle(), col_num, col_name, sizeof(col_name), &name_len, &data_type,
                                 &col_size, &decimal_digits, &nullable);
    REQUIRE_ODBC(r, stmt);
    NEW_DRIVER_ONLY("#26") REQUIRE(data_type == SQL_VARBINARY);
    OLD_DRIVER_ONLY("#26") REQUIRE(data_type == SQL_BINARY);
    REQUIRE(decimal_digits == 0);
    return col_size;
  };

  REQUIRE(describe_col(1) == 5);
  REQUIRE(describe_col(2) == 10);
  REQUIRE(describe_col(3) == 8388608);

  SQLCHAR buf[64] = {};
  SQLLEN ind = 0;

  SQLRETURN ret = SQLGetData(stmt.getHandle(), 1, SQL_C_BINARY, buf, sizeof(buf), &ind);
  REQUIRE_ODBC(ret, stmt);
  REQUIRE(ind == 5);
  {
    const SQLCHAR expected[] = {0x01, 0x02, 0x03, 0x04, 0x05};
    REQUIRE(memcmp(buf, expected, 5) == 0);
  }

  ret = SQLGetData(stmt.getHandle(), 2, SQL_C_BINARY, buf, sizeof(buf), &ind);
  REQUIRE_ODBC(ret, stmt);
  REQUIRE(ind == 10);
  {
    const SQLCHAR expected[] = {0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x10};
    REQUIRE(memcmp(buf, expected, 10) == 0);
  }

  ret = SQLGetData(stmt.getHandle(), 3, SQL_C_BINARY, buf, sizeof(buf), &ind);
  REQUIRE_ODBC(ret, stmt);
  REQUIRE(ind == 5);
  {
    const SQLCHAR expected[] = {0x48, 0x65, 0x6C, 0x6C, 0x6F};
    REQUIRE(memcmp(buf, expected, 5) == 0);
  }
}

TEST_CASE("should select binary literals using parameter binding", "[datatype][binary]") {
  // Given Snowflake client is logged in
  Connection conn;

  // When Query "SELECT ?::BINARY, ?::BINARY, ?::BINARY" is executed with bound binary values [0x48656C6C6F,
  // 0x576F726C64, 0x0123456789ABCDEF]
  const auto stmt = conn.createStatement();
  SQLCHAR val1[] = {0x48, 0x65, 0x6C, 0x6C, 0x6F};
  SQLCHAR val2[] = {0x57, 0x6F, 0x72, 0x6C, 0x64};
  SQLCHAR val3[] = {0x01, 0x23, 0x45, 0x67, 0x89, 0xAB, 0xCD, 0xEF};
  SQLLEN ind1 = sizeof(val1), ind2 = sizeof(val2), ind3 = sizeof(val3);

  SQLRETURN ret = SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_BINARY, SQL_VARBINARY, sizeof(val1), 0,
                                   val1, sizeof(val1), &ind1);
  REQUIRE_ODBC(ret, stmt);
  ret = SQLBindParameter(stmt.getHandle(), 2, SQL_PARAM_INPUT, SQL_C_BINARY, SQL_VARBINARY, sizeof(val2), 0, val2,
                         sizeof(val2), &ind2);
  REQUIRE_ODBC(ret, stmt);
  ret = SQLBindParameter(stmt.getHandle(), 3, SQL_PARAM_INPUT, SQL_C_BINARY, SQL_VARBINARY, sizeof(val3), 0, val3,
                         sizeof(val3), &ind3);
  REQUIRE_ODBC(ret, stmt);

  ret = SQLExecDirect(stmt.getHandle(), sqlchar("SELECT ?::BINARY, ?::BINARY, ?::BINARY"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  ret = SQLFetch(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);

  // Then the result should contain:
  REQUIRE(get_binary_hex(stmt, 1) == "48656C6C6F");
  REQUIRE(get_binary_hex(stmt, 2) == "576F726C64");
  REQUIRE(get_binary_hex(stmt, 3) == "0123456789ABCDEF");
}

TEST_CASE("should insert binary using parameter binding", "[datatype][binary]") {
  // Given Snowflake client is logged in
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);

  // And Table with BINARY column exists
  conn.execute("CREATE TABLE binary_insert_table (col BINARY)");

  // When Binary values [0x48656C6C6F, 0x576F726C64, 0x00, 0xFF, 0x] are inserted using binding
  SQLCHAR vals[][5] = {{0x48, 0x65, 0x6C, 0x6C, 0x6F}, {0x57, 0x6F, 0x72, 0x6C, 0x64}, {}, {0xFF}, {}};
  SQLLEN lens[] = {5, 5, 1, 1, 0};
  for (int i = 0; i < 5; ++i) {
    const auto ins = conn.createStatement();
    SQLRETURN ret = SQLBindParameter(ins.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_BINARY, SQL_VARBINARY,
                                     std::max(lens[i], static_cast<SQLLEN>(1)), 0, vals[i], lens[i], &lens[i]);
    REQUIRE_ODBC(ret, ins);
    ret = SQLExecDirect(ins.getHandle(), sqlchar("INSERT INTO binary_insert_table VALUES (?)"), SQL_NTS);
    REQUIRE_ODBC(ret, ins);
  }

  // And Query "SELECT * FROM {table}" is executed
  const auto stmt = conn.createStatement();
  SQLRETURN ret = SQLExecDirect(stmt.getHandle(), sqlchar("SELECT col FROM binary_insert_table"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);

  // Then Result should contain binary values [0x48656C6C6F, 0x576F726C64, 0x00, 0xFF, 0x]
  std::set<std::string> actual;
  int row_count = 0;
  while (true) {
    ret = SQLFetch(stmt.getHandle());
    if (ret == SQL_NO_DATA) {
      break;
    }
    REQUIRE_ODBC(ret, stmt);
    actual.insert(get_binary_hex(stmt, 1));
    row_count++;
  }
  REQUIRE(row_count == 5);
  std::set<std::string> expected = {"48656C6C6F", "576F726C64", "00", "FF", ""};
  REQUIRE(actual == expected);
}

TEST_CASE("should bind corner case binary values", "[datatype][binary]") {
  // Given Snowflake client is logged in
  Connection conn;

  // When Query "SELECT ?::BINARY" is executed with each corner case binary value bound
  auto bind_and_check = [&](SQLCHAR* data, SQLLEN len) {
    const auto stmt = conn.createStatement();
    SQLLEN ind = len;
    SQLRETURN ret = SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_BINARY, SQL_VARBINARY,
                                     std::max(len, static_cast<SQLLEN>(1)), 0, data, len, &ind);
    REQUIRE_ODBC(ret, stmt);
    ret = SQLExecDirect(stmt.getHandle(), sqlchar("SELECT ?::BINARY"), SQL_NTS);
    REQUIRE_ODBC(ret, stmt);
    ret = SQLFetch(stmt.getHandle());
    REQUIRE_ODBC(ret, stmt);

    SQLCHAR buf[64] = {};
    SQLLEN buf_ind = 0;
    ret = SQLGetData(stmt.getHandle(), 1, SQL_C_BINARY, buf, sizeof(buf), &buf_ind);
    REQUIRE_ODBC(ret, stmt);
    REQUIRE(buf_ind == len);
    if (len > 0) {
      REQUIRE(memcmp(buf, data, len) == 0);
    }
  };

  SQLCHAR empty[1] = {};
  bind_and_check(empty, 0);

  SQLCHAR null_byte[] = {0x00};
  bind_and_check(null_byte, 1);

  SQLCHAR max_byte[] = {0xFF};
  bind_and_check(max_byte, 1);

  SQLCHAR embedded_nulls[] = {0x48, 0x00, 0x65, 0x00};
  bind_and_check(embedded_nulls, 4);

  // Then the result should match the bound corner case value
  {
    const auto stmt = conn.createStatement();
    SQLCHAR val = 0;
    SQLLEN ind = SQL_NULL_DATA;
    SQLRETURN ret =
        SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_BINARY, SQL_VARBINARY, 1, 0, &val, 1, &ind);
    REQUIRE_ODBC(ret, stmt);
    ret = SQLExecDirect(stmt.getHandle(), sqlchar("SELECT ?::BINARY"), SQL_NTS);
    REQUIRE_ODBC(ret, stmt);
    ret = SQLFetch(stmt.getHandle());
    REQUIRE_ODBC(ret, stmt);
    REQUIRE(get_binary_hex_optional(stmt, 1) == std::nullopt);
  }
}

TEST_CASE("should handle VARBINARY as synonym for BINARY", "[datatype][binary]") {
  // Given Snowflake client is logged in
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);

  // And A temporary table with VARBINARY column is created
  conn.execute("CREATE TABLE varbinary_test (col VARBINARY)");

  // And The table is populated with binary values via VARBINARY column
  conn.execute("INSERT INTO varbinary_test VALUES (X'48656C6C6F'), (X'ABCDEF'), (X'00FF01')");

  // When Query "SELECT * FROM {table} ORDER BY col" is executed
  const auto stmt = conn.execute_fetch("SELECT * FROM varbinary_test ORDER BY col");

  // Then the result should match the equivalent BINARY behavior
  REQUIRE(get_binary_hex(stmt, 1) == "00FF01");
  SQLRETURN ret = SQLFetch(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);
  REQUIRE(get_binary_hex(stmt, 1) == "48656C6C6F");
  ret = SQLFetch(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);
  REQUIRE(get_binary_hex(stmt, 1) == "ABCDEF");
  REQUIRE(SQLFetch(stmt.getHandle()) == SQL_NO_DATA);
}

TEST_CASE("should download binary data in multiple chunks using GENERATOR", "[datatype][binary]") {
  // Given Snowflake client is logged in
  Connection conn;

  // When Query "SELECT seq8() AS id, TO_BINARY(LPAD(TO_VARCHAR(seq8()), 10, '0'), 'UTF-8') AS bin_val FROM
  // TABLE(GENERATOR(ROWCOUNT => 30000)) v ORDER BY id" is executed
  const auto stmt = conn.createStatement();
  const auto sql =
      "SELECT id, TO_BINARY(LPAD(TO_VARCHAR(id), 10, '0'), 'UTF-8') AS bin_val "
      "FROM (SELECT seq8() AS id FROM TABLE(GENERATOR(ROWCOUNT => 30000))) v ORDER BY id";
  SQLRETURN ret = SQLExecDirect(stmt.getHandle(), sqlchar(sql), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);

  // Then there are 30000 rows returned
  int row_count = 0;
  while (true) {
    ret = SQLFetch(stmt.getHandle());
    if (ret == SQL_NO_DATA) {
      break;
    }
    REQUIRE_ODBC(ret, stmt);

    // And all returned binary values should match the generated values in order
    auto hex = get_binary_hex(stmt, 2);
    REQUIRE(hex == expected_padded_hex(row_count));
    row_count++;
  }

  REQUIRE(row_count == 30000);
}

TEST_CASE("should download binary data in multiple chunks from table", "[datatype][binary]") {
  // Given Snowflake client is logged in
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);

  // And Table with (bin_data BINARY) exists with 30000 sequential binary values
  conn.execute("CREATE TABLE binary_large_table (bin_data BINARY)");
  conn.execute(
      "INSERT INTO binary_large_table "
      "SELECT TO_BINARY(LPAD(TO_VARCHAR(seq8()), 10, '0'), 'UTF-8') FROM TABLE(GENERATOR(ROWCOUNT => 30000))");

  // When Query "SELECT * FROM {table} ORDER BY bin_data" is executed
  const auto stmt = conn.createStatement();
  SQLRETURN ret =
      SQLExecDirect(stmt.getHandle(), sqlchar("SELECT * FROM binary_large_table ORDER BY bin_data"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);

  // Then there are 30000 rows returned
  int row_count = 0;
  while (true) {
    ret = SQLFetch(stmt.getHandle());
    if (ret == SQL_NO_DATA) {
      break;
    }
    REQUIRE_ODBC(ret, stmt);

    // And all returned binary values should match the inserted values in order
    auto hex = get_binary_hex(stmt, 1);
    REQUIRE(hex == expected_padded_hex(row_count));
    row_count++;
  }

  REQUIRE(row_count == 30000);
}
