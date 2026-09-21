#include <sql.h>
#include <sqlext.h>

#include <cstdlib>
#include <random>
#include <string>

#include <catch2/catch_test_macros.hpp>

#include "Connection.hpp"
#include "get_data.hpp"

#ifdef _WIN32
#include <process.h>
#define GET_PID() _getpid()
#else
#include <unistd.h>
#define GET_PID() getpid()
#endif

namespace {

void set_autocommit(Connection& conn, bool on) {
  SQLPOINTER value =
      on ? reinterpret_cast<SQLPOINTER>(SQL_AUTOCOMMIT_ON) : reinterpret_cast<SQLPOINTER>(SQL_AUTOCOMMIT_OFF);
  SQLRETURN ret = SQLSetConnectAttr(conn.handleWrapper().getHandle(), SQL_ATTR_AUTOCOMMIT, value, 0);
  REQUIRE(ret == SQL_SUCCESS);
}

SQLULEN get_autocommit(Connection& conn) {
  SQLULEN autocommit = 99;
  SQLRETURN ret = SQLGetConnectAttr(conn.handleWrapper().getHandle(), SQL_ATTR_AUTOCOMMIT, &autocommit, 0, nullptr);
  REQUIRE(ret == SQL_SUCCESS);
  return autocommit;
}

class ScopedAutocommit {
 public:
  ScopedAutocommit(Connection& conn, bool on) : conn_(conn) { set_autocommit(conn_, on); }
  ~ScopedAutocommit() noexcept {
    try {
      set_autocommit(conn_, true);
    } catch (...) {
    }
  }
  ScopedAutocommit(const ScopedAutocommit&) = delete;
  ScopedAutocommit& operator=(const ScopedAutocommit&) = delete;

 private:
  Connection& conn_;
};

// Cross-session autocommit checks need a table visible to a second connection.
// TEMPORARY is session-scoped; TRANSIENT matches the shared Gherkin step and JDBC.
class ScopedTransientTable {
 public:
  ScopedTransientTable(Connection& conn, const std::string& prefix, const std::string& columns)
      : conn_(conn), name_(generate_name(prefix)) {
    conn_.execute("CREATE TRANSIENT TABLE " + name_ + " (" + columns + ")");
  }

  ~ScopedTransientTable() {
    try {
      conn_.execute("DROP TABLE IF EXISTS " + name_);
    } catch (...) {
    }
  }

  ScopedTransientTable(const ScopedTransientTable&) = delete;
  ScopedTransientTable& operator=(const ScopedTransientTable&) = delete;

  const std::string& name() const { return name_; }

 private:
  static std::string generate_name(const std::string& prefix) {
    std::random_device rd;
    std::mt19937_64 gen(rd());
    return prefix + "X" + std::to_string(GET_PID()) + "X" + std::to_string(gen());
  }

  Connection& conn_;
  std::string name_;
};

SQLINTEGER count_rows(Connection& conn, const std::string& table) {
  auto stmt = conn.execute_fetch("SELECT COUNT(*) FROM " + table);
  return get_data<SQL_C_SLONG>(stmt, 1);
}

SQLRETURN end_tran(Connection& conn, SQLSMALLINT completion_type) {
  return SQLEndTran(SQL_HANDLE_DBC, conn.handleWrapper().getHandle(), completion_type);
}

}  // namespace

TEST_CASE("should report autocommit as disabled after it was disabled on the connection", "[session][autocommit]") {
  // Given Snowflake client is logged in
  Connection conn;

  // When autocommit is disabled on the connection
  set_autocommit(conn, false);

  // Then the autocommit setting reports as disabled
  REQUIRE(get_autocommit(conn) == SQL_AUTOCOMMIT_OFF);
  set_autocommit(conn, true);
}

TEST_CASE("should report autocommit as enabled after it was re-enabled on the connection", "[session][autocommit]") {
  // Given Snowflake client is logged in
  Connection conn;

  // And autocommit was disabled on the connection
  set_autocommit(conn, false);

  // When autocommit is enabled on the connection
  set_autocommit(conn, true);

  // Then the autocommit setting reports as enabled
  REQUIRE(get_autocommit(conn) == SQL_AUTOCOMMIT_ON);
}

TEST_CASE("should discard uncommitted inserts on rollback", "[session][autocommit]") {
  // Given Snowflake client is logged in
  Connection writer;
  Connection reader;

  // And a transient table exists in the test schema
  ScopedTransientTable table(writer, "E2E_AUTOCOMMIT_RB", "id NUMBER");

  // When the writer disables autocommit, inserts a row, and rolls back
  ScopedAutocommit ac{writer, false};
  writer.execute("INSERT INTO " + table.name() + " VALUES (1)");
  REQUIRE(end_tran(writer, SQL_ROLLBACK) == SQL_SUCCESS);

  // Then a reader session sees zero rows
  REQUIRE(count_rows(reader, table.name()) == 0);
}

TEST_CASE("should publish committed inserts to other sessions", "[session][autocommit]") {
  // Given Snowflake client is logged in
  Connection writer;
  Connection reader;

  // And a transient table exists in the test schema
  ScopedTransientTable table(writer, "E2E_AUTOCOMMIT_CM", "id NUMBER");

  // When the writer disables autocommit, inserts a row, and commits
  ScopedAutocommit ac{writer, false};
  writer.execute("INSERT INTO " + table.name() + " VALUES (1)");
  REQUIRE(end_tran(writer, SQL_COMMIT) == SQL_SUCCESS);

  // Then a reader session sees one row
  REQUIRE(count_rows(reader, table.name()) == 1);
}
