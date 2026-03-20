#include <sql.h>
#include <sqlext.h>
#include <sqltypes.h>

#include <array>
#include <string>

#include <catch2/catch_test_macros.hpp>

#include "HandleWrapper.hpp"
#include "ODBCConfig.hpp"
#include "get_diag_rec.hpp"
#include "odbc_matchers.hpp"
#include "test_setup.hpp"

// ── Helpers ──────────────────────────────────────────────────────────────────

static EnvironmentHandleWrapper make_env() {
  EnvironmentHandleWrapper env;
  SQLRETURN ret = SQLSetEnvAttr(env.getHandle(), SQL_ATTR_ODBC_VERSION, (SQLPOINTER)SQL_OV_ODBC3, 0);
  REQUIRE(ret == SQL_SUCCESS);
  return env;
}

static void verify_select_one(ConnectionHandleWrapper& dbc) {
  StatementHandleWrapper stmt = dbc.createStatementHandle();
  SQLRETURN ret = SQLExecDirect(stmt.getHandle(), (SQLCHAR*)"SELECT 1", SQL_NTS);
  REQUIRE_ODBC(ret, stmt);

  ret = SQLFetch(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);

  SQLINTEGER result = 0;
  ret = SQLGetData(stmt.getHandle(), 1, SQL_C_LONG, &result, sizeof(result), nullptr);
  REQUIRE_ODBC(ret, stmt);
  REQUIRE(result == 1);
}

// ── SQLConnect tests ──────────────────────────────────────────────────────────

TEST_CASE("SQLConnect connects via DSN with all credentials in DSN", "[connection]") {
  // Given A DSN is installed with all connection parameters
  auto installation = DataSourceConfig::Snowflake().install();
  const std::string dsn = installation.dsn_name();

  auto env = make_env();
  auto dbc = env.createConnectionHandle();

  // When SQLConnect is called with the DSN name and no explicit credentials
  SQLRETURN ret = SQLConnect(dbc.getHandle(), (SQLCHAR*)dsn.c_str(), SQL_NTS,
                             /*UID=*/nullptr, 0,
                             /*PWD=*/nullptr, 0);

  // Then The connection succeeds and a simple query can be executed
  REQUIRE_ODBC(ret, dbc);
  verify_select_one(dbc);

  SQLDisconnect(dbc.getHandle());
}

TEST_CASE("SQLConnect returns IM002 for an unknown DSN", "[connection]") {
  // Given No DSN named NonExistentDSN exists
  static std::shared_ptr<DriverConfig> driver = DriverConfig::Default();
  static auto installation = ConfigInstallation::install_driver(driver);

  auto env = make_env();
  auto dbc = env.createConnectionHandle();

  // When SQLConnect is called with DSN NonExistentDSN
  const std::string bogus_dsn = "NonExistentDSN_e2e_test";
  SQLRETURN ret = SQLConnect(dbc.getHandle(), (SQLCHAR*)bogus_dsn.c_str(), SQL_NTS, (SQLCHAR*)"user", SQL_NTS,
                             (SQLCHAR*)"pass", SQL_NTS);

  // Then SQL_ERROR is returned with SQLSTATE IM002
  REQUIRE(ret == SQL_ERROR);
  auto records = get_diag_rec(dbc);
  REQUIRE(!records.empty());
  CHECK(records[0].sqlState == "IM002");
}

// ── SQLBrowseConnect tests ────────────────────────────────────────────────────

TEST_CASE("SQLBrowseConnect returns SQL_NEED_DATA when server info is missing", "[connection]") {
  // Given A connection handle is allocated
  static std::shared_ptr<DriverConfig> driver = DriverConfig::Default();
  static auto installation = ConfigInstallation::install_driver(driver);

  auto env = make_env();
  auto dbc = env.createConnectionHandle();

  std::array<SQLCHAR, 1024> out_buf{};
  SQLSMALLINT out_len = 0;

  // When SQLBrowseConnect is called with an empty connection string
  std::string empty_cs = "DRIVER={" + DriverConfig::get_driver_path() + "};";
  SQLRETURN ret = SQLBrowseConnect(dbc.getHandle(), (SQLCHAR*)empty_cs.c_str(), SQL_NTS, out_buf.data(),
                                   static_cast<SQLSMALLINT>(out_buf.size()), &out_len);

  // Then SQL_NEED_DATA is returned and the output contains a connection template
  REQUIRE(ret == SQL_NEED_DATA);
  REQUIRE(out_len > 0);
  std::string out_str(reinterpret_cast<char*>(out_buf.data()));
  CHECK(!out_str.empty());
}
