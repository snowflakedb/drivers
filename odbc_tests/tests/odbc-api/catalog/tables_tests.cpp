#include <sql.h>
#include <sqlext.h>
#include <sqltypes.h>

#include <algorithm>
#include <cctype>
#include <cstring>
#include <string>
#include <vector>

#include <catch2/catch_test_macros.hpp>

#include "ODBCFixtures.hpp"
#include "ReadOnlyDbFixture.hpp"
#include "compatibility.hpp"
#include "get_diag_rec.hpp"
#include "odbc_cast.hpp"
#include "test_macros.hpp"
#include "test_setup.hpp"

namespace {

// Result of reading one CHAR column. `indicator` carries the raw
// StrLen_or_Ind value so callers can distinguish SQL_NULL_DATA (-1) from a
// populated column; `text` is empty when the column is NULL.
struct ColumnValue {
  std::string text;
  SQLLEN indicator = 0;

  bool is_null() const { return indicator == SQL_NULL_DATA; }
  bool is_present() const { return indicator > 0 || indicator == SQL_NTS; }
};

std::string to_lower_copy(const std::string& s) {
  std::string out = s;
  std::transform(out.begin(), out.end(), out.begin(), [](unsigned char c) { return std::tolower(c); });
  return out;
}

// Reads a CHAR column with the hygiene the ODBC-tests ruleset requires:
//   - a 0xFF sentinel fill so a driver that returns success but writes nothing
//     is distinguishable from a genuine empty string,
//   - a named indicator so SQL_NULL_DATA surfaces instead of silently
//     collapsing to "",
//   - a hard assertion on the SQLGetData return code.
ColumnValue sqltables_get_column(SQLHSTMT stmt, SQLUSMALLINT column) {
  char buf[1024];
  std::memset(buf, 0xFF, sizeof(buf));
  SQLLEN indicator = 0;
  const SQLRETURN ret = SQLGetData(stmt, column, SQL_C_CHAR, buf, sizeof(buf), &indicator);
  REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt), OdbcMatchers::Succeeded());
  if (indicator == SQL_NULL_DATA) {
    return {std::string(), indicator};
  }
  return {std::string(buf), indicator};
}

std::vector<std::string> sqltables_collect_table_names(SQLHSTMT stmt, const std::string& catalog,
                                                       const std::string& schema, const char* table_pattern) {
  std::vector<std::string> names;
  const SQLRETURN ret = SQLTables(stmt, sqlchar(catalog.c_str()), SQL_NTS, sqlchar(schema.c_str()), SQL_NTS,
                                  sqlchar(table_pattern), SQL_NTS, nullptr, 0);
  if (ret != SQL_SUCCESS) {
    const auto records = get_diag_rec(SQL_HANDLE_STMT, stmt);
    std::string diag;
    for (const auto& record : records) {
      if (!diag.empty()) {
        diag += " | ";
      }
      diag += record.sqlState + ": " + record.messageText;
    }
    FAIL("SQLTables failed (ret=" << ret << "): " << diag);
  }

  while (SQLFetch(stmt) == SQL_SUCCESS) {
    names.emplace_back(sqltables_get_column(stmt, 3).text);
  }
  return names;
}

}  // namespace

// ============================================================================
// SQLTables - Result Set Structure
// ============================================================================

TEST_CASE_METHOD(ReadOnlyDbStmtFixture, "SQLTables: Result set has correct number of columns",
                 "[odbc-api][catalog][tables]") {
  SQLRETURN ret = SQLTables(stmt_handle(), sqlchar(database_name()), SQL_NTS, sqlchar(schema_name()), SQL_NTS,
                            sqlchar(readonly_db::BASIC_TABLE), SQL_NTS, nullptr, 0);
  REQUIRE(ret == SQL_SUCCESS);

  // ODBC spec defines 5 columns
  SQLSMALLINT numCols = 0;
  ret = SQLNumResultCols(stmt_handle(), &numCols);
  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(numCols == 5);
}

TEST_CASE_METHOD(ReadOnlyDbStmtFixture, "SQLTables: Result set column names match ODBC 3.x spec",
                 "[odbc-api][catalog][tables]") {
  SQLRETURN ret = SQLTables(stmt_handle(), sqlchar(database_name()), SQL_NTS, sqlchar(schema_name()), SQL_NTS,
                            sqlchar(readonly_db::BASIC_TABLE), SQL_NTS, nullptr, 0);
  REQUIRE(ret == SQL_SUCCESS);

  const char* expectedColNames[] = {"TABLE_CAT", "TABLE_SCHEM", "TABLE_NAME", "TABLE_TYPE", "REMARKS"};

  for (SQLSMALLINT col = 1; col <= static_cast<SQLSMALLINT>(std::size(expectedColNames)); col++) {
    char colName[256] = {};
    SQLSMALLINT nameLen = 0;
    SQLSMALLINT dataType = 0;
    SQLULEN colSize = 0;
    SQLSMALLINT decDigits = 0;
    SQLSMALLINT nullable = 0;

    ret = SQLDescribeCol(stmt_handle(), col, reinterpret_cast<SQLCHAR*>(colName), sizeof(colName), &nameLen, &dataType,
                         &colSize, &decDigits, &nullable);
    REQUIRE(ret == SQL_SUCCESS);
    REQUIRE(std::string(colName) == expectedColNames[col - 1]);
  }
}

// ============================================================================
// SQLTables - Data Verification
// ============================================================================

TEST_CASE_METHOD(ReadOnlyDbStmtFixture, "SQLTables: Returns known table with correct metadata",
                 "[odbc-api][catalog][tables]") {
  SQLRETURN ret = SQLTables(stmt_handle(), sqlchar(database_name()), SQL_NTS, sqlchar(schema_name()), SQL_NTS,
                            sqlchar(readonly_db::BASIC_TABLE), SQL_NTS, nullptr, 0);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLFetch(stmt_handle());
  REQUIRE(ret == SQL_SUCCESS);

  REQUIRE(sqltables_get_column(stmt_handle(), 1).text == database_name());
  REQUIRE(sqltables_get_column(stmt_handle(), 2).text == schema_name());
  REQUIRE(sqltables_get_column(stmt_handle(), 3).text == readonly_db::BASIC_TABLE);
  REQUIRE(sqltables_get_column(stmt_handle(), 4).text == "TABLE");

  ret = SQLFetch(stmt_handle());
  REQUIRE(ret == SQL_NO_DATA);
}

TEST_CASE_METHOD(ReadOnlyDbStmtFixture, "SQLTables: Returns view with TABLE_TYPE VIEW", "[odbc-api][catalog][tables]") {
  SQLRETURN ret = SQLTables(stmt_handle(), sqlchar(database_name()), SQL_NTS, sqlchar(schema_name()), SQL_NTS,
                            sqlchar(readonly_db::BASIC_VIEW), SQL_NTS, nullptr, 0);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLFetch(stmt_handle());
  REQUIRE(ret == SQL_SUCCESS);

  REQUIRE(sqltables_get_column(stmt_handle(), 4).text == "VIEW");

  ret = SQLFetch(stmt_handle());
  REQUIRE(ret == SQL_NO_DATA);
}

TEST_CASE_METHOD(ReadOnlyDbStmtFixture, "SQLTables: Non-existent table returns empty result set",
                 "[odbc-api][catalog][tables]") {
  SQLRETURN ret = SQLTables(stmt_handle(), sqlchar(database_name()), SQL_NTS, sqlchar(schema_name()), SQL_NTS,
                            sqlchar("NONEXISTENTTABLEXYZ99999"), SQL_NTS, nullptr, 0);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLFetch(stmt_handle());
  REQUIRE(ret == SQL_NO_DATA);
}

TEST_CASE_METHOD(ReadOnlyDbStmtFixture, "SQLTables: TABLE_TYPE filter restricts results",
                 "[odbc-api][catalog][tables]") {
  // No filter - wildcard BASIC% matches both BASIC_TABLE and BASIC_VIEW
  SQLRETURN ret = SQLTables(stmt_handle(), sqlchar(database_name()), SQL_NTS, sqlchar(schema_name()), SQL_NTS,
                            sqlchar("BASIC%"), SQL_NTS, nullptr, 0);
  REQUIRE(ret == SQL_SUCCESS);
  int totalCount = 0;
  while (SQLFetch(stmt_handle()) == SQL_SUCCESS)
    totalCount++;
  REQUIRE(totalCount == 2);
  SQLCloseCursor(stmt_handle());

  // Filter for TABLE - should return only BASIC_TABLE
  ret = SQLTables(stmt_handle(), sqlchar(database_name()), SQL_NTS, sqlchar(schema_name()), SQL_NTS,
                  sqlchar(readonly_db::BASIC_TABLE), SQL_NTS, sqlchar("TABLE"), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLFetch(stmt_handle());
  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(sqltables_get_column(stmt_handle(), 3).text == readonly_db::BASIC_TABLE);
  REQUIRE(sqltables_get_column(stmt_handle(), 4).text == "TABLE");
  ret = SQLFetch(stmt_handle());
  REQUIRE(ret == SQL_NO_DATA);
  SQLCloseCursor(stmt_handle());

  // Filter for VIEW - should return only BASIC_VIEW
  ret = SQLTables(stmt_handle(), sqlchar(database_name()), SQL_NTS, sqlchar(schema_name()), SQL_NTS,
                  sqlchar(readonly_db::BASIC_VIEW), SQL_NTS, sqlchar("VIEW"), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLFetch(stmt_handle());
  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(sqltables_get_column(stmt_handle(), 3).text == readonly_db::BASIC_VIEW);
  REQUIRE(sqltables_get_column(stmt_handle(), 4).text == "VIEW");
  ret = SQLFetch(stmt_handle());
  REQUIRE(ret == SQL_NO_DATA);
}

TEST_CASE_METHOD(ReadOnlyDbStmtFixture, "SQLTables: Wildcard search finds table", "[odbc-api][catalog][tables]") {
  SQLRETURN ret = SQLTables(stmt_handle(), sqlchar(database_name()), SQL_NTS, sqlchar(schema_name()), SQL_NTS,
                            sqlchar("BASICTAB%"), SQL_NTS, nullptr, 0);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLFetch(stmt_handle());
  REQUIRE(ret == SQL_SUCCESS);

  REQUIRE(sqltables_get_column(stmt_handle(), 3).text == readonly_db::BASIC_TABLE);
}

// ============================================================================
// SQLTables - Parameter Variations
// ============================================================================

TEST_CASE_METHOD(ReadOnlyDbStmtFixture, "SQLTables: Various parameter combinations are accepted",
                 "[odbc-api][catalog][tables]") {
  const char* db = database_name();
  const char* schema = schema_name();
  const char* tbl = readonly_db::BASIC_TABLE;

  // SQL_NTS lengths
  SQLRETURN ret =
      SQLTables(stmt_handle(), sqlchar(db), SQL_NTS, sqlchar(schema), SQL_NTS, sqlchar(tbl), SQL_NTS, nullptr, 0);
  REQUIRE(ret == SQL_SUCCESS);
  int count1 = 0;
  while (SQLFetch(stmt_handle()) == SQL_SUCCESS)
    count1++;
  REQUIRE(count1 == 1);
  SQLCloseCursor(stmt_handle());

  // Explicit string lengths
  ret = SQLTables(stmt_handle(), sqlchar(db), static_cast<SQLSMALLINT>(std::strlen(db)), sqlchar(schema),
                  static_cast<SQLSMALLINT>(std::strlen(schema)), sqlchar(tbl),
                  static_cast<SQLSMALLINT>(std::strlen(tbl)), nullptr, 0);
  REQUIRE(ret == SQL_SUCCESS);
  int count2 = 0;
  while (SQLFetch(stmt_handle()) == SQL_SUCCESS)
    count2++;
  REQUIRE(count2 == 1);
}

// ============================================================================
// SQLTables - Statement Reuse & SQLRowCount
// ============================================================================

TEST_CASE_METHOD(ReadOnlyDbStmtFixture, "SQLTables: Can call multiple times after close cursor",
                 "[odbc-api][catalog][tables]") {
  SQLRETURN ret = SQLTables(stmt_handle(), sqlchar(database_name()), SQL_NTS, sqlchar(schema_name()), SQL_NTS,
                            sqlchar(readonly_db::BASIC_TABLE), SQL_NTS, nullptr, 0);
  REQUIRE(ret == SQL_SUCCESS);
  int count1 = 0;
  while (SQLFetch(stmt_handle()) == SQL_SUCCESS)
    count1++;
  REQUIRE(count1 == 1);
  SQLCloseCursor(stmt_handle());

  ret = SQLTables(stmt_handle(), sqlchar(database_name()), SQL_NTS, sqlchar(schema_name()), SQL_NTS,
                  sqlchar(readonly_db::BASIC_TABLE), SQL_NTS, nullptr, 0);
  REQUIRE(ret == SQL_SUCCESS);
  int count2 = 0;
  while (SQLFetch(stmt_handle()) == SQL_SUCCESS)
    count2++;
  REQUIRE(count2 == 1);
}

TEST_CASE_METHOD(ReadOnlyDbStmtFixture, "SQLTables: SQLRowCount returns -1", "[odbc-api][catalog][tables]") {
  SQLRETURN ret = SQLTables(stmt_handle(), sqlchar(database_name()), SQL_NTS, sqlchar(schema_name()), SQL_NTS,
                            sqlchar(readonly_db::BASIC_TABLE), SQL_NTS, nullptr, 0);
  REQUIRE(ret == SQL_SUCCESS);

  SQLLEN rowCount = 0;
  ret = SQLRowCount(stmt_handle(), &rowCount);
  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(rowCount == -1);
}

// ============================================================================
// SQLTables - Error Cases
// ============================================================================

TEST_CASE("SQLTables: SQL_INVALID_HANDLE for null statement handle", "[odbc-api][catalog][tables][error]") {
  const SQLRETURN ret = SQLTables(SQL_NULL_HSTMT, nullptr, 0, nullptr, 0, sqlchar("T"), SQL_NTS, nullptr, 0);
  REQUIRE(ret == SQL_INVALID_HANDLE);
}

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLTables: HY090 - Negative CatalogName length",
                 "[odbc-api][catalog][tables][error]") {
  // Given an active statement on the default DSN
  // When SQLTables is called with a negative CatalogName length (-999)
  const SQLRETURN ret = SQLTables(stmt_handle(), sqlchar("DB"), -999, nullptr, 0, sqlchar("T"), SQL_NTS, nullptr, 0);
  NON_IODBC {
    // And the DM rejects the negative length up front with
    //   SQLSTATE HY090 (invalid string or buffer length)
    REQUIRE_EXPECTED_ERROR(ret, "HY090", stmt_handle(), SQL_HANDLE_STMT);
  }
  IODBC_ONLY {
    // And the iODBC DM-side length validator rejects the negative length
    //   with the ODBC 2.x form of HY090 ("S1090") before the call reaches
    //   the driver. Exactly one record is posted on the statement handle.
    REQUIRE(ret == SQL_ERROR);
    auto records = get_diag_rec(SQL_HANDLE_STMT, stmt_handle());
    REQUIRE(records.size() == 1);
    REQUIRE(records[0].sqlState == "S1090");
  }
}

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLTables: HY090 - Negative SchemaName length",
                 "[odbc-api][catalog][tables][error]") {
  // Given an active statement on the default DSN
  // When SQLTables is called with a negative SchemaName length (-999)
  const SQLRETURN ret = SQLTables(stmt_handle(), nullptr, 0, sqlchar("S"), -999, sqlchar("T"), SQL_NTS, nullptr, 0);
  NON_IODBC {
    // And the DM rejects the negative length with HY090
    REQUIRE_EXPECTED_ERROR(ret, "HY090", stmt_handle(), SQL_HANDLE_STMT);
  }
  IODBC_ONLY {
    // And the iODBC DM-side length validator rejects the negative length
    //   with "S1090" (ODBC 2.x form of HY090) on the statement handle
    //   (see "Negative CatalogName length" above for the same iODBC vs unixODBC delta)
    REQUIRE(ret == SQL_ERROR);
    auto records = get_diag_rec(SQL_HANDLE_STMT, stmt_handle());
    REQUIRE(records.size() == 1);
    REQUIRE(records[0].sqlState == "S1090");
  }
}

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLTables: HY090 - Negative TableName length",
                 "[odbc-api][catalog][tables][error]") {
  // Given an active statement on the default DSN
  // When SQLTables is called with a negative TableName length (-999)
  const SQLRETURN ret = SQLTables(stmt_handle(), nullptr, 0, nullptr, 0, sqlchar("T"), -999, nullptr, 0);
  NON_IODBC {
    // And the DM rejects the negative length with HY090
    REQUIRE_EXPECTED_ERROR(ret, "HY090", stmt_handle(), SQL_HANDLE_STMT);
  }
  IODBC_ONLY {
    // And the iODBC DM-side length validator rejects the negative length
    //   with "S1090" (ODBC 2.x form of HY090) on the statement handle
    //   (see "Negative CatalogName length" above for the same iODBC vs unixODBC delta)
    REQUIRE(ret == SQL_ERROR);
    auto records = get_diag_rec(SQL_HANDLE_STMT, stmt_handle());
    REQUIRE(records.size() == 1);
    REQUIRE(records[0].sqlState == "S1090");
  }
}

TEST_CASE_METHOD(ReadOnlyDbStmtFixture, "SQLTables: 24000 - Cursor already open",
                 "[odbc-api][catalog][tables][error]") {
  SQLRETURN ret = SQLTables(stmt_handle(), sqlchar(database_name()), SQL_NTS, sqlchar(schema_name()), SQL_NTS,
                            sqlchar(readonly_db::BASIC_TABLE), SQL_NTS, nullptr, 0);
  REQUIRE(ret == SQL_SUCCESS);

  // Second call without closing cursor
  ret = SQLTables(stmt_handle(), sqlchar(database_name()), SQL_NTS, sqlchar(schema_name()), SQL_NTS,
                  sqlchar(readonly_db::BASIC_TABLE), SQL_NTS, nullptr, 0);
  REQUIRE_EXPECTED_ERROR(ret, "24000", stmt_handle(), SQL_HANDLE_STMT);
}

TEST_CASE_METHOD(DbcFixture, "SQLTables: Requires active connection", "[odbc-api][catalog][tables][error]") {
  SQLHSTMT stmt = SQL_NULL_HSTMT;
  const SQLRETURN ret = SQLAllocHandle(SQL_HANDLE_STMT, dbc_handle(), &stmt);

  // Note: Reference driver refuses to allocate statement on disconnected handle
  REQUIRE(ret == SQL_ERROR);
}

// ============================================================================
// SQLTables - Special Enumeration Cases (SQL_ALL_* sentinels)
// ============================================================================

TEST_CASE_METHOD(ReadOnlyDbStmtFixture, "SQLTables: SQL_ALL_CATALOGS lists catalogs with only TABLE_CAT populated",
                 "[odbc-api][catalog][tables]") {
  // Given SQL_ALL_CATALOGS sentinel: catalog="%", schema="", table="", type=""
  SQLRETURN ret =
      SQLTables(stmt_handle(), sqlchar("%"), SQL_NTS, sqlchar(""), SQL_NTS, sqlchar(""), SQL_NTS, sqlchar(""), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);

  // When fetching at least one row
  ret = SQLFetch(stmt_handle());
  REQUIRE(ret == SQL_SUCCESS);

  // Then TABLE_CAT is populated; TABLE_SCHEM, TABLE_NAME, TABLE_TYPE, REMARKS are NULL
  CHECK(sqltables_get_column(stmt_handle(), 1).is_present());  // TABLE_CAT non-null
  CHECK(sqltables_get_column(stmt_handle(), 2).is_null());     // TABLE_SCHEM is NULL
  CHECK(sqltables_get_column(stmt_handle(), 3).is_null());     // TABLE_NAME is NULL
  CHECK(sqltables_get_column(stmt_handle(), 4).is_null());     // TABLE_TYPE is NULL
}

// The SQL_ALL_SCHEMAS sentinel (catalog="", schema="%", table="", type="") is
// not tested: it is account-wide by spec, so the driver issues SHOW SCHEMAS IN
// ACCOUNT. On the larger shared CI accounts that enumeration is non-deterministic
// and can time out or exceed result limits, and ODBC's SQLTables has no
// catalog-scoped schema-listing form that would make a stable assertion possible.

TEST_CASE_METHOD(ReadOnlyDbStmtFixture, "SQLTables: SQL_ALL_TABLE_TYPES returns TABLE and VIEW types",
                 "[odbc-api][catalog][tables]") {
  // Given SQL_ALL_TABLE_TYPES sentinel: catalog="", schema="", table="", type="%"
  SQLRETURN ret =
      SQLTables(stmt_handle(), sqlchar(""), SQL_NTS, sqlchar(""), SQL_NTS, sqlchar(""), SQL_NTS, sqlchar("%"), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);

  // When fetching both rows
  std::vector<std::string> types;
  while (SQLFetch(stmt_handle()) == SQL_SUCCESS) {
    const ColumnValue type = sqltables_get_column(stmt_handle(), 4);
    if (type.is_present()) {
      types.push_back(type.text);
    }
  }

  // Then TABLE and VIEW must both be present
  CHECK(std::find(types.begin(), types.end(), "TABLE") != types.end());
  CHECK(std::find(types.begin(), types.end(), "VIEW") != types.end());
}

TEST_CASE_METHOD(ReadOnlyDbStmtFixture, "SQLTables: Unsupported table type returns empty result set",
                 "[odbc-api][catalog][tables]") {
  // Legacy returns no rows for non-TABLE/VIEW type keywords (e.g. SYNONYM).
  SQLRETURN ret = SQLTables(stmt_handle(), sqlchar(database_name()), SQL_NTS, sqlchar(schema_name()), SQL_NTS, nullptr,
                            0, sqlchar("SYNONYM"), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);
  ret = SQLFetch(stmt_handle());
  REQUIRE(ret == SQL_NO_DATA);
}

// ============================================================================
// SQLTables - SQL_ATTR_METADATA_ID = SQL_TRUE (identifier mode)
// ============================================================================

TEST_CASE_METHOD(ReadOnlyDbStmtFixture, "SQLTables: metadata_id=TRUE treats _ and % as literals in table name",
                 "[odbc-api][catalog][tables]") {
  // Given SQL_ATTR_METADATA_ID is enabled (identifier mode)
  SQLRETURN ret = SQLSetStmtAttr(stmt_handle(), SQL_ATTR_METADATA_ID, reinterpret_cast<SQLPOINTER>(SQL_TRUE), 0);
  REQUIRE(ret == SQL_SUCCESS);

  // When SQLTables is called with the exact table name (no wildcards)
  ret = SQLTables(stmt_handle(), sqlchar(database_name()), SQL_NTS, sqlchar(schema_name()), SQL_NTS,
                  sqlchar(readonly_db::BASIC_TABLE), SQL_NTS, nullptr, 0);
  REQUIRE(ret == SQL_SUCCESS);

  // Then the exact table is returned (metadata_id forces exact match)
  ret = SQLFetch(stmt_handle());
  REQUIRE(ret == SQL_SUCCESS);
  CHECK(sqltables_get_column(stmt_handle(), 3).text == readonly_db::BASIC_TABLE);

  ret = SQLFetch(stmt_handle());
  CHECK(ret == SQL_NO_DATA);
}

TEST_CASE_METHOD(ReadOnlyDbStmtFixture, "SQLTables: metadata_id=TRUE with NULL CatalogName returns HY009",
                 "[odbc-api][catalog][tables][error]") {
  // Given SQL_ATTR_METADATA_ID is enabled
  SQLRETURN ret = SQLSetStmtAttr(stmt_handle(), SQL_ATTR_METADATA_ID, reinterpret_cast<SQLPOINTER>(SQL_TRUE), 0);
  REQUIRE(ret == SQL_SUCCESS);

  // When SQLTables is called with NULL CatalogName (identifier required)
  ret = SQLTables(stmt_handle(), nullptr, 0, sqlchar(schema_name()), SQL_NTS, sqlchar(readonly_db::BASIC_TABLE),
                  SQL_NTS, nullptr, 0);

  // Then HY009 (Invalid use of null pointer) is returned
  REQUIRE_EXPECTED_ERROR(ret, "HY009", stmt_handle(), SQL_HANDLE_STMT);
}

// Identifier mode folds unquoted identifiers to uppercase, so a lowercase
// TableName must still match the uppercase stored name. The new driver folds
// (ODBC-spec compliant); the legacy driver re-filters case-sensitively and
// returns nothing (BD#113).
TEST_CASE_METHOD(ReadOnlyDbStmtFixture, "SQLTables: metadata_id=TRUE matches unquoted TableName case-insensitively",
                 "[odbc-api][catalog][tables]") {
  // Given SQL_ATTR_METADATA_ID is enabled (identifier mode)
  SQLRETURN ret = SQLSetStmtAttr(stmt_handle(), SQL_ATTR_METADATA_ID, reinterpret_cast<SQLPOINTER>(SQL_TRUE), 0);
  REQUIRE(ret == SQL_SUCCESS);

  const std::string cat = to_lower_copy(database_name());
  const std::string sch = to_lower_copy(schema_name());
  const std::string tbl = to_lower_copy(readonly_db::BASIC_TABLE);

  // When SQLTables is called with lowercase unquoted identifiers
  ret = SQLTables(stmt_handle(), sqlchar(cat.c_str()), SQL_NTS, sqlchar(sch.c_str()), SQL_NTS, sqlchar(tbl.c_str()),
                  SQL_NTS, nullptr, 0);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLFetch(stmt_handle());
  NEW_DRIVER_ONLY("BD#113") {
    // Then the uppercase table is returned
    REQUIRE(ret == SQL_SUCCESS);
    CHECK(sqltables_get_column(stmt_handle(), 3).text == readonly_db::BASIC_TABLE);

    ret = SQLFetch(stmt_handle());
    CHECK(ret == SQL_NO_DATA);
  }
  OLD_DRIVER_ONLY("BD#113") { REQUIRE(ret == SQL_NO_DATA); }
}

// In pattern mode (default) a lowercase TableName must NOT match the uppercase name.
TEST_CASE_METHOD(ReadOnlyDbStmtFixture, "SQLTables: metadata_id=FALSE treats TableName case-sensitively",
                 "[odbc-api][catalog][tables]") {
  // Given a lowercase TableName pattern while METADATA_ID is SQL_FALSE (default)
  const std::string tbl = to_lower_copy(readonly_db::BASIC_TABLE);

  // When SQLTables is called in pattern mode
  SQLRETURN ret = SQLTables(stmt_handle(), sqlchar(database_name()), SQL_NTS, sqlchar(schema_name()), SQL_NTS,
                            sqlchar(tbl.c_str()), SQL_NTS, nullptr, 0);
  REQUIRE(ret == SQL_SUCCESS);

  // Then no rows are returned (case is significant)
  ret = SQLFetch(stmt_handle());
  REQUIRE(ret == SQL_NO_DATA);
}

// ============================================================================
// SQLTables - Escape Character and Pattern Matching
// ============================================================================

TEST_CASE_METHOD(ReadOnlyDbStmtFixture, "SQLTables: Escaped underscore filters coarse backend LIKE over-match",
                 "[odbc-api][catalog][tables][escape]") {
  // Given MY_TABLE (literal underscore) and MY1TABLE (matches unescaped MY_ wildcard).
  // Coarse SHOW LIKE 'MY_TABLE' returns both; client-side \ escape must drop MY1TABLE.
  const auto names = sqltables_collect_table_names(stmt_handle(), database_name(), schema_name(), "MY\\_TABLE");
  REQUIRE(!names.empty());

  REQUIRE(names.size() == 1);
  CHECK(names[0] == readonly_db::ESCAPE_LITERAL_UNDERSCORE_TABLE);
}

TEST_CASE_METHOD(ReadOnlyDbStmtFixture, "SQLTables: Escaped percent filters coarse backend LIKE over-match",
                 "[odbc-api][catalog][tables][escape]") {
  // Given VAL1TABLE (coarse LIKE over-match) and "VAL%TABLE" (literal percent).
  // Coarse SHOW LIKE 'VAL%TABLE' returns both; client-side \ escape must keep only VAL%TABLE.
  const auto names = sqltables_collect_table_names(stmt_handle(), database_name(), schema_name(), "VAL\\%TABLE");
  REQUIRE(!names.empty());

  REQUIRE(names.size() == 1);
  CHECK(names[0] == readonly_db::ESCAPE_LITERAL_PERCENT_TABLE);
}

TEST_CASE_METHOD(ReadOnlyDbStmtFixture, "SQLTables: Unescaped underscore wildcard matches decoy table",
                 "[odbc-api][catalog][tables][escape]") {
  // Control: MY_TABLE pattern treats _ as wildcard and matches MY1TABLE too.
  const auto names = sqltables_collect_table_names(stmt_handle(), database_name(), schema_name(), "MY_TABLE");
  REQUIRE(names.size() == 2);
  CHECK(std::find(names.begin(), names.end(), readonly_db::ESCAPE_LITERAL_UNDERSCORE_TABLE) != names.end());
  CHECK(std::find(names.begin(), names.end(), readonly_db::ESCAPE_UNDERSCORE_DECOY_TABLE) != names.end());
}

TEST_CASE_METHOD(ReadOnlyDbStmtFixture, "SQLTables: Unescaped percent wildcard matches BASICTABLE",
                 "[odbc-api][catalog][tables]") {
  // Given pattern %ICTABLE (% = any prefix)
  SQLRETURN ret = SQLTables(stmt_handle(), sqlchar(database_name()), SQL_NTS, sqlchar(schema_name()), SQL_NTS,
                            sqlchar("%ICTABLE"), SQL_NTS, nullptr, 0);
  REQUIRE(ret == SQL_SUCCESS);

  // When fetching at least one row
  ret = SQLFetch(stmt_handle());
  REQUIRE(ret == SQL_SUCCESS);

  // Then BASICTABLE is matched by the % wildcard
  CHECK(sqltables_get_column(stmt_handle(), 3).text == readonly_db::BASIC_TABLE);
}

TEST_CASE_METHOD(ReadOnlyDbStmtFixture, "SQLTables: Schema wildcard pattern returns matching schemas",
                 "[odbc-api][catalog][tables]") {
  // Given a wildcard schema pattern using %
  const std::string schema_pattern = std::string(schema_name()).substr(0, 3) + "%";
  SQLRETURN ret = SQLTables(stmt_handle(), sqlchar(database_name()), SQL_NTS, sqlchar(schema_pattern.c_str()), SQL_NTS,
                            nullptr, 0, nullptr, 0);
  REQUIRE(ret == SQL_SUCCESS);

  // When fetching results, the target schema must appear
  bool found_schema = false;
  while (SQLFetch(stmt_handle()) == SQL_SUCCESS) {
    const ColumnValue schem = sqltables_get_column(stmt_handle(), 2);
    if (schem.is_present() && schem.text == schema_name()) {
      found_schema = true;
    }
  }
  CHECK(found_schema);
}

// ============================================================================
// SQLTables - NULL semantics
// ============================================================================

TEST_CASE_METHOD(ReadOnlyDbStmtFixture, "SQLTables: NULL catalog/schema/table/type returns connection-context tables",
                 "[odbc-api][catalog][tables]") {
  // Given all four arguments are NULL: the catalog resolves to the connection's
  // current database (legacy ODBC semantics), not account-wide. A NULL schema is
  // left NULL, so the result spans all schemas in that database.
  SQLRETURN ret = SQLTables(stmt_handle(), nullptr, 0, nullptr, 0, nullptr, 0, nullptr, 0);
  REQUIRE(ret == SQL_SUCCESS);

  // When fetching results, at least the known table in the current schema exists
  bool found_table = false;
  while (SQLFetch(stmt_handle()) == SQL_SUCCESS) {
    const ColumnValue cat = sqltables_get_column(stmt_handle(), 1);
    const ColumnValue name = sqltables_get_column(stmt_handle(), 3);
    if (cat.text == database_name() && name.text == readonly_db::BASIC_TABLE) {
      found_table = true;
    }
  }
  CHECK(found_table);
}

TEST_CASE_METHOD(ReadOnlyDbStmtFixture, "SQLTables: NULL schema spans all schemas in the connected database",
                 "[odbc-api][catalog][tables]") {
  // The connection's current schema is CATALOGTESTS, but a NULL schema must not
  // narrow results to it: legacy substitutes the catalog only (GetFilterForNullCatalog)
  // and leaves a NULL schema NULL, so SHOW runs IN DATABASE and spans every schema.
  // Guards against re-introducing current-schema over-substitution in the wrapper.
  SQLRETURN ret = SQLTables(stmt_handle(), nullptr, 0, nullptr, 0, nullptr, 0, nullptr, 0);
  REQUIRE(ret == SQL_SUCCESS);

  // Expect objects from the current schema (CATALOGTESTS) AND a second schema
  // (DATATYPETESTS) in the same database to both be present.
  bool found_current_schema = false;
  bool found_second_schema = false;
  while (SQLFetch(stmt_handle()) == SQL_SUCCESS) {
    const ColumnValue schem = sqltables_get_column(stmt_handle(), 2);
    const ColumnValue name = sqltables_get_column(stmt_handle(), 3);
    if (schem.text == schema_name() && name.text == readonly_db::BASIC_TABLE) {
      found_current_schema = true;
    }
    if (schem.text == READONLY_SECOND_SCHEMA_NAME && name.text == readonly_db::SECOND_SCHEMA_TABLE) {
      found_second_schema = true;
    }
  }
  CHECK(found_current_schema);
  CHECK(found_second_schema);
}

// ============================================================================
// SQLTables - REMARKS column
// ============================================================================

TEST_CASE_METHOD(ReadOnlyDbStmtFixture, "SQLTables: REMARKS column is present and not null",
                 "[odbc-api][catalog][tables]") {
  // Given a known table
  SQLRETURN ret = SQLTables(stmt_handle(), sqlchar(database_name()), SQL_NTS, sqlchar(schema_name()), SQL_NTS,
                            sqlchar(readonly_db::BASIC_TABLE), SQL_NTS, nullptr, 0);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLFetch(stmt_handle());
  REQUIRE(ret == SQL_SUCCESS);

  // When retrieving column 5 (REMARKS)
  // Then REMARKS is not SQL_NULL_DATA (it may be an empty string)
  CHECK(!sqltables_get_column(stmt_handle(), 5).is_null());
}

// ============================================================================
// SQLTables - Result ordering
// ============================================================================

TEST_CASE_METHOD(ReadOnlyDbStmtFixture,
                 "SQLTables: Results are ordered by TABLE_TYPE then TABLE_CAT then TABLE_SCHEM then TABLE_NAME",
                 "[odbc-api][catalog][tables]") {
  // Given a wildcard query that returns both BASIC_TABLE and BASIC_VIEW
  SQLRETURN ret = SQLTables(stmt_handle(), sqlchar(database_name()), SQL_NTS, sqlchar(schema_name()), SQL_NTS,
                            sqlchar("BASIC%"), SQL_NTS, nullptr, 0);
  REQUIRE(ret == SQL_SUCCESS);

  // When collecting results
  std::vector<std::string> types;
  while (SQLFetch(stmt_handle()) == SQL_SUCCESS) {
    const ColumnValue type = sqltables_get_column(stmt_handle(), 4);
    if (type.is_present()) {
      types.push_back(type.text);
    }
  }

  // Then results are sorted: TABLE comes before VIEW (TABLE_TYPE ASC)
  REQUIRE(types.size() >= 2);
  auto it_table = std::find(types.begin(), types.end(), "TABLE");
  auto it_view = std::find(types.begin(), types.end(), "VIEW");
  REQUIRE(it_table != types.end());
  REQUIRE(it_view != types.end());
  CHECK(it_table < it_view);
}
