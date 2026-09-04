#include <sql.h>
#include <sqlext.h>

#include <array>
#include <string>

#include <catch2/catch_test_macros.hpp>

#include "Connection.hpp"
#include "compatibility.hpp"
#include "get_data.hpp"
#include "test_setup.hpp"

TEST_CASE("should forward unrecognized connection option as session parameter", "[session]") {
  SKIP_OLD_DRIVER("", "Old driver does not forward unknown connection options as session parameters");
  // Given Snowflake client is logged in with connection option TIMEZONE set to
  // "Europe/Warsaw"
  auto conn_str = get_connection_string() + "TIMEZONE=Europe/Warsaw;";
  Connection conn(conn_str);

  // When Query "SHOW PARAMETERS LIKE 'TIMEZONE'" is executed
  auto stmt = conn.execute_fetch("SHOW PARAMETERS LIKE 'TIMEZONE'");

  // Then the session parameter value should be "Europe/Warsaw"
  auto value = get_data<SQL_C_CHAR>(stmt, 2);  // SHOW PARAMETERS: value is column 2
  CHECK(value == "Europe/Warsaw");
}

TEST_CASE("should enable session keep-alive via connection string", "[session]") {
  // Given Snowflake client is logged in with connection option CLIENT_SESSION_KEEP_ALIVE set to "true"
  auto conn_str = get_connection_string() + "CLIENT_SESSION_KEEP_ALIVE=true;";
  Connection conn(conn_str);

  // When Query "SHOW PARAMETERS LIKE 'CLIENT_SESSION_KEEP_ALIVE'" is executed
  auto stmt = conn.execute_fetch("SHOW PARAMETERS LIKE 'CLIENT_SESSION_KEEP_ALIVE'");

  // Then the session parameter value should be "true"
  auto value = get_data<SQL_C_CHAR>(stmt, 2);
  CHECK(value == "true");
}

TEST_CASE("should set heartbeat frequency via connection string", "[session]") {
  // Given Snowflake client is logged in with CLIENT_SESSION_KEEP_ALIVE=true and
  // CLIENT_SESSION_KEEP_ALIVE_HEARTBEAT_FREQUENCY=1800
  auto conn_str = get_connection_string() +
                  "CLIENT_SESSION_KEEP_ALIVE=true;"
                  "CLIENT_SESSION_KEEP_ALIVE_HEARTBEAT_FREQUENCY=1800;";
  Connection conn(conn_str);

  // When Query "SHOW PARAMETERS LIKE 'CLIENT_SESSION_KEEP_ALIVE_HEARTBEAT_FREQUENCY'" is executed
  auto stmt = conn.execute_fetch("SHOW PARAMETERS LIKE 'CLIENT_SESSION_KEEP_ALIVE_HEARTBEAT_FREQUENCY'");

  // Then the session parameter value reflects the configured frequency
  auto value = get_data<SQL_C_CHAR>(stmt, 2);
  NEW_DRIVER_ONLY("BD#57") { CHECK(value == "1800"); }
  OLD_DRIVER_ONLY("BD#57") { CHECK(value == "3600"); }
}

TEST_CASE("should report canonical AUTOCOMMIT values through SQLGetConnectAttr", "[session][autocommit]") {
  struct TestCase {
    const char* value;
    SQLULEN expected;
  };
  const std::array<TestCase, 4> cases = {{
      {"TRUE", SQL_AUTOCOMMIT_ON},
      {"true", SQL_AUTOCOMMIT_ON},
      {"FALSE", SQL_AUTOCOMMIT_OFF},
      {"false", SQL_AUTOCOMMIT_OFF},
  }};

  // Given Snowflake client is logged in
  Connection conn;

  for (const auto& test_case : cases) {
    INFO("AUTOCOMMIT=" << test_case.value);

    // When AUTOCOMMIT is set to <value> with ALTER SESSION
    conn.execute(std::string("ALTER SESSION SET AUTOCOMMIT = ") + test_case.value);

    // Then SQLGetConnectAttr should report <expected>
    SQLULEN autocommit = 99;
    SQLRETURN ret = SQLGetConnectAttr(conn.handleWrapper().getHandle(), SQL_ATTR_AUTOCOMMIT, &autocommit, 0, nullptr);

    REQUIRE(ret == SQL_SUCCESS);
    CHECK(autocommit == test_case.expected);
  }
}

TEST_CASE("should reject non-boolean session parameter values", "[session][boolean-parameters]") {
  struct TestCase {
    const char* parameter;
    const char* value;
  };
  const std::array<TestCase, 12> cases = {{
      {"AUTOCOMMIT", "1"},
      {"AUTOCOMMIT", "'1'"},
      {"AUTOCOMMIT", "'on'"},
      {"AUTOCOMMIT", "'yes'"},
      {"ODBC_TREAT_DECIMAL_AS_INT", "1"},
      {"ODBC_TREAT_DECIMAL_AS_INT", "'1'"},
      {"ODBC_TREAT_DECIMAL_AS_INT", "'on'"},
      {"ODBC_TREAT_DECIMAL_AS_INT", "'yes'"},
      {"ODBC_TREAT_BIG_NUMBER_AS_STRING", "1"},
      {"ODBC_TREAT_BIG_NUMBER_AS_STRING", "'1'"},
      {"ODBC_TREAT_BIG_NUMBER_AS_STRING", "'on'"},
      {"ODBC_TREAT_BIG_NUMBER_AS_STRING", "'yes'"},
  }};

  // Given Snowflake client is logged in
  Connection conn;

  for (const auto& test_case : cases) {
    INFO(test_case.parameter << "=" << test_case.value);

    // When ALTER SESSION sets <parameter> to <value>
    auto stmt = conn.createStatement();
    std::string query = std::string("ALTER SESSION SET ") + test_case.parameter + " = " + test_case.value;
    SQLRETURN ret = SQLExecDirect(stmt.getHandle(), reinterpret_cast<SQLCHAR*>(query.data()), SQL_NTS);

    // Then the statement should fail with SQL_ERROR
    CHECK(ret == SQL_ERROR);
  }
}
