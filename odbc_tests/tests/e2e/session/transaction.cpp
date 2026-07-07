#include <sql.h>
#include <sqlext.h>

#include <string>

#include <catch2/catch_test_macros.hpp>

#include "Connection.hpp"
#include "ScopedTable.hpp"
#include "compatibility.hpp"
#include "get_data.hpp"
#include "test_setup.hpp"

// End-to-end transaction tests. These exercise commit/rollback semantics across
// two independent sessions, which the odbc-api SQLEndTran tests (single session,
// TEMPORARY tables) do not cover. They run against both the old and new drivers.

namespace {

// Row count of `table` as observed by `conn` in its own (committed) view.
SQLINTEGER count_rows(Connection& conn, const std::string& table) {
  auto stmt = conn.execute_fetch("SELECT COUNT(*) FROM " + table);
  return get_data<SQL_C_SLONG>(stmt, 1);
}

void set_autocommit(Connection& conn, bool on) {
  SQLPOINTER value =
      on ? reinterpret_cast<SQLPOINTER>(SQL_AUTOCOMMIT_ON) : reinterpret_cast<SQLPOINTER>(SQL_AUTOCOMMIT_OFF);
  SQLRETURN ret = SQLSetConnectAttr(conn.handleWrapper().getHandle(), SQL_ATTR_AUTOCOMMIT, value, 0);
  REQUIRE(ret == SQL_SUCCESS);
}

// RAII guard that flips autocommit on a connection for the test body and
// restores SQL_AUTOCOMMIT_ON on destruction. Without this, a REQUIRE failure
// mid-test leaves the connection in SQL_AUTOCOMMIT_OFF and corrupts the next
// test that reuses it. The destructor swallows any failure (Catch's REQUIRE
// throws) so we don't terminate during stack unwinding.
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

SQLRETURN end_tran(Connection& conn, SQLSMALLINT completion_type) {
  return SQLEndTran(SQL_HANDLE_DBC, conn.handleWrapper().getHandle(), completion_type);
}

}  // namespace

TEST_CASE("should make inserted rows visible to another session after commit", "[e2e][session][transaction]") {
  // Given two independent sessions and a committed empty table
  Connection writer;
  Connection reader;
  // Permanent table so a second session can see it (committed at create time).
  ScopedTable table(writer, "E2E_TXN_COMMIT", "id INT");

  // When the writer inserts a row inside a manual transaction
  ScopedAutocommit ac{writer, false};
  writer.execute("INSERT INTO " + table.name() + " VALUES (1)");

  // Then the open transaction is invisible to the other session before commit
  CHECK(count_rows(reader, table.name()) == 0);

  // When the writer commits
  REQUIRE(end_tran(writer, SQL_COMMIT) == SQL_SUCCESS);

  // Then the row becomes visible to the other session
  CHECK(count_rows(reader, table.name()) == 1);
}

TEST_CASE("should discard inserted rows for both sessions on rollback", "[e2e][session][transaction]") {
  // Given two sessions and a committed empty table
  Connection writer;
  Connection reader;
  ScopedTable table(writer, "E2E_TXN_ROLLBACK", "id INT");

  // When the writer inserts rows in a manual transaction and rolls back
  ScopedAutocommit ac{writer, false};
  writer.execute("INSERT INTO " + table.name() + " VALUES (1)");
  writer.execute("INSERT INTO " + table.name() + " VALUES (2)");
  REQUIRE(end_tran(writer, SQL_ROLLBACK) == SQL_SUCCESS);

  // Then no rows remain for either session
  CHECK(count_rows(writer, table.name()) == 0);
  CHECK(count_rows(reader, table.name()) == 0);
}

TEST_CASE("should preserve committed rows after a later rollback", "[e2e][session][transaction]") {
  // Given a session with a committed empty table
  Connection writer;
  ScopedTable table(writer, "E2E_TXN_MIXED", "id INT");

  // When one row is committed and a second row is rolled back
  ScopedAutocommit ac{writer, false};
  writer.execute("INSERT INTO " + table.name() + " VALUES (10)");
  REQUIRE(end_tran(writer, SQL_COMMIT) == SQL_SUCCESS);
  writer.execute("INSERT INTO " + table.name() + " VALUES (20)");
  REQUIRE(end_tran(writer, SQL_ROLLBACK) == SQL_SUCCESS);

  // Then only the committed row remains
  CHECK(count_rows(writer, table.name()) == 1);
}

TEST_CASE("should commit each statement immediately in autocommit mode", "[e2e][session][transaction]") {
  // Given two sessions and a committed empty table, with default autocommit ON
  Connection writer;
  Connection reader;
  ScopedTable table(writer, "E2E_TXN_AUTOCOMMIT", "id INT");

  // When the writer inserts a row without an explicit SQLEndTran
  writer.execute("INSERT INTO " + table.name() + " VALUES (1)");

  // Then the row is immediately visible to the other session
  CHECK(count_rows(reader, table.name()) == 1);
}
