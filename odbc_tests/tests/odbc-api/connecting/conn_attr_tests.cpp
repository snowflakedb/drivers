#include <sql.h>
#include <sqlext.h>
#include <sqltypes.h>

#include <cstring>
#include <string>

#include <catch2/catch_test_macros.hpp>
#include <catch2/generators/catch_generators.hpp>

#include "Connection.hpp"
#include "ODBCFixtures.hpp"
#include "compatibility.hpp"
#include "get_diag_rec.hpp"
#include "test_macros.hpp"

// ============================================================================
// SQL_ATTR_ACCESS_MODE (101) — advisory, no connection needed
// ============================================================================

TEST_CASE("should set and get SQL_ATTR_ACCESS_MODE", "[odbc-api][conn_attr][access_mode]") {
  // Given An allocated but unconnected DBC handle
  EnvironmentHandleWrapper env;
  SQLRETURN ret = SQLSetEnvAttr(env.getHandle(), SQL_ATTR_ODBC_VERSION, reinterpret_cast<SQLPOINTER>(SQL_OV_ODBC3), 0);
  REQUIRE(ret == SQL_SUCCESS);
  ConnectionHandleWrapper dbc = env.createConnectionHandle();

  // When SQL_ATTR_ACCESS_MODE is set to each supported value
  const SQLULEN expected_mode = GENERATE(SQL_MODE_READ_WRITE, SQL_MODE_READ_ONLY);
  ret = SQLSetConnectAttr(dbc.getHandle(), SQL_ATTR_ACCESS_MODE, reinterpret_cast<SQLPOINTER>(expected_mode), 0);
  REQUIRE(ret == SQL_SUCCESS);

  // Then Getting the attribute should return the value that was set
  SQLULEN mode = 99;
  ret = SQLGetConnectAttr(dbc.getHandle(), SQL_ATTR_ACCESS_MODE, &mode, 0, nullptr);
  REQUIRE(ret == SQL_SUCCESS);
  CHECK(mode == expected_mode);
}

// ============================================================================
// SQL_ATTR_TXN_ISOLATION (108)
// Note: Both the Windows DM and unixODBC manage SQL_ATTR_TXN_ISOLATION
// internally. The DM caches its own isolation level and may not call the
// driver for GET or SET. These attributes are therefore not directly testable
// through the Driver Manager. Snowflake's effective isolation level
// (READ_COMMITTED) is validated at the SQLGetInfo level instead.
// ============================================================================
// SQL_ATTR_CONNECTION_DEAD (1209) — read-only
// ============================================================================

TEST_CASE("should return SQL_CD_TRUE after disconnect", "[odbc-api][conn_attr][connection_dead][connecting]") {
  // Given A DBC handle that was connected then disconnected
  std::string connection_string = get_connection_string();
  EnvironmentHandleWrapper env;
  SQLRETURN ret = SQLSetEnvAttr(env.getHandle(), SQL_ATTR_ODBC_VERSION, reinterpret_cast<SQLPOINTER>(SQL_OV_ODBC3), 0);
  REQUIRE(ret == SQL_SUCCESS);
  ConnectionHandleWrapper dbc = env.createConnectionHandle();

  // Connect
  ret = SQLDriverConnect(dbc.getHandle(), nullptr, reinterpret_cast<SQLCHAR*>(connection_string.data()), SQL_NTS,
                         nullptr, 0, nullptr, SQL_DRIVER_NOPROMPT);
  REQUIRE(SQL_SUCCEEDED(ret));

  // Verify connected (CD_FALSE) — use CHECK (non-fatal) so SQLDisconnect still runs on failure
  SQLULEN dead = 99;
  ret = SQLGetConnectAttr(dbc.getHandle(), SQL_ATTR_CONNECTION_DEAD, &dead, 0, nullptr);
  CHECK(ret == SQL_SUCCESS);
  CHECK(dead == SQL_CD_FALSE);

  // Disconnect
  ret = SQLDisconnect(dbc.getHandle());
  REQUIRE(ret == SQL_SUCCESS);

  // When SQL_ATTR_CONNECTION_DEAD is queried after disconnect
  dead = 99;
  ret = SQLGetConnectAttr(dbc.getHandle(), SQL_ATTR_CONNECTION_DEAD, &dead, 0, nullptr);

  // Then the driver must return SQL_CD_TRUE.
  // Note: Some Driver Managers (both Windows and unixODBC) may intercept
  // SQLGetConnectAttr after SQLDisconnect and return SQL_ERROR (HY010 or 08003)
  // instead of forwarding the call to our driver. We accept either outcome.
  if (SQL_SUCCEEDED(ret)) {
    CHECK(dead == SQL_CD_TRUE);
  } else {
    // Validate that the failure is for an expected reason: HY010 (function sequence error)
    // or 08003 (connection not open), depending on the DM.
    char sqlstate[6] = {};
    SQLRETURN diag_ret = SQLGetDiagRec(SQL_HANDLE_DBC, dbc.getHandle(), 1, reinterpret_cast<SQLCHAR*>(sqlstate),
                                       nullptr, nullptr, 0, nullptr);
    CHECK(SQL_SUCCEEDED(diag_ret));
    std::string state(sqlstate);
    CHECK((state == "HY010" || state == "08003"));
  }
}

TEST_CASE("should return SQL_CD_FALSE when connected", "[odbc-api][conn_attr][connection_dead][connecting]") {
  // Given A connected DBC handle
  Connection conn;

  // When SQL_ATTR_CONNECTION_DEAD is queried
  SQLULEN dead = 99;
  SQLRETURN ret = SQLGetConnectAttr(conn.handleWrapper().getHandle(), SQL_ATTR_CONNECTION_DEAD, &dead, 0, nullptr);

  // Then It should return SQL_CD_FALSE (0 = alive)
  REQUIRE(ret == SQL_SUCCESS);
  CHECK(dead == SQL_CD_FALSE);
}

TEST_CASE("should reject set on SQL_ATTR_CONNECTION_DEAD with HY092",
          "[odbc-api][conn_attr][connection_dead][error][connecting]") {
  // Given A connected DBC handle
  Connection conn;

  // When Attempting to set SQL_ATTR_CONNECTION_DEAD
  SQLRETURN ret =
      SQLSetConnectAttr(conn.handleWrapper().getHandle(), SQL_ATTR_CONNECTION_DEAD, reinterpret_cast<SQLPOINTER>(0), 0);

  // Then It should return SQL_ERROR with SQLSTATE HY092
  REQUIRE_EXPECTED_ERROR(ret, "HY092", conn.handleWrapper().getHandle(), SQL_HANDLE_DBC);
}

// ============================================================================
// SQL_ATTR_AUTO_IPD (10001) — read-only
// ============================================================================

// Note: SQLGetConnectAttr(SQL_ATTR_AUTO_IPD) is intercepted by both the Windows
// DM and unixODBC, which return their own cached value rather than calling the
// driver. The getter is therefore not testable through the Driver Manager.

TEST_CASE("should reject set on SQL_ATTR_AUTO_IPD with HY092", "[odbc-api][conn_attr][auto_ipd][error][connecting]") {
  // Given A connected DBC handle
  Connection conn;

  // When Attempting to set SQL_ATTR_AUTO_IPD
  SQLRETURN ret =
      SQLSetConnectAttr(conn.handleWrapper().getHandle(), SQL_ATTR_AUTO_IPD, reinterpret_cast<SQLPOINTER>(SQL_TRUE), 0);

  // Then It should return SQL_ERROR with SQLSTATE HY092
  REQUIRE_EXPECTED_ERROR(ret, "HY092", conn.handleWrapper().getHandle(), SQL_HANDLE_DBC);
}

// ============================================================================
// SQL_ATTR_PACKET_SIZE (112) — pre-connect only
// ============================================================================

TEST_CASE("should set and get SQL_ATTR_PACKET_SIZE before connect", "[odbc-api][conn_attr][packet_size]") {
  // Given An allocated but unconnected DBC handle
  EnvironmentHandleWrapper env;
  SQLRETURN ret = SQLSetEnvAttr(env.getHandle(), SQL_ATTR_ODBC_VERSION, reinterpret_cast<SQLPOINTER>(SQL_OV_ODBC3), 0);
  REQUIRE(ret == SQL_SUCCESS);
  ConnectionHandleWrapper dbc = env.createConnectionHandle();

  // When SQL_ATTR_PACKET_SIZE is set before connecting
  ret = SQLSetConnectAttr(dbc.getHandle(), SQL_ATTR_PACKET_SIZE, reinterpret_cast<SQLPOINTER>(4096), 0);
  REQUIRE(ret == SQL_SUCCESS);

  // Then Getting the attribute should return the stored value
  SQLULEN size = 0;
  ret = SQLGetConnectAttr(dbc.getHandle(), SQL_ATTR_PACKET_SIZE, &size, 0, nullptr);
  REQUIRE(ret == SQL_SUCCESS);
  CHECK(size == 4096);
}

// ============================================================================
// SQL_ATTR_QUIET_MODE (111) — window handle pointer
// ============================================================================

TEST_CASE("should set and get SQL_ATTR_QUIET_MODE", "[odbc-api][conn_attr][quiet_mode]") {
  // Given An allocated DBC handle
  EnvironmentHandleWrapper env;
  SQLRETURN ret = SQLSetEnvAttr(env.getHandle(), SQL_ATTR_ODBC_VERSION, reinterpret_cast<SQLPOINTER>(SQL_OV_ODBC3), 0);
  REQUIRE(ret == SQL_SUCCESS);
  ConnectionHandleWrapper dbc = env.createConnectionHandle();

  // When SQL_ATTR_QUIET_MODE is set to a pointer value
  SQLPOINTER hwnd = reinterpret_cast<SQLPOINTER>(0xDEADBEEF);
  ret = SQLSetConnectAttr(dbc.getHandle(), SQL_ATTR_QUIET_MODE, hwnd, 0);
  REQUIRE(ret == SQL_SUCCESS);

  // Then Getting the attribute should return the same pointer
  SQLPOINTER result = nullptr;
  ret = SQLGetConnectAttr(dbc.getHandle(), SQL_ATTR_QUIET_MODE, &result, 0, nullptr);
  REQUIRE(ret == SQL_SUCCESS);
  CHECK(result == hwnd);
}

// ============================================================================
// SQL_ATTR_CURRENT_CATALOG (109) — requires connection
// ============================================================================

TEST_CASE("should get SQL_ATTR_CURRENT_CATALOG when connected", "[odbc-api][conn_attr][current_catalog][connecting]") {
  // Given A connected DBC handle
  Connection conn;

  // When SQL_ATTR_CURRENT_CATALOG is queried
  char catalog[256] = {};
  SQLINTEGER catalog_len = 0;
  SQLRETURN ret = SQLGetConnectAttr(conn.handleWrapper().getHandle(), SQL_ATTR_CURRENT_CATALOG, catalog,
                                    sizeof(catalog), &catalog_len);

  // Then It should return the active database name
  REQUIRE(SQL_SUCCEEDED(ret));
  CHECK(catalog_len > 0);
  CHECK(std::string(catalog).length() > 0);
}

TEST_CASE("should return 08003 for SQL_ATTR_CURRENT_CATALOG when not connected",
          "[odbc-api][conn_attr][current_catalog][error]") {
  // Given An unconnected DBC handle
  EnvironmentHandleWrapper env;
  SQLRETURN ret = SQLSetEnvAttr(env.getHandle(), SQL_ATTR_ODBC_VERSION, reinterpret_cast<SQLPOINTER>(SQL_OV_ODBC3), 0);
  REQUIRE(ret == SQL_SUCCESS);
  ConnectionHandleWrapper dbc = env.createConnectionHandle();

  // When SQL_ATTR_CURRENT_CATALOG is queried without a connection
  char catalog[256] = {};
  ret = SQLGetConnectAttr(dbc.getHandle(), SQL_ATTR_CURRENT_CATALOG, catalog, sizeof(catalog), nullptr);

  // Then It should return SQL_ERROR with SQLSTATE 08003
  REQUIRE_EXPECTED_ERROR(ret, "08003", dbc.getHandle(), SQL_HANDLE_DBC);
}

TEST_CASE("should set SQL_ATTR_CURRENT_CATALOG to current value successfully",
          "[odbc-api][conn_attr][current_catalog][connecting]") {
  // Given A connected DBC handle with a known current catalog
  Connection conn;
  char initial_catalog[256] = {};
  SQLINTEGER initial_len = 0;
  SQLRETURN ret = SQLGetConnectAttr(conn.handleWrapper().getHandle(), SQL_ATTR_CURRENT_CATALOG, initial_catalog,
                                    sizeof(initial_catalog), &initial_len);
  REQUIRE(SQL_SUCCEEDED(ret));
  REQUIRE(initial_len > 0);

  // When SQL_ATTR_CURRENT_CATALOG is set to the same catalog (no-op round-trip smoke test).
  // Note: this test verifies that SET succeeds and GET remains consistent after the call,
  // but it does NOT verify that USE DATABASE switches to a different catalog — a regression
  // where the driver skips execution when names match would still pass. A full switch test
  // requires a second known-valid catalog in the environment (see 3D000 test for proof
  // that the driver actually communicates with the server on SET).
  ret = SQLSetConnectAttr(conn.handleWrapper().getHandle(), SQL_ATTR_CURRENT_CATALOG,
                          reinterpret_cast<SQLPOINTER>(initial_catalog), SQL_NTS);

  // Then It should succeed and the catalog should remain the same
  REQUIRE(SQL_SUCCEEDED(ret));
  char result_catalog[256] = {};
  SQLINTEGER result_len = 0;
  ret = SQLGetConnectAttr(conn.handleWrapper().getHandle(), SQL_ATTR_CURRENT_CATALOG, result_catalog,
                          sizeof(result_catalog), &result_len);
  REQUIRE(SQL_SUCCEEDED(ret));
  CHECK(std::string(result_catalog) == std::string(initial_catalog));

  // Verify the catalog reported by the driver matches the server's view.
  // Note: this confirms consistency between driver and server, but does not prove that a
  // USE DATABASE round-trip was made (a driver caching the name and skipping the server call
  // would still pass — see the note above about what this test does and does not verify).
  {
    char db_from_server[256];
    std::memset(db_from_server, 0xFF, sizeof(db_from_server));
    SQLLEN cb = 0;
    auto stmt = conn.execute_fetch("SELECT CURRENT_DATABASE()");
    REQUIRE(SQL_SUCCEEDED(SQLGetData(stmt.getHandle(), 1, SQL_C_CHAR, db_from_server, sizeof(db_from_server), &cb)));
    REQUIRE(cb != SQL_NULL_DATA);
    CHECK(std::string(db_from_server) == std::string(initial_catalog));
  }
}

#if !defined(_WIN32)
// The Windows Driver Manager intercepts SQL_ATTR_CURRENT_CATALOG set calls and
// stores the value without calling the driver, returning SQL_SUCCESS regardless
// of whether the catalog exists. This test can only run on non-Windows platforms
// where the DM passes the call through to our driver.
TEST_CASE("should return 3D000 when setting SQL_ATTR_CURRENT_CATALOG to nonexistent database",
          "[odbc-api][conn_attr][current_catalog][error][connecting]") {
  // Given A connected DBC handle
  Connection conn;

  // When SQL_ATTR_CURRENT_CATALOG is set to a database that does not exist
  char bad_catalog[] = "NONEXISTENT_DATABASE_XYZZY_ODBC_TEST";
  SQLRETURN ret = SQLSetConnectAttr(conn.handleWrapper().getHandle(), SQL_ATTR_CURRENT_CATALOG,
                                    reinterpret_cast<SQLPOINTER>(bad_catalog), SQL_NTS);

  // Then:
  // - New driver: validates the catalog against the server and explicitly returns 3D000.
  // - Old driver: executes USE "<db>" on the server via the Simba SDK; the resulting Snowflake
  //   server error is propagated as SQL_ERROR, but with HY000/42000 rather than 3D000 (the
  //   Simba framework does not map USE failures to the ODBC 3D000 state).
  if (get_driver_type() == DRIVER_TYPE::NEW) {
    REQUIRE_EXPECTED_ERROR(ret, "3D000", conn.handleWrapper().getHandle(), SQL_HANDLE_DBC);
  } else {
    CHECK(ret == SQL_ERROR);
  }
}
#endif  // !defined(_WIN32)

// ============================================================================
// SQL_ATTR_AUTOCOMMIT (102) — default state
// ============================================================================

TEST_CASE("should get SQL_ATTR_AUTOCOMMIT default as SQL_AUTOCOMMIT_ON", "[odbc-api][conn_attr][autocommit]") {
  // Given An allocated DBC handle (not connected)
  EnvironmentHandleWrapper env;
  SQLRETURN ret = SQLSetEnvAttr(env.getHandle(), SQL_ATTR_ODBC_VERSION, reinterpret_cast<SQLPOINTER>(SQL_OV_ODBC3), 0);
  REQUIRE(ret == SQL_SUCCESS);
  ConnectionHandleWrapper dbc = env.createConnectionHandle();

  // When SQL_ATTR_AUTOCOMMIT is queried
  SQLULEN autocommit = 99;
  ret = SQLGetConnectAttr(dbc.getHandle(), SQL_ATTR_AUTOCOMMIT, &autocommit, 0, nullptr);

  // Then It should return SQL_AUTOCOMMIT_ON by default
  REQUIRE(ret == SQL_SUCCESS);
  CHECK(autocommit == SQL_AUTOCOMMIT_ON);
}

TEST_CASE("should set SQL_ATTR_AUTOCOMMIT to OFF on a live connection",
          "[odbc-api][conn_attr][autocommit][connecting]") {
  // Given A connected DBC handle
  Connection conn;

  // When SQL_ATTR_AUTOCOMMIT is set to OFF
  SQLRETURN ret = SQLSetConnectAttr(conn.handleWrapper().getHandle(), SQL_ATTR_AUTOCOMMIT,
                                    reinterpret_cast<SQLPOINTER>(SQL_AUTOCOMMIT_OFF), 0);
  REQUIRE(ret == SQL_SUCCESS);

  // Then Getting the attribute should return SQL_AUTOCOMMIT_OFF
  SQLULEN autocommit = 99;
  ret = SQLGetConnectAttr(conn.handleWrapper().getHandle(), SQL_ATTR_AUTOCOMMIT, &autocommit, 0, nullptr);
  REQUIRE(ret == SQL_SUCCESS);
  CHECK(autocommit == SQL_AUTOCOMMIT_OFF);
}

TEST_CASE("should set SQL_ATTR_AUTOCOMMIT back to ON on a live connection",
          "[odbc-api][conn_attr][autocommit][connecting]") {
  // Given A connected DBC handle with autocommit explicitly disabled first
  Connection conn;
  SQLRETURN ret = SQLSetConnectAttr(conn.handleWrapper().getHandle(), SQL_ATTR_AUTOCOMMIT,
                                    reinterpret_cast<SQLPOINTER>(SQL_AUTOCOMMIT_OFF), 0);
  REQUIRE(ret == SQL_SUCCESS);

  // When SQL_ATTR_AUTOCOMMIT is restored to ON
  ret = SQLSetConnectAttr(conn.handleWrapper().getHandle(), SQL_ATTR_AUTOCOMMIT,
                          reinterpret_cast<SQLPOINTER>(SQL_AUTOCOMMIT_ON), 0);
  REQUIRE(ret == SQL_SUCCESS);

  // Then Getting the attribute should return SQL_AUTOCOMMIT_ON
  SQLULEN autocommit = 99;
  ret = SQLGetConnectAttr(conn.handleWrapper().getHandle(), SQL_ATTR_AUTOCOMMIT, &autocommit, 0, nullptr);
  REQUIRE(ret == SQL_SUCCESS);
  CHECK(autocommit == SQL_AUTOCOMMIT_ON);
}
