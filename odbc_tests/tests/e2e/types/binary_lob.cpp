#include <sql.h>
#include <sqlext.h>
#include <sqltypes.h>

#include <chrono>
#include <random>
#include <string>
#include <vector>

#include <catch2/catch_test_macros.hpp>

#include "Connection.hpp"
#include "SchemaFixtures.hpp"
#include "odbc_cast.hpp"
#include "odbc_matchers.hpp"

namespace {

std::vector<SQLCHAR> generate_random_bytes(std::mt19937& gen, size_t length) {
  std::uniform_int_distribution<int> dist(0, 255);
  std::vector<SQLCHAR> result(length);
  for (size_t i = 0; i < length; ++i) {
    result[i] = static_cast<SQLCHAR>(dist(gen));
  }
  return result;
}

}  // namespace

TEST_CASE_METHOD(ConnSchemaFixture, "should handle maximum default binary size", "[datatype][binary][lob][flaky]") {
  // Given Snowflake client is logged in

  auto seed = static_cast<unsigned int>(std::chrono::steady_clock::now().time_since_epoch().count());
  INFO("Random seed: " << seed);
  std::mt19937 gen(seed);
  const size_t length = 8388608;
  std::vector<SQLCHAR> payload = generate_random_bytes(gen, length);

  // And Table with BINARY column exists
  conn.execute("CREATE TEMPORARY TABLE bin_8mb (val BINARY)");

  // When Binary value of 8MB size (8,388,608 bytes) is inserted
  {
    auto stmt = conn.createStatement();
    SQLRETURN ret = SQLPrepare(stmt.getHandle(), sqlchar("INSERT INTO bin_8mb VALUES (?)"), SQL_NTS);
    REQUIRE_ODBC(ret, stmt);
    SQLLEN value_len = static_cast<SQLLEN>(payload.size());
    ret = SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_BINARY, SQL_BINARY, payload.size(), 0,
                           payload.data(), static_cast<SQLLEN>(payload.size()), &value_len);
    REQUIRE_ODBC(ret, stmt);
    ret = SQLExecute(stmt.getHandle());
    REQUIRE_ODBC(ret, stmt);
  }

  // And Query "SELECT * FROM {table}" is executed
  auto stmt = conn.createStatement();
  SQLRETURN ret = SQLExecDirect(stmt.getHandle(), sqlchar("SELECT val, LENGTH(val) as len FROM bin_8mb"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  std::vector<SQLCHAR> buffer(length);
  SQLLEN indicator = 0;
  ret = SQLBindCol(stmt.getHandle(), 1, SQL_C_BINARY, buffer.data(), static_cast<SQLLEN>(buffer.size()), &indicator);
  REQUIRE_ODBC(ret, stmt);
  SQLBIGINT len = 0;
  SQLLEN len_indicator = 0;
  ret = SQLBindCol(stmt.getHandle(), 2, SQL_C_SBIGINT, &len, sizeof(len), &len_indicator);
  REQUIRE_ODBC(ret, stmt);
  ret = SQLFetch(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);

  // Then the retrieved value size should be 8MB (8,388,608 bytes)
  REQUIRE(len == static_cast<SQLBIGINT>(length));
  REQUIRE(indicator == static_cast<SQLLEN>(length));

  // And data integrity should be maintained
  REQUIRE(buffer == payload);
}

TEST_CASE_METHOD(ConnSchemaFixture, "should handle extended maximum binary size", "[datatype][binary][lob][flaky]") {
  // Given Snowflake client is logged in

  auto seed = static_cast<unsigned int>(std::chrono::steady_clock::now().time_since_epoch().count());
  INFO("Random seed: " << seed);
  std::mt19937 gen(seed);
  const size_t length = 67108864;
  std::vector<SQLCHAR> payload = generate_random_bytes(gen, length);

  // And Table with BINARY(67108864) column exists
  conn.execute("CREATE TEMPORARY TABLE bin_64mb (val BINARY(67108864))");

  // When Binary value of 64MB size (67,108,864 bytes) is inserted
  {
    auto stmt = conn.createStatement();
    SQLRETURN ret = SQLPrepare(stmt.getHandle(), sqlchar("INSERT INTO bin_64mb VALUES (?)"), SQL_NTS);
    REQUIRE_ODBC(ret, stmt);
    SQLLEN value_len = static_cast<SQLLEN>(payload.size());
    ret = SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_BINARY, SQL_BINARY, payload.size(), 0,
                           payload.data(), static_cast<SQLLEN>(payload.size()), &value_len);
    REQUIRE_ODBC(ret, stmt);
    ret = SQLExecute(stmt.getHandle());
    REQUIRE_ODBC(ret, stmt);
  }

  // And Query "SELECT * FROM {table}" is executed
  auto stmt = conn.createStatement();
  SQLRETURN ret = SQLExecDirect(stmt.getHandle(), sqlchar("SELECT val, LENGTH(val) as len FROM bin_64mb"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  std::vector<SQLCHAR> buffer(length);
  SQLLEN indicator = 0;
  ret = SQLBindCol(stmt.getHandle(), 1, SQL_C_BINARY, buffer.data(), static_cast<SQLLEN>(buffer.size()), &indicator);
  REQUIRE_ODBC(ret, stmt);
  SQLBIGINT len = 0;
  SQLLEN len_indicator = 0;
  ret = SQLBindCol(stmt.getHandle(), 2, SQL_C_SBIGINT, &len, sizeof(len), &len_indicator);
  REQUIRE_ODBC(ret, stmt);
  ret = SQLFetch(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);

  // Then the retrieved value size should be 64MB (67,108,864 bytes)
  REQUIRE(len == static_cast<SQLBIGINT>(length));
  REQUIRE(indicator == static_cast<SQLLEN>(length));

  // And data integrity should be maintained
  REQUIRE(buffer == payload);
}
