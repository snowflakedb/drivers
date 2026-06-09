//! Bitmask families advertised by `SQLGetInfo` (e.g. `SQL_AGGREGATE_FUNCTIONS`,
//! `SQL_NUMERIC_FUNCTIONS`).
//!
//! Each family is encoded as a positional `&'static [Flag]` slice. The `Flag`
//! at index `N` corresponds to bit `1 << N` of the resulting `SQLUINTEGER`
//! bitmask, mirroring the layout of the `SQL_*` constants in
//! `<sqlext.h>`. Set `enabled: true` to advertise support for that flag.
//! [`synthesize`] folds a slice into the final `u32` value.
//!
//! `synthesize` is a `const fn` that operates purely on the immutable input
//! slice, so callers like `write_u32(synthesize(SLICE), ...)` are
//! constant-folded by LLVM in release builds — there is no runtime loop.
//!
//! The flag tables match the values the reference Snowflake ODBC driver
//! advertises (see
//! `~/snowflake-odbc/Tests/EndToEndTests/ApiTest/SQLGetInfoValues.hpp`).
//! Flags that the reference driver does not enable for a given family are
//! still listed as `enabled: false` so the bit layout stays explicit.

/// A single bit in a `SQLGetInfo` bitmask. The bit's value is its position in
/// the parent `&[Flag]` slice — index 0 is `1 << 0`, index 1 is `1 << 1`, and
/// so on.
#[derive(Debug, Clone, Copy)]
pub struct Flag {
    /// `SQL_*` constant name — purely documentation, never read at runtime.
    #[allow(dead_code)]
    pub name: &'static str,
    /// Whether the driver advertises support for this flag.
    pub enabled: bool,
}

/// Collapse a positional `&[Flag]` slice into the `SQLUINTEGER` bitmask the
/// driver returns from `SQLGetInfo`.
pub const fn synthesize(flags: &[Flag]) -> u32 {
    let mut mask = 0u32;
    let mut i = 0;
    while i < flags.len() {
        if flags[i].enabled {
            mask |= 1u32 << i;
        }
        i += 1;
    }
    mask
}

/// `SQL_AGGREGATE_FUNCTIONS` (169) — supported aggregate functions.
pub const AGGREGATE_FUNCTIONS: &[Flag] = &[
    Flag {
        name: "SQL_AF_AVG",
        enabled: true,
    }, // bit 0
    Flag {
        name: "SQL_AF_COUNT",
        enabled: true,
    }, // bit 1
    Flag {
        name: "SQL_AF_MAX",
        enabled: true,
    }, // bit 2
    Flag {
        name: "SQL_AF_MIN",
        enabled: true,
    }, // bit 3
    Flag {
        name: "SQL_AF_SUM",
        enabled: true,
    }, // bit 4
    Flag {
        name: "SQL_AF_DISTINCT",
        enabled: true,
    }, // bit 5
    Flag {
        name: "SQL_AF_ALL",
        enabled: true,
    }, // bit 6
];

/// `SQL_CATALOG_USAGE` (92).
pub const CATALOG_USAGE: &[Flag] = &[
    Flag {
        name: "SQL_CU_DML_STATEMENTS",
        enabled: true,
    }, // bit 0
    Flag {
        name: "SQL_CU_PROCEDURE_INVOCATION",
        enabled: false,
    }, // bit 1
    Flag {
        name: "SQL_CU_TABLE_DEFINITION",
        enabled: true,
    }, // bit 2
    Flag {
        name: "SQL_CU_INDEX_DEFINITION",
        enabled: false,
    }, // bit 3
    Flag {
        name: "SQL_CU_PRIVILEGE_DEFINITION",
        enabled: true,
    }, // bit 4
];

/// `SQL_SCHEMA_USAGE` (91) — same bit layout as `SQL_CATALOG_USAGE`.
pub const SCHEMA_USAGE: &[Flag] = &[
    Flag {
        name: "SQL_SU_DML_STATEMENTS",
        enabled: true,
    }, // bit 0
    Flag {
        name: "SQL_SU_PROCEDURE_INVOCATION",
        enabled: false,
    }, // bit 1
    Flag {
        name: "SQL_SU_TABLE_DEFINITION",
        enabled: true,
    }, // bit 2
    Flag {
        name: "SQL_SU_INDEX_DEFINITION",
        enabled: false,
    }, // bit 3
    Flag {
        name: "SQL_SU_PRIVILEGE_DEFINITION",
        enabled: true,
    }, // bit 4
];

/// `SQL_CONVERT_FUNCTIONS` (48) — supported `CAST`/`CONVERT`.
pub const CONVERT_FUNCTIONS: &[Flag] = &[
    Flag {
        name: "SQL_FN_CVT_CONVERT",
        enabled: true,
    }, // bit 0
    Flag {
        name: "SQL_FN_CVT_CAST",
        enabled: true,
    }, // bit 1
];

/// `SQL_NUMERIC_FUNCTIONS` (49) — supported numeric scalar functions.
pub const NUMERIC_FUNCTIONS: &[Flag] = &[
    Flag {
        name: "SQL_FN_NUM_ABS",
        enabled: true,
    }, // bit 0
    Flag {
        name: "SQL_FN_NUM_ACOS",
        enabled: true,
    }, // bit 1
    Flag {
        name: "SQL_FN_NUM_ASIN",
        enabled: true,
    }, // bit 2
    Flag {
        name: "SQL_FN_NUM_ATAN",
        enabled: true,
    }, // bit 3
    Flag {
        name: "SQL_FN_NUM_ATAN2",
        enabled: true,
    }, // bit 4
    Flag {
        name: "SQL_FN_NUM_CEILING",
        enabled: true,
    }, // bit 5
    Flag {
        name: "SQL_FN_NUM_COS",
        enabled: true,
    }, // bit 6
    Flag {
        name: "SQL_FN_NUM_COT",
        enabled: true,
    }, // bit 7
    Flag {
        name: "SQL_FN_NUM_EXP",
        enabled: true,
    }, // bit 8
    Flag {
        name: "SQL_FN_NUM_FLOOR",
        enabled: true,
    }, // bit 9
    Flag {
        name: "SQL_FN_NUM_LOG",
        enabled: true,
    }, // bit 10
    Flag {
        name: "SQL_FN_NUM_MOD",
        enabled: true,
    }, // bit 11
    Flag {
        name: "SQL_FN_NUM_SIGN",
        enabled: true,
    }, // bit 12
    Flag {
        name: "SQL_FN_NUM_SIN",
        enabled: true,
    }, // bit 13
    Flag {
        name: "SQL_FN_NUM_SQRT",
        enabled: true,
    }, // bit 14
    Flag {
        name: "SQL_FN_NUM_TAN",
        enabled: true,
    }, // bit 15
    Flag {
        name: "SQL_FN_NUM_PI",
        enabled: true,
    }, // bit 16
    Flag {
        name: "SQL_FN_NUM_RAND",
        enabled: true,
    }, // bit 17
    Flag {
        name: "SQL_FN_NUM_DEGREES",
        enabled: true,
    }, // bit 18
    Flag {
        name: "SQL_FN_NUM_LOG10",
        enabled: false,
    }, // bit 19
    Flag {
        name: "SQL_FN_NUM_POWER",
        enabled: true,
    }, // bit 20
    Flag {
        name: "SQL_FN_NUM_RADIANS",
        enabled: true,
    }, // bit 21
    Flag {
        name: "SQL_FN_NUM_ROUND",
        enabled: true,
    }, // bit 22
    Flag {
        name: "SQL_FN_NUM_TRUNCATE",
        enabled: true,
    }, // bit 23
];

/// `SQL_STRING_FUNCTIONS` (50) — supported string scalar functions.
pub const STRING_FUNCTIONS: &[Flag] = &[
    Flag {
        name: "SQL_FN_STR_CONCAT",
        enabled: true,
    }, // bit 0
    Flag {
        name: "SQL_FN_STR_INSERT",
        enabled: true,
    }, // bit 1
    Flag {
        name: "SQL_FN_STR_LEFT",
        enabled: true,
    }, // bit 2
    Flag {
        name: "SQL_FN_STR_LTRIM",
        enabled: true,
    }, // bit 3
    Flag {
        name: "SQL_FN_STR_LENGTH",
        enabled: true,
    }, // bit 4
    Flag {
        name: "SQL_FN_STR_LOCATE",
        enabled: true,
    }, // bit 5
    Flag {
        name: "SQL_FN_STR_LCASE",
        enabled: true,
    }, // bit 6
    Flag {
        name: "SQL_FN_STR_REPEAT",
        enabled: true,
    }, // bit 7
    Flag {
        name: "SQL_FN_STR_REPLACE",
        enabled: true,
    }, // bit 8
    Flag {
        name: "SQL_FN_STR_RIGHT",
        enabled: true,
    }, // bit 9
    Flag {
        name: "SQL_FN_STR_RTRIM",
        enabled: true,
    }, // bit 10
    Flag {
        name: "SQL_FN_STR_SUBSTRING",
        enabled: true,
    }, // bit 11
    Flag {
        name: "SQL_FN_STR_UCASE",
        enabled: true,
    }, // bit 12
    Flag {
        name: "SQL_FN_STR_ASCII",
        enabled: true,
    }, // bit 13
    Flag {
        name: "SQL_FN_STR_CHAR",
        enabled: true,
    }, // bit 14
    Flag {
        name: "SQL_FN_STR_DIFFERENCE",
        enabled: false,
    }, // bit 15
    Flag {
        name: "SQL_FN_STR_LOCATE_2",
        enabled: true,
    }, // bit 16
    Flag {
        name: "SQL_FN_STR_SOUNDEX",
        enabled: false,
    }, // bit 17
    Flag {
        name: "SQL_FN_STR_SPACE",
        enabled: true,
    }, // bit 18
    Flag {
        name: "SQL_FN_STR_BIT_LENGTH",
        enabled: true,
    }, // bit 19
    Flag {
        name: "SQL_FN_STR_CHAR_LENGTH",
        enabled: true,
    }, // bit 20
    Flag {
        name: "SQL_FN_STR_CHARACTER_LENGTH",
        enabled: true,
    }, // bit 21
    Flag {
        name: "SQL_FN_STR_OCTET_LENGTH",
        enabled: true,
    }, // bit 22
    Flag {
        name: "SQL_FN_STR_POSITION",
        enabled: true,
    }, // bit 23
];

/// `SQL_SYSTEM_FUNCTIONS` (51) — supported system scalar functions.
pub const SYSTEM_FUNCTIONS: &[Flag] = &[
    Flag {
        name: "SQL_FN_SYS_USERNAME",
        enabled: true,
    }, // bit 0
    Flag {
        name: "SQL_FN_SYS_DBNAME",
        enabled: true,
    }, // bit 1
    Flag {
        name: "SQL_FN_SYS_IFNULL",
        enabled: true,
    }, // bit 2
];

/// `SQL_TIMEDATE_FUNCTIONS` (52) — supported timedate scalar functions.
pub const TIMEDATE_FUNCTIONS: &[Flag] = &[
    Flag {
        name: "SQL_FN_TD_NOW",
        enabled: true,
    }, // bit 0
    Flag {
        name: "SQL_FN_TD_CURDATE",
        enabled: true,
    }, // bit 1
    Flag {
        name: "SQL_FN_TD_DAYOFMONTH",
        enabled: true,
    }, // bit 2
    Flag {
        name: "SQL_FN_TD_DAYOFWEEK",
        enabled: true,
    }, // bit 3
    Flag {
        name: "SQL_FN_TD_DAYOFYEAR",
        enabled: true,
    }, // bit 4
    Flag {
        name: "SQL_FN_TD_MONTH",
        enabled: true,
    }, // bit 5
    Flag {
        name: "SQL_FN_TD_QUARTER",
        enabled: true,
    }, // bit 6
    Flag {
        name: "SQL_FN_TD_WEEK",
        enabled: true,
    }, // bit 7
    Flag {
        name: "SQL_FN_TD_YEAR",
        enabled: true,
    }, // bit 8
    Flag {
        name: "SQL_FN_TD_CURTIME",
        enabled: true,
    }, // bit 9
    Flag {
        name: "SQL_FN_TD_HOUR",
        enabled: true,
    }, // bit 10
    Flag {
        name: "SQL_FN_TD_MINUTE",
        enabled: true,
    }, // bit 11
    Flag {
        name: "SQL_FN_TD_SECOND",
        enabled: true,
    }, // bit 12
    Flag {
        name: "SQL_FN_TD_TIMESTAMPADD",
        enabled: true,
    }, // bit 13
    Flag {
        name: "SQL_FN_TD_TIMESTAMPDIFF",
        enabled: true,
    }, // bit 14
    Flag {
        name: "SQL_FN_TD_DAYNAME",
        enabled: true,
    }, // bit 15
    Flag {
        name: "SQL_FN_TD_MONTHNAME",
        enabled: true,
    }, // bit 16
    Flag {
        name: "SQL_FN_TD_CURRENT_DATE",
        enabled: true,
    }, // bit 17
    Flag {
        name: "SQL_FN_TD_CURRENT_TIME",
        enabled: true,
    }, // bit 18
    Flag {
        name: "SQL_FN_TD_CURRENT_TIMESTAMP",
        enabled: true,
    }, // bit 19
    Flag {
        name: "SQL_FN_TD_EXTRACT",
        enabled: true,
    }, // bit 20
];

/// `SQL_TIMEDATE_ADD_INTERVALS` (109) and `SQL_TIMEDATE_DIFF_INTERVALS` (110)
/// share the same `SQL_FN_TSI_*` bit layout. The reference driver advertises
/// the same set for both.
pub const TIMEDATE_TSI_INTERVALS: &[Flag] = &[
    Flag {
        name: "SQL_FN_TSI_FRAC_SECOND",
        enabled: true,
    }, // bit 0
    Flag {
        name: "SQL_FN_TSI_SECOND",
        enabled: true,
    }, // bit 1
    Flag {
        name: "SQL_FN_TSI_MINUTE",
        enabled: true,
    }, // bit 2
    Flag {
        name: "SQL_FN_TSI_HOUR",
        enabled: true,
    }, // bit 3
    Flag {
        name: "SQL_FN_TSI_DAY",
        enabled: true,
    }, // bit 4
    Flag {
        name: "SQL_FN_TSI_WEEK",
        enabled: true,
    }, // bit 5
    Flag {
        name: "SQL_FN_TSI_MONTH",
        enabled: true,
    }, // bit 6
    Flag {
        name: "SQL_FN_TSI_QUARTER",
        enabled: true,
    }, // bit 7
    Flag {
        name: "SQL_FN_TSI_YEAR",
        enabled: true,
    }, // bit 8
];

/// `SQL_SQL92_PREDICATES` (160).
pub const SQL92_PREDICATES: &[Flag] = &[
    Flag {
        name: "SQL_SP_EXISTS",
        enabled: true,
    }, // bit 0
    Flag {
        name: "SQL_SP_ISNOTNULL",
        enabled: false,
    }, // bit 1
    Flag {
        name: "SQL_SP_ISNULL",
        enabled: true,
    }, // bit 2
    Flag {
        name: "SQL_SP_MATCH_FULL",
        enabled: false,
    }, // bit 3
    Flag {
        name: "SQL_SP_MATCH_PARTIAL",
        enabled: false,
    }, // bit 4
    Flag {
        name: "SQL_SP_MATCH_UNIQUE_FULL",
        enabled: false,
    }, // bit 5
    Flag {
        name: "SQL_SP_MATCH_UNIQUE_PARTIAL",
        enabled: false,
    }, // bit 6
    Flag {
        name: "SQL_SP_OVERLAPS",
        enabled: false,
    }, // bit 7
    Flag {
        name: "SQL_SP_UNIQUE",
        enabled: true,
    }, // bit 8
    Flag {
        name: "SQL_SP_LIKE",
        enabled: true,
    }, // bit 9
    Flag {
        name: "SQL_SP_IN",
        enabled: true,
    }, // bit 10
    Flag {
        name: "SQL_SP_BETWEEN",
        enabled: true,
    }, // bit 11
    Flag {
        name: "SQL_SP_COMPARISON",
        enabled: true,
    }, // bit 12
    Flag {
        name: "SQL_SP_QUANTIFIED_COMPARISON",
        enabled: true,
    }, // bit 13
];

/// `SQL_SQL92_RELATIONAL_JOIN_OPERATORS` (161).
pub const SQL92_RELATIONAL_JOIN_OPERATORS: &[Flag] = &[
    Flag {
        name: "SQL_SRJO_CORRESPONDING_CLAUSE",
        enabled: false,
    }, // bit 0
    Flag {
        name: "SQL_SRJO_CROSS_JOIN",
        enabled: true,
    }, // bit 1
    Flag {
        name: "SQL_SRJO_EXCEPT_JOIN",
        enabled: false,
    }, // bit 2
    Flag {
        name: "SQL_SRJO_FULL_OUTER_JOIN",
        enabled: true,
    }, // bit 3
    Flag {
        name: "SQL_SRJO_INNER_JOIN",
        enabled: true,
    }, // bit 4
    Flag {
        name: "SQL_SRJO_INTERSECT_JOIN",
        enabled: false,
    }, // bit 5
    Flag {
        name: "SQL_SRJO_LEFT_OUTER_JOIN",
        enabled: true,
    }, // bit 6
    Flag {
        name: "SQL_SRJO_NATURAL_JOIN",
        enabled: false,
    }, // bit 7
    Flag {
        name: "SQL_SRJO_RIGHT_OUTER_JOIN",
        enabled: true,
    }, // bit 8
    Flag {
        name: "SQL_SRJO_UNION_JOIN",
        enabled: false,
    }, // bit 9
];

/// `SQL_SQL92_VALUE_EXPRESSIONS` (165).
pub const SQL92_VALUE_EXPRESSIONS: &[Flag] = &[
    Flag {
        name: "SQL_SVE_CASE",
        enabled: true,
    }, // bit 0
    Flag {
        name: "SQL_SVE_CAST",
        enabled: true,
    }, // bit 1
    Flag {
        name: "SQL_SVE_COALESCE",
        enabled: true,
    }, // bit 2
    Flag {
        name: "SQL_SVE_NULLIF",
        enabled: true,
    }, // bit 3
];

// ----- `SQL_CONVERT_<source>` families -------------------------------------
//
// Each `SQL_CONVERT_<source>` InfoType returns a bitmask over the possible
// `SQL_CVT_<target>` conversion targets. The bit positions below mirror the
// `SQL_CVT_*` constants in `<sqlext.h>`. The reference Snowflake ODBC driver
// advertises these target sets per source type via the Simba SDK's
// `DSI_CONN_SUPPORTED_SQL_<source>_CONVERSIONS` properties (see
// `~/snowflake-odbc/Source/Core/SFConnection.cpp`). The strict tests in
// `odbc_tests/tests/odbc-api/driver_info/get_info_tests.cpp` pin the per-bit
// expectations; the flag tables below match those expectations exactly.

/// `SQL_CONVERT_BIGINT` (53) — conversion targets from `BIGINT` source.
pub const CONVERT_BIGINT: &[Flag] = &[
    Flag {
        name: "SQL_CVT_CHAR",
        enabled: true,
    }, // bit 0
    Flag {
        name: "SQL_CVT_NUMERIC",
        enabled: true,
    }, // bit 1
    Flag {
        name: "SQL_CVT_DECIMAL",
        enabled: true,
    }, // bit 2
    Flag {
        name: "SQL_CVT_INTEGER",
        enabled: true,
    }, // bit 3
    Flag {
        name: "SQL_CVT_SMALLINT",
        enabled: true,
    }, // bit 4
    Flag {
        name: "SQL_CVT_FLOAT",
        enabled: true,
    }, // bit 5
    Flag {
        name: "SQL_CVT_REAL",
        enabled: true,
    }, // bit 6
    Flag {
        name: "SQL_CVT_DOUBLE",
        enabled: true,
    }, // bit 7
    Flag {
        name: "SQL_CVT_VARCHAR",
        enabled: true,
    }, // bit 8
    Flag {
        name: "SQL_CVT_LONGVARCHAR",
        enabled: true,
    }, // bit 9
    Flag {
        name: "SQL_CVT_BINARY",
        enabled: true,
    }, // bit 10
    Flag {
        name: "SQL_CVT_VARBINARY",
        enabled: true,
    }, // bit 11
    Flag {
        name: "SQL_CVT_BIT",
        enabled: true,
    }, // bit 12
    Flag {
        name: "SQL_CVT_TINYINT",
        enabled: true,
    }, // bit 13
    Flag {
        name: "SQL_CVT_BIGINT",
        enabled: true,
    }, // bit 14
    Flag {
        name: "SQL_CVT_DATE",
        enabled: false,
    }, // bit 15
    Flag {
        name: "SQL_CVT_TIME",
        enabled: false,
    }, // bit 16
    Flag {
        name: "SQL_CVT_TIMESTAMP",
        enabled: false,
    }, // bit 17
    Flag {
        name: "SQL_CVT_LONGVARBINARY",
        enabled: false,
    }, // bit 18
    Flag {
        name: "SQL_CVT_INTERVAL_YEAR_MONTH",
        enabled: true,
    }, // bit 19
    Flag {
        name: "SQL_CVT_INTERVAL_DAY_TIME",
        enabled: true,
    }, // bit 20
    Flag {
        name: "SQL_CVT_WCHAR",
        enabled: true,
    }, // bit 21
    Flag {
        name: "SQL_CVT_WLONGVARCHAR",
        enabled: true,
    }, // bit 22
    Flag {
        name: "SQL_CVT_WVARCHAR",
        enabled: true,
    }, // bit 23
    Flag {
        name: "SQL_CVT_GUID",
        enabled: false,
    }, // bit 24
];

/// `SQL_CONVERT_BINARY` (54) — conversion targets from `BINARY` source.
pub const CONVERT_BINARY: &[Flag] = &[
    Flag {
        name: "SQL_CVT_CHAR",
        enabled: true,
    }, // bit 0
    Flag {
        name: "SQL_CVT_NUMERIC",
        enabled: false,
    }, // bit 1
    Flag {
        name: "SQL_CVT_DECIMAL",
        enabled: false,
    }, // bit 2
    Flag {
        name: "SQL_CVT_INTEGER",
        enabled: false,
    }, // bit 3
    Flag {
        name: "SQL_CVT_SMALLINT",
        enabled: false,
    }, // bit 4
    Flag {
        name: "SQL_CVT_FLOAT",
        enabled: false,
    }, // bit 5
    Flag {
        name: "SQL_CVT_REAL",
        enabled: false,
    }, // bit 6
    Flag {
        name: "SQL_CVT_DOUBLE",
        enabled: false,
    }, // bit 7
    Flag {
        name: "SQL_CVT_VARCHAR",
        enabled: true,
    }, // bit 8
    Flag {
        name: "SQL_CVT_LONGVARCHAR",
        enabled: false,
    }, // bit 9
    Flag {
        name: "SQL_CVT_BINARY",
        enabled: true,
    }, // bit 10
    Flag {
        name: "SQL_CVT_VARBINARY",
        enabled: true,
    }, // bit 11
    Flag {
        name: "SQL_CVT_BIT",
        enabled: false,
    }, // bit 12
    Flag {
        name: "SQL_CVT_TINYINT",
        enabled: false,
    }, // bit 13
    Flag {
        name: "SQL_CVT_BIGINT",
        enabled: false,
    }, // bit 14
    Flag {
        name: "SQL_CVT_DATE",
        enabled: false,
    }, // bit 15
    Flag {
        name: "SQL_CVT_TIME",
        enabled: false,
    }, // bit 16
    Flag {
        name: "SQL_CVT_TIMESTAMP",
        enabled: false,
    }, // bit 17
    Flag {
        name: "SQL_CVT_LONGVARBINARY",
        enabled: false,
    }, // bit 18
    Flag {
        name: "SQL_CVT_INTERVAL_YEAR_MONTH",
        enabled: false,
    }, // bit 19
    Flag {
        name: "SQL_CVT_INTERVAL_DAY_TIME",
        enabled: false,
    }, // bit 20
    Flag {
        name: "SQL_CVT_WCHAR",
        enabled: true,
    }, // bit 21
    Flag {
        name: "SQL_CVT_WLONGVARCHAR",
        enabled: false,
    }, // bit 22
    Flag {
        name: "SQL_CVT_WVARCHAR",
        enabled: true,
    }, // bit 23
    Flag {
        name: "SQL_CVT_GUID",
        enabled: false,
    }, // bit 24
];

/// `SQL_CONVERT_BIT` (55) — conversion targets from `BIT` source.
pub const CONVERT_BIT: &[Flag] = &[
    Flag {
        name: "SQL_CVT_CHAR",
        enabled: true,
    }, // bit 0
    Flag {
        name: "SQL_CVT_NUMERIC",
        enabled: true,
    }, // bit 1
    Flag {
        name: "SQL_CVT_DECIMAL",
        enabled: true,
    }, // bit 2
    Flag {
        name: "SQL_CVT_INTEGER",
        enabled: true,
    }, // bit 3
    Flag {
        name: "SQL_CVT_SMALLINT",
        enabled: true,
    }, // bit 4
    Flag {
        name: "SQL_CVT_FLOAT",
        enabled: true,
    }, // bit 5
    Flag {
        name: "SQL_CVT_REAL",
        enabled: true,
    }, // bit 6
    Flag {
        name: "SQL_CVT_DOUBLE",
        enabled: true,
    }, // bit 7
    Flag {
        name: "SQL_CVT_VARCHAR",
        enabled: true,
    }, // bit 8
    Flag {
        name: "SQL_CVT_LONGVARCHAR",
        enabled: true,
    }, // bit 9
    Flag {
        name: "SQL_CVT_BINARY",
        enabled: true,
    }, // bit 10
    Flag {
        name: "SQL_CVT_VARBINARY",
        enabled: true,
    }, // bit 11
    Flag {
        name: "SQL_CVT_BIT",
        enabled: true,
    }, // bit 12
    Flag {
        name: "SQL_CVT_TINYINT",
        enabled: true,
    }, // bit 13
    Flag {
        name: "SQL_CVT_BIGINT",
        enabled: true,
    }, // bit 14
    Flag {
        name: "SQL_CVT_DATE",
        enabled: false,
    }, // bit 15
    Flag {
        name: "SQL_CVT_TIME",
        enabled: false,
    }, // bit 16
    Flag {
        name: "SQL_CVT_TIMESTAMP",
        enabled: false,
    }, // bit 17
    Flag {
        name: "SQL_CVT_LONGVARBINARY",
        enabled: true,
    }, // bit 18
    Flag {
        name: "SQL_CVT_INTERVAL_YEAR_MONTH",
        enabled: false,
    }, // bit 19
    Flag {
        name: "SQL_CVT_INTERVAL_DAY_TIME",
        enabled: false,
    }, // bit 20
    Flag {
        name: "SQL_CVT_WCHAR",
        enabled: true,
    }, // bit 21
    Flag {
        name: "SQL_CVT_WLONGVARCHAR",
        enabled: true,
    }, // bit 22
    Flag {
        name: "SQL_CVT_WVARCHAR",
        enabled: true,
    }, // bit 23
    Flag {
        name: "SQL_CVT_GUID",
        enabled: false,
    }, // bit 24
];

/// `SQL_CONVERT_CHAR` (56) — conversion targets from `CHAR` source.
pub const CONVERT_CHAR: &[Flag] = &[
    Flag {
        name: "SQL_CVT_CHAR",
        enabled: true,
    }, // bit 0
    Flag {
        name: "SQL_CVT_NUMERIC",
        enabled: true,
    }, // bit 1
    Flag {
        name: "SQL_CVT_DECIMAL",
        enabled: true,
    }, // bit 2
    Flag {
        name: "SQL_CVT_INTEGER",
        enabled: true,
    }, // bit 3
    Flag {
        name: "SQL_CVT_SMALLINT",
        enabled: true,
    }, // bit 4
    Flag {
        name: "SQL_CVT_FLOAT",
        enabled: true,
    }, // bit 5
    Flag {
        name: "SQL_CVT_REAL",
        enabled: true,
    }, // bit 6
    Flag {
        name: "SQL_CVT_DOUBLE",
        enabled: true,
    }, // bit 7
    Flag {
        name: "SQL_CVT_VARCHAR",
        enabled: true,
    }, // bit 8
    Flag {
        name: "SQL_CVT_LONGVARCHAR",
        enabled: false,
    }, // bit 9
    Flag {
        name: "SQL_CVT_BINARY",
        enabled: true,
    }, // bit 10
    Flag {
        name: "SQL_CVT_VARBINARY",
        enabled: true,
    }, // bit 11
    Flag {
        name: "SQL_CVT_BIT",
        enabled: false,
    }, // bit 12
    Flag {
        name: "SQL_CVT_TINYINT",
        enabled: true,
    }, // bit 13
    Flag {
        name: "SQL_CVT_BIGINT",
        enabled: true,
    }, // bit 14
    Flag {
        name: "SQL_CVT_DATE",
        enabled: true,
    }, // bit 15
    Flag {
        name: "SQL_CVT_TIME",
        enabled: true,
    }, // bit 16
    Flag {
        name: "SQL_CVT_TIMESTAMP",
        enabled: true,
    }, // bit 17
    Flag {
        name: "SQL_CVT_LONGVARBINARY",
        enabled: false,
    }, // bit 18
    Flag {
        name: "SQL_CVT_INTERVAL_YEAR_MONTH",
        enabled: false,
    }, // bit 19
    Flag {
        name: "SQL_CVT_INTERVAL_DAY_TIME",
        enabled: false,
    }, // bit 20
    Flag {
        name: "SQL_CVT_WCHAR",
        enabled: true,
    }, // bit 21
    Flag {
        name: "SQL_CVT_WLONGVARCHAR",
        enabled: false,
    }, // bit 22
    Flag {
        name: "SQL_CVT_WVARCHAR",
        enabled: true,
    }, // bit 23
    Flag {
        name: "SQL_CVT_GUID",
        enabled: false,
    }, // bit 24
];

/// `SQL_CONVERT_DATE` (57) — conversion targets from `DATE` source.
pub const CONVERT_DATE: &[Flag] = &[
    Flag {
        name: "SQL_CVT_CHAR",
        enabled: true,
    }, // bit 0
    Flag {
        name: "SQL_CVT_NUMERIC",
        enabled: false,
    }, // bit 1
    Flag {
        name: "SQL_CVT_DECIMAL",
        enabled: false,
    }, // bit 2
    Flag {
        name: "SQL_CVT_INTEGER",
        enabled: false,
    }, // bit 3
    Flag {
        name: "SQL_CVT_SMALLINT",
        enabled: false,
    }, // bit 4
    Flag {
        name: "SQL_CVT_FLOAT",
        enabled: false,
    }, // bit 5
    Flag {
        name: "SQL_CVT_REAL",
        enabled: false,
    }, // bit 6
    Flag {
        name: "SQL_CVT_DOUBLE",
        enabled: false,
    }, // bit 7
    Flag {
        name: "SQL_CVT_VARCHAR",
        enabled: true,
    }, // bit 8
    Flag {
        name: "SQL_CVT_LONGVARCHAR",
        enabled: false,
    }, // bit 9
    Flag {
        name: "SQL_CVT_BINARY",
        enabled: false,
    }, // bit 10
    Flag {
        name: "SQL_CVT_VARBINARY",
        enabled: false,
    }, // bit 11
    Flag {
        name: "SQL_CVT_BIT",
        enabled: false,
    }, // bit 12
    Flag {
        name: "SQL_CVT_TINYINT",
        enabled: false,
    }, // bit 13
    Flag {
        name: "SQL_CVT_BIGINT",
        enabled: false,
    }, // bit 14
    Flag {
        name: "SQL_CVT_DATE",
        enabled: true,
    }, // bit 15
    Flag {
        name: "SQL_CVT_TIME",
        enabled: false,
    }, // bit 16
    Flag {
        name: "SQL_CVT_TIMESTAMP",
        enabled: true,
    }, // bit 17
    Flag {
        name: "SQL_CVT_LONGVARBINARY",
        enabled: false,
    }, // bit 18
    Flag {
        name: "SQL_CVT_INTERVAL_YEAR_MONTH",
        enabled: false,
    }, // bit 19
    Flag {
        name: "SQL_CVT_INTERVAL_DAY_TIME",
        enabled: false,
    }, // bit 20
    Flag {
        name: "SQL_CVT_WCHAR",
        enabled: true,
    }, // bit 21
    Flag {
        name: "SQL_CVT_WLONGVARCHAR",
        enabled: false,
    }, // bit 22
    Flag {
        name: "SQL_CVT_WVARCHAR",
        enabled: true,
    }, // bit 23
    Flag {
        name: "SQL_CVT_GUID",
        enabled: false,
    }, // bit 24
];

/// `SQL_CONVERT_DECIMAL` (58) — conversion targets from `DECIMAL` source.
pub const CONVERT_DECIMAL: &[Flag] = &[
    Flag {
        name: "SQL_CVT_CHAR",
        enabled: true,
    }, // bit 0
    Flag {
        name: "SQL_CVT_NUMERIC",
        enabled: true,
    }, // bit 1
    Flag {
        name: "SQL_CVT_DECIMAL",
        enabled: true,
    }, // bit 2
    Flag {
        name: "SQL_CVT_INTEGER",
        enabled: true,
    }, // bit 3
    Flag {
        name: "SQL_CVT_SMALLINT",
        enabled: false,
    }, // bit 4
    Flag {
        name: "SQL_CVT_FLOAT",
        enabled: true,
    }, // bit 5
    Flag {
        name: "SQL_CVT_REAL",
        enabled: false,
    }, // bit 6
    Flag {
        name: "SQL_CVT_DOUBLE",
        enabled: true,
    }, // bit 7
    Flag {
        name: "SQL_CVT_VARCHAR",
        enabled: true,
    }, // bit 8
    Flag {
        name: "SQL_CVT_LONGVARCHAR",
        enabled: false,
    }, // bit 9
    Flag {
        name: "SQL_CVT_BINARY",
        enabled: false,
    }, // bit 10
    Flag {
        name: "SQL_CVT_VARBINARY",
        enabled: false,
    }, // bit 11
    Flag {
        name: "SQL_CVT_BIT",
        enabled: false,
    }, // bit 12
    Flag {
        name: "SQL_CVT_TINYINT",
        enabled: false,
    }, // bit 13
    Flag {
        name: "SQL_CVT_BIGINT",
        enabled: false,
    }, // bit 14
    Flag {
        name: "SQL_CVT_DATE",
        enabled: false,
    }, // bit 15
    Flag {
        name: "SQL_CVT_TIME",
        enabled: false,
    }, // bit 16
    Flag {
        name: "SQL_CVT_TIMESTAMP",
        enabled: false,
    }, // bit 17
    Flag {
        name: "SQL_CVT_LONGVARBINARY",
        enabled: false,
    }, // bit 18
    Flag {
        name: "SQL_CVT_INTERVAL_YEAR_MONTH",
        enabled: false,
    }, // bit 19
    Flag {
        name: "SQL_CVT_INTERVAL_DAY_TIME",
        enabled: false,
    }, // bit 20
    Flag {
        name: "SQL_CVT_WCHAR",
        enabled: true,
    }, // bit 21
    Flag {
        name: "SQL_CVT_WLONGVARCHAR",
        enabled: false,
    }, // bit 22
    Flag {
        name: "SQL_CVT_WVARCHAR",
        enabled: true,
    }, // bit 23
    Flag {
        name: "SQL_CVT_GUID",
        enabled: false,
    }, // bit 24
];

/// `SQL_CONVERT_DOUBLE` (59) — conversion targets from `DOUBLE` source.
pub const CONVERT_DOUBLE: &[Flag] = &[
    Flag {
        name: "SQL_CVT_CHAR",
        enabled: true,
    }, // bit 0
    Flag {
        name: "SQL_CVT_NUMERIC",
        enabled: true,
    }, // bit 1
    Flag {
        name: "SQL_CVT_DECIMAL",
        enabled: true,
    }, // bit 2
    Flag {
        name: "SQL_CVT_INTEGER",
        enabled: true,
    }, // bit 3
    Flag {
        name: "SQL_CVT_SMALLINT",
        enabled: false,
    }, // bit 4
    Flag {
        name: "SQL_CVT_FLOAT",
        enabled: true,
    }, // bit 5
    Flag {
        name: "SQL_CVT_REAL",
        enabled: false,
    }, // bit 6
    Flag {
        name: "SQL_CVT_DOUBLE",
        enabled: true,
    }, // bit 7
    Flag {
        name: "SQL_CVT_VARCHAR",
        enabled: true,
    }, // bit 8
    Flag {
        name: "SQL_CVT_LONGVARCHAR",
        enabled: false,
    }, // bit 9
    Flag {
        name: "SQL_CVT_BINARY",
        enabled: false,
    }, // bit 10
    Flag {
        name: "SQL_CVT_VARBINARY",
        enabled: false,
    }, // bit 11
    Flag {
        name: "SQL_CVT_BIT",
        enabled: false,
    }, // bit 12
    Flag {
        name: "SQL_CVT_TINYINT",
        enabled: false,
    }, // bit 13
    Flag {
        name: "SQL_CVT_BIGINT",
        enabled: false,
    }, // bit 14
    Flag {
        name: "SQL_CVT_DATE",
        enabled: false,
    }, // bit 15
    Flag {
        name: "SQL_CVT_TIME",
        enabled: false,
    }, // bit 16
    Flag {
        name: "SQL_CVT_TIMESTAMP",
        enabled: false,
    }, // bit 17
    Flag {
        name: "SQL_CVT_LONGVARBINARY",
        enabled: false,
    }, // bit 18
    Flag {
        name: "SQL_CVT_INTERVAL_YEAR_MONTH",
        enabled: false,
    }, // bit 19
    Flag {
        name: "SQL_CVT_INTERVAL_DAY_TIME",
        enabled: false,
    }, // bit 20
    Flag {
        name: "SQL_CVT_WCHAR",
        enabled: true,
    }, // bit 21
    Flag {
        name: "SQL_CVT_WLONGVARCHAR",
        enabled: false,
    }, // bit 22
    Flag {
        name: "SQL_CVT_WVARCHAR",
        enabled: true,
    }, // bit 23
    Flag {
        name: "SQL_CVT_GUID",
        enabled: false,
    }, // bit 24
];

/// `SQL_CONVERT_FLOAT` (60) — conversion targets from `FLOAT` source.
pub const CONVERT_FLOAT: &[Flag] = &[
    Flag {
        name: "SQL_CVT_CHAR",
        enabled: true,
    }, // bit 0
    Flag {
        name: "SQL_CVT_NUMERIC",
        enabled: true,
    }, // bit 1
    Flag {
        name: "SQL_CVT_DECIMAL",
        enabled: true,
    }, // bit 2
    Flag {
        name: "SQL_CVT_INTEGER",
        enabled: true,
    }, // bit 3
    Flag {
        name: "SQL_CVT_SMALLINT",
        enabled: false,
    }, // bit 4
    Flag {
        name: "SQL_CVT_FLOAT",
        enabled: true,
    }, // bit 5
    Flag {
        name: "SQL_CVT_REAL",
        enabled: false,
    }, // bit 6
    Flag {
        name: "SQL_CVT_DOUBLE",
        enabled: true,
    }, // bit 7
    Flag {
        name: "SQL_CVT_VARCHAR",
        enabled: true,
    }, // bit 8
    Flag {
        name: "SQL_CVT_LONGVARCHAR",
        enabled: false,
    }, // bit 9
    Flag {
        name: "SQL_CVT_BINARY",
        enabled: false,
    }, // bit 10
    Flag {
        name: "SQL_CVT_VARBINARY",
        enabled: false,
    }, // bit 11
    Flag {
        name: "SQL_CVT_BIT",
        enabled: false,
    }, // bit 12
    Flag {
        name: "SQL_CVT_TINYINT",
        enabled: false,
    }, // bit 13
    Flag {
        name: "SQL_CVT_BIGINT",
        enabled: false,
    }, // bit 14
    Flag {
        name: "SQL_CVT_DATE",
        enabled: false,
    }, // bit 15
    Flag {
        name: "SQL_CVT_TIME",
        enabled: false,
    }, // bit 16
    Flag {
        name: "SQL_CVT_TIMESTAMP",
        enabled: false,
    }, // bit 17
    Flag {
        name: "SQL_CVT_LONGVARBINARY",
        enabled: false,
    }, // bit 18
    Flag {
        name: "SQL_CVT_INTERVAL_YEAR_MONTH",
        enabled: false,
    }, // bit 19
    Flag {
        name: "SQL_CVT_INTERVAL_DAY_TIME",
        enabled: false,
    }, // bit 20
    Flag {
        name: "SQL_CVT_WCHAR",
        enabled: true,
    }, // bit 21
    Flag {
        name: "SQL_CVT_WLONGVARCHAR",
        enabled: false,
    }, // bit 22
    Flag {
        name: "SQL_CVT_WVARCHAR",
        enabled: true,
    }, // bit 23
    Flag {
        name: "SQL_CVT_GUID",
        enabled: false,
    }, // bit 24
];

/// `SQL_CONVERT_GUID` (173) — conversion targets from `GUID` source.
pub const CONVERT_GUID: &[Flag] = &[
    Flag {
        name: "SQL_CVT_CHAR",
        enabled: true,
    }, // bit 0
    Flag {
        name: "SQL_CVT_NUMERIC",
        enabled: false,
    }, // bit 1
    Flag {
        name: "SQL_CVT_DECIMAL",
        enabled: false,
    }, // bit 2
    Flag {
        name: "SQL_CVT_INTEGER",
        enabled: false,
    }, // bit 3
    Flag {
        name: "SQL_CVT_SMALLINT",
        enabled: false,
    }, // bit 4
    Flag {
        name: "SQL_CVT_FLOAT",
        enabled: false,
    }, // bit 5
    Flag {
        name: "SQL_CVT_REAL",
        enabled: false,
    }, // bit 6
    Flag {
        name: "SQL_CVT_DOUBLE",
        enabled: false,
    }, // bit 7
    Flag {
        name: "SQL_CVT_VARCHAR",
        enabled: true,
    }, // bit 8
    Flag {
        name: "SQL_CVT_LONGVARCHAR",
        enabled: true,
    }, // bit 9
    Flag {
        name: "SQL_CVT_BINARY",
        enabled: true,
    }, // bit 10
    Flag {
        name: "SQL_CVT_VARBINARY",
        enabled: true,
    }, // bit 11
    Flag {
        name: "SQL_CVT_BIT",
        enabled: false,
    }, // bit 12
    Flag {
        name: "SQL_CVT_TINYINT",
        enabled: false,
    }, // bit 13
    Flag {
        name: "SQL_CVT_BIGINT",
        enabled: false,
    }, // bit 14
    Flag {
        name: "SQL_CVT_DATE",
        enabled: false,
    }, // bit 15
    Flag {
        name: "SQL_CVT_TIME",
        enabled: false,
    }, // bit 16
    Flag {
        name: "SQL_CVT_TIMESTAMP",
        enabled: false,
    }, // bit 17
    Flag {
        name: "SQL_CVT_LONGVARBINARY",
        enabled: false,
    }, // bit 18
    Flag {
        name: "SQL_CVT_INTERVAL_YEAR_MONTH",
        enabled: false,
    }, // bit 19
    Flag {
        name: "SQL_CVT_INTERVAL_DAY_TIME",
        enabled: false,
    }, // bit 20
    Flag {
        name: "SQL_CVT_WCHAR",
        enabled: false,
    }, // bit 21
    Flag {
        name: "SQL_CVT_WLONGVARCHAR",
        enabled: false,
    }, // bit 22
    Flag {
        name: "SQL_CVT_WVARCHAR",
        enabled: false,
    }, // bit 23
    Flag {
        name: "SQL_CVT_GUID",
        enabled: true,
    }, // bit 24
];

/// `SQL_CONVERT_INTEGER` (61) — conversion targets from `INTEGER` source.
pub const CONVERT_INTEGER: &[Flag] = &[
    Flag {
        name: "SQL_CVT_CHAR",
        enabled: true,
    }, // bit 0
    Flag {
        name: "SQL_CVT_NUMERIC",
        enabled: true,
    }, // bit 1
    Flag {
        name: "SQL_CVT_DECIMAL",
        enabled: true,
    }, // bit 2
    Flag {
        name: "SQL_CVT_INTEGER",
        enabled: true,
    }, // bit 3
    Flag {
        name: "SQL_CVT_SMALLINT",
        enabled: false,
    }, // bit 4
    Flag {
        name: "SQL_CVT_FLOAT",
        enabled: true,
    }, // bit 5
    Flag {
        name: "SQL_CVT_REAL",
        enabled: false,
    }, // bit 6
    Flag {
        name: "SQL_CVT_DOUBLE",
        enabled: true,
    }, // bit 7
    Flag {
        name: "SQL_CVT_VARCHAR",
        enabled: true,
    }, // bit 8
    Flag {
        name: "SQL_CVT_LONGVARCHAR",
        enabled: false,
    }, // bit 9
    Flag {
        name: "SQL_CVT_BINARY",
        enabled: false,
    }, // bit 10
    Flag {
        name: "SQL_CVT_VARBINARY",
        enabled: false,
    }, // bit 11
    Flag {
        name: "SQL_CVT_BIT",
        enabled: false,
    }, // bit 12
    Flag {
        name: "SQL_CVT_TINYINT",
        enabled: false,
    }, // bit 13
    Flag {
        name: "SQL_CVT_BIGINT",
        enabled: false,
    }, // bit 14
    Flag {
        name: "SQL_CVT_DATE",
        enabled: false,
    }, // bit 15
    Flag {
        name: "SQL_CVT_TIME",
        enabled: false,
    }, // bit 16
    Flag {
        name: "SQL_CVT_TIMESTAMP",
        enabled: false,
    }, // bit 17
    Flag {
        name: "SQL_CVT_LONGVARBINARY",
        enabled: false,
    }, // bit 18
    Flag {
        name: "SQL_CVT_INTERVAL_YEAR_MONTH",
        enabled: false,
    }, // bit 19
    Flag {
        name: "SQL_CVT_INTERVAL_DAY_TIME",
        enabled: false,
    }, // bit 20
    Flag {
        name: "SQL_CVT_WCHAR",
        enabled: true,
    }, // bit 21
    Flag {
        name: "SQL_CVT_WLONGVARCHAR",
        enabled: false,
    }, // bit 22
    Flag {
        name: "SQL_CVT_WVARCHAR",
        enabled: true,
    }, // bit 23
    Flag {
        name: "SQL_CVT_GUID",
        enabled: false,
    }, // bit 24
];

/// `SQL_CONVERT_LONGVARBINARY` (71) — conversion targets from `LONGVARBINARY`
/// source.
pub const CONVERT_LONGVARBINARY: &[Flag] = &[
    Flag {
        name: "SQL_CVT_CHAR",
        enabled: true,
    }, // bit 0
    Flag {
        name: "SQL_CVT_NUMERIC",
        enabled: false,
    }, // bit 1
    Flag {
        name: "SQL_CVT_DECIMAL",
        enabled: false,
    }, // bit 2
    Flag {
        name: "SQL_CVT_INTEGER",
        enabled: false,
    }, // bit 3
    Flag {
        name: "SQL_CVT_SMALLINT",
        enabled: false,
    }, // bit 4
    Flag {
        name: "SQL_CVT_FLOAT",
        enabled: false,
    }, // bit 5
    Flag {
        name: "SQL_CVT_REAL",
        enabled: false,
    }, // bit 6
    Flag {
        name: "SQL_CVT_DOUBLE",
        enabled: false,
    }, // bit 7
    Flag {
        name: "SQL_CVT_VARCHAR",
        enabled: true,
    }, // bit 8
    Flag {
        name: "SQL_CVT_LONGVARCHAR",
        enabled: true,
    }, // bit 9
    Flag {
        name: "SQL_CVT_BINARY",
        enabled: true,
    }, // bit 10
    Flag {
        name: "SQL_CVT_VARBINARY",
        enabled: true,
    }, // bit 11
    Flag {
        name: "SQL_CVT_BIT",
        enabled: false,
    }, // bit 12
    Flag {
        name: "SQL_CVT_TINYINT",
        enabled: false,
    }, // bit 13
    Flag {
        name: "SQL_CVT_BIGINT",
        enabled: false,
    }, // bit 14
    Flag {
        name: "SQL_CVT_DATE",
        enabled: false,
    }, // bit 15
    Flag {
        name: "SQL_CVT_TIME",
        enabled: false,
    }, // bit 16
    Flag {
        name: "SQL_CVT_TIMESTAMP",
        enabled: false,
    }, // bit 17
    Flag {
        name: "SQL_CVT_LONGVARBINARY",
        enabled: true,
    }, // bit 18
    Flag {
        name: "SQL_CVT_INTERVAL_YEAR_MONTH",
        enabled: false,
    }, // bit 19
    Flag {
        name: "SQL_CVT_INTERVAL_DAY_TIME",
        enabled: false,
    }, // bit 20
    Flag {
        name: "SQL_CVT_WCHAR",
        enabled: true,
    }, // bit 21
    Flag {
        name: "SQL_CVT_WLONGVARCHAR",
        enabled: true,
    }, // bit 22
    Flag {
        name: "SQL_CVT_WVARCHAR",
        enabled: true,
    }, // bit 23
    Flag {
        name: "SQL_CVT_GUID",
        enabled: false,
    }, // bit 24
];

/// `SQL_CONVERT_LONGVARCHAR` (62) — conversion targets from `LONGVARCHAR` source.
pub const CONVERT_LONGVARCHAR: &[Flag] = &[
    Flag {
        name: "SQL_CVT_CHAR",
        enabled: true,
    }, // bit 0
    Flag {
        name: "SQL_CVT_NUMERIC",
        enabled: true,
    }, // bit 1
    Flag {
        name: "SQL_CVT_DECIMAL",
        enabled: true,
    }, // bit 2
    Flag {
        name: "SQL_CVT_INTEGER",
        enabled: true,
    }, // bit 3
    Flag {
        name: "SQL_CVT_SMALLINT",
        enabled: true,
    }, // bit 4
    Flag {
        name: "SQL_CVT_FLOAT",
        enabled: true,
    }, // bit 5
    Flag {
        name: "SQL_CVT_REAL",
        enabled: true,
    }, // bit 6
    Flag {
        name: "SQL_CVT_DOUBLE",
        enabled: true,
    }, // bit 7
    Flag {
        name: "SQL_CVT_VARCHAR",
        enabled: true,
    }, // bit 8
    Flag {
        name: "SQL_CVT_LONGVARCHAR",
        enabled: true,
    }, // bit 9
    Flag {
        name: "SQL_CVT_BINARY",
        enabled: false,
    }, // bit 10
    Flag {
        name: "SQL_CVT_VARBINARY",
        enabled: false,
    }, // bit 11
    Flag {
        name: "SQL_CVT_BIT",
        enabled: true,
    }, // bit 12
    Flag {
        name: "SQL_CVT_TINYINT",
        enabled: true,
    }, // bit 13
    Flag {
        name: "SQL_CVT_BIGINT",
        enabled: true,
    }, // bit 14
    Flag {
        name: "SQL_CVT_DATE",
        enabled: true,
    }, // bit 15
    Flag {
        name: "SQL_CVT_TIME",
        enabled: true,
    }, // bit 16
    Flag {
        name: "SQL_CVT_TIMESTAMP",
        enabled: true,
    }, // bit 17
    Flag {
        name: "SQL_CVT_LONGVARBINARY",
        enabled: false,
    }, // bit 18
    Flag {
        name: "SQL_CVT_INTERVAL_YEAR_MONTH",
        enabled: true,
    }, // bit 19
    Flag {
        name: "SQL_CVT_INTERVAL_DAY_TIME",
        enabled: true,
    }, // bit 20
    Flag {
        name: "SQL_CVT_WCHAR",
        enabled: true,
    }, // bit 21
    Flag {
        name: "SQL_CVT_WLONGVARCHAR",
        enabled: true,
    }, // bit 22
    Flag {
        name: "SQL_CVT_WVARCHAR",
        enabled: true,
    }, // bit 23
    Flag {
        name: "SQL_CVT_GUID",
        enabled: true,
    }, // bit 24
];

/// `SQL_CONVERT_NUMERIC` (63) — conversion targets from `NUMERIC` source.
pub const CONVERT_NUMERIC: &[Flag] = &[
    Flag {
        name: "SQL_CVT_CHAR",
        enabled: true,
    }, // bit 0
    Flag {
        name: "SQL_CVT_NUMERIC",
        enabled: true,
    }, // bit 1
    Flag {
        name: "SQL_CVT_DECIMAL",
        enabled: true,
    }, // bit 2
    Flag {
        name: "SQL_CVT_INTEGER",
        enabled: true,
    }, // bit 3
    Flag {
        name: "SQL_CVT_SMALLINT",
        enabled: false,
    }, // bit 4
    Flag {
        name: "SQL_CVT_FLOAT",
        enabled: true,
    }, // bit 5
    Flag {
        name: "SQL_CVT_REAL",
        enabled: false,
    }, // bit 6
    Flag {
        name: "SQL_CVT_DOUBLE",
        enabled: true,
    }, // bit 7
    Flag {
        name: "SQL_CVT_VARCHAR",
        enabled: true,
    }, // bit 8
    Flag {
        name: "SQL_CVT_LONGVARCHAR",
        enabled: false,
    }, // bit 9
    Flag {
        name: "SQL_CVT_BINARY",
        enabled: false,
    }, // bit 10
    Flag {
        name: "SQL_CVT_VARBINARY",
        enabled: false,
    }, // bit 11
    Flag {
        name: "SQL_CVT_BIT",
        enabled: false,
    }, // bit 12
    Flag {
        name: "SQL_CVT_TINYINT",
        enabled: false,
    }, // bit 13
    Flag {
        name: "SQL_CVT_BIGINT",
        enabled: false,
    }, // bit 14
    Flag {
        name: "SQL_CVT_DATE",
        enabled: false,
    }, // bit 15
    Flag {
        name: "SQL_CVT_TIME",
        enabled: false,
    }, // bit 16
    Flag {
        name: "SQL_CVT_TIMESTAMP",
        enabled: false,
    }, // bit 17
    Flag {
        name: "SQL_CVT_LONGVARBINARY",
        enabled: false,
    }, // bit 18
    Flag {
        name: "SQL_CVT_INTERVAL_YEAR_MONTH",
        enabled: false,
    }, // bit 19
    Flag {
        name: "SQL_CVT_INTERVAL_DAY_TIME",
        enabled: false,
    }, // bit 20
    Flag {
        name: "SQL_CVT_WCHAR",
        enabled: true,
    }, // bit 21
    Flag {
        name: "SQL_CVT_WLONGVARCHAR",
        enabled: false,
    }, // bit 22
    Flag {
        name: "SQL_CVT_WVARCHAR",
        enabled: true,
    }, // bit 23
    Flag {
        name: "SQL_CVT_GUID",
        enabled: false,
    }, // bit 24
];

/// `SQL_CONVERT_REAL` (64) — conversion targets from `REAL` source.
pub const CONVERT_REAL: &[Flag] = &[
    Flag {
        name: "SQL_CVT_CHAR",
        enabled: true,
    }, // bit 0
    Flag {
        name: "SQL_CVT_NUMERIC",
        enabled: true,
    }, // bit 1
    Flag {
        name: "SQL_CVT_DECIMAL",
        enabled: true,
    }, // bit 2
    Flag {
        name: "SQL_CVT_INTEGER",
        enabled: true,
    }, // bit 3
    Flag {
        name: "SQL_CVT_SMALLINT",
        enabled: true,
    }, // bit 4
    Flag {
        name: "SQL_CVT_FLOAT",
        enabled: true,
    }, // bit 5
    Flag {
        name: "SQL_CVT_REAL",
        enabled: true,
    }, // bit 6
    Flag {
        name: "SQL_CVT_DOUBLE",
        enabled: true,
    }, // bit 7
    Flag {
        name: "SQL_CVT_VARCHAR",
        enabled: true,
    }, // bit 8
    Flag {
        name: "SQL_CVT_LONGVARCHAR",
        enabled: true,
    }, // bit 9
    Flag {
        name: "SQL_CVT_BINARY",
        enabled: true,
    }, // bit 10
    Flag {
        name: "SQL_CVT_VARBINARY",
        enabled: true,
    }, // bit 11
    Flag {
        name: "SQL_CVT_BIT",
        enabled: true,
    }, // bit 12
    Flag {
        name: "SQL_CVT_TINYINT",
        enabled: true,
    }, // bit 13
    Flag {
        name: "SQL_CVT_BIGINT",
        enabled: true,
    }, // bit 14
    Flag {
        name: "SQL_CVT_DATE",
        enabled: false,
    }, // bit 15
    Flag {
        name: "SQL_CVT_TIME",
        enabled: false,
    }, // bit 16
    Flag {
        name: "SQL_CVT_TIMESTAMP",
        enabled: false,
    }, // bit 17
    Flag {
        name: "SQL_CVT_LONGVARBINARY",
        enabled: false,
    }, // bit 18
    Flag {
        name: "SQL_CVT_INTERVAL_YEAR_MONTH",
        enabled: true,
    }, // bit 19
    Flag {
        name: "SQL_CVT_INTERVAL_DAY_TIME",
        enabled: true,
    }, // bit 20
    Flag {
        name: "SQL_CVT_WCHAR",
        enabled: true,
    }, // bit 21
    Flag {
        name: "SQL_CVT_WLONGVARCHAR",
        enabled: true,
    }, // bit 22
    Flag {
        name: "SQL_CVT_WVARCHAR",
        enabled: true,
    }, // bit 23
    Flag {
        name: "SQL_CVT_GUID",
        enabled: false,
    }, // bit 24
];

/// `SQL_CONVERT_SMALLINT` (65) — conversion targets from `SMALLINT` source.
pub const CONVERT_SMALLINT: &[Flag] = &[
    Flag {
        name: "SQL_CVT_CHAR",
        enabled: true,
    }, // bit 0
    Flag {
        name: "SQL_CVT_NUMERIC",
        enabled: true,
    }, // bit 1
    Flag {
        name: "SQL_CVT_DECIMAL",
        enabled: true,
    }, // bit 2
    Flag {
        name: "SQL_CVT_INTEGER",
        enabled: true,
    }, // bit 3
    Flag {
        name: "SQL_CVT_SMALLINT",
        enabled: true,
    }, // bit 4
    Flag {
        name: "SQL_CVT_FLOAT",
        enabled: true,
    }, // bit 5
    Flag {
        name: "SQL_CVT_REAL",
        enabled: true,
    }, // bit 6
    Flag {
        name: "SQL_CVT_DOUBLE",
        enabled: true,
    }, // bit 7
    Flag {
        name: "SQL_CVT_VARCHAR",
        enabled: true,
    }, // bit 8
    Flag {
        name: "SQL_CVT_LONGVARCHAR",
        enabled: true,
    }, // bit 9
    Flag {
        name: "SQL_CVT_BINARY",
        enabled: true,
    }, // bit 10
    Flag {
        name: "SQL_CVT_VARBINARY",
        enabled: true,
    }, // bit 11
    Flag {
        name: "SQL_CVT_BIT",
        enabled: true,
    }, // bit 12
    Flag {
        name: "SQL_CVT_TINYINT",
        enabled: true,
    }, // bit 13
    Flag {
        name: "SQL_CVT_BIGINT",
        enabled: true,
    }, // bit 14
    Flag {
        name: "SQL_CVT_DATE",
        enabled: false,
    }, // bit 15
    Flag {
        name: "SQL_CVT_TIME",
        enabled: false,
    }, // bit 16
    Flag {
        name: "SQL_CVT_TIMESTAMP",
        enabled: false,
    }, // bit 17
    Flag {
        name: "SQL_CVT_LONGVARBINARY",
        enabled: false,
    }, // bit 18
    Flag {
        name: "SQL_CVT_INTERVAL_YEAR_MONTH",
        enabled: true,
    }, // bit 19
    Flag {
        name: "SQL_CVT_INTERVAL_DAY_TIME",
        enabled: true,
    }, // bit 20
    Flag {
        name: "SQL_CVT_WCHAR",
        enabled: true,
    }, // bit 21
    Flag {
        name: "SQL_CVT_WLONGVARCHAR",
        enabled: true,
    }, // bit 22
    Flag {
        name: "SQL_CVT_WVARCHAR",
        enabled: true,
    }, // bit 23
    Flag {
        name: "SQL_CVT_GUID",
        enabled: false,
    }, // bit 24
];

/// `SQL_CONVERT_TIME` (66) — conversion targets from `TIME` source.
pub const CONVERT_TIME: &[Flag] = &[
    Flag {
        name: "SQL_CVT_CHAR",
        enabled: true,
    }, // bit 0
    Flag {
        name: "SQL_CVT_NUMERIC",
        enabled: false,
    }, // bit 1
    Flag {
        name: "SQL_CVT_DECIMAL",
        enabled: false,
    }, // bit 2
    Flag {
        name: "SQL_CVT_INTEGER",
        enabled: false,
    }, // bit 3
    Flag {
        name: "SQL_CVT_SMALLINT",
        enabled: false,
    }, // bit 4
    Flag {
        name: "SQL_CVT_FLOAT",
        enabled: false,
    }, // bit 5
    Flag {
        name: "SQL_CVT_REAL",
        enabled: false,
    }, // bit 6
    Flag {
        name: "SQL_CVT_DOUBLE",
        enabled: false,
    }, // bit 7
    Flag {
        name: "SQL_CVT_VARCHAR",
        enabled: true,
    }, // bit 8
    Flag {
        name: "SQL_CVT_LONGVARCHAR",
        enabled: false,
    }, // bit 9
    Flag {
        name: "SQL_CVT_BINARY",
        enabled: false,
    }, // bit 10
    Flag {
        name: "SQL_CVT_VARBINARY",
        enabled: false,
    }, // bit 11
    Flag {
        name: "SQL_CVT_BIT",
        enabled: false,
    }, // bit 12
    Flag {
        name: "SQL_CVT_TINYINT",
        enabled: false,
    }, // bit 13
    Flag {
        name: "SQL_CVT_BIGINT",
        enabled: false,
    }, // bit 14
    Flag {
        name: "SQL_CVT_DATE",
        enabled: false,
    }, // bit 15
    Flag {
        name: "SQL_CVT_TIME",
        enabled: true,
    }, // bit 16
    Flag {
        name: "SQL_CVT_TIMESTAMP",
        enabled: false,
    }, // bit 17
    Flag {
        name: "SQL_CVT_LONGVARBINARY",
        enabled: false,
    }, // bit 18
    Flag {
        name: "SQL_CVT_INTERVAL_YEAR_MONTH",
        enabled: false,
    }, // bit 19
    Flag {
        name: "SQL_CVT_INTERVAL_DAY_TIME",
        enabled: false,
    }, // bit 20
    Flag {
        name: "SQL_CVT_WCHAR",
        enabled: true,
    }, // bit 21
    Flag {
        name: "SQL_CVT_WLONGVARCHAR",
        enabled: false,
    }, // bit 22
    Flag {
        name: "SQL_CVT_WVARCHAR",
        enabled: true,
    }, // bit 23
    Flag {
        name: "SQL_CVT_GUID",
        enabled: false,
    }, // bit 24
];

/// `SQL_CONVERT_TIMESTAMP` (67) — conversion targets from `TIMESTAMP` source.
pub const CONVERT_TIMESTAMP: &[Flag] = &[
    Flag {
        name: "SQL_CVT_CHAR",
        enabled: true,
    }, // bit 0
    Flag {
        name: "SQL_CVT_NUMERIC",
        enabled: false,
    }, // bit 1
    Flag {
        name: "SQL_CVT_DECIMAL",
        enabled: false,
    }, // bit 2
    Flag {
        name: "SQL_CVT_INTEGER",
        enabled: false,
    }, // bit 3
    Flag {
        name: "SQL_CVT_SMALLINT",
        enabled: false,
    }, // bit 4
    Flag {
        name: "SQL_CVT_FLOAT",
        enabled: false,
    }, // bit 5
    Flag {
        name: "SQL_CVT_REAL",
        enabled: false,
    }, // bit 6
    Flag {
        name: "SQL_CVT_DOUBLE",
        enabled: false,
    }, // bit 7
    Flag {
        name: "SQL_CVT_VARCHAR",
        enabled: true,
    }, // bit 8
    Flag {
        name: "SQL_CVT_LONGVARCHAR",
        enabled: false,
    }, // bit 9
    Flag {
        name: "SQL_CVT_BINARY",
        enabled: false,
    }, // bit 10
    Flag {
        name: "SQL_CVT_VARBINARY",
        enabled: false,
    }, // bit 11
    Flag {
        name: "SQL_CVT_BIT",
        enabled: false,
    }, // bit 12
    Flag {
        name: "SQL_CVT_TINYINT",
        enabled: false,
    }, // bit 13
    Flag {
        name: "SQL_CVT_BIGINT",
        enabled: false,
    }, // bit 14
    Flag {
        name: "SQL_CVT_DATE",
        enabled: true,
    }, // bit 15
    Flag {
        name: "SQL_CVT_TIME",
        enabled: true,
    }, // bit 16
    Flag {
        name: "SQL_CVT_TIMESTAMP",
        enabled: true,
    }, // bit 17
    Flag {
        name: "SQL_CVT_LONGVARBINARY",
        enabled: false,
    }, // bit 18
    Flag {
        name: "SQL_CVT_INTERVAL_YEAR_MONTH",
        enabled: false,
    }, // bit 19
    Flag {
        name: "SQL_CVT_INTERVAL_DAY_TIME",
        enabled: false,
    }, // bit 20
    Flag {
        name: "SQL_CVT_WCHAR",
        enabled: true,
    }, // bit 21
    Flag {
        name: "SQL_CVT_WLONGVARCHAR",
        enabled: false,
    }, // bit 22
    Flag {
        name: "SQL_CVT_WVARCHAR",
        enabled: true,
    }, // bit 23
    Flag {
        name: "SQL_CVT_GUID",
        enabled: false,
    }, // bit 24
];

/// `SQL_CONVERT_TINYINT` (68) — conversion targets from `TINYINT` source.
pub const CONVERT_TINYINT: &[Flag] = &[
    Flag {
        name: "SQL_CVT_CHAR",
        enabled: true,
    }, // bit 0
    Flag {
        name: "SQL_CVT_NUMERIC",
        enabled: true,
    }, // bit 1
    Flag {
        name: "SQL_CVT_DECIMAL",
        enabled: true,
    }, // bit 2
    Flag {
        name: "SQL_CVT_INTEGER",
        enabled: true,
    }, // bit 3
    Flag {
        name: "SQL_CVT_SMALLINT",
        enabled: true,
    }, // bit 4
    Flag {
        name: "SQL_CVT_FLOAT",
        enabled: true,
    }, // bit 5
    Flag {
        name: "SQL_CVT_REAL",
        enabled: true,
    }, // bit 6
    Flag {
        name: "SQL_CVT_DOUBLE",
        enabled: true,
    }, // bit 7
    Flag {
        name: "SQL_CVT_VARCHAR",
        enabled: true,
    }, // bit 8
    Flag {
        name: "SQL_CVT_LONGVARCHAR",
        enabled: true,
    }, // bit 9
    Flag {
        name: "SQL_CVT_BINARY",
        enabled: true,
    }, // bit 10
    Flag {
        name: "SQL_CVT_VARBINARY",
        enabled: true,
    }, // bit 11
    Flag {
        name: "SQL_CVT_BIT",
        enabled: true,
    }, // bit 12
    Flag {
        name: "SQL_CVT_TINYINT",
        enabled: true,
    }, // bit 13
    Flag {
        name: "SQL_CVT_BIGINT",
        enabled: true,
    }, // bit 14
    Flag {
        name: "SQL_CVT_DATE",
        enabled: false,
    }, // bit 15
    Flag {
        name: "SQL_CVT_TIME",
        enabled: false,
    }, // bit 16
    Flag {
        name: "SQL_CVT_TIMESTAMP",
        enabled: false,
    }, // bit 17
    Flag {
        name: "SQL_CVT_LONGVARBINARY",
        enabled: false,
    }, // bit 18
    Flag {
        name: "SQL_CVT_INTERVAL_YEAR_MONTH",
        enabled: true,
    }, // bit 19
    Flag {
        name: "SQL_CVT_INTERVAL_DAY_TIME",
        enabled: true,
    }, // bit 20
    Flag {
        name: "SQL_CVT_WCHAR",
        enabled: true,
    }, // bit 21
    Flag {
        name: "SQL_CVT_WLONGVARCHAR",
        enabled: true,
    }, // bit 22
    Flag {
        name: "SQL_CVT_WVARCHAR",
        enabled: true,
    }, // bit 23
    Flag {
        name: "SQL_CVT_GUID",
        enabled: false,
    }, // bit 24
];

/// `SQL_CONVERT_VARBINARY` (69) — conversion targets from `VARBINARY` source.
pub const CONVERT_VARBINARY: &[Flag] = &[
    Flag {
        name: "SQL_CVT_CHAR",
        enabled: true,
    }, // bit 0
    Flag {
        name: "SQL_CVT_NUMERIC",
        enabled: false,
    }, // bit 1
    Flag {
        name: "SQL_CVT_DECIMAL",
        enabled: false,
    }, // bit 2
    Flag {
        name: "SQL_CVT_INTEGER",
        enabled: false,
    }, // bit 3
    Flag {
        name: "SQL_CVT_SMALLINT",
        enabled: false,
    }, // bit 4
    Flag {
        name: "SQL_CVT_FLOAT",
        enabled: false,
    }, // bit 5
    Flag {
        name: "SQL_CVT_REAL",
        enabled: false,
    }, // bit 6
    Flag {
        name: "SQL_CVT_DOUBLE",
        enabled: false,
    }, // bit 7
    Flag {
        name: "SQL_CVT_VARCHAR",
        enabled: true,
    }, // bit 8
    Flag {
        name: "SQL_CVT_LONGVARCHAR",
        enabled: false,
    }, // bit 9
    Flag {
        name: "SQL_CVT_BINARY",
        enabled: true,
    }, // bit 10
    Flag {
        name: "SQL_CVT_VARBINARY",
        enabled: true,
    }, // bit 11
    Flag {
        name: "SQL_CVT_BIT",
        enabled: false,
    }, // bit 12
    Flag {
        name: "SQL_CVT_TINYINT",
        enabled: false,
    }, // bit 13
    Flag {
        name: "SQL_CVT_BIGINT",
        enabled: false,
    }, // bit 14
    Flag {
        name: "SQL_CVT_DATE",
        enabled: false,
    }, // bit 15
    Flag {
        name: "SQL_CVT_TIME",
        enabled: false,
    }, // bit 16
    Flag {
        name: "SQL_CVT_TIMESTAMP",
        enabled: false,
    }, // bit 17
    Flag {
        name: "SQL_CVT_LONGVARBINARY",
        enabled: false,
    }, // bit 18
    Flag {
        name: "SQL_CVT_INTERVAL_YEAR_MONTH",
        enabled: false,
    }, // bit 19
    Flag {
        name: "SQL_CVT_INTERVAL_DAY_TIME",
        enabled: false,
    }, // bit 20
    Flag {
        name: "SQL_CVT_WCHAR",
        enabled: true,
    }, // bit 21
    Flag {
        name: "SQL_CVT_WLONGVARCHAR",
        enabled: false,
    }, // bit 22
    Flag {
        name: "SQL_CVT_WVARCHAR",
        enabled: true,
    }, // bit 23
    Flag {
        name: "SQL_CVT_GUID",
        enabled: false,
    }, // bit 24
];

/// `SQL_CONVERT_VARCHAR` (70) — conversion targets from `VARCHAR` source.
pub const CONVERT_VARCHAR: &[Flag] = &[
    Flag {
        name: "SQL_CVT_CHAR",
        enabled: true,
    }, // bit 0
    Flag {
        name: "SQL_CVT_NUMERIC",
        enabled: true,
    }, // bit 1
    Flag {
        name: "SQL_CVT_DECIMAL",
        enabled: true,
    }, // bit 2
    Flag {
        name: "SQL_CVT_INTEGER",
        enabled: true,
    }, // bit 3
    Flag {
        name: "SQL_CVT_SMALLINT",
        enabled: false,
    }, // bit 4
    Flag {
        name: "SQL_CVT_FLOAT",
        enabled: true,
    }, // bit 5
    Flag {
        name: "SQL_CVT_REAL",
        enabled: false,
    }, // bit 6
    Flag {
        name: "SQL_CVT_DOUBLE",
        enabled: true,
    }, // bit 7
    Flag {
        name: "SQL_CVT_VARCHAR",
        enabled: true,
    }, // bit 8
    Flag {
        name: "SQL_CVT_LONGVARCHAR",
        enabled: false,
    }, // bit 9
    Flag {
        name: "SQL_CVT_BINARY",
        enabled: true,
    }, // bit 10
    Flag {
        name: "SQL_CVT_VARBINARY",
        enabled: true,
    }, // bit 11
    Flag {
        name: "SQL_CVT_BIT",
        enabled: false,
    }, // bit 12
    Flag {
        name: "SQL_CVT_TINYINT",
        enabled: false,
    }, // bit 13
    Flag {
        name: "SQL_CVT_BIGINT",
        enabled: false,
    }, // bit 14
    Flag {
        name: "SQL_CVT_DATE",
        enabled: true,
    }, // bit 15
    Flag {
        name: "SQL_CVT_TIME",
        enabled: true,
    }, // bit 16
    Flag {
        name: "SQL_CVT_TIMESTAMP",
        enabled: true,
    }, // bit 17
    Flag {
        name: "SQL_CVT_LONGVARBINARY",
        enabled: false,
    }, // bit 18
    Flag {
        name: "SQL_CVT_INTERVAL_YEAR_MONTH",
        enabled: false,
    }, // bit 19
    Flag {
        name: "SQL_CVT_INTERVAL_DAY_TIME",
        enabled: false,
    }, // bit 20
    Flag {
        name: "SQL_CVT_WCHAR",
        enabled: true,
    }, // bit 21
    Flag {
        name: "SQL_CVT_WLONGVARCHAR",
        enabled: false,
    }, // bit 22
    Flag {
        name: "SQL_CVT_WVARCHAR",
        enabled: true,
    }, // bit 23
    Flag {
        name: "SQL_CVT_GUID",
        enabled: false,
    }, // bit 24
];

/// `SQL_CONVERT_WCHAR` (122) — conversion targets from `WCHAR` source.
pub const CONVERT_WCHAR: &[Flag] = &[
    Flag {
        name: "SQL_CVT_CHAR",
        enabled: true,
    }, // bit 0
    Flag {
        name: "SQL_CVT_NUMERIC",
        enabled: true,
    }, // bit 1
    Flag {
        name: "SQL_CVT_DECIMAL",
        enabled: true,
    }, // bit 2
    Flag {
        name: "SQL_CVT_INTEGER",
        enabled: true,
    }, // bit 3
    Flag {
        name: "SQL_CVT_SMALLINT",
        enabled: true,
    }, // bit 4
    Flag {
        name: "SQL_CVT_FLOAT",
        enabled: true,
    }, // bit 5
    Flag {
        name: "SQL_CVT_REAL",
        enabled: true,
    }, // bit 6
    Flag {
        name: "SQL_CVT_DOUBLE",
        enabled: true,
    }, // bit 7
    Flag {
        name: "SQL_CVT_VARCHAR",
        enabled: true,
    }, // bit 8
    Flag {
        name: "SQL_CVT_LONGVARCHAR",
        enabled: true,
    }, // bit 9
    Flag {
        name: "SQL_CVT_BINARY",
        enabled: true,
    }, // bit 10
    Flag {
        name: "SQL_CVT_VARBINARY",
        enabled: true,
    }, // bit 11
    Flag {
        name: "SQL_CVT_BIT",
        enabled: true,
    }, // bit 12
    Flag {
        name: "SQL_CVT_TINYINT",
        enabled: true,
    }, // bit 13
    Flag {
        name: "SQL_CVT_BIGINT",
        enabled: true,
    }, // bit 14
    Flag {
        name: "SQL_CVT_DATE",
        enabled: true,
    }, // bit 15
    Flag {
        name: "SQL_CVT_TIME",
        enabled: true,
    }, // bit 16
    Flag {
        name: "SQL_CVT_TIMESTAMP",
        enabled: true,
    }, // bit 17
    Flag {
        name: "SQL_CVT_LONGVARBINARY",
        enabled: true,
    }, // bit 18
    Flag {
        name: "SQL_CVT_INTERVAL_YEAR_MONTH",
        enabled: true,
    }, // bit 19
    Flag {
        name: "SQL_CVT_INTERVAL_DAY_TIME",
        enabled: true,
    }, // bit 20
    Flag {
        name: "SQL_CVT_WCHAR",
        enabled: true,
    }, // bit 21
    Flag {
        name: "SQL_CVT_WLONGVARCHAR",
        enabled: true,
    }, // bit 22
    Flag {
        name: "SQL_CVT_WVARCHAR",
        enabled: true,
    }, // bit 23
    Flag {
        name: "SQL_CVT_GUID",
        enabled: true,
    }, // bit 24
];

/// `SQL_CONVERT_WLONGVARCHAR` (125) — conversion targets from `WLONGVARCHAR`
/// source.
pub const CONVERT_WLONGVARCHAR: &[Flag] = &[
    Flag {
        name: "SQL_CVT_CHAR",
        enabled: true,
    }, // bit 0
    Flag {
        name: "SQL_CVT_NUMERIC",
        enabled: true,
    }, // bit 1
    Flag {
        name: "SQL_CVT_DECIMAL",
        enabled: true,
    }, // bit 2
    Flag {
        name: "SQL_CVT_INTEGER",
        enabled: true,
    }, // bit 3
    Flag {
        name: "SQL_CVT_SMALLINT",
        enabled: true,
    }, // bit 4
    Flag {
        name: "SQL_CVT_FLOAT",
        enabled: true,
    }, // bit 5
    Flag {
        name: "SQL_CVT_REAL",
        enabled: true,
    }, // bit 6
    Flag {
        name: "SQL_CVT_DOUBLE",
        enabled: true,
    }, // bit 7
    Flag {
        name: "SQL_CVT_VARCHAR",
        enabled: true,
    }, // bit 8
    Flag {
        name: "SQL_CVT_LONGVARCHAR",
        enabled: true,
    }, // bit 9
    Flag {
        name: "SQL_CVT_BINARY",
        enabled: false,
    }, // bit 10
    Flag {
        name: "SQL_CVT_VARBINARY",
        enabled: false,
    }, // bit 11
    Flag {
        name: "SQL_CVT_BIT",
        enabled: true,
    }, // bit 12
    Flag {
        name: "SQL_CVT_TINYINT",
        enabled: true,
    }, // bit 13
    Flag {
        name: "SQL_CVT_BIGINT",
        enabled: true,
    }, // bit 14
    Flag {
        name: "SQL_CVT_DATE",
        enabled: true,
    }, // bit 15
    Flag {
        name: "SQL_CVT_TIME",
        enabled: true,
    }, // bit 16
    Flag {
        name: "SQL_CVT_TIMESTAMP",
        enabled: true,
    }, // bit 17
    Flag {
        name: "SQL_CVT_LONGVARBINARY",
        enabled: false,
    }, // bit 18
    Flag {
        name: "SQL_CVT_INTERVAL_YEAR_MONTH",
        enabled: true,
    }, // bit 19
    Flag {
        name: "SQL_CVT_INTERVAL_DAY_TIME",
        enabled: true,
    }, // bit 20
    Flag {
        name: "SQL_CVT_WCHAR",
        enabled: true,
    }, // bit 21
    Flag {
        name: "SQL_CVT_WLONGVARCHAR",
        enabled: true,
    }, // bit 22
    Flag {
        name: "SQL_CVT_WVARCHAR",
        enabled: true,
    }, // bit 23
    Flag {
        name: "SQL_CVT_GUID",
        enabled: true,
    }, // bit 24
];

/// `SQL_CONVERT_WVARCHAR` (126) — conversion targets from `WVARCHAR` source.
pub const CONVERT_WVARCHAR: &[Flag] = &[
    Flag {
        name: "SQL_CVT_CHAR",
        enabled: true,
    }, // bit 0
    Flag {
        name: "SQL_CVT_NUMERIC",
        enabled: true,
    }, // bit 1
    Flag {
        name: "SQL_CVT_DECIMAL",
        enabled: true,
    }, // bit 2
    Flag {
        name: "SQL_CVT_INTEGER",
        enabled: true,
    }, // bit 3
    Flag {
        name: "SQL_CVT_SMALLINT",
        enabled: true,
    }, // bit 4
    Flag {
        name: "SQL_CVT_FLOAT",
        enabled: true,
    }, // bit 5
    Flag {
        name: "SQL_CVT_REAL",
        enabled: true,
    }, // bit 6
    Flag {
        name: "SQL_CVT_DOUBLE",
        enabled: true,
    }, // bit 7
    Flag {
        name: "SQL_CVT_VARCHAR",
        enabled: true,
    }, // bit 8
    Flag {
        name: "SQL_CVT_LONGVARCHAR",
        enabled: true,
    }, // bit 9
    Flag {
        name: "SQL_CVT_BINARY",
        enabled: true,
    }, // bit 10
    Flag {
        name: "SQL_CVT_VARBINARY",
        enabled: true,
    }, // bit 11
    Flag {
        name: "SQL_CVT_BIT",
        enabled: true,
    }, // bit 12
    Flag {
        name: "SQL_CVT_TINYINT",
        enabled: true,
    }, // bit 13
    Flag {
        name: "SQL_CVT_BIGINT",
        enabled: true,
    }, // bit 14
    Flag {
        name: "SQL_CVT_DATE",
        enabled: true,
    }, // bit 15
    Flag {
        name: "SQL_CVT_TIME",
        enabled: true,
    }, // bit 16
    Flag {
        name: "SQL_CVT_TIMESTAMP",
        enabled: true,
    }, // bit 17
    Flag {
        name: "SQL_CVT_LONGVARBINARY",
        enabled: true,
    }, // bit 18
    Flag {
        name: "SQL_CVT_INTERVAL_YEAR_MONTH",
        enabled: true,
    }, // bit 19
    Flag {
        name: "SQL_CVT_INTERVAL_DAY_TIME",
        enabled: true,
    }, // bit 20
    Flag {
        name: "SQL_CVT_WCHAR",
        enabled: true,
    }, // bit 21
    Flag {
        name: "SQL_CVT_WLONGVARCHAR",
        enabled: true,
    }, // bit 22
    Flag {
        name: "SQL_CVT_WVARCHAR",
        enabled: true,
    }, // bit 23
    Flag {
        name: "SQL_CVT_GUID",
        enabled: true,
    }, // bit 24
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn synthesize_empty_slice_is_zero() {
        assert_eq!(synthesize(&[]), 0);
    }

    #[test]
    fn synthesize_uses_positional_bit_ordering() {
        let flags = &[
            Flag {
                name: "bit_0",
                enabled: true,
            },
            Flag {
                name: "bit_1",
                enabled: false,
            },
            Flag {
                name: "bit_2",
                enabled: true,
            },
        ];
        assert_eq!(synthesize(flags), 0b101);
    }

    /// Lock the bitmask values against the strict-test expectations in
    /// `odbc_tests/tests/odbc-api/driver_info/get_info_tests.cpp`. The strict
    /// tests pin the reference driver's runtime output, so any drift here
    /// indicates either a flag-table edit or a reference-behavior change.
    #[test]
    fn family_values_match_reference_driver() {
        // ---- Non-CONVERT families -------------------------------------------
        assert_eq!(synthesize(AGGREGATE_FUNCTIONS), 0x7F);
        assert_eq!(synthesize(CATALOG_USAGE), 0x15);
        assert_eq!(synthesize(SCHEMA_USAGE), 0x15);
        assert_eq!(synthesize(CONVERT_FUNCTIONS), 0x3);
        assert_eq!(synthesize(NUMERIC_FUNCTIONS), 0xF7_FFFF);
        assert_eq!(synthesize(STRING_FUNCTIONS), 0xFD_7FFF);
        assert_eq!(synthesize(SYSTEM_FUNCTIONS), 0x7);
        assert_eq!(synthesize(TIMEDATE_FUNCTIONS), 0x1F_FFFF);
        assert_eq!(synthesize(TIMEDATE_TSI_INTERVALS), 0x1FF);
        assert_eq!(synthesize(SQL92_PREDICATES), 0x3F05);
        assert_eq!(synthesize(SQL92_RELATIONAL_JOIN_OPERATORS), 0x15A);
        assert_eq!(synthesize(SQL92_VALUE_EXPRESSIONS), 0xF);

        // ---- SQL_CONVERT_<source> families ----------------------------------
        // Each must include the source type itself as a target plus the
        // explicit allow-list in the corresponding strict test. A small
        // helper keeps the assertion concise: every slice is 25 bits wide
        // (`SQL_CVT_CHAR`..`SQL_CVT_GUID`) and contains at least one true.
        for (name, slice) in [
            ("CONVERT_BIGINT", CONVERT_BIGINT),
            ("CONVERT_BINARY", CONVERT_BINARY),
            ("CONVERT_BIT", CONVERT_BIT),
            ("CONVERT_CHAR", CONVERT_CHAR),
            ("CONVERT_DATE", CONVERT_DATE),
            ("CONVERT_DECIMAL", CONVERT_DECIMAL),
            ("CONVERT_DOUBLE", CONVERT_DOUBLE),
            ("CONVERT_FLOAT", CONVERT_FLOAT),
            ("CONVERT_GUID", CONVERT_GUID),
            ("CONVERT_INTEGER", CONVERT_INTEGER),
            ("CONVERT_LONGVARBINARY", CONVERT_LONGVARBINARY),
            ("CONVERT_LONGVARCHAR", CONVERT_LONGVARCHAR),
            ("CONVERT_NUMERIC", CONVERT_NUMERIC),
            ("CONVERT_REAL", CONVERT_REAL),
            ("CONVERT_SMALLINT", CONVERT_SMALLINT),
            ("CONVERT_TIME", CONVERT_TIME),
            ("CONVERT_TIMESTAMP", CONVERT_TIMESTAMP),
            ("CONVERT_TINYINT", CONVERT_TINYINT),
            ("CONVERT_VARBINARY", CONVERT_VARBINARY),
            ("CONVERT_VARCHAR", CONVERT_VARCHAR),
            ("CONVERT_WCHAR", CONVERT_WCHAR),
            ("CONVERT_WLONGVARCHAR", CONVERT_WLONGVARCHAR),
            ("CONVERT_WVARCHAR", CONVERT_WVARCHAR),
        ] {
            assert_eq!(slice.len(), 25, "{name} slice should cover bits 0..=24");
            assert!(
                synthesize(slice) != 0,
                "{name}: every CONVERT family must advertise at least one target",
            );
        }
    }
}
