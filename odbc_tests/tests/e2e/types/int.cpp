#include <optional>
#include <string>
#include <vector>

#include <catch2/catch_test_macros.hpp>

#include "Connection.hpp"
#include "Schema.hpp"
#include "compatibility.hpp"
#include "get_data.hpp"

TEST_CASE("should cast integer values to appropriate type for int and synonyms", "[int]") {
  // Given Snowflake client is logged in
  Connection conn;

  // When Query "SELECT 0::<type>, 1000000::<type>, 9223372036854775807::<type>" is executed
  auto stmt = conn.execute_fetch("SELECT 0::INT, 1000000::INT, 9223372036854775807::BIGINT");

  // Then All values should be returned as appropriate type with no precision loss
  for (SQLUSMALLINT col = 1; col <= 3; ++col) {
    SQLSMALLINT data_type = 0;
    SQLULEN column_size = 0;
    SQLSMALLINT dec_digits = 0;
    SQLRETURN ret =
        SQLDescribeCol(stmt.getHandle(), col, nullptr, 0, nullptr, &data_type, &column_size, &dec_digits, nullptr);
    CHECK_ODBC(ret, stmt);
    INFO("col=" << col);
    CHECK(data_type == SQL_DECIMAL);
    CHECK(column_size == 38);
    CHECK(dec_digits == 0);
  }
  {
    auto stmt_syn = conn.execute_fetch("SELECT 42::INTEGER, 42::SMALLINT, 42::TINYINT, 42::BYTEINT");
    for (SQLUSMALLINT col = 1; col <= 4; ++col) {
      SQLSMALLINT data_type = 0;
      SQLULEN column_size = 0;
      SQLSMALLINT dec_digits = 0;
      SQLRETURN ret = SQLDescribeCol(stmt_syn.getHandle(), col, nullptr, 0, nullptr, &data_type, &column_size,
                                     &dec_digits, nullptr);
      CHECK_ODBC(ret, stmt_syn);
      INFO("synonym col=" << col);
      CHECK(data_type == SQL_DECIMAL);
      CHECK(column_size == 38);
      CHECK(dec_digits == 0);
    }
  }
  CHECK(get_data<SQL_C_SBIGINT>(stmt, 1) == 0);
  CHECK(get_data<SQL_C_SBIGINT>(stmt, 2) == 1000000);
  CHECK(get_data<SQL_C_SBIGINT>(stmt, 3) == 9223372036854775807LL);
}

TEST_CASE("should select integer values for int and synonyms", "[int]") {
  // Given Snowflake client is logged in
  Connection conn;

  {
    INFO("zero");
    // When Query "SELECT <query_values>" is executed
    auto stmt = conn.execute_fetch("SELECT 0::INT");

    // Then Result should contain integers <expected_values>
    CHECK(get_data<SQL_C_SBIGINT>(stmt, 1) == 0);
  }

  {
    INFO("tinyint");
    // When Query "SELECT -128::INT, 127::INT, 255::INT" is executed
    auto stmt = conn.execute_fetch("SELECT -128::INT, 127::INT, 255::INT");

    // Then Result should contain integers [-128, 127, 255]
    CHECK(get_data<SQL_C_SBIGINT>(stmt, 1) == -128);
    CHECK(get_data<SQL_C_SBIGINT>(stmt, 2) == 127);
    CHECK(get_data<SQL_C_SBIGINT>(stmt, 3) == 255);
  }

  {
    INFO("smallint");
    // When Query "SELECT -32768::INT, 32767::INT, 65535::INT" is executed
    auto stmt = conn.execute_fetch("SELECT -32768::INT, 32767::INT, 65535::INT");

    // Then Result should contain integers [-32768, 32767, 65535]
    CHECK(get_data<SQL_C_SBIGINT>(stmt, 1) == -32768);
    CHECK(get_data<SQL_C_SBIGINT>(stmt, 2) == 32767);
    CHECK(get_data<SQL_C_SBIGINT>(stmt, 3) == 65535);
  }

  {
    INFO("int");
    // When Query "SELECT -2147483648::INT, 2147483647::INT, 4294967295::BIGINT" is executed
    auto stmt = conn.execute_fetch("SELECT -2147483648::INT, 2147483647::INT, 4294967295::BIGINT");

    // Then Result should contain integers [-2147483648, 2147483647, 4294967295]
    CHECK(get_data<SQL_C_SBIGINT>(stmt, 1) == -2147483648LL);
    CHECK(get_data<SQL_C_SBIGINT>(stmt, 2) == 2147483647LL);
    CHECK(get_data<SQL_C_SBIGINT>(stmt, 3) == 4294967295LL);
  }

  {
    INFO("bigint");
    // When Query "SELECT -9223372036854775808::BIGINT, 9223372036854775807::BIGINT" is executed
    auto stmt = conn.execute_fetch("SELECT -9223372036854775808::BIGINT, 9223372036854775807::BIGINT");

    // Then Result should contain integers [-9223372036854775808, 9223372036854775807]
    CHECK(get_data<SQL_C_SBIGINT>(stmt, 1) == -9223372036854775807LL - 1);
    CHECK(get_data<SQL_C_SBIGINT>(stmt, 2) == 9223372036854775807LL);
  }
}

TEST_CASE("should download large result set with multiple chunks for int and synonyms", "[int]") {
  // Given Snowflake client is logged in
  Connection conn;

  // When Query "SELECT seq8()::<type> as id FROM TABLE(GENERATOR(ROWCOUNT => 50000)) v ORDER BY id" is executed
  auto stmt = conn.createStatement();
  const auto sql = "SELECT seq8()::BIGINT as id FROM TABLE(GENERATOR(ROWCOUNT => 50000)) v ORDER BY id";
  SQLRETURN ret = SQLExecDirect(stmt.getHandle(), (SQLCHAR*)sql, SQL_NTS);
  CHECK_ODBC(ret, stmt);

  // Then Result should contain 50000 sequentially numbered rows from 0 to 49999
  int row_count = 0;
  int64_t expected_value = 0;

  while (true) {
    ret = SQLFetch(stmt.getHandle());
    if (ret == SQL_NO_DATA) {
      break;
    }
    CHECK_ODBC(ret, stmt);

    SQLBIGINT result = 0;
    ret = SQLGetData(stmt.getHandle(), 1, SQL_C_SBIGINT, &result, sizeof(result), NULL);
    CHECK_ODBC(ret, stmt);

    REQUIRE(result == expected_value);
    expected_value++;
    row_count++;
  }

  REQUIRE(row_count == 50000);
}

TEST_CASE("should select values from table for int and synonyms", "[int]") {
  // Given Snowflake client is logged in
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);

  {
    INFO("positive");
    // And Table with <type> column exists with values <insert_values>
    conn.execute("CREATE TABLE int_table_positive (col BIGINT)");
    conn.execute(
        "INSERT INTO int_table_positive VALUES "
        "(0), (1), (127), (255), (32767), (65535), (2147483647), (4294967295), (9223372036854775807)");

    // When Query "SELECT * FROM <table> ORDER BY col" is executed
    auto stmt = conn.createStatement();
    SQLRETURN ret = SQLExecDirect(stmt.getHandle(), (SQLCHAR*)"SELECT * FROM int_table_positive ORDER BY col", SQL_NTS);
    CHECK_ODBC(ret, stmt);

    // Then Result should contain integers <expected_values>
    std::vector<std::optional<int64_t>> expected = {
        {0}, {1}, {127}, {255}, {32767}, {65535}, {2147483647LL}, {4294967295LL}, {9223372036854775807LL}};
    for (size_t i = 0; i < expected.size(); i++) {
      ret = SQLFetch(stmt.getHandle());
      CHECK_ODBC(ret, stmt);
      auto result = get_data_optional<SQL_C_SBIGINT>(stmt, 1);
      REQUIRE(result == expected[i]);
    }
  }

  // negative
  {
    // And Table with <type> column exists with values <insert_values>
    conn.execute("CREATE TABLE int_table_negative (col BIGINT)");
    conn.execute("INSERT INTO int_table_negative VALUES (-1), (-128), (-32768), (-2147483648), (-9223372036854775808)");

    // When Query "SELECT * FROM <table> ORDER BY col" is executed
    auto stmt = conn.createStatement();
    SQLRETURN ret = SQLExecDirect(stmt.getHandle(), (SQLCHAR*)"SELECT * FROM int_table_negative ORDER BY col", SQL_NTS);
    CHECK_ODBC(ret, stmt);

    // Then Result should contain integers <expected_values>
    std::vector<std::optional<int64_t>> expected = {
        {-9223372036854775807LL - 1}, {-2147483648LL}, {-32768}, {-128}, {-1}};
    for (size_t i = 0; i < expected.size(); i++) {
      ret = SQLFetch(stmt.getHandle());
      CHECK_ODBC(ret, stmt);
      auto result = get_data_optional<SQL_C_SBIGINT>(stmt, 1);
      REQUIRE(result == expected[i]);
    }
  }

  // null
  {
    // And Table with <type> column exists with values <insert_values>
    conn.execute("CREATE TABLE int_table_null (col BIGINT)");
    conn.execute("INSERT INTO int_table_null VALUES (0), (NULL), (42)");

    // When Query "SELECT * FROM <table> ORDER BY col" is executed
    auto stmt = conn.createStatement();
    SQLRETURN ret = SQLExecDirect(stmt.getHandle(), (SQLCHAR*)"SELECT * FROM int_table_null ORDER BY col", SQL_NTS);
    CHECK_ODBC(ret, stmt);

    // Then Result should contain integers [0, 42, NULL]
    std::vector<std::optional<int64_t>> expected = {{0}, {42}, std::nullopt};
    for (size_t i = 0; i < expected.size(); i++) {
      ret = SQLFetch(stmt.getHandle());
      CHECK_ODBC(ret, stmt);
      auto result = get_data_optional<SQL_C_SBIGINT>(stmt, 1);
      REQUIRE(result == expected[i]);
    }
  }
}

TEST_CASE("should handle server-side Arrow memory optimization for int columns on multiple chunks", "[int]") {
  constexpr int total_rows = 50000;
  constexpr int64_t expected_col1 = 100;
  constexpr int64_t expected_col2 = 30000;
  constexpr int64_t expected_col3 = 2000000000;
  constexpr int64_t expected_col4 = 9000000000000000000LL;

  // Given Snowflake client is logged in
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);

  // And Table with four INT columns exists
  conn.execute("CREATE TABLE int_different_column_sizes (col_int8 INT, col_int16 INT, col_int32 INT, col_int64 INT)");

  // And Each column contains values of different magnitudes (50000 rows to span multiple Arrow chunks)
  conn.execute(
      "INSERT INTO int_different_column_sizes "
      "SELECT 100, 30000, 2000000000, 9000000000000000000 "
      "FROM TABLE(GENERATOR(ROWCOUNT => " +
      std::to_string(total_rows) + "))");

  // When Query "SELECT * FROM <table>" is executed
  auto stmt = conn.createStatement();
  const auto sql = "SELECT * FROM int_different_column_sizes";
  SQLRETURN ret = SQLExecDirect(stmt.getHandle(), (SQLCHAR*)sql, SQL_NTS);
  CHECK_ODBC(ret, stmt);

  // Then Result should contain 50000 rows with all values equal to expected data
  int row_count = 0;

  while (true) {
    ret = SQLFetch(stmt.getHandle());
    if (ret == SQL_NO_DATA) {
      break;
    }
    CHECK_ODBC(ret, stmt);

    SQLBIGINT col1, col2, col3, col4;
    ret = SQLGetData(stmt.getHandle(), 1, SQL_C_SBIGINT, &col1, sizeof(col1), NULL);
    CHECK_ODBC(ret, stmt);
    ret = SQLGetData(stmt.getHandle(), 2, SQL_C_SBIGINT, &col2, sizeof(col2), NULL);
    CHECK_ODBC(ret, stmt);
    ret = SQLGetData(stmt.getHandle(), 3, SQL_C_SBIGINT, &col3, sizeof(col3), NULL);
    CHECK_ODBC(ret, stmt);
    ret = SQLGetData(stmt.getHandle(), 4, SQL_C_SBIGINT, &col4, sizeof(col4), NULL);
    CHECK_ODBC(ret, stmt);

    REQUIRE(col1 == expected_col1);
    REQUIRE(col2 == expected_col2);
    REQUIRE(col3 == expected_col3);
    REQUIRE(col4 == expected_col4);

    row_count++;
  }

  REQUIRE(row_count == total_rows);
}

// ============================================================================
// Large integer values (exceed int64 range, fetch as strings)
// ============================================================================

TEST_CASE("should handle large integer values for int and synonyms", "[int]") {
  // Given Snowflake client is logged in
  Connection conn;

  // When Query "SELECT -99999999999999999999999999999999999999::<type>,
  // 99999999999999999999999999999999999999::<type>" is executed
  auto stmt = conn.execute_fetch(
      "SELECT -99999999999999999999999999999999999999::INT, "
      "99999999999999999999999999999999999999::INT");

  // Then Result should contain integers [-99999999999999999999999999999999999999,
  // 99999999999999999999999999999999999999]
  CHECK(get_data<SQL_C_CHAR>(stmt, 1) == "-99999999999999999999999999999999999999");
  CHECK(get_data<SQL_C_CHAR>(stmt, 2) == "99999999999999999999999999999999999999");
}

TEST_CASE("should handle NULL values for int and synonyms", "[int]") {
  // Given Snowflake client is logged in
  Connection conn;

  // When Query "SELECT NULL::<type>, 42::<type>, NULL::<type>" is executed
  auto stmt = conn.execute_fetch("SELECT NULL::INT, 42::INT, NULL::INT");

  // Then Result should contain [NULL, 42, NULL]
  CHECK(get_data_optional<SQL_C_SBIGINT>(stmt, 1) == std::nullopt);
  CHECK(get_data_optional<SQL_C_SBIGINT>(stmt, 2) == std::optional<int64_t>(42));
  CHECK(get_data_optional<SQL_C_SBIGINT>(stmt, 3) == std::nullopt);
}

// ============================================================================
// Table operations - large integers
// ============================================================================

TEST_CASE("should select large integer values from table for int and synonyms", "[int]") {
  // Given Snowflake client is logged in
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);

  // And Table with <type> column exists with values [-99999999999999999999999999999999999999,
  // 99999999999999999999999999999999999999]
  conn.execute("CREATE TABLE int_large_table (col INT)");
  conn.execute(
      "INSERT INTO int_large_table VALUES "
      "(-99999999999999999999999999999999999999), "
      "(99999999999999999999999999999999999999)");

  // When Query "SELECT * FROM <table> ORDER BY col" is executed
  auto stmt = conn.execute_fetch("SELECT * FROM int_large_table ORDER BY col");

  // Then Result should contain integers [-99999999999999999999999999999999999999,
  // 99999999999999999999999999999999999999]
  CHECK(get_data<SQL_C_CHAR>(stmt, 1) == "-99999999999999999999999999999999999999");

  SQLRETURN ret = SQLFetch(stmt.getHandle());
  CHECK_ODBC(ret, stmt);
  CHECK(get_data<SQL_C_CHAR>(stmt, 1) == "99999999999999999999999999999999999999");
}

// ============================================================================
// Parameter binding
// ============================================================================

TEST_CASE("should insert integer using parameter binding for int and synonyms", "[int]") {
  // Given Snowflake client is logged in
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);

  // And Table with <type> column exists
  conn.execute("CREATE TABLE int_bind_insert (col BIGINT)");

  // When Integer values [0, -2147483648, 2147483647, 9223372036854775807] are inserted using binding
  auto insert_value = [&](int64_t val) {
    auto stmt = conn.createStatement();
    SQLRETURN ret = SQLPrepare(stmt.getHandle(), (SQLCHAR*)"INSERT INTO int_bind_insert VALUES (?)", SQL_NTS);
    CHECK_ODBC(ret, stmt);

    SQLBIGINT bind_val = val;
    SQLLEN len = 0;
    ret = SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_SBIGINT, SQL_BIGINT, 0, 0, &bind_val, 0, &len);
    CHECK_ODBC(ret, stmt);

    ret = SQLExecute(stmt.getHandle());
    CHECK_ODBC(ret, stmt);
  };

  insert_value(0);
  insert_value(-2147483648LL);
  insert_value(2147483647LL);
  insert_value(9223372036854775807LL);

  // And Query "SELECT * FROM <table>" is executed
  auto stmt = conn.execute_fetch("SELECT * FROM int_bind_insert ORDER BY col");

  // Then Result should contain integers [0, -2147483648, 2147483647, 9223372036854775807]
  CHECK(get_data<SQL_C_SBIGINT>(stmt, 1) == -2147483648LL);

  SQLRETURN ret = SQLFetch(stmt.getHandle());
  CHECK_ODBC(ret, stmt);
  CHECK(get_data<SQL_C_SBIGINT>(stmt, 1) == 0);

  ret = SQLFetch(stmt.getHandle());
  CHECK_ODBC(ret, stmt);
  CHECK(get_data<SQL_C_SBIGINT>(stmt, 1) == 2147483647LL);

  ret = SQLFetch(stmt.getHandle());
  CHECK_ODBC(ret, stmt);
  CHECK(get_data<SQL_C_SBIGINT>(stmt, 1) == 9223372036854775807LL);
}

TEST_CASE("should insert and select integers from table using batch parameter binding for int and synonyms", "[int]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();
  // Given Snowflake client is logged in
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);

  // And Table with <type> column exists
  conn.execute("CREATE TABLE int_batch_bind (col BIGINT)");

  // When Integer values [0, 42, -2147483648, 2147483647, 9223372036854775807] are inserted using binding
  constexpr SQLULEN num_rows = 5;
  SQLBIGINT values[num_rows] = {0, 42, -2147483648LL, 2147483647LL, 9223372036854775807LL};
  SQLLEN indicators[num_rows] = {0, 0, 0, 0, 0};
  SQLUSMALLINT param_status[num_rows] = {};
  SQLULEN params_processed = 0;

  auto insert_stmt = conn.createStatement();
  SQLRETURN ret = SQLSetStmtAttr(insert_stmt.getHandle(), SQL_ATTR_PARAM_BIND_TYPE, SQL_PARAM_BIND_BY_COLUMN, 0);
  CHECK_ODBC(ret, insert_stmt);
  ret = SQLSetStmtAttr(insert_stmt.getHandle(), SQL_ATTR_PARAMSET_SIZE, reinterpret_cast<SQLPOINTER>(num_rows), 0);
  CHECK_ODBC(ret, insert_stmt);
  ret = SQLSetStmtAttr(insert_stmt.getHandle(), SQL_ATTR_PARAM_STATUS_PTR, param_status, 0);
  CHECK_ODBC(ret, insert_stmt);
  ret = SQLSetStmtAttr(insert_stmt.getHandle(), SQL_ATTR_PARAMS_PROCESSED_PTR, &params_processed, 0);
  CHECK_ODBC(ret, insert_stmt);

  ret = SQLBindParameter(insert_stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_SBIGINT, SQL_BIGINT, 0, 0, values, 0,
                         indicators);
  CHECK_ODBC(ret, insert_stmt);

  ret = SQLExecDirect(insert_stmt.getHandle(), (SQLCHAR*)"INSERT INTO int_batch_bind VALUES (?)", SQL_NTS);
  CHECK_ODBC(ret, insert_stmt);
  REQUIRE(params_processed == num_rows);

  // And Query "SELECT * FROM <table>" is executed
  auto stmt = conn.execute_fetch("SELECT * FROM int_batch_bind ORDER BY col");

  // Then Result should contain integers [0, 42, -2147483648, 2147483647, 9223372036854775807]
  CHECK(get_data<SQL_C_SBIGINT>(stmt, 1) == -2147483648LL);

  ret = SQLFetch(stmt.getHandle());
  CHECK_ODBC(ret, stmt);
  CHECK(get_data<SQL_C_SBIGINT>(stmt, 1) == 0);

  ret = SQLFetch(stmt.getHandle());
  CHECK_ODBC(ret, stmt);
  CHECK(get_data<SQL_C_SBIGINT>(stmt, 1) == 42);

  ret = SQLFetch(stmt.getHandle());
  CHECK_ODBC(ret, stmt);
  CHECK(get_data<SQL_C_SBIGINT>(stmt, 1) == 2147483647LL);

  ret = SQLFetch(stmt.getHandle());
  CHECK_ODBC(ret, stmt);
  CHECK(get_data<SQL_C_SBIGINT>(stmt, 1) == 9223372036854775807LL);
}
