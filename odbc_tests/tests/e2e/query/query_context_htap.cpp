#include <sql.h>
#include <sqlext.h>
#include <sqltypes.h>

#include <cctype>
#include <cstdint>
#include <cstring>
#include <random>
#include <sstream>
#include <string>
#include <utility>
#include <vector>

#include <catch2/catch_test_macros.hpp>

#include "Connection.hpp"
#include "get_data.hpp"
#include "odbc_cast.hpp"
#include "odbc_matchers.hpp"
#include "test_setup.hpp"

std::string unique_ident(const char* prefix) {
  std::random_device rd;
  std::mt19937_64 gen(rd());
  std::uniform_int_distribution<uint64_t> dist(0, UINT64_MAX);
  std::stringstream ss;
  ss << prefix;
  constexpr char hex[] = "0123456789abcdef";
  for (int i = 0; i < 8; ++i) {
    const auto v = static_cast<uint8_t>(dist(gen) & 0xFF);
    ss << hex[(v >> 4) & 0x0F] << hex[v & 0x0F];
  }
  std::string name = ss.str();
  for (char& ch : name) {
    ch = static_cast<char>(std::toupper(static_cast<unsigned char>(ch)));
  }
  return name;
}

std::string upper_copy(std::string value) {
  for (char& ch : value) {
    ch = static_cast<char>(std::toupper(static_cast<unsigned char>(ch)));
  }
  return value;
}

std::string current_name(Connection& conn, const char* sql) {
  auto stmt = conn.execute_fetch(sql);
  return upper_copy(get_data<SQL_C_CHAR>(stmt, 1));
}

std::string current_catalog(Connection& conn) {
  char catalog[256];
  std::memset(catalog, 0xFF, sizeof(catalog));
  SQLINTEGER catalog_len = -1;
  SQLRETURN ret = SQLGetConnectAttr(conn.handleWrapper().getHandle(), SQL_ATTR_CURRENT_CATALOG, catalog,
                                    sizeof(catalog), &catalog_len);
  REQUIRE_ODBC(ret, conn.handleWrapper());
  REQUIRE(catalog_len >= 0);
  REQUIRE(static_cast<size_t>(catalog_len) < sizeof(catalog));
  catalog[catalog_len] = '\0';
  return upper_copy(std::string(catalog));
}

void require_hybrid_rows(Connection& conn, const std::string& sql,
                         const std::vector<std::pair<SQLINTEGER, std::string>>& expected) {
  auto stmt = conn.execute(sql);
  std::vector<std::pair<SQLINTEGER, std::string>> rows;
  while (true) {
    SQLRETURN ret = SQLFetch(stmt.getHandle());
    if (ret == SQL_NO_DATA) {
      break;
    }
    REQUIRE_ODBC(ret, stmt);
    rows.emplace_back(get_data<SQL_C_LONG>(stmt, 1), get_data<SQL_C_CHAR>(stmt, 2));
  }
  REQUIRE(rows.size() == expected.size());
  for (size_t i = 0; i < expected.size(); ++i) {
    CHECK(rows[i].first == expected[i].first);
    CHECK(rows[i].second == expected[i].second);
  }
}

struct DropSchema {
  Connection& conn;
  std::string name;
  ~DropSchema() { conn.try_execute("DROP SCHEMA IF EXISTS " + name); }
};

struct DropDatabase {
  Connection& conn;
  std::string original;
  std::string name;
  ~DropDatabase() {
    if (!original.empty()) {
      conn.try_execute("USE DATABASE " + original);
    }
    conn.try_execute("DROP DATABASE IF EXISTS " + name);
  }
};

void enable_htap_or_skip(Connection& conn) {
  auto stmt = conn.createStatement();
  std::string sql = "ALTER SESSION SET ENABLE_SNOW_654741_FOR_TESTING = true";
  SQLRETURN ret = SQLExecDirect(stmt.getHandle(), sqlchar(sql.c_str()), SQL_NTS);
  if (!SQL_SUCCEEDED(ret)) {
    SKIP("ENABLE_SNOW_654741_FOR_TESTING is not available on this account");
  }
}

bool exec_ok(Connection& conn, const std::string& sql) {
  auto stmt = conn.createStatement();
  return SQL_SUCCEEDED(SQLExecDirect(stmt.getHandle(), sqlchar(sql.c_str()), SQL_NTS));
}

std::string parameter_value(Connection& conn, const char* name) {
  auto stmt = conn.execute_fetch(std::string("SHOW PARAMETERS LIKE '") + name + "'");
  return get_data<SQL_C_CHAR>(stmt, 2);
}

TEST_CASE("should preserve schema after SELECT under HTAP optimization", "[query][query_context_htap]") {
  // Given the account has ENABLE_SNOW_654741_FOR_TESTING enabled
  Connection conn;
  enable_htap_or_skip(conn);
  const auto schema = unique_ident("htap_sch_");
  REQUIRE(current_name(conn, "SELECT CURRENT_SCHEMA()") != schema);

  // When the client creates a new schema and executes SELECT
  REQUIRE(exec_ok(conn, "CREATE SCHEMA " + schema));
  DropSchema drop{conn, schema};
  REQUIRE(exec_ok(conn, "USE SCHEMA " + schema));
  conn.execute("SELECT 1");

  // Then the connection still reports the new schema
  CHECK(current_name(conn, "SELECT CURRENT_SCHEMA()") == schema);
}

TEST_CASE("should preserve database after SELECT under HTAP optimization", "[query][query_context_htap]") {
  // Given the account has ENABLE_SNOW_654741_FOR_TESTING enabled
  Connection conn;
  enable_htap_or_skip(conn);
  const auto database = unique_ident("htap_db_");
  const auto original = current_catalog(conn);
  REQUIRE(original != database);

  // When the client creates a new database and executes SELECT
  if (!exec_ok(conn, "CREATE DATABASE " + database)) {
    SKIP("CREATE DATABASE is not permitted on this account");
  }
  DropDatabase drop{conn, original, database};
  REQUIRE(exec_ok(conn, "USE DATABASE " + database));
  conn.execute("SELECT 1");

  // Then the connection still reports the new database
  CHECK(current_catalog(conn) == database);
}

TEST_CASE("should preserve role after SELECT under HTAP optimization", "[query][query_context_htap]") {
  // Given the account has ENABLE_SNOW_654741_FOR_TESTING enabled
  Connection conn;
  enable_htap_or_skip(conn);
  const auto original = current_name(conn, "SELECT CURRENT_ROLE()");

  // When the client switches to a different role and executes SELECT
  REQUIRE(exec_ok(conn, "USE ROLE PUBLIC"));
  CHECK(current_name(conn, "SELECT CURRENT_ROLE()") == "PUBLIC");
  conn.execute("SELECT 1");

  // Then the connection still reports the switched role
  CHECK(current_name(conn, "SELECT CURRENT_ROLE()") == "PUBLIC");
  if (!original.empty() && original != "PUBLIC") {
    conn.try_execute("USE ROLE " + original);
  }
}

TEST_CASE("should preserve session parameter after SELECT under HTAP optimization", "[query][query_context_htap]") {
  // Given the account has ENABLE_SNOW_654741_FOR_TESTING enabled
  Connection conn;
  enable_htap_or_skip(conn);

  // When the client changes DATE_OUTPUT_FORMAT and executes SELECT
  REQUIRE(exec_ok(conn, "ALTER SESSION SET DATE_OUTPUT_FORMAT = 'DD-MM-YYYY'"));
  CHECK(parameter_value(conn, "DATE_OUTPUT_FORMAT") == "DD-MM-YYYY");
  conn.execute("SELECT 1");

  // Then the session parameter still reflects the changed value
  CHECK(parameter_value(conn, "DATE_OUTPUT_FORMAT") == "DD-MM-YYYY");
  conn.try_execute("ALTER SESSION SET DATE_OUTPUT_FORMAT = 'YYYY-MM-DD'");
}

TEST_CASE("should operate on hybrid tables across multiple databases", "[query][query_context_htap]") {
  // Given a connection to Snowflake
  Connection conn;
  const auto original = current_catalog(conn);
  const auto db1 = unique_ident("htap_hy1_");
  const auto db2 = unique_ident("htap_hy2_");

  // When the client creates hybrid tables in two databases and inserts rows
  if (!exec_ok(conn, "CREATE DATABASE " + db1)) {
    SKIP("CREATE DATABASE is not permitted on this account");
  }
  DropDatabase drop1{conn, original, db1};
  if (!exec_ok(conn, "USE DATABASE " + db1) ||
      !exec_ok(conn, "CREATE HYBRID TABLE test_hybrid_table (id INT PRIMARY KEY, text VARCHAR)")) {
    SKIP("HYBRID TABLE is not available on this account");
  }
  REQUIRE(exec_ok(conn, "INSERT INTO test_hybrid_table VALUES (1, 'a')"));
  require_hybrid_rows(conn, "SELECT * FROM test_hybrid_table ORDER BY id", {{1, "a"}});

  REQUIRE(exec_ok(conn, "INSERT INTO test_hybrid_table VALUES (2, 'b')"));

  if (!exec_ok(conn, "CREATE DATABASE " + db2)) {
    SKIP("CREATE DATABASE is not permitted on this account");
  }
  DropDatabase drop2{conn, original, db2};
  if (!exec_ok(conn, "USE DATABASE " + db2) ||
      !exec_ok(conn, "CREATE HYBRID TABLE test_hybrid_table_2 (id INT PRIMARY KEY, text VARCHAR)")) {
    SKIP("HYBRID TABLE is not available on this account");
  }
  REQUIRE(exec_ok(conn, "INSERT INTO test_hybrid_table_2 VALUES (3, 'c')"));
  require_hybrid_rows(conn, "SELECT * FROM test_hybrid_table_2 ORDER BY id", {{3, "c"}});

  REQUIRE(exec_ok(conn, "USE DATABASE " + db1));
  REQUIRE(exec_ok(conn, "INSERT INTO test_hybrid_table VALUES (4, 'd')"));

  // Then selecting from each database returns the correct rows after switching back
  require_hybrid_rows(conn, "SELECT * FROM test_hybrid_table ORDER BY id", {{1, "a"}, {2, "b"}, {4, "d"}});
  REQUIRE(exec_ok(conn, "USE DATABASE " + db2));
  require_hybrid_rows(conn, "SELECT * FROM test_hybrid_table_2 ORDER BY id", {{3, "c"}});
}
