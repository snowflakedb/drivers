// ODBC E2E: SQL_C_INTERVAL_* C types bound via SQLBindParameter to the
// SQL_INTERVAL_* parameter types (interval C -> interval SQL). This suite is
// exhaustive over the two interval families.
//
// Per ODBC Appendix D ("Converting Data from C to SQL Data Types",
// "Converting Interval C Data"), an interval C type may be bound to ANY
// interval SQL target of the SAME family:
//   * year-month family C sources  (SQL_C_INTERVAL_YEAR / _MONTH /
//     _YEAR_TO_MONTH) -> year-month SQL targets (SQL_INTERVAL_YEAR / _MONTH /
//     _YEAR_TO_MONTH); and
//   * day-time family C sources (SQL_C_INTERVAL_DAY / _HOUR / _MINUTE /
//     _SECOND / _DAY_TO_HOUR / _DAY_TO_MINUTE / _DAY_TO_SECOND /
//     _HOUR_TO_MINUTE / _HOUR_TO_SECOND / _MINUTE_TO_SECOND) -> day-time SQL
//     targets (the ten matching SQL_INTERVAL_* subtypes).
// Cross-family binds (a year-month source to a day-time target or vice versa)
// are rejected with SQLSTATE 07006.
//
// The conversion is keyed by the *C source* subtype, not the bound SQL target
// subtype: the driver reads the fields the C subtype carries and renders the
// canonical interval literal for that subtype (e.g. SQL_C_INTERVAL_DAY_TO_SECOND
// renders "1 02:03:04.000000"). The bound SQL_INTERVAL_* parameter type only
// selects the interval family (year-month vs day-time) used for the
// compatibility check; within a family the rendered text is identical across
// every target subtype. Each positive test therefore pins the rendered literal
// for one C subtype and asserts it is unchanged across ALL same-family SQL
// targets. The parameter is inserted into a VARCHAR column so the rendered
// literal is stored verbatim and can be asserted exactly; the driver-side
// formatting is additionally pinned by the Rust unit tests in interval.rs.
//
// The new driver round-trips every same-family bind (BD#71). On the old driver
// a same-family bind fails at SQLExecute, and the SQLSTATE depends on the value.
// As observed today, the old driver converts the value into the bound
// SQL_INTERVAL_* subtype with a two-digit leading precision (ColumnSize is 0)
// and no fractional seconds (DecimalDigits is 0). The conversion fails with
// 22015 when the target's leading field would exceed 99, or when the target has
// no field for a non-zero part of the value (e.g. 11 months bound as
// SQL_INTERVAL_YEAR, or any fractional second). A value that converts reaches
// the server, which rejects the SQL_INTERVAL_* bind type with HY000 and native
// error 18. Neither driver range-checks a compound value's trailing field
// (e.g. 2 years 12 months), and the old driver does not carry an out-of-range
// trailing field into the leading one. Each test
// therefore asserts, in separate SECTIONs, a value for every outcome its
// targets can produce.
// Cross-family binds are rejected with 07006 at SQLBindParameter on both
// drivers, before the value is read.

#include <sql.h>
#include <sqlext.h>
#include <sqltypes.h>

#include <string>
#include <vector>

#include <catch2/catch_test_macros.hpp>
#include <catch2/generators/catch_generators.hpp>

#include "Connection.hpp"
#include "SchemaFixtures.hpp"
#include "compatibility.hpp"
#include "conversion_checks.hpp"
#include "get_data.hpp"
#include "get_diag_rec.hpp"
#include "odbc_cast.hpp"
#include "odbc_matchers.hpp"

namespace {

// All SQL_INTERVAL_* targets of each family. Within a family every target is a
// legal destination for every same-family C interval source.
const std::vector<SQLSMALLINT> kYearMonthTargets = {
    SQL_INTERVAL_YEAR,
    SQL_INTERVAL_MONTH,
    SQL_INTERVAL_YEAR_TO_MONTH,
};
const std::vector<SQLSMALLINT> kDayTimeTargets = {
    SQL_INTERVAL_DAY,
    SQL_INTERVAL_HOUR,
    SQL_INTERVAL_MINUTE,
    SQL_INTERVAL_SECOND,
    SQL_INTERVAL_DAY_TO_HOUR,
    SQL_INTERVAL_DAY_TO_MINUTE,
    SQL_INTERVAL_DAY_TO_SECOND,
    SQL_INTERVAL_HOUR_TO_MINUTE,
    SQL_INTERVAL_HOUR_TO_SECOND,
    SQL_INTERVAL_MINUTE_TO_SECOND,
};

SQL_INTERVAL_STRUCT ym_interval(SQLSMALLINT sign, SQLUINTEGER year, SQLUINTEGER month) {
  SQL_INTERVAL_STRUCT iv = {};
  iv.interval_sign = sign;
  iv.intval.year_month.year = year;
  iv.intval.year_month.month = month;
  return iv;
}

SQL_INTERVAL_STRUCT ds_interval(SQLSMALLINT sign, SQLUINTEGER day, SQLUINTEGER hour, SQLUINTEGER minute,
                                SQLUINTEGER second, SQLUINTEGER fraction) {
  SQL_INTERVAL_STRUCT iv = {};
  iv.interval_sign = sign;
  iv.intval.day_second.day = day;
  iv.intval.day_second.hour = hour;
  iv.intval.day_second.minute = minute;
  iv.intval.day_second.second = second;
  iv.intval.day_second.fraction = fraction;
  return iv;
}

// A SQL_C_INTERVAL_* value and the literal the new driver renders for it.
struct IntervalSource {
  SQLSMALLINT c_type;
  SQL_INTERVAL_STRUCT val;
  std::string literal;
};

// Binds `src` as `sql_type` and inserts it into a fresh VARCHAR column. The new
// driver stores `src.literal`; the old driver fails SQLExecute with
// `old_driver_sqlstate`.
void check_interval_bind(Connection& conn, const IntervalSource& src, SQLSMALLINT sql_type,
                         const char* old_driver_sqlstate) {
  CAPTURE(src.c_type, sql_type);
  conn.execute("CREATE OR REPLACE TEMPORARY TABLE t (col VARCHAR(200))");
  auto stmt = conn.createStatement();
  SQLRETURN ret = SQLPrepare(stmt.getHandle(), sqlchar("INSERT INTO t VALUES (?)"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  SQL_INTERVAL_STRUCT val = src.val;
  SQLLEN ind = sizeof(val);
  ret = SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, src.c_type, sql_type, 0, 0, &val, sizeof(val), &ind);
  REQUIRE_ODBC(ret, stmt);
  ret = SQLExecute(stmt.getHandle());
  NEW_DRIVER_ONLY("BD#71") {
    REQUIRE_ODBC(ret, stmt);
    auto fetch_stmt = conn.execute_fetch("SELECT col FROM t");
    CHECK(get_data<SQL_C_CHAR>(fetch_stmt, 1) == src.literal);
  }
  OLD_DRIVER_ONLY("BD#71") {
    REQUIRE_EXPECTED_ERROR(ret, old_driver_sqlstate, stmt.getHandle(), SQL_HANDLE_STMT);
    // HY000 is also the SQLSTATE of unrelated failures; native error 18 identifies the server rejecting the
    // SQL_INTERVAL_* bind type.
    if (std::string(old_driver_sqlstate) == "HY000") {
      CHECK_THAT(OdbcResult(ret, stmt), OdbcMatchers::HasNativeError(18));
    }
  }
}

}  // namespace

// ============================================================================
// Year-month family: SQL_C_INTERVAL_YEAR / _MONTH / _YEAR_TO_MONTH bound to
// every year-month SQL_INTERVAL_* target.
// ============================================================================

TEST_CASE_METHOD(ConnSchemaFixture, "should bind SQL_C_INTERVAL_YEAR to every year-month SQL_INTERVAL target",
                 "[e2e][types][c_interval][conversion][sql_interval]") {
  // Given a SQL_C_INTERVAL_YEAR source
  // When it is bound to each year-month SQL_INTERVAL target and inserted
  // Then the new driver stores the same literal for every target, and the old driver fails with 22015 where
  // the value does not fit the target and with HY000 where it does
  SECTION("5 years") {
    const IntervalSource src{SQL_C_INTERVAL_YEAR, ym_interval(SQL_FALSE, 5, 0), "5"};
    check_interval_bind(conn, src, SQL_INTERVAL_YEAR, "HY000");
    check_interval_bind(conn, src, SQL_INTERVAL_MONTH, "HY000");
    check_interval_bind(conn, src, SQL_INTERVAL_YEAR_TO_MONTH, "HY000");
  }
  SECTION("99 years") {
    // SQL_INTERVAL_MONTH would need 1188
    const IntervalSource src{SQL_C_INTERVAL_YEAR, ym_interval(SQL_FALSE, 99, 0), "99"};
    check_interval_bind(conn, src, SQL_INTERVAL_YEAR, "HY000");
    check_interval_bind(conn, src, SQL_INTERVAL_MONTH, "22015");
    check_interval_bind(conn, src, SQL_INTERVAL_YEAR_TO_MONTH, "HY000");
  }
  SECTION("100 years overflows every target") {
    const IntervalSource src{SQL_C_INTERVAL_YEAR, ym_interval(SQL_FALSE, 100, 0), "100"};
    for (SQLSMALLINT sql_type : kYearMonthTargets) {
      check_interval_bind(conn, src, sql_type, "22015");
    }
  }
}

TEST_CASE_METHOD(ConnSchemaFixture, "should bind SQL_C_INTERVAL_MONTH to every year-month SQL_INTERVAL target",
                 "[e2e][types][c_interval][conversion][sql_interval]") {
  // Given a SQL_C_INTERVAL_MONTH source
  // When it is bound to each year-month SQL_INTERVAL target and inserted
  // Then the new driver stores the same literal for every target, and the old driver fails with 22015 where
  // the value does not fit the target and with HY000 where it does
  SECTION("11 months") {
    // SQL_INTERVAL_YEAR cannot hold the 11 months
    const IntervalSource src{SQL_C_INTERVAL_MONTH, ym_interval(SQL_FALSE, 0, 11), "11"};
    check_interval_bind(conn, src, SQL_INTERVAL_YEAR, "22015");
    check_interval_bind(conn, src, SQL_INTERVAL_MONTH, "HY000");
    check_interval_bind(conn, src, SQL_INTERVAL_YEAR_TO_MONTH, "HY000");
  }
  SECTION("12 months is exactly 1 year and fits every target") {
    const IntervalSource src{SQL_C_INTERVAL_MONTH, ym_interval(SQL_FALSE, 0, 12), "12"};
    for (SQLSMALLINT sql_type : kYearMonthTargets) {
      check_interval_bind(conn, src, sql_type, "HY000");
    }
  }
  SECTION("0 months fits every target") {
    const IntervalSource src{SQL_C_INTERVAL_MONTH, ym_interval(SQL_FALSE, 0, 0), "0"};
    for (SQLSMALLINT sql_type : kYearMonthTargets) {
      check_interval_bind(conn, src, sql_type, "HY000");
    }
  }
  SECTION("1200 months (100 years) overflows every target") {
    const IntervalSource src{SQL_C_INTERVAL_MONTH, ym_interval(SQL_FALSE, 0, 1200), "1200"};
    for (SQLSMALLINT sql_type : kYearMonthTargets) {
      check_interval_bind(conn, src, sql_type, "22015");
    }
  }
}

TEST_CASE_METHOD(ConnSchemaFixture, "should bind SQL_C_INTERVAL_YEAR_TO_MONTH to every year-month SQL_INTERVAL target",
                 "[e2e][types][c_interval][conversion][sql_interval]") {
  // Given a compound SQL_C_INTERVAL_YEAR_TO_MONTH source
  // When it is bound to each year-month SQL_INTERVAL target and inserted
  // Then the new driver stores the same literal for every target, and the old driver fails with 22015 where
  // the value does not fit the target and with HY000 where it does
  SECTION("3 years 6 months") {
    // SQL_INTERVAL_YEAR cannot hold the 6 months
    const IntervalSource src{SQL_C_INTERVAL_YEAR_TO_MONTH, ym_interval(SQL_FALSE, 3, 6), "3-06"};
    check_interval_bind(conn, src, SQL_INTERVAL_YEAR, "22015");
    check_interval_bind(conn, src, SQL_INTERVAL_MONTH, "HY000");
    check_interval_bind(conn, src, SQL_INTERVAL_YEAR_TO_MONTH, "HY000");
  }
  SECTION("2 years 12 months has an out-of-range month field") {
    // Neither driver range-checks the month field; the old driver does not carry it into SQL_INTERVAL_YEAR
    const IntervalSource src{SQL_C_INTERVAL_YEAR_TO_MONTH, ym_interval(SQL_FALSE, 2, 12), "2-12"};
    check_interval_bind(conn, src, SQL_INTERVAL_YEAR, "22015");
    check_interval_bind(conn, src, SQL_INTERVAL_MONTH, "HY000");
    check_interval_bind(conn, src, SQL_INTERVAL_YEAR_TO_MONTH, "HY000");
  }
  SECTION("0 years 0 months fits every target") {
    const IntervalSource src{SQL_C_INTERVAL_YEAR_TO_MONTH, ym_interval(SQL_FALSE, 0, 0), "0-00"};
    for (SQLSMALLINT sql_type : kYearMonthTargets) {
      check_interval_bind(conn, src, sql_type, "HY000");
    }
  }
  SECTION("100 years 0 months overflows every target") {
    const IntervalSource src{SQL_C_INTERVAL_YEAR_TO_MONTH, ym_interval(SQL_FALSE, 100, 0), "100-00"};
    for (SQLSMALLINT sql_type : kYearMonthTargets) {
      check_interval_bind(conn, src, sql_type, "22015");
    }
  }
}

// ============================================================================
// Day-time family: each of the ten SQL_C_INTERVAL_* day-time sources bound to
// every day-time SQL_INTERVAL_* target.
// ============================================================================

TEST_CASE_METHOD(ConnSchemaFixture, "should bind SQL_C_INTERVAL_DAY to every day-time SQL_INTERVAL target",
                 "[e2e][types][c_interval][conversion][sql_interval]") {
  // Given a SQL_C_INTERVAL_DAY source
  // When it is bound to each day-time SQL_INTERVAL target and inserted
  // Then the new driver stores the same literal for every target, and the old driver fails with 22015 where
  // the value does not fit the target and with HY000 where it does
  SECTION("7 days") {
    // Hour-, minute- and second-leading targets would need 168 or more in the leading field
    const IntervalSource src{SQL_C_INTERVAL_DAY, ds_interval(SQL_FALSE, 7, 0, 0, 0, 0), "7"};
    check_interval_bind(conn, src, SQL_INTERVAL_DAY, "HY000");
    check_interval_bind(conn, src, SQL_INTERVAL_HOUR, "22015");
    check_interval_bind(conn, src, SQL_INTERVAL_MINUTE, "22015");
    check_interval_bind(conn, src, SQL_INTERVAL_SECOND, "22015");
    check_interval_bind(conn, src, SQL_INTERVAL_DAY_TO_HOUR, "HY000");
    check_interval_bind(conn, src, SQL_INTERVAL_DAY_TO_MINUTE, "HY000");
    check_interval_bind(conn, src, SQL_INTERVAL_DAY_TO_SECOND, "HY000");
    check_interval_bind(conn, src, SQL_INTERVAL_HOUR_TO_MINUTE, "22015");
    check_interval_bind(conn, src, SQL_INTERVAL_HOUR_TO_SECOND, "22015");
    check_interval_bind(conn, src, SQL_INTERVAL_MINUTE_TO_SECOND, "22015");
  }
  SECTION("0 days fits every target") {
    const IntervalSource src{SQL_C_INTERVAL_DAY, ds_interval(SQL_FALSE, 0, 0, 0, 0, 0), "0"};
    for (SQLSMALLINT sql_type : kDayTimeTargets) {
      check_interval_bind(conn, src, sql_type, "HY000");
    }
  }
  SECTION("100 days overflows every target") {
    const IntervalSource src{SQL_C_INTERVAL_DAY, ds_interval(SQL_FALSE, 100, 0, 0, 0, 0), "100"};
    for (SQLSMALLINT sql_type : kDayTimeTargets) {
      check_interval_bind(conn, src, sql_type, "22015");
    }
  }
}

TEST_CASE_METHOD(ConnSchemaFixture, "should bind SQL_C_INTERVAL_HOUR to every day-time SQL_INTERVAL target",
                 "[e2e][types][c_interval][conversion][sql_interval]") {
  // Given a SQL_C_INTERVAL_HOUR source
  // When it is bound to each day-time SQL_INTERVAL target and inserted
  // Then the new driver stores the same literal for every target, and the old driver fails with 22015 where
  // the value does not fit the target and with HY000 where it does
  SECTION("9 hours") {
    // SQL_INTERVAL_DAY cannot hold the 9 hours; minute- and second-leading targets would need 540 or more
    const IntervalSource src{SQL_C_INTERVAL_HOUR, ds_interval(SQL_FALSE, 0, 9, 0, 0, 0), "9"};
    check_interval_bind(conn, src, SQL_INTERVAL_DAY, "22015");
    check_interval_bind(conn, src, SQL_INTERVAL_HOUR, "HY000");
    check_interval_bind(conn, src, SQL_INTERVAL_MINUTE, "22015");
    check_interval_bind(conn, src, SQL_INTERVAL_SECOND, "22015");
    check_interval_bind(conn, src, SQL_INTERVAL_DAY_TO_HOUR, "HY000");
    check_interval_bind(conn, src, SQL_INTERVAL_DAY_TO_MINUTE, "HY000");
    check_interval_bind(conn, src, SQL_INTERVAL_DAY_TO_SECOND, "HY000");
    check_interval_bind(conn, src, SQL_INTERVAL_HOUR_TO_MINUTE, "HY000");
    check_interval_bind(conn, src, SQL_INTERVAL_HOUR_TO_SECOND, "HY000");
    check_interval_bind(conn, src, SQL_INTERVAL_MINUTE_TO_SECOND, "22015");
  }
  SECTION("24 hours is exactly 1 day") {
    // Minute- and second-leading targets would need 1440 or more
    const IntervalSource src{SQL_C_INTERVAL_HOUR, ds_interval(SQL_FALSE, 0, 24, 0, 0, 0), "24"};
    check_interval_bind(conn, src, SQL_INTERVAL_DAY, "HY000");
    check_interval_bind(conn, src, SQL_INTERVAL_HOUR, "HY000");
    check_interval_bind(conn, src, SQL_INTERVAL_MINUTE, "22015");
    check_interval_bind(conn, src, SQL_INTERVAL_SECOND, "22015");
    check_interval_bind(conn, src, SQL_INTERVAL_DAY_TO_HOUR, "HY000");
    check_interval_bind(conn, src, SQL_INTERVAL_DAY_TO_MINUTE, "HY000");
    check_interval_bind(conn, src, SQL_INTERVAL_DAY_TO_SECOND, "HY000");
    check_interval_bind(conn, src, SQL_INTERVAL_HOUR_TO_MINUTE, "HY000");
    check_interval_bind(conn, src, SQL_INTERVAL_HOUR_TO_SECOND, "HY000");
    check_interval_bind(conn, src, SQL_INTERVAL_MINUTE_TO_SECOND, "22015");
  }
  SECTION("0 hours fits every target") {
    const IntervalSource src{SQL_C_INTERVAL_HOUR, ds_interval(SQL_FALSE, 0, 0, 0, 0, 0), "0"};
    for (SQLSMALLINT sql_type : kDayTimeTargets) {
      check_interval_bind(conn, src, sql_type, "HY000");
    }
  }
  SECTION("2400 hours (100 days) overflows every target") {
    const IntervalSource src{SQL_C_INTERVAL_HOUR, ds_interval(SQL_FALSE, 0, 2400, 0, 0, 0), "2400"};
    for (SQLSMALLINT sql_type : kDayTimeTargets) {
      check_interval_bind(conn, src, sql_type, "22015");
    }
  }
}

TEST_CASE_METHOD(ConnSchemaFixture, "should bind SQL_C_INTERVAL_MINUTE to every day-time SQL_INTERVAL target",
                 "[e2e][types][c_interval][conversion][sql_interval]") {
  // Given a SQL_C_INTERVAL_MINUTE source
  // When it is bound to each day-time SQL_INTERVAL target and inserted
  // Then the new driver stores the same literal for every target, and the old driver fails with 22015 where
  // the value does not fit the target and with HY000 where it does
  SECTION("30 minutes") {
    // Day- and hour-only targets cannot hold the 30 minutes; SQL_INTERVAL_SECOND would need 1800
    const IntervalSource src{SQL_C_INTERVAL_MINUTE, ds_interval(SQL_FALSE, 0, 0, 30, 0, 0), "30"};
    check_interval_bind(conn, src, SQL_INTERVAL_DAY, "22015");
    check_interval_bind(conn, src, SQL_INTERVAL_HOUR, "22015");
    check_interval_bind(conn, src, SQL_INTERVAL_MINUTE, "HY000");
    check_interval_bind(conn, src, SQL_INTERVAL_SECOND, "22015");
    check_interval_bind(conn, src, SQL_INTERVAL_DAY_TO_HOUR, "22015");
    check_interval_bind(conn, src, SQL_INTERVAL_DAY_TO_MINUTE, "HY000");
    check_interval_bind(conn, src, SQL_INTERVAL_DAY_TO_SECOND, "HY000");
    check_interval_bind(conn, src, SQL_INTERVAL_HOUR_TO_MINUTE, "HY000");
    check_interval_bind(conn, src, SQL_INTERVAL_HOUR_TO_SECOND, "HY000");
    check_interval_bind(conn, src, SQL_INTERVAL_MINUTE_TO_SECOND, "HY000");
  }
  SECTION("0 minutes fits every target") {
    const IntervalSource src{SQL_C_INTERVAL_MINUTE, ds_interval(SQL_FALSE, 0, 0, 0, 0, 0), "0"};
    for (SQLSMALLINT sql_type : kDayTimeTargets) {
      check_interval_bind(conn, src, sql_type, "HY000");
    }
  }
  SECTION("144000 minutes (100 days) overflows every target") {
    const IntervalSource src{SQL_C_INTERVAL_MINUTE, ds_interval(SQL_FALSE, 0, 0, 144000, 0, 0), "144000"};
    for (SQLSMALLINT sql_type : kDayTimeTargets) {
      check_interval_bind(conn, src, sql_type, "22015");
    }
  }
}

TEST_CASE_METHOD(ConnSchemaFixture, "should bind SQL_C_INTERVAL_SECOND to every day-time SQL_INTERVAL target",
                 "[e2e][types][c_interval][conversion][sql_interval]") {
  // Given a SQL_C_INTERVAL_SECOND source
  // When it is bound to each day-time SQL_INTERVAL target and inserted
  // Then the new driver stores the same literal for every target, and the old driver fails with 22015 where
  // the value does not fit the target and with HY000 where it does
  SECTION("45 seconds") {
    // Targets without a seconds field cannot hold the 45 seconds
    const IntervalSource src{SQL_C_INTERVAL_SECOND, ds_interval(SQL_FALSE, 0, 0, 0, 45, 0), "45.000000"};
    check_interval_bind(conn, src, SQL_INTERVAL_DAY, "22015");
    check_interval_bind(conn, src, SQL_INTERVAL_HOUR, "22015");
    check_interval_bind(conn, src, SQL_INTERVAL_MINUTE, "22015");
    check_interval_bind(conn, src, SQL_INTERVAL_SECOND, "HY000");
    check_interval_bind(conn, src, SQL_INTERVAL_DAY_TO_HOUR, "22015");
    check_interval_bind(conn, src, SQL_INTERVAL_DAY_TO_MINUTE, "22015");
    check_interval_bind(conn, src, SQL_INTERVAL_DAY_TO_SECOND, "HY000");
    check_interval_bind(conn, src, SQL_INTERVAL_HOUR_TO_MINUTE, "22015");
    check_interval_bind(conn, src, SQL_INTERVAL_HOUR_TO_SECOND, "HY000");
    check_interval_bind(conn, src, SQL_INTERVAL_MINUTE_TO_SECOND, "HY000");
  }
  SECTION("0 seconds fits every target") {
    const IntervalSource src{SQL_C_INTERVAL_SECOND, ds_interval(SQL_FALSE, 0, 0, 0, 0, 0), "0.000000"};
    for (SQLSMALLINT sql_type : kDayTimeTargets) {
      check_interval_bind(conn, src, sql_type, "HY000");
    }
  }
  SECTION("8640000 seconds (100 days) overflows every target") {
    const IntervalSource src{SQL_C_INTERVAL_SECOND, ds_interval(SQL_FALSE, 0, 0, 0, 8640000, 0), "8640000.000000"};
    for (SQLSMALLINT sql_type : kDayTimeTargets) {
      check_interval_bind(conn, src, sql_type, "22015");
    }
  }
}

TEST_CASE_METHOD(ConnSchemaFixture, "should bind SQL_C_INTERVAL_DAY_TO_HOUR to every day-time SQL_INTERVAL target",
                 "[e2e][types][c_interval][conversion][sql_interval]") {
  // Given a compound SQL_C_INTERVAL_DAY_TO_HOUR source
  // When it is bound to each day-time SQL_INTERVAL target and inserted
  // Then the new driver stores the same literal for every target, and the old driver fails with 22015 where
  // the value does not fit the target and with HY000 where it does
  SECTION("1 day 2 hours") {
    // SQL_INTERVAL_DAY cannot hold the 2 hours; minute- and second-leading targets would need 1560 or more
    const IntervalSource src{SQL_C_INTERVAL_DAY_TO_HOUR, ds_interval(SQL_FALSE, 1, 2, 0, 0, 0), "1 02"};
    check_interval_bind(conn, src, SQL_INTERVAL_DAY, "22015");
    check_interval_bind(conn, src, SQL_INTERVAL_HOUR, "HY000");
    check_interval_bind(conn, src, SQL_INTERVAL_MINUTE, "22015");
    check_interval_bind(conn, src, SQL_INTERVAL_SECOND, "22015");
    check_interval_bind(conn, src, SQL_INTERVAL_DAY_TO_HOUR, "HY000");
    check_interval_bind(conn, src, SQL_INTERVAL_DAY_TO_MINUTE, "HY000");
    check_interval_bind(conn, src, SQL_INTERVAL_DAY_TO_SECOND, "HY000");
    check_interval_bind(conn, src, SQL_INTERVAL_HOUR_TO_MINUTE, "HY000");
    check_interval_bind(conn, src, SQL_INTERVAL_HOUR_TO_SECOND, "HY000");
    check_interval_bind(conn, src, SQL_INTERVAL_MINUTE_TO_SECOND, "22015");
  }
  SECTION("1 day 25 hours has an out-of-range hour field") {
    // Neither driver range-checks the hour field; SQL_INTERVAL_DAY cannot hold the hours, and minute- and
    // second-leading targets would need 2940 or more
    const IntervalSource src{SQL_C_INTERVAL_DAY_TO_HOUR, ds_interval(SQL_FALSE, 1, 25, 0, 0, 0), "1 25"};
    check_interval_bind(conn, src, SQL_INTERVAL_DAY, "22015");
    check_interval_bind(conn, src, SQL_INTERVAL_HOUR, "HY000");
    check_interval_bind(conn, src, SQL_INTERVAL_MINUTE, "22015");
    check_interval_bind(conn, src, SQL_INTERVAL_SECOND, "22015");
    check_interval_bind(conn, src, SQL_INTERVAL_DAY_TO_HOUR, "HY000");
    check_interval_bind(conn, src, SQL_INTERVAL_DAY_TO_MINUTE, "HY000");
    check_interval_bind(conn, src, SQL_INTERVAL_DAY_TO_SECOND, "HY000");
    check_interval_bind(conn, src, SQL_INTERVAL_HOUR_TO_MINUTE, "HY000");
    check_interval_bind(conn, src, SQL_INTERVAL_HOUR_TO_SECOND, "HY000");
    check_interval_bind(conn, src, SQL_INTERVAL_MINUTE_TO_SECOND, "22015");
  }
  SECTION("0 days 0 hours fits every target") {
    const IntervalSource src{SQL_C_INTERVAL_DAY_TO_HOUR, ds_interval(SQL_FALSE, 0, 0, 0, 0, 0), "0 00"};
    for (SQLSMALLINT sql_type : kDayTimeTargets) {
      check_interval_bind(conn, src, sql_type, "HY000");
    }
  }
  SECTION("100 days 0 hours overflows every target") {
    const IntervalSource src{SQL_C_INTERVAL_DAY_TO_HOUR, ds_interval(SQL_FALSE, 100, 0, 0, 0, 0), "100 00"};
    for (SQLSMALLINT sql_type : kDayTimeTargets) {
      check_interval_bind(conn, src, sql_type, "22015");
    }
  }
}

TEST_CASE_METHOD(ConnSchemaFixture, "should bind SQL_C_INTERVAL_DAY_TO_MINUTE to every day-time SQL_INTERVAL target",
                 "[e2e][types][c_interval][conversion][sql_interval]") {
  // Given a compound SQL_C_INTERVAL_DAY_TO_MINUTE source
  // When it is bound to each day-time SQL_INTERVAL target and inserted
  // Then the new driver stores the same literal for every target, and the old driver fails with 22015 where
  // the value does not fit the target and with HY000 where it does
  SECTION("1 day 2 hours 3 minutes") {
    // Targets without a minutes field cannot hold the 3 minutes; minute/second-leading targets need 1563 or more
    const IntervalSource src{SQL_C_INTERVAL_DAY_TO_MINUTE, ds_interval(SQL_FALSE, 1, 2, 3, 0, 0), "1 02:03"};
    check_interval_bind(conn, src, SQL_INTERVAL_DAY, "22015");
    check_interval_bind(conn, src, SQL_INTERVAL_HOUR, "22015");
    check_interval_bind(conn, src, SQL_INTERVAL_MINUTE, "22015");
    check_interval_bind(conn, src, SQL_INTERVAL_SECOND, "22015");
    check_interval_bind(conn, src, SQL_INTERVAL_DAY_TO_HOUR, "22015");
    check_interval_bind(conn, src, SQL_INTERVAL_DAY_TO_MINUTE, "HY000");
    check_interval_bind(conn, src, SQL_INTERVAL_DAY_TO_SECOND, "HY000");
    check_interval_bind(conn, src, SQL_INTERVAL_HOUR_TO_MINUTE, "HY000");
    check_interval_bind(conn, src, SQL_INTERVAL_HOUR_TO_SECOND, "HY000");
    check_interval_bind(conn, src, SQL_INTERVAL_MINUTE_TO_SECOND, "22015");
  }
  SECTION("0 days 00:00 fits every target") {
    const IntervalSource src{SQL_C_INTERVAL_DAY_TO_MINUTE, ds_interval(SQL_FALSE, 0, 0, 0, 0, 0), "0 00:00"};
    for (SQLSMALLINT sql_type : kDayTimeTargets) {
      check_interval_bind(conn, src, sql_type, "HY000");
    }
  }
  SECTION("100 days 00:00 overflows every target") {
    const IntervalSource src{SQL_C_INTERVAL_DAY_TO_MINUTE, ds_interval(SQL_FALSE, 100, 0, 0, 0, 0), "100 00:00"};
    for (SQLSMALLINT sql_type : kDayTimeTargets) {
      check_interval_bind(conn, src, sql_type, "22015");
    }
  }
}

TEST_CASE_METHOD(ConnSchemaFixture, "should bind SQL_C_INTERVAL_DAY_TO_SECOND to every day-time SQL_INTERVAL target",
                 "[e2e][types][c_interval][conversion][sql_interval]") {
  // Given a compound SQL_C_INTERVAL_DAY_TO_SECOND source
  // When it is bound to each day-time SQL_INTERVAL target and inserted
  // Then the new driver stores the same literal for every target, and the old driver fails with 22015 where
  // the value does not fit the target and with HY000 where it does
  SECTION("1 day 2 hours 3 minutes 4 seconds") {
    // Targets without a seconds field cannot hold the 4 seconds; minute/second-leading targets need 1563 or more
    const IntervalSource src{SQL_C_INTERVAL_DAY_TO_SECOND, ds_interval(SQL_FALSE, 1, 2, 3, 4, 0), "1 02:03:04.000000"};
    check_interval_bind(conn, src, SQL_INTERVAL_DAY, "22015");
    check_interval_bind(conn, src, SQL_INTERVAL_HOUR, "22015");
    check_interval_bind(conn, src, SQL_INTERVAL_MINUTE, "22015");
    check_interval_bind(conn, src, SQL_INTERVAL_SECOND, "22015");
    check_interval_bind(conn, src, SQL_INTERVAL_DAY_TO_HOUR, "22015");
    check_interval_bind(conn, src, SQL_INTERVAL_DAY_TO_MINUTE, "22015");
    check_interval_bind(conn, src, SQL_INTERVAL_DAY_TO_SECOND, "HY000");
    check_interval_bind(conn, src, SQL_INTERVAL_HOUR_TO_MINUTE, "22015");
    check_interval_bind(conn, src, SQL_INTERVAL_HOUR_TO_SECOND, "HY000");
    check_interval_bind(conn, src, SQL_INTERVAL_MINUTE_TO_SECOND, "22015");
  }
  SECTION("0 days 00:00:00 fits every target") {
    const IntervalSource src{SQL_C_INTERVAL_DAY_TO_SECOND, ds_interval(SQL_FALSE, 0, 0, 0, 0, 0), "0 00:00:00.000000"};
    for (SQLSMALLINT sql_type : kDayTimeTargets) {
      check_interval_bind(conn, src, sql_type, "HY000");
    }
  }
  SECTION("100 days 00:00:00 overflows every target") {
    const IntervalSource src{SQL_C_INTERVAL_DAY_TO_SECOND, ds_interval(SQL_FALSE, 100, 0, 0, 0, 0),
                             "100 00:00:00.000000"};
    for (SQLSMALLINT sql_type : kDayTimeTargets) {
      check_interval_bind(conn, src, sql_type, "22015");
    }
  }
}

TEST_CASE_METHOD(ConnSchemaFixture, "should bind SQL_C_INTERVAL_HOUR_TO_MINUTE to every day-time SQL_INTERVAL target",
                 "[e2e][types][c_interval][conversion][sql_interval]") {
  // Given a compound SQL_C_INTERVAL_HOUR_TO_MINUTE source
  // When it is bound to each day-time SQL_INTERVAL target and inserted
  // Then the new driver stores the same literal for every target, and the old driver fails with 22015 where
  // the value does not fit the target and with HY000 where it does
  SECTION("10 hours 30 minutes") {
    // Targets without a minutes field cannot hold the 30 minutes; minute/second-leading targets need 630 or more
    const IntervalSource src{SQL_C_INTERVAL_HOUR_TO_MINUTE, ds_interval(SQL_FALSE, 0, 10, 30, 0, 0), "10:30"};
    check_interval_bind(conn, src, SQL_INTERVAL_DAY, "22015");
    check_interval_bind(conn, src, SQL_INTERVAL_HOUR, "22015");
    check_interval_bind(conn, src, SQL_INTERVAL_MINUTE, "22015");
    check_interval_bind(conn, src, SQL_INTERVAL_SECOND, "22015");
    check_interval_bind(conn, src, SQL_INTERVAL_DAY_TO_HOUR, "22015");
    check_interval_bind(conn, src, SQL_INTERVAL_DAY_TO_MINUTE, "HY000");
    check_interval_bind(conn, src, SQL_INTERVAL_DAY_TO_SECOND, "HY000");
    check_interval_bind(conn, src, SQL_INTERVAL_HOUR_TO_MINUTE, "HY000");
    check_interval_bind(conn, src, SQL_INTERVAL_HOUR_TO_SECOND, "HY000");
    check_interval_bind(conn, src, SQL_INTERVAL_MINUTE_TO_SECOND, "22015");
  }
  SECTION("0:00 fits every target") {
    const IntervalSource src{SQL_C_INTERVAL_HOUR_TO_MINUTE, ds_interval(SQL_FALSE, 0, 0, 0, 0, 0), "0:00"};
    for (SQLSMALLINT sql_type : kDayTimeTargets) {
      check_interval_bind(conn, src, sql_type, "HY000");
    }
  }
  SECTION("2400:00 (100 days) overflows every target") {
    const IntervalSource src{SQL_C_INTERVAL_HOUR_TO_MINUTE, ds_interval(SQL_FALSE, 0, 2400, 0, 0, 0), "2400:00"};
    for (SQLSMALLINT sql_type : kDayTimeTargets) {
      check_interval_bind(conn, src, sql_type, "22015");
    }
  }
}

TEST_CASE_METHOD(ConnSchemaFixture, "should bind SQL_C_INTERVAL_HOUR_TO_SECOND to every day-time SQL_INTERVAL target",
                 "[e2e][types][c_interval][conversion][sql_interval]") {
  // Given a compound SQL_C_INTERVAL_HOUR_TO_SECOND source
  // When it is bound to each day-time SQL_INTERVAL target and inserted
  // Then the new driver stores the same literal for every target, and the old driver fails with 22015 where
  // the value does not fit the target and with HY000 where it does
  SECTION("10 hours 30 minutes 15 seconds") {
    // Targets without a seconds field cannot hold the 15 seconds; minute/second-leading targets need 630 or more
    const IntervalSource src{SQL_C_INTERVAL_HOUR_TO_SECOND, ds_interval(SQL_FALSE, 0, 10, 30, 15, 0),
                             "10:30:15.000000"};
    check_interval_bind(conn, src, SQL_INTERVAL_DAY, "22015");
    check_interval_bind(conn, src, SQL_INTERVAL_HOUR, "22015");
    check_interval_bind(conn, src, SQL_INTERVAL_MINUTE, "22015");
    check_interval_bind(conn, src, SQL_INTERVAL_SECOND, "22015");
    check_interval_bind(conn, src, SQL_INTERVAL_DAY_TO_HOUR, "22015");
    check_interval_bind(conn, src, SQL_INTERVAL_DAY_TO_MINUTE, "22015");
    check_interval_bind(conn, src, SQL_INTERVAL_DAY_TO_SECOND, "HY000");
    check_interval_bind(conn, src, SQL_INTERVAL_HOUR_TO_MINUTE, "22015");
    check_interval_bind(conn, src, SQL_INTERVAL_HOUR_TO_SECOND, "HY000");
    check_interval_bind(conn, src, SQL_INTERVAL_MINUTE_TO_SECOND, "22015");
  }
  SECTION("0:00:00 fits every target") {
    const IntervalSource src{SQL_C_INTERVAL_HOUR_TO_SECOND, ds_interval(SQL_FALSE, 0, 0, 0, 0, 0), "0:00:00.000000"};
    for (SQLSMALLINT sql_type : kDayTimeTargets) {
      check_interval_bind(conn, src, sql_type, "HY000");
    }
  }
  SECTION("2400:00:00 (100 days) overflows every target") {
    const IntervalSource src{SQL_C_INTERVAL_HOUR_TO_SECOND, ds_interval(SQL_FALSE, 0, 2400, 0, 0, 0),
                             "2400:00:00.000000"};
    for (SQLSMALLINT sql_type : kDayTimeTargets) {
      check_interval_bind(conn, src, sql_type, "22015");
    }
  }
}

TEST_CASE_METHOD(ConnSchemaFixture, "should bind SQL_C_INTERVAL_MINUTE_TO_SECOND to every day-time SQL_INTERVAL target",
                 "[e2e][types][c_interval][conversion][sql_interval]") {
  // Given a compound SQL_C_INTERVAL_MINUTE_TO_SECOND source
  // When it is bound to each day-time SQL_INTERVAL target and inserted
  // Then the new driver stores the same literal for every target, and the old driver fails with 22015 where
  // the value does not fit the target and with HY000 where it does
  SECTION("30 minutes 15 seconds") {
    // Targets without a seconds field cannot hold the 15 seconds; SQL_INTERVAL_SECOND would need 1815
    const IntervalSource src{SQL_C_INTERVAL_MINUTE_TO_SECOND, ds_interval(SQL_FALSE, 0, 0, 30, 15, 0), "30:15.000000"};
    check_interval_bind(conn, src, SQL_INTERVAL_DAY, "22015");
    check_interval_bind(conn, src, SQL_INTERVAL_HOUR, "22015");
    check_interval_bind(conn, src, SQL_INTERVAL_MINUTE, "22015");
    check_interval_bind(conn, src, SQL_INTERVAL_SECOND, "22015");
    check_interval_bind(conn, src, SQL_INTERVAL_DAY_TO_HOUR, "22015");
    check_interval_bind(conn, src, SQL_INTERVAL_DAY_TO_MINUTE, "22015");
    check_interval_bind(conn, src, SQL_INTERVAL_DAY_TO_SECOND, "HY000");
    check_interval_bind(conn, src, SQL_INTERVAL_HOUR_TO_MINUTE, "22015");
    check_interval_bind(conn, src, SQL_INTERVAL_HOUR_TO_SECOND, "HY000");
    check_interval_bind(conn, src, SQL_INTERVAL_MINUTE_TO_SECOND, "HY000");
  }
  SECTION("0:00 fits every target") {
    const IntervalSource src{SQL_C_INTERVAL_MINUTE_TO_SECOND, ds_interval(SQL_FALSE, 0, 0, 0, 0, 0), "0:00.000000"};
    for (SQLSMALLINT sql_type : kDayTimeTargets) {
      check_interval_bind(conn, src, sql_type, "HY000");
    }
  }
  SECTION("144000:00 (100 days) overflows every target") {
    const IntervalSource src{SQL_C_INTERVAL_MINUTE_TO_SECOND, ds_interval(SQL_FALSE, 0, 0, 144000, 0, 0),
                             "144000:00.000000"};
    for (SQLSMALLINT sql_type : kDayTimeTargets) {
      check_interval_bind(conn, src, sql_type, "22015");
    }
  }
}

// ============================================================================
// Negative sign — the leading sign of the source struct is applied to the
// rendered literal, for single-field and compound sources in both families.
// The old driver checks the magnitude, so the sign does not change its SQLSTATE.
// ============================================================================

TEST_CASE_METHOD(ConnSchemaFixture, "should bind negative SQL_C_INTERVAL_YEAR to SQL_INTERVAL_YEAR",
                 "[e2e][types][c_interval][conversion][sql_interval]") {
  // Given a negative SQL_C_INTERVAL_YEAR source
  // When it is bound to SQL_INTERVAL_YEAR and inserted
  // Then the new driver applies the leading sign to the literal, and the old driver fails with HY000 when the
  // magnitude fits two digits and with 22015 when it does not
  SECTION("-7 years") {
    check_interval_bind(conn, {SQL_C_INTERVAL_YEAR, ym_interval(SQL_TRUE, 7, 0), "-7"}, SQL_INTERVAL_YEAR, "HY000");
  }
  SECTION("-100 years overflows the leading field") {
    check_interval_bind(conn, {SQL_C_INTERVAL_YEAR, ym_interval(SQL_TRUE, 100, 0), "-100"}, SQL_INTERVAL_YEAR, "22015");
  }
}

TEST_CASE_METHOD(ConnSchemaFixture, "should bind negative SQL_C_INTERVAL_YEAR_TO_MONTH to SQL_INTERVAL_YEAR_TO_MONTH",
                 "[e2e][types][c_interval][conversion][sql_interval]") {
  // Given a negative compound SQL_C_INTERVAL_YEAR_TO_MONTH source
  // When it is bound to SQL_INTERVAL_YEAR_TO_MONTH and inserted
  // Then the new driver applies the leading sign to the compound literal, and the old driver fails with HY000 when
  // the years fit two digits and with 22015 when they do not
  SECTION("-2 years 3 months") {
    check_interval_bind(conn, {SQL_C_INTERVAL_YEAR_TO_MONTH, ym_interval(SQL_TRUE, 2, 3), "-2-03"},
                        SQL_INTERVAL_YEAR_TO_MONTH, "HY000");
  }
  SECTION("-100 years 3 months overflows the leading field") {
    check_interval_bind(conn, {SQL_C_INTERVAL_YEAR_TO_MONTH, ym_interval(SQL_TRUE, 100, 3), "-100-03"},
                        SQL_INTERVAL_YEAR_TO_MONTH, "22015");
  }
}

TEST_CASE_METHOD(ConnSchemaFixture, "should bind negative SQL_C_INTERVAL_SECOND to SQL_INTERVAL_SECOND",
                 "[e2e][types][c_interval][conversion][sql_interval]") {
  // Given a negative SQL_C_INTERVAL_SECOND source
  // When it is bound to SQL_INTERVAL_SECOND and inserted
  // Then the new driver applies the leading sign to the literal, and the old driver fails with HY000 when the
  // magnitude fits two digits and with 22015 when it does not
  SECTION("-3 seconds") {
    check_interval_bind(conn, {SQL_C_INTERVAL_SECOND, ds_interval(SQL_TRUE, 0, 0, 0, 3, 0), "-3.000000"},
                        SQL_INTERVAL_SECOND, "HY000");
  }
  SECTION("-100 seconds overflows the leading field") {
    check_interval_bind(conn, {SQL_C_INTERVAL_SECOND, ds_interval(SQL_TRUE, 0, 0, 0, 100, 0), "-100.000000"},
                        SQL_INTERVAL_SECOND, "22015");
  }
}

TEST_CASE_METHOD(ConnSchemaFixture, "should bind negative SQL_C_INTERVAL_DAY_TO_SECOND to SQL_INTERVAL_DAY_TO_SECOND",
                 "[e2e][types][c_interval][conversion][sql_interval]") {
  // Given a negative compound SQL_C_INTERVAL_DAY_TO_SECOND source
  // When it is bound to SQL_INTERVAL_DAY_TO_SECOND and inserted
  // Then the new driver applies the leading sign to the compound literal, and the old driver fails with HY000 when
  // the days fit two digits and with 22015 when they do not
  SECTION("-(1 day 2 hours 3 minutes 4 seconds)") {
    check_interval_bind(conn,
                        {SQL_C_INTERVAL_DAY_TO_SECOND, ds_interval(SQL_TRUE, 1, 2, 3, 4, 0), "-1 02:03:04.000000"},
                        SQL_INTERVAL_DAY_TO_SECOND, "HY000");
  }
  SECTION("-(100 days 2 hours 3 minutes 4 seconds) overflows the leading field") {
    check_interval_bind(conn,
                        {SQL_C_INTERVAL_DAY_TO_SECOND, ds_interval(SQL_TRUE, 100, 2, 3, 4, 0), "-100 02:03:04.000000"},
                        SQL_INTERVAL_DAY_TO_SECOND, "22015");
  }
}

// ============================================================================
// Fractional seconds — the microsecond `fraction` field is rendered zero-padded
// to six digits, for both the leading-field and compound seconds positions.
// ============================================================================

TEST_CASE_METHOD(ConnSchemaFixture, "should bind fractional SQL_C_INTERVAL_SECOND to SQL_INTERVAL_SECOND",
                 "[e2e][types][c_interval][conversion][sql_interval]") {
  // Given a SQL_C_INTERVAL_SECOND source
  // When it is bound to SQL_INTERVAL_SECOND and inserted
  // Then the new driver renders the fraction zero-padded to six digits, and the old driver fails with 22015 for a
  // non-zero fraction (DecimalDigits is 0) and with HY000 for whole seconds
  SECTION("12.5 seconds") {
    check_interval_bind(conn, {SQL_C_INTERVAL_SECOND, ds_interval(SQL_FALSE, 0, 0, 0, 12, 500'000), "12.500000"},
                        SQL_INTERVAL_SECOND, "22015");
  }
  SECTION("12 seconds") {
    check_interval_bind(conn, {SQL_C_INTERVAL_SECOND, ds_interval(SQL_FALSE, 0, 0, 0, 12, 0), "12.000000"},
                        SQL_INTERVAL_SECOND, "HY000");
  }
}

TEST_CASE_METHOD(ConnSchemaFixture, "should bind fractional SQL_C_INTERVAL_DAY_TO_SECOND to SQL_INTERVAL_DAY_TO_SECOND",
                 "[e2e][types][c_interval][conversion][sql_interval]") {
  // Given a compound SQL_C_INTERVAL_DAY_TO_SECOND source
  // When it is bound to SQL_INTERVAL_DAY_TO_SECOND and inserted
  // Then the new driver renders the seconds sub-field zero-padded to six fractional digits, and the old driver
  // fails with 22015 for a non-zero fraction (DecimalDigits is 0) and with HY000 for whole seconds
  SECTION("1 day 2 hours 3 minutes 4.5 seconds") {
    check_interval_bind(
        conn, {SQL_C_INTERVAL_DAY_TO_SECOND, ds_interval(SQL_FALSE, 1, 2, 3, 4, 500'000), "1 02:03:04.500000"},
        SQL_INTERVAL_DAY_TO_SECOND, "22015");
  }
  SECTION("1 day 2 hours 3 minutes 4 seconds") {
    check_interval_bind(conn,
                        {SQL_C_INTERVAL_DAY_TO_SECOND, ds_interval(SQL_FALSE, 1, 2, 3, 4, 0), "1 02:03:04.000000"},
                        SQL_INTERVAL_DAY_TO_SECOND, "HY000");
  }
}

// ============================================================================
// Cross-family binds are rejected with 07006 — a year-month source cannot be
// bound to a day-time target and vice versa, for single-field AND compound
// sources/targets.
// ============================================================================

TEST_CASE_METHOD(ConnSchemaFixture, "should reject cross-family SQL_C_INTERVAL to SQL_INTERVAL with 07006",
                 "[e2e][types][c_interval][incompatible][sql_interval]") {
  auto [c_type, sql_type] = GENERATE(Catch::Generators::table<SQLSMALLINT, SQLSMALLINT>({
      // year-month source -> day-time target
      {SQL_C_INTERVAL_YEAR, SQL_INTERVAL_DAY},
      {SQL_C_INTERVAL_MONTH, SQL_INTERVAL_SECOND},
      {SQL_C_INTERVAL_YEAR_TO_MONTH, SQL_INTERVAL_DAY},
      {SQL_C_INTERVAL_YEAR_TO_MONTH, SQL_INTERVAL_DAY_TO_SECOND},
      {SQL_C_INTERVAL_MONTH, SQL_INTERVAL_HOUR_TO_MINUTE},
      // day-time source -> year-month target
      {SQL_C_INTERVAL_DAY, SQL_INTERVAL_YEAR},
      {SQL_C_INTERVAL_SECOND, SQL_INTERVAL_MONTH},
      {SQL_C_INTERVAL_DAY_TO_SECOND, SQL_INTERVAL_YEAR},
      {SQL_C_INTERVAL_DAY_TO_SECOND, SQL_INTERVAL_YEAR_TO_MONTH},
      {SQL_C_INTERVAL_MINUTE_TO_SECOND, SQL_INTERVAL_YEAR_TO_MONTH},
  }));
  CAPTURE(c_type, sql_type);

  // Given a prepared statement targeting a VARCHAR column and an interval struct
  conn.execute("CREATE OR REPLACE TEMPORARY TABLE t (col VARCHAR(200))");
  auto stmt = conn.createStatement();
  SQLRETURN ret = SQLPrepare(stmt.getHandle(), sqlchar("INSERT INTO t VALUES (?)"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  SQL_INTERVAL_STRUCT val = ds_interval(SQL_FALSE, 1, 1, 1, 1, 0);
  SQLLEN ind = sizeof(val);

  // When the cross-family interval source is bound to the mismatched-family SQL_INTERVAL target and executed
  // Then the driver rejects the incompatible conversion with SQLSTATE 07006
  check_incompatible_bindparam(stmt, c_type, sql_type, &val, sizeof(val), &ind);
}

// ============================================================================
// NULL indicator
// ============================================================================

TEST_CASE_METHOD(ConnSchemaFixture, "should bind SQL_C_INTERVAL_YEAR with NULL indicator to SQL_INTERVAL_YEAR",
                 "[e2e][types][c_interval][conversion][sql_interval]") {
  // Given a VARCHAR column
  conn.execute("CREATE OR REPLACE TEMPORARY TABLE t (col VARCHAR(200))");

  // When a SQL_C_INTERVAL_YEAR is bound with SQL_NULL_DATA to SQL_INTERVAL_YEAR and inserted
  auto stmt = conn.createStatement();
  SQLRETURN ret = SQLPrepare(stmt.getHandle(), sqlchar("INSERT INTO t VALUES (?)"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  SQLLEN ind = SQL_NULL_DATA;
  ret = SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_INTERVAL_YEAR, SQL_INTERVAL_YEAR, 0, 0, nullptr, 0,
                         &ind);
  REQUIRE_ODBC(ret, stmt);
  ret = SQLExecute(stmt.getHandle());
  // Then the new driver stores NULL and the old driver returns HY000
  NEW_DRIVER_ONLY("BD#71") {
    REQUIRE_ODBC(ret, stmt);
    auto fetch_stmt = conn.execute_fetch("SELECT col FROM t");
    CHECK_FALSE(get_data_optional<SQL_C_CHAR>(fetch_stmt, 1).has_value());
  }
  OLD_DRIVER_ONLY("BD#71") {
    REQUIRE_THAT(OdbcResult(ret, stmt),
                 OdbcMatchers::IsError() && OdbcMatchers::HasSqlState("HY000") && OdbcMatchers::HasNativeError(18));
  }
}
