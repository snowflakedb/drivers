#ifndef READONLY_DB_FIXTURE_HPP
#define READONLY_DB_FIXTURE_HPP

#include <string>

#include "ODBCFixtures.hpp"
#include "test_setup.hpp"

// =============================================================================
// Readonly Metadata Test Database
// =============================================================================
//
// Pre-provisioned database for ODBC catalog and metadata tests. Contains
// fixed tables, views, and procedures so that tests don't need per-test DDL.
//
// To (re)create the database, configure with -DBUILD_SETUP_TOOLS=ON and run:
//   cmake -B cmake-build -DBUILD_SETUP_TOOLS=ON ... && cmake --build cmake-build
//   ctest --test-dir cmake-build -R setup_readonly_db
//
// Tests MUST NOT modify objects in this database.
// =============================================================================

inline constexpr auto READONLY_DB_NAME = "ODBCMETADATATESTDB";
inline constexpr auto READONLY_SCHEMA_NAME = "CATALOGTESTS";

// =============================================================================
// Object name constants
// =============================================================================

namespace readonly_db {

// Basic tables
inline constexpr auto BASIC_TABLE = "BASICTABLE";
inline constexpr auto MULTI_TYPE_TABLE = "MULTITYPETABLE";
inline constexpr auto THREE_COL_TABLE = "THREECOLTABLE";
inline constexpr auto NULLABILITY_TABLE = "NULLABILITYTABLE";
inline constexpr auto WILDCARD_COL_TABLE = "WILDCARDCOLTABLE";
inline constexpr auto NO_PK_TABLE = "NOPKTABLE";

// Primary key tables
inline constexpr auto SINGLE_PK_TABLE = "SINGLEPKTABLE";
inline constexpr auto COMPOSITE_PK_TABLE = "COMPOSITEPKTABLE";
inline constexpr auto NAMED_PK_TABLE = "NAMEDPKTABLE";

// Foreign key tables
inline constexpr auto FK_PARENT = "FKPARENT";
inline constexpr auto FK_CHILD = "FKCHILD";
inline constexpr auto FK_MULTI_PARENT = "FKMULTIPARENT";
inline constexpr auto FK_MULTI_CHILD_A = "FKMULTICHILDA";
inline constexpr auto FK_MULTI_CHILD_B = "FKMULTICHILDB";

// Views
inline constexpr auto BASIC_VIEW = "BASICVIEW";

// Procedures
inline constexpr auto BASIC_PROC = "BASICPROC";
inline constexpr auto MULTI_PARAM_PROC = "MULTIPARAMPROC";
inline constexpr auto PROC_FILTER = "PROCFILTER";
inline constexpr auto PROC_MULTI_A = "PROCMULTIA";
inline constexpr auto PROC_MULTI_B = "PROCMULTIB";
inline constexpr auto PROC_DTYPE_A = "PROCDTYPEA";
inline constexpr auto PROC_DTYPE_B = "PROCDTYPEB";
inline constexpr auto PROC_NUM_A = "PROCNUMA";
inline constexpr auto PROC_NUM_B = "PROCNUMB";

// Describe-col tables
inline constexpr auto DESC_VARCHAR_TABLE = "DESCVARCHARTABLE";
inline constexpr auto DESC_NUMBER_TABLE = "DESCNUMBERTABLE";
inline constexpr auto DESC_BOOL_TABLE = "DESCBOOLTABLE";
inline constexpr auto DESC_FLOAT_TABLE = "DESCFLOATTABLE";
inline constexpr auto DESC_DATE_TABLE = "DESCDATETABLE";
inline constexpr auto DESC_TIMESTAMP_TABLE = "DESCTIMESTAMPTABLE";
inline constexpr auto DESC_SIZE_VARCHAR_TABLE = "DESCSIZEVARCHARTABLE";
inline constexpr auto DESC_SIZE_NUMBER_TABLE = "DESCSIZENUMBERTABLE";
inline constexpr auto DESC_DIGITS_TABLE = "DESCDIGITSTABLE";
inline constexpr auto DESC_DIGITS_VARCHAR_TABLE = "DESCDIGITSVARCHARTABLE";
inline constexpr auto DESC_NULLABLE_TABLE = "DESCNULLABLETABLE";
inline constexpr auto DESC_NOTNULL_TABLE = "DESCNOTNULLTABLE";
inline constexpr auto DESC_MULTI_TABLE = "DESCMULTITABLE";

}  // namespace readonly_db

// =============================================================================
// Fixture for TEST_CASE_METHOD catalog tests (DSN-based connection)
// =============================================================================

class ReadOnlyDbStmtFixture : public StmtFixture {
 public:
  ReadOnlyDbStmtFixture()
      : StmtFixture(
            DataSourceConfig::Snowflake().set("DATABASE", READONLY_DB_NAME).set("SCHEMA", READONLY_SCHEMA_NAME)) {
    const std::string fqn = std::string(READONLY_DB_NAME) + "." + READONLY_SCHEMA_NAME + "." + readonly_db::BASIC_TABLE;
    const std::string probe = "SELECT 1 FROM " + fqn + " WHERE 1=0";
    SQLRETURN ret = SQLExecDirect(stmt_handle(), sqlchar(probe.c_str()), SQL_NTS);
    if (!SQL_SUCCEEDED(ret)) {
      FAIL("Readonly metadata DB not provisioned (" << fqn
                                                    << " not found). "
                                                       "Build with -DBUILD_SETUP_TOOLS=ON and run: "
                                                       "ctest --test-dir cmake-build -R setup_readonly_db");
    }
    SQLFreeStmt(stmt_handle(), SQL_CLOSE);
  }

  [[nodiscard]] static auto schema_name() { return READONLY_SCHEMA_NAME; }
  [[nodiscard]] static auto database_name() { return READONLY_DB_NAME; }
};

// =============================================================================
// Connection string helper for e2e tests (Connection class pattern)
// =============================================================================

inline std::string get_readonly_db_connection_string() {
  const auto params = get_test_parameters("testconnection");
  std::stringstream ss;
  read_default_params(ss, params, {"DATABASE", "SCHEMA"});
  add_param_required<std::string>(ss, params, "SNOWFLAKE_TEST_PASSWORD", "PWD");
  ss << "DATABASE=" << READONLY_DB_NAME << ";";
  ss << "SCHEMA=" << READONLY_SCHEMA_NAME << ";";
  return ss.str();
}

#endif  // READONLY_DB_FIXTURE_HPP
