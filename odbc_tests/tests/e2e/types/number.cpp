#include <catch2/catch_approx.hpp>
#include <catch2/catch_test_macros.hpp>

#include "Connection.hpp"
#include "Schema.hpp"
#include "get_data.hpp"

TEST_CASE("should cast number values to appropriate type for number and synonyms", "[number]") {
  // Given Snowflake client is logged in
  Connection conn;

  // When Query "SELECT 0::<type>(10,0), 123::<type>(10,0), 0.00::<type>(10,2), 123.45::<type>(10,2)" is executed
  auto stmt = conn.execute_fetch("SELECT 0::NUMBER(10,0), 123::NUMBER(10,0), 0.00::NUMBER(10,2), 123.45::NUMBER(10,2)");

  // Then All values should be returned as appropriate type matching [0, 123, 0.00, 123.45]
  for (SQLUSMALLINT col = 1; col <= 4; ++col) {
    SQLSMALLINT data_type = 0;
    SQLULEN column_size = 0;
    SQLSMALLINT dec_digits = 0;
    SQLRETURN ret =
        SQLDescribeCol(stmt.getHandle(), col, nullptr, 0, nullptr, &data_type, &column_size, &dec_digits, nullptr);
    CHECK_ODBC(ret, stmt);
    INFO("col=" << col);
    CHECK(data_type == SQL_DECIMAL);
    CHECK(column_size == 10);
    CHECK(dec_digits == (col <= 2 ? 0 : 2));
  }
  {
    auto stmt_syn = conn.execute_fetch("SELECT 42::DECIMAL(10,0), 42::NUMERIC(10,0)");
    for (SQLUSMALLINT col = 1; col <= 2; ++col) {
      SQLSMALLINT data_type = 0;
      SQLULEN column_size = 0;
      SQLSMALLINT dec_digits = 0;
      SQLRETURN ret = SQLDescribeCol(stmt_syn.getHandle(), col, nullptr, 0, nullptr, &data_type, &column_size,
                                     &dec_digits, nullptr);
      CHECK_ODBC(ret, stmt_syn);
      INFO("synonym col=" << col);
      CHECK(data_type == SQL_DECIMAL);
      CHECK(column_size == 10);
      CHECK(dec_digits == 0);
    }
  }
  CHECK(get_data<SQL_C_LONG>(stmt, 1) == 0);
  CHECK(get_data<SQL_C_LONG>(stmt, 2) == 123);
  CHECK(get_data<SQL_C_DOUBLE>(stmt, 3) == 0.0);
  CHECK(get_data<SQL_C_DOUBLE>(stmt, 4) == 123.45);
}

TEST_CASE("should select number literals for number and synonyms", "[number]") {
  // Given Snowflake client is logged in
  Connection conn;

  // When Query "SELECT 0::<type>(10,0), -456::<type>(10,0), 1.50::<type>(10,2), -123.45::<type>(10,2),
  // 123.456::<type>(15,3), -789.012::<type>(15,3)" is executed
  auto stmt = conn.execute_fetch(
      "SELECT 0::NUMBER(10,0), -456::NUMBER(10,0), 1.50::NUMBER(10,2), -123.45::NUMBER(10,2), 123.456::NUMBER(15,3), "
      "-789.012::NUMBER(15,3)");

  // Then Result should contain [0, -456, 1.50, -123.45, 123.456, -789.012]
  CHECK(get_data<SQL_C_LONG>(stmt, 1) == 0);
  CHECK(get_data<SQL_C_LONG>(stmt, 2) == -456);
  CHECK(get_data<SQL_C_DOUBLE>(stmt, 3) == 1.50);
  CHECK(get_data<SQL_C_DOUBLE>(stmt, 4) == -123.45);
  CHECK(get_data<SQL_C_DOUBLE>(stmt, 5) == 123.456);
  CHECK(get_data<SQL_C_DOUBLE>(stmt, 6) == -789.012);
}

TEST_CASE("should handle scale and precision boundaries from literals for number and synonyms", "[number]") {
  // Given Snowflake client is logged in
  Connection conn;

  // When Query "SELECT 999.99::<type>(5,2), -999.99::<type>(5,2), 99999999::<type>(8,0), -99999999::<type>(8,0)" is
  // executed
  auto stmt = conn.execute_fetch(
      "SELECT 999.99::NUMBER(5,2), -999.99::NUMBER(5,2), 99999999::NUMBER(8,0), -99999999::NUMBER(8,0)");

  // Then Result should contain [999.99, -999.99, 99999999, -99999999]
  CHECK(get_data<SQL_C_DOUBLE>(stmt, 1) == 999.99);
  CHECK(get_data<SQL_C_DOUBLE>(stmt, 2) == -999.99);
  CHECK(get_data<SQL_C_LONG>(stmt, 3) == 99999999);
  CHECK(get_data<SQL_C_LONG>(stmt, 4) == -99999999);
}

TEST_CASE("should download large result set with multiple chunks from GENERATOR for number and synonyms", "[number]") {
  // Given Snowflake client is logged in
  Connection conn;

  // When Query "SELECT seq8()::<type>(38,0), (seq8() + 0.12345)::<type>(20,5) FROM TABLE(GENERATOR(ROWCOUNT =>
  // 30000)) v" is executed
  auto stmt = conn.createStatement();
  const auto sql =
      "SELECT seq8()::NUMBER(38,0), (seq8() + 0.12345)::NUMBER(20,5) FROM TABLE(GENERATOR(ROWCOUNT => 30000)) v "
      "ORDER BY 1";
  SQLRETURN ret = SQLExecDirect(stmt.getHandle(), (SQLCHAR*)sql, SQL_NTS);
  CHECK_ODBC(ret, stmt);

  // Then Result should contain 30000 rows with sequential integers in column 1 and sequential decimals starting from
  // 0.12345 in column 2
  int row_count = 0;
  int64_t expected_int = 0;
  double expected_decimal = 0.12345;

  while (true) {
    ret = SQLFetch(stmt.getHandle());
    if (ret == SQL_NO_DATA) {
      break;
    }
    CHECK_ODBC(ret, stmt);

    SQLBIGINT col1 = 0;
    ret = SQLGetData(stmt.getHandle(), 1, SQL_C_SBIGINT, &col1, sizeof(col1), NULL);
    CHECK_ODBC(ret, stmt);
    REQUIRE(col1 == expected_int);

    double col2 = 0;
    ret = SQLGetData(stmt.getHandle(), 2, SQL_C_DOUBLE, &col2, sizeof(col2), NULL);
    CHECK_ODBC(ret, stmt);
    REQUIRE(col2 == Catch::Approx(expected_decimal).epsilon(0.00001));

    expected_int++;
    expected_decimal += 1.0;
    row_count++;
  }

  REQUIRE(row_count == 30000);
}

TEST_CASE("should select numbers from table with multiple scales for number and synonyms", "[number]") {
  // Given Snowflake client is logged in
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);

  // And Table with columns (<type>(10,0), <type>(10,2), <type>(15,3), <type>(20,5)) exists
  conn.execute("DROP TABLE IF EXISTS number_table");
  conn.execute(
      "CREATE TABLE number_table (col1 NUMBER(10,0), col2 NUMBER(10,2), col3 NUMBER(15,3), col4 NUMBER(20,5))");

  // And Row (123, 123.45, 123.456, 12345.67890) is inserted
  conn.execute("INSERT INTO number_table VALUES (123, 123.45, 123.456, 12345.67890)");
  // And Row (-456, -67.89, -789.012, -98765.43210) is inserted
  conn.execute("INSERT INTO number_table VALUES (-456, -67.89, -789.012, -98765.43210)");
  // And Row (0, 0.00, 0.000, 0.00000) is inserted
  conn.execute("INSERT INTO number_table VALUES (0, 0.00, 0.000, 0.00000)");
  // And Row (999999, 999.99, 1000.500, 123456.78901) is inserted
  conn.execute("INSERT INTO number_table VALUES (999999, 999.99, 1000.500, 123456.78901)");

  // When Query "SELECT * FROM <table>" is executed
  auto stmt = conn.execute_fetch("SELECT * FROM number_table");

  // Then Result should contain 4 rows with expected values
  CHECK(get_data<SQL_C_LONG>(stmt, 1) == 123);
  CHECK(get_data<SQL_C_DOUBLE>(stmt, 2) == 123.45);
  CHECK(get_data<SQL_C_DOUBLE>(stmt, 3) == 123.456);
  CHECK(get_data<SQL_C_DOUBLE>(stmt, 4) == 12345.67890);

  // Row 2
  SQLRETURN ret = SQLFetch(stmt.getHandle());
  CHECK_ODBC(ret, stmt);
  CHECK(get_data<SQL_C_LONG>(stmt, 1) == -456);
  CHECK(get_data<SQL_C_DOUBLE>(stmt, 2) == -67.89);
  CHECK(get_data<SQL_C_DOUBLE>(stmt, 3) == -789.012);
  CHECK(get_data<SQL_C_DOUBLE>(stmt, 4) == -98765.43210);

  // Row 3
  ret = SQLFetch(stmt.getHandle());
  CHECK_ODBC(ret, stmt);
  CHECK(get_data<SQL_C_LONG>(stmt, 1) == 0);
  CHECK(get_data<SQL_C_DOUBLE>(stmt, 2) == 0.00);
  CHECK(get_data<SQL_C_DOUBLE>(stmt, 3) == 0.000);
  CHECK(get_data<SQL_C_DOUBLE>(stmt, 4) == 0.00000);

  // Row 4
  ret = SQLFetch(stmt.getHandle());
  CHECK_ODBC(ret, stmt);
  CHECK(get_data<SQL_C_LONG>(stmt, 1) == 999999);
  CHECK(get_data<SQL_C_DOUBLE>(stmt, 2) == 999.99);
  CHECK(get_data<SQL_C_DOUBLE>(stmt, 3) == 1000.500);
  CHECK(get_data<SQL_C_DOUBLE>(stmt, 4) == Catch::Approx(123456.78901).epsilon(0.00001));
}

TEST_CASE("should handle scale and precision boundaries from table for number and synonyms", "[number]") {
  // Given Snowflake client is logged in
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);

  // And Table with columns (<type>(5,2), <type>(8,0)) exists
  conn.execute("DROP TABLE IF EXISTS number_boundary_table");
  conn.execute("CREATE TABLE number_boundary_table (col1 NUMBER(5,2), col2 NUMBER(8,0))");

  // And Row (999.99, 99999999) is inserted
  conn.execute("INSERT INTO number_boundary_table VALUES (999.99, 99999999)");
  // And Row (-999.99, -99999999) is inserted
  conn.execute("INSERT INTO number_boundary_table VALUES (-999.99, -99999999)");
  // And Row (123.45, 12345678) is inserted
  conn.execute("INSERT INTO number_boundary_table VALUES (123.45, 12345678)");
  // And Row (0.01, 0) is inserted
  conn.execute("INSERT INTO number_boundary_table VALUES (0.01, 0)");

  // When Query "SELECT * FROM <table>" is executed
  auto stmt = conn.execute_fetch("SELECT * FROM number_boundary_table");

  // Then Result should contain 4 rows with expected boundary values
  CHECK(get_data<SQL_C_DOUBLE>(stmt, 1) == 999.99);
  CHECK(get_data<SQL_C_LONG>(stmt, 2) == 99999999);

  SQLRETURN ret = SQLFetch(stmt.getHandle());
  CHECK_ODBC(ret, stmt);
  CHECK(get_data<SQL_C_DOUBLE>(stmt, 1) == -999.99);
  CHECK(get_data<SQL_C_LONG>(stmt, 2) == -99999999);

  ret = SQLFetch(stmt.getHandle());
  CHECK_ODBC(ret, stmt);
  CHECK(get_data<SQL_C_DOUBLE>(stmt, 1) == 123.45);
  CHECK(get_data<SQL_C_LONG>(stmt, 2) == 12345678);

  ret = SQLFetch(stmt.getHandle());
  CHECK_ODBC(ret, stmt);
  CHECK(get_data<SQL_C_DOUBLE>(stmt, 1) == 0.01);
  CHECK(get_data<SQL_C_LONG>(stmt, 2) == 0);
}

TEST_CASE("should handle NULL values from literals for number and synonyms", "[number]") {
  // Given Snowflake client is logged in
  Connection conn;

  // When Query "SELECT NULL::<type>(10,0), 42::<type>(10,0), NULL::<type>(10,2), 42.50::<type>(10,2)" is executed
  auto stmt =
      conn.execute_fetch("SELECT NULL::NUMBER(10,0), 42::NUMBER(10,0), NULL::NUMBER(10,2), 42.50::NUMBER(10,2)");

  // Then Result should contain [NULL, 42, NULL, 42.50]
  CHECK(get_data_optional<SQL_C_LONG>(stmt, 1) == std::nullopt);
  CHECK(get_data_optional<SQL_C_LONG>(stmt, 2) == std::optional<SQLINTEGER>(42));
  CHECK(get_data_optional<SQL_C_DOUBLE>(stmt, 3) == std::nullopt);
  CHECK(get_data_optional<SQL_C_DOUBLE>(stmt, 4) == std::optional<double>(42.50));
}

TEST_CASE("should handle NULL values from table with multiple scales for number and synonyms", "[number]") {
  // Given Snowflake client is logged in
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);

  // And Table with columns (<type>(10,0), <type>(10,2), <type>(15,3)) exists
  conn.execute("CREATE TABLE number_null_table (col1 NUMBER(10,0), col2 NUMBER(10,2), col3 NUMBER(15,3))");
  // And Row (NULL, NULL, NULL) is inserted
  conn.execute("INSERT INTO number_null_table VALUES (NULL, NULL, NULL)");
  // And Row (123, 123.45, 123.456) is inserted
  conn.execute("INSERT INTO number_null_table VALUES (123, 123.45, 123.456)");
  // And Row (NULL, NULL, NULL) is inserted
  conn.execute("INSERT INTO number_null_table VALUES (NULL, NULL, NULL)");
  // And Row (-456, -67.89, -789.012) is inserted
  conn.execute("INSERT INTO number_null_table VALUES (-456, -67.89, -789.012)");

  // When Query "SELECT * FROM <table>" is executed
  auto stmt = conn.execute_fetch("SELECT * FROM number_null_table");

  // Then Result should contain 4 rows with 2 NULL rows and 2 non-NULL rows with expected values
  CHECK(get_data_optional<SQL_C_LONG>(stmt, 1) == std::nullopt);
  CHECK(get_data_optional<SQL_C_DOUBLE>(stmt, 2) == std::nullopt);
  CHECK(get_data_optional<SQL_C_DOUBLE>(stmt, 3) == std::nullopt);

  SQLRETURN ret = SQLFetch(stmt.getHandle());
  CHECK_ODBC(ret, stmt);
  CHECK(get_data<SQL_C_LONG>(stmt, 1) == 123);
  CHECK(get_data<SQL_C_DOUBLE>(stmt, 2) == 123.45);
  CHECK(get_data<SQL_C_DOUBLE>(stmt, 3) == 123.456);

  ret = SQLFetch(stmt.getHandle());
  CHECK_ODBC(ret, stmt);
  CHECK(get_data_optional<SQL_C_LONG>(stmt, 1) == std::nullopt);
  CHECK(get_data_optional<SQL_C_DOUBLE>(stmt, 2) == std::nullopt);
  CHECK(get_data_optional<SQL_C_DOUBLE>(stmt, 3) == std::nullopt);

  ret = SQLFetch(stmt.getHandle());
  CHECK_ODBC(ret, stmt);
  CHECK(get_data<SQL_C_LONG>(stmt, 1) == -456);
  CHECK(get_data<SQL_C_DOUBLE>(stmt, 2) == -67.89);
  CHECK(get_data<SQL_C_DOUBLE>(stmt, 3) == -789.012);
}

TEST_CASE("should download large result set from table for number and synonyms", "[number]") {
  // Given Snowflake client is logged in
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);

  // And Table with columns (<type>(38,0), <type>(20,5)) exists with 30000 sequential rows, from 0 to 29999 in the
  // first column and from 0.12345 to 29999.12345 in the second column
  conn.execute("DROP TABLE IF EXISTS number_large_table");
  conn.execute("CREATE TABLE number_large_table (col1 NUMBER(38,0), col2 NUMBER(20,5))");
  conn.execute(
      "INSERT INTO number_large_table SELECT seq8(), seq8() + 0.12345 FROM TABLE(GENERATOR(ROWCOUNT => 30000))");

  // When Query "SELECT * FROM <table>" is executed
  auto stmt = conn.createStatement();
  const auto sql = "SELECT * FROM number_large_table ORDER BY col1";
  SQLRETURN ret = SQLExecDirect(stmt.getHandle(), (SQLCHAR*)sql, SQL_NTS);
  CHECK_ODBC(ret, stmt);

  // Then Result should contain 30000 rows with sequential integers in column 1 and sequential decimals starting from
  // 0.12345 in column 2
  int row_count = 0;
  int64_t expected_int = 0;
  double expected_decimal = 0.12345;

  while (true) {
    ret = SQLFetch(stmt.getHandle());
    if (ret == SQL_NO_DATA) {
      break;
    }
    CHECK_ODBC(ret, stmt);

    SQLBIGINT col1 = 0;
    ret = SQLGetData(stmt.getHandle(), 1, SQL_C_SBIGINT, &col1, sizeof(col1), NULL);
    CHECK_ODBC(ret, stmt);
    REQUIRE(col1 == expected_int);

    double col2 = 0;
    ret = SQLGetData(stmt.getHandle(), 2, SQL_C_DOUBLE, &col2, sizeof(col2), NULL);
    CHECK_ODBC(ret, stmt);
    REQUIRE(col2 == Catch::Approx(expected_decimal).epsilon(0.00001));

    expected_int++;
    expected_decimal += 1.0;
    row_count++;
  }

  REQUIRE(row_count == 30000);
}

// ============================================================================
// High precision literals (NUMBER(38,x) - values exceed native int64/double)
// ============================================================================

TEST_CASE("should handle high precision values from literals for number and synonyms", "[number]") {
  // Given Snowflake client is logged in
  Connection conn;

  // When Query "SELECT 12345678901234567890123456789012345678::<type>(38,0),
  // 123456789012345678901234567890123456.78::<type>(38,2),
  // 1234567890123456789012345678.1234567890::<type>(38,10),
  // 0.0000000000000000000000000000000000001::<type>(38,37)" is executed
  auto stmt = conn.execute_fetch(
      "SELECT 12345678901234567890123456789012345678::NUMBER(38,0), "
      "123456789012345678901234567890123456.78::NUMBER(38,2), "
      "1234567890123456789012345678.1234567890::NUMBER(38,10), "
      "0.0000000000000000000000000000000000001::NUMBER(38,37)");

  // Then Result should contain [12345678901234567890123456789012345678,
  // 123456789012345678901234567890123456.78, 1234567890123456789012345678.1234567890,
  // 0.0000000000000000000000000000000000001]
  CHECK(get_data<SQL_C_CHAR>(stmt, 1) == "12345678901234567890123456789012345678");
  CHECK(get_data<SQL_C_CHAR>(stmt, 2) == "123456789012345678901234567890123456.78");
  CHECK(get_data<SQL_C_CHAR>(stmt, 3) == "1234567890123456789012345678.1234567890");
  CHECK(get_data<SQL_C_CHAR>(stmt, 4) == "0.0000000000000000000000000000000000001");
}

TEST_CASE("should handle high precision boundaries from literals for number and synonyms", "[number]") {
  // Given Snowflake client is logged in
  Connection conn;

  // When Query "SELECT 99999999999999999999999999999999999999::<type>(38,0),
  // -99999999999999999999999999999999999999::<type>(38,0)" is executed
  auto stmt = conn.execute_fetch(
      "SELECT 99999999999999999999999999999999999999::NUMBER(38,0), "
      "-99999999999999999999999999999999999999::NUMBER(38,0)");

  // Then Result should contain max and min 38-digit integers
  CHECK(get_data<SQL_C_CHAR>(stmt, 1) == "99999999999999999999999999999999999999");
  CHECK(get_data<SQL_C_CHAR>(stmt, 2) == "-99999999999999999999999999999999999999");
}

// ============================================================================
// High precision table operations
// ============================================================================

TEST_CASE("should handle high precision values from table for number and synonyms", "[number]") {
  // Given Snowflake client is logged in
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);

  // And Table with columns (<type>(38,0), <type>(38,2), <type>(38,10), <type>(38,37)) exists
  conn.execute(
      "CREATE TABLE number_high_prec ("
      "col1 NUMBER(38,0), col2 NUMBER(38,2), col3 NUMBER(38,10), col4 NUMBER(38,37))");

  // And Row (12345678901234567890123456789012345678, 123456789012345678901234567890123456.78,
  // 1234567890123456789012345678.1234567890, 1.2345678901234567890123456789012345678) is inserted
  conn.execute(
      "INSERT INTO number_high_prec VALUES ("
      "12345678901234567890123456789012345678, "
      "123456789012345678901234567890123456.78, "
      "1234567890123456789012345678.1234567890, "
      "1.2345678901234567890123456789012345678)");

  // When Query "SELECT * FROM <table>" is executed
  auto stmt = conn.execute_fetch("SELECT * FROM number_high_prec");

  // Then Result should contain [12345678901234567890123456789012345678,
  // 123456789012345678901234567890123456.78, 1234567890123456789012345678.1234567890,
  // 1.2345678901234567890123456789012345678]
  CHECK(get_data<SQL_C_CHAR>(stmt, 1) == "12345678901234567890123456789012345678");
  CHECK(get_data<SQL_C_CHAR>(stmt, 2) == "123456789012345678901234567890123456.78");
  CHECK(get_data<SQL_C_CHAR>(stmt, 3) == "1234567890123456789012345678.1234567890");
  CHECK(get_data<SQL_C_CHAR>(stmt, 4) == "1.2345678901234567890123456789012345678");
}

TEST_CASE("should handle high precision boundaries from table for number and synonyms", "[number]") {
  // Given Snowflake client is logged in
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);

  // And Table with columns (<type>(38,0), <type>(38,37)) exists
  conn.execute("CREATE TABLE number_high_prec_boundary (col1 NUMBER(38,0), col2 NUMBER(38,37))");

  // And Row (99999999999999999999999999999999999999, 1.2345678901234567890123456789012345678) is inserted
  conn.execute(
      "INSERT INTO number_high_prec_boundary VALUES ("
      "99999999999999999999999999999999999999, "
      "1.2345678901234567890123456789012345678)");
  // And Row (-99999999999999999999999999999999999999, -1.2345678901234567890123456789012345678) is inserted
  conn.execute(
      "INSERT INTO number_high_prec_boundary VALUES ("
      "-99999999999999999999999999999999999999, "
      "-1.2345678901234567890123456789012345678)");
  // And Row (12345678901234567890123456789012345678, 0.0000000000000000000000000000000000001) is inserted
  conn.execute(
      "INSERT INTO number_high_prec_boundary VALUES ("
      "12345678901234567890123456789012345678, "
      "0.0000000000000000000000000000000000001)");

  // When Query "SELECT * FROM <table>" is executed
  auto stmt = conn.execute_fetch("SELECT * FROM number_high_prec_boundary");

  // Then Result should contain 3 rows with expected high precision boundary values
  CHECK(get_data<SQL_C_CHAR>(stmt, 1) == "99999999999999999999999999999999999999");
  CHECK(get_data<SQL_C_CHAR>(stmt, 2) == "1.2345678901234567890123456789012345678");

  SQLRETURN ret = SQLFetch(stmt.getHandle());
  CHECK_ODBC(ret, stmt);
  CHECK(get_data<SQL_C_CHAR>(stmt, 1) == "-99999999999999999999999999999999999999");
  CHECK(get_data<SQL_C_CHAR>(stmt, 2) == "-1.2345678901234567890123456789012345678");

  ret = SQLFetch(stmt.getHandle());
  CHECK_ODBC(ret, stmt);
  CHECK(get_data<SQL_C_CHAR>(stmt, 1) == "12345678901234567890123456789012345678");
  CHECK(get_data<SQL_C_CHAR>(stmt, 2) == "0.0000000000000000000000000000000000001");
}

// ============================================================================
// Parameter binding - SELECT
// ============================================================================

TEST_CASE("should select number using parameter binding for number and synonyms", "[number]") {
  // Given Snowflake client is logged in
  Connection conn;

  // When Query "SELECT ?::<type>(10,0), ?::<type>(10,0), ?::<type>(10,2), ?::<type>(10,2), ?::<type>(10,0)" is executed
  // with bound values [123, -456, 12.34, -56.78, NULL]
  auto stmt = conn.createStatement();
  SQLRETURN ret = SQLPrepare(stmt.getHandle(),
                             (SQLCHAR*)"SELECT ?::NUMBER(10,0), ?::NUMBER(10,0), ?::NUMBER(10,2), "
                                       "?::NUMBER(10,2), ?::NUMBER(10,0)",
                             SQL_NTS);
  CHECK_ODBC(ret, stmt);

  std::string v1 = "123";
  std::string v2 = "-456";
  std::string v3 = "12.34";
  std::string v4 = "-56.78";
  SQLLEN len1 = v1.size();
  SQLLEN len2 = v2.size();
  SQLLEN len3 = v3.size();
  SQLLEN len4 = v4.size();
  SQLLEN null_ind = SQL_NULL_DATA;

  ret = SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_CHAR, SQL_DECIMAL, 10, 0, (SQLCHAR*)v1.c_str(),
                         v1.size(), &len1);
  CHECK_ODBC(ret, stmt);
  ret = SQLBindParameter(stmt.getHandle(), 2, SQL_PARAM_INPUT, SQL_C_CHAR, SQL_DECIMAL, 10, 0, (SQLCHAR*)v2.c_str(),
                         v2.size(), &len2);
  CHECK_ODBC(ret, stmt);
  ret = SQLBindParameter(stmt.getHandle(), 3, SQL_PARAM_INPUT, SQL_C_CHAR, SQL_DECIMAL, 10, 2, (SQLCHAR*)v3.c_str(),
                         v3.size(), &len3);
  CHECK_ODBC(ret, stmt);
  ret = SQLBindParameter(stmt.getHandle(), 4, SQL_PARAM_INPUT, SQL_C_CHAR, SQL_DECIMAL, 10, 2, (SQLCHAR*)v4.c_str(),
                         v4.size(), &len4);
  CHECK_ODBC(ret, stmt);
  ret = SQLBindParameter(stmt.getHandle(), 5, SQL_PARAM_INPUT, SQL_C_CHAR, SQL_DECIMAL, 10, 0, nullptr, 0, &null_ind);
  CHECK_ODBC(ret, stmt);

  ret = SQLExecute(stmt.getHandle());
  CHECK_ODBC(ret, stmt);

  ret = SQLFetch(stmt.getHandle());
  CHECK_ODBC(ret, stmt);

  // Then Result should contain [123, -456, 12.34, -56.78, NULL]
  CHECK(get_data<SQL_C_LONG>(stmt, 1) == 123);
  CHECK(get_data<SQL_C_LONG>(stmt, 2) == -456);
  CHECK(get_data<SQL_C_DOUBLE>(stmt, 3) == 12.34);
  CHECK(get_data<SQL_C_DOUBLE>(stmt, 4) == -56.78);
  CHECK(get_data_optional<SQL_C_LONG>(stmt, 5) == std::nullopt);
}

TEST_CASE("should select high precision number using parameter binding for number and synonyms", "[number]") {
  // Given Snowflake client is logged in
  Connection conn;

  // When Query "SELECT ?::<type>(38,0), ?::<type>(38,2)" is executed with bound values
  // [12345678901234567890123456789012345678, 123456789012345678901234567890123456.78]
  auto stmt = conn.createStatement();
  SQLRETURN ret = SQLPrepare(stmt.getHandle(), (SQLCHAR*)"SELECT ?::NUMBER(38,0), ?::NUMBER(38,2)", SQL_NTS);
  CHECK_ODBC(ret, stmt);

  std::string v1 = "12345678901234567890123456789012345678";
  std::string v2 = "123456789012345678901234567890123456.78";
  SQLLEN len1 = v1.size();
  SQLLEN len2 = v2.size();

  ret = SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_CHAR, SQL_DECIMAL, 38, 0, (SQLCHAR*)v1.c_str(),
                         v1.size(), &len1);
  CHECK_ODBC(ret, stmt);
  ret = SQLBindParameter(stmt.getHandle(), 2, SQL_PARAM_INPUT, SQL_C_CHAR, SQL_DECIMAL, 38, 2, (SQLCHAR*)v2.c_str(),
                         v2.size(), &len2);
  CHECK_ODBC(ret, stmt);

  ret = SQLExecute(stmt.getHandle());
  CHECK_ODBC(ret, stmt);

  ret = SQLFetch(stmt.getHandle());
  CHECK_ODBC(ret, stmt);

  // Then Result should contain [12345678901234567890123456789012345678,
  // 123456789012345678901234567890123456.78]
  CHECK(get_data<SQL_C_CHAR>(stmt, 1) == "12345678901234567890123456789012345678");
  CHECK(get_data<SQL_C_CHAR>(stmt, 2) == "123456789012345678901234567890123456.78");
}

// ============================================================================
// Parameter binding - INSERT
// ============================================================================

TEST_CASE("should insert number using parameter binding for number and synonyms", "[number]") {
  // Given Snowflake client is logged in
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);

  // And Table with columns (<type>(10,0), <type>(10,2)) exists
  conn.execute("CREATE TABLE number_bind_insert (col1 NUMBER(10,0), col2 NUMBER(10,2))");

  // When Rows (0, 0.00), (123, 123.45), (-456, -67.89), (999999, 999.99), (NULL, NULL) are inserted using binding
  auto insert_row = [&](const char* val1, const char* val2) {
    auto stmt = conn.createStatement();
    SQLRETURN ret = SQLPrepare(stmt.getHandle(), (SQLCHAR*)"INSERT INTO number_bind_insert VALUES (?, ?)", SQL_NTS);
    CHECK_ODBC(ret, stmt);

    std::string v1 = val1;
    std::string v2 = val2;
    SQLLEN len1 = v1.size();
    SQLLEN len2 = v2.size();

    ret = SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_CHAR, SQL_DECIMAL, 10, 0, (SQLCHAR*)v1.c_str(),
                           v1.size(), &len1);
    CHECK_ODBC(ret, stmt);
    ret = SQLBindParameter(stmt.getHandle(), 2, SQL_PARAM_INPUT, SQL_C_CHAR, SQL_DECIMAL, 10, 2, (SQLCHAR*)v2.c_str(),
                           v2.size(), &len2);
    CHECK_ODBC(ret, stmt);

    ret = SQLExecute(stmt.getHandle());
    CHECK_ODBC(ret, stmt);
  };

  auto insert_null_row = [&]() {
    auto stmt = conn.createStatement();
    SQLRETURN ret = SQLPrepare(stmt.getHandle(), (SQLCHAR*)"INSERT INTO number_bind_insert VALUES (?, ?)", SQL_NTS);
    CHECK_ODBC(ret, stmt);

    SQLLEN null_ind = SQL_NULL_DATA;
    ret = SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_CHAR, SQL_DECIMAL, 10, 0, nullptr, 0, &null_ind);
    CHECK_ODBC(ret, stmt);
    ret = SQLBindParameter(stmt.getHandle(), 2, SQL_PARAM_INPUT, SQL_C_CHAR, SQL_DECIMAL, 10, 2, nullptr, 0, &null_ind);
    CHECK_ODBC(ret, stmt);

    ret = SQLExecute(stmt.getHandle());
    CHECK_ODBC(ret, stmt);
  };

  insert_row("0", "0.00");
  insert_row("123", "123.45");
  insert_row("-456", "-67.89");
  insert_row("999999", "999.99");
  insert_null_row();

  // Then Result should contain 5 rows with expected values
  auto stmt = conn.execute_fetch("SELECT * FROM number_bind_insert ORDER BY col1 NULLS LAST");

  CHECK(get_data<SQL_C_LONG>(stmt, 1) == -456);
  CHECK(get_data<SQL_C_DOUBLE>(stmt, 2) == -67.89);

  SQLRETURN ret = SQLFetch(stmt.getHandle());
  CHECK_ODBC(ret, stmt);
  CHECK(get_data<SQL_C_LONG>(stmt, 1) == 0);
  CHECK(get_data<SQL_C_DOUBLE>(stmt, 2) == 0.00);

  ret = SQLFetch(stmt.getHandle());
  CHECK_ODBC(ret, stmt);
  CHECK(get_data<SQL_C_LONG>(stmt, 1) == 123);
  CHECK(get_data<SQL_C_DOUBLE>(stmt, 2) == 123.45);

  ret = SQLFetch(stmt.getHandle());
  CHECK_ODBC(ret, stmt);
  CHECK(get_data<SQL_C_LONG>(stmt, 1) == 999999);
  CHECK(get_data<SQL_C_DOUBLE>(stmt, 2) == 999.99);

  ret = SQLFetch(stmt.getHandle());
  CHECK_ODBC(ret, stmt);
  CHECK(get_data_optional<SQL_C_LONG>(stmt, 1) == std::nullopt);
  CHECK(get_data_optional<SQL_C_DOUBLE>(stmt, 2) == std::nullopt);
}

TEST_CASE("should insert high precision number using parameter binding for number and synonyms", "[number]") {
  // Given Snowflake client is logged in
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);

  // And Table with columns (<type>(38,0), <type>(38,2)) exists
  conn.execute("CREATE TABLE number_bind_high_prec (col1 NUMBER(38,0), col2 NUMBER(38,2))");

  // When Rows (12345678901234567890123456789012345678, 123456789012345678901234567890123456.78),
  // (99999999999999999999999999999999999999, 0.01), (-99999999999999999999999999999999999999, -0.01) are inserted
  // using binding
  auto insert_row = [&](const std::string& val1, const std::string& val2) {
    auto stmt = conn.createStatement();
    SQLRETURN ret = SQLPrepare(stmt.getHandle(), (SQLCHAR*)"INSERT INTO number_bind_high_prec VALUES (?, ?)", SQL_NTS);
    CHECK_ODBC(ret, stmt);

    SQLLEN len1 = val1.size();
    SQLLEN len2 = val2.size();
    ret = SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_CHAR, SQL_DECIMAL, 38, 0, (SQLCHAR*)val1.c_str(),
                           val1.size(), &len1);
    CHECK_ODBC(ret, stmt);
    ret = SQLBindParameter(stmt.getHandle(), 2, SQL_PARAM_INPUT, SQL_C_CHAR, SQL_DECIMAL, 38, 2, (SQLCHAR*)val2.c_str(),
                           val2.size(), &len2);
    CHECK_ODBC(ret, stmt);

    ret = SQLExecute(stmt.getHandle());
    CHECK_ODBC(ret, stmt);
  };

  insert_row("12345678901234567890123456789012345678", "123456789012345678901234567890123456.78");
  insert_row("99999999999999999999999999999999999999", "0.01");
  insert_row("-99999999999999999999999999999999999999", "-0.01");

  // Then Result should contain 3 rows with expected values keeping the precision
  auto stmt = conn.execute_fetch("SELECT * FROM number_bind_high_prec ORDER BY col1");

  CHECK(get_data<SQL_C_CHAR>(stmt, 1) == "-99999999999999999999999999999999999999");
  CHECK(get_data<SQL_C_CHAR>(stmt, 2) == "-0.01");

  SQLRETURN ret = SQLFetch(stmt.getHandle());
  CHECK_ODBC(ret, stmt);
  CHECK(get_data<SQL_C_CHAR>(stmt, 1) == "12345678901234567890123456789012345678");
  CHECK(get_data<SQL_C_CHAR>(stmt, 2) == "123456789012345678901234567890123456.78");

  ret = SQLFetch(stmt.getHandle());
  CHECK_ODBC(ret, stmt);
  CHECK(get_data<SQL_C_CHAR>(stmt, 1) == "99999999999999999999999999999999999999");
  CHECK(get_data<SQL_C_CHAR>(stmt, 2) == "0.01");
}
