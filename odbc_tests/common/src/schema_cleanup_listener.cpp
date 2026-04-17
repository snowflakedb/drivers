#include <sql.h>
#include <sqlext.h>

#include <iostream>
#include <string>

#include <catch2/reporters/catch_reporter_event_listener.hpp>
#include <catch2/reporters/catch_reporter_registrars.hpp>

#include "ODBCConfig.hpp"
#include "Schema.hpp"
#include "odbc_cast.hpp"

// Catch2 event listener that drops per-process schemas in fallback mode.
//
// testRunEnded fires during normal Catch2 execution, before main() returns
// and before Rust TLS teardown — so ODBC calls are still safe here (unlike
// std::atexit which fires too late).
//
// IMPORTANT: This listener must NOT use the Connection class or any function
// that calls Catch2 assertion macros (REQUIRE, CHECK, FAIL, etc.), because
// testRunEnded fires outside any test case context.  Catch2 assertions require
// an active test case; calling them here dereferences a null IResultCapture*
// and SEGFAULTs.
//
// Skips cleanup when:
//  - ODBC_TEST_SCHEMA is set (runner script handles the shared schema)
//  - Schema was never initiated in this process (no schema to drop)
struct SchemaCleanupListener : Catch::EventListenerBase {
  using EventListenerBase::EventListenerBase;

  void testRunEnded(const Catch::TestRunStats&) override {
    if (Schema::is_external_schema() || !Schema::was_initiated()) {
      return;
    }
    try {
      drop_schema_raw(Schema::name());
    } catch (const std::exception& e) {
      std::cerr << "SchemaCleanupListener: failed to drop schema " << Schema::name() << ": " << e.what() << "\n";
    } catch (...) {
      std::cerr << "SchemaCleanupListener: failed to drop schema " << Schema::name() << ": unknown error\n";
    }
  }

 private:
  static void drop_schema_raw(const std::string& schema_name) {
    auto dsn_config = DataSourceConfig::Snowflake();
    [[maybe_unused]] auto installation = dsn_config.install();
    std::string conn_str = dsn_config.connection_string();

    SQLHENV env = SQL_NULL_HENV;
    SQLHDBC dbc = SQL_NULL_HDBC;

    if (!SQL_SUCCEEDED(SQLAllocHandle(SQL_HANDLE_ENV, SQL_NULL_HANDLE, &env))) {
      return;
    }
    if (!SQL_SUCCEEDED(SQLSetEnvAttr(env, SQL_ATTR_ODBC_VERSION, reinterpret_cast<SQLPOINTER>(SQL_OV_ODBC3), 0))) {
      std::cerr << "SchemaCleanupListener: SQLSetEnvAttr(SQL_OV_ODBC3) failed\n";
      SQLFreeHandle(SQL_HANDLE_ENV, env);
      return;
    }

    if (!SQL_SUCCEEDED(SQLAllocHandle(SQL_HANDLE_DBC, env, &dbc))) {
      SQLFreeHandle(SQL_HANDLE_ENV, env);
      return;
    }

    SQLRETURN ret =
        SQLDriverConnect(dbc, nullptr, sqlchar(conn_str.c_str()), SQL_NTS, nullptr, 0, nullptr, SQL_DRIVER_NOPROMPT);
    if (!SQL_SUCCEEDED(ret)) {
      SQLFreeHandle(SQL_HANDLE_DBC, dbc);
      SQLFreeHandle(SQL_HANDLE_ENV, env);
      return;
    }

    std::string sql = "DROP SCHEMA IF EXISTS " + schema_name + " CASCADE";
    SQLHSTMT stmt = SQL_NULL_HSTMT;
    if (SQL_SUCCEEDED(SQLAllocHandle(SQL_HANDLE_STMT, dbc, &stmt))) {
      SQLExecDirect(stmt, sqlchar(sql.c_str()), SQL_NTS);
      SQLFreeHandle(SQL_HANDLE_STMT, stmt);
    }

    SQLDisconnect(dbc);
    SQLFreeHandle(SQL_HANDLE_DBC, dbc);
    SQLFreeHandle(SQL_HANDLE_ENV, env);
  }
};

CATCH_REGISTER_LISTENER(SchemaCleanupListener)
