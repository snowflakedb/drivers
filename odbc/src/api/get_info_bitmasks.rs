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

/// `SQL_SCROLL_CONCURRENCY` (43) — supported scroll concurrency options.
pub const SCROLL_CONCURRENCY: &[Flag] = &[
    Flag {
        name: "SQL_SCCO_READ_ONLY",
        enabled: true,
    }, // bit 0
    Flag {
        name: "SQL_SCCO_LOCK",
        enabled: false,
    }, // bit 1
    Flag {
        name: "SQL_SCCO_OPT_ROWVER",
        enabled: false,
    }, // bit 2
    Flag {
        name: "SQL_SCCO_OPT_VALUES",
        enabled: false,
    }, // bit 3
];

/// `SQL_SCROLL_OPTIONS` (44) — supported scroll options.
pub const SCROLL_OPTIONS: &[Flag] = &[
    Flag {
        name: "SQL_SO_FORWARD_ONLY",
        enabled: true,
    }, // bit 0
    Flag {
        name: "SQL_SO_KEYSET_DRIVEN",
        enabled: false,
    }, // bit 1
    Flag {
        name: "SQL_SO_DYNAMIC",
        enabled: false,
    }, // bit 2
    Flag {
        name: "SQL_SO_MIXED",
        enabled: false,
    }, // bit 3
    Flag {
        name: "SQL_SO_STATIC",
        enabled: false,
    }, // bit 4
];

/// `SQL_TXN_ISOLATION_OPTION` (72) — supported transaction isolation levels.
pub const TXN_ISOLATION_OPTION: &[Flag] = &[
    Flag {
        name: "SQL_TXN_READ_UNCOMMITTED",
        enabled: false,
    }, // bit 0
    Flag {
        name: "SQL_TXN_READ_COMMITTED",
        enabled: true,
    }, // bit 1
    Flag {
        name: "SQL_TXN_REPEATABLE_READ",
        enabled: false,
    }, // bit 2
    Flag {
        name: "SQL_TXN_SERIALIZABLE",
        enabled: false,
    }, // bit 3
];

/// `SQL_LOCK_TYPES` (78) — supported lock types.
pub const LOCK_TYPES: &[Flag] = &[
    Flag {
        name: "SQL_LCK_NO_CHANGE",
        enabled: false,
    }, // bit 0
    Flag {
        name: "SQL_LCK_EXCLUSIVE",
        enabled: true,
    }, // bit 1
    Flag {
        name: "SQL_LCK_UNLOCK",
        enabled: false,
    }, // bit 2
];

/// `SQL_POS_OPERATIONS` (79) — supported positioned operations.
pub const POS_OPERATIONS: &[Flag] = &[
    Flag {
        name: "SQL_POS_POSITION",
        enabled: false,
    }, // bit 0
    Flag {
        name: "SQL_POS_REFRESH",
        enabled: false,
    }, // bit 1
    Flag {
        name: "SQL_POS_UPDATE",
        enabled: false,
    }, // bit 2
    Flag {
        name: "SQL_POS_DELETE",
        enabled: false,
    }, // bit 3
    Flag {
        name: "SQL_POS_ADD",
        enabled: false,
    }, // bit 4
];

/// `SQL_BOOKMARK_PERSISTENCE` (82) — bookmark persistence options.
pub const BOOKMARK_PERSISTENCE: &[Flag] = &[
    Flag {
        name: "SQL_BP_CLOSE",
        enabled: false,
    }, // bit 0
    Flag {
        name: "SQL_BP_DELETE",
        enabled: false,
    }, // bit 1
    Flag {
        name: "SQL_BP_DROP",
        enabled: false,
    }, // bit 2
    Flag {
        name: "SQL_BP_TRANSACTION",
        enabled: false,
    }, // bit 3
    Flag {
        name: "SQL_BP_UPDATE",
        enabled: false,
    }, // bit 4
    Flag {
        name: "SQL_BP_OTHER_HSTMT",
        enabled: false,
    }, // bit 5
    Flag {
        name: "SQL_BP_SCROLL",
        enabled: false,
    }, // bit 6
];

/// `SQL_STATIC_SENSITIVITY` (83) — static cursor sensitivity options.
pub const STATIC_SENSITIVITY: &[Flag] = &[
    Flag {
        name: "SQL_SS_ADDITIONS",
        enabled: true,
    }, // bit 0
    Flag {
        name: "SQL_SS_DELETIONS",
        enabled: true,
    }, // bit 1
    Flag {
        name: "SQL_SS_UPDATES",
        enabled: false,
    }, // bit 2
];

/// `SQL_CA1_*` layout shared by the cursor-attributes1 info types. Bits 4–5 are
/// reserved in the ODBC spec; placeholder entries keep index aligned to bit number.
macro_rules! ca1_cursor_attributes {
    ($next:expr) => {
        &[
            Flag {
                name: "SQL_CA1_NEXT",
                enabled: $next,
            }, // bit 0
            Flag {
                name: "SQL_CA1_ABSOLUTE",
                enabled: false,
            }, // bit 1
            Flag {
                name: "SQL_CA1_RELATIVE",
                enabled: false,
            }, // bit 2
            Flag {
                name: "SQL_CA1_BOOKMARK",
                enabled: false,
            }, // bit 3
            Flag {
                name: "reserved",
                enabled: false,
            }, // bit 4
            Flag {
                name: "reserved",
                enabled: false,
            }, // bit 5
            Flag {
                name: "SQL_CA1_LOCK_NO_CHANGE",
                enabled: false,
            }, // bit 6
            Flag {
                name: "SQL_CA1_LOCK_EXCLUSIVE",
                enabled: false,
            }, // bit 7
            Flag {
                name: "SQL_CA1_LOCK_UNLOCK",
                enabled: false,
            }, // bit 8
            Flag {
                name: "SQL_CA1_POS_POSITION",
                enabled: false,
            }, // bit 9
            Flag {
                name: "SQL_CA1_POS_UPDATE",
                enabled: false,
            }, // bit 10
            Flag {
                name: "SQL_CA1_POS_DELETE",
                enabled: false,
            }, // bit 11
            Flag {
                name: "SQL_CA1_SELECT_FOR_UPDATE",
                enabled: false,
            }, // bit 12
            Flag {
                name: "SQL_CA1_BULK_ADD",
                enabled: false,
            }, // bit 13
            Flag {
                name: "SQL_CA1_BULK_UPDATE_BY_BOOKMARK",
                enabled: false,
            }, // bit 14
            Flag {
                name: "SQL_CA1_BULK_DELETE_BY_BOOKMARK",
                enabled: false,
            }, // bit 15
            Flag {
                name: "SQL_CA1_BULK_FETCH_BY_BOOKMARK",
                enabled: false,
            }, // bit 16
            Flag {
                name: "SQL_CA1_POS_REFRESH",
                enabled: false,
            }, // bit 17
            Flag {
                name: "SQL_CA1_POSITIONED_UPDATE",
                enabled: false,
            }, // bit 18
            Flag {
                name: "SQL_CA1_POSITIONED_DELETE",
                enabled: false,
            }, // bit 19
        ]
    };
}

/// `SQL_FORWARD_ONLY_CURSOR_ATTRIBUTES1` (146).
pub const FORWARD_ONLY_CURSOR_ATTRIBUTES1: &[Flag] = ca1_cursor_attributes!(true);

/// `SQL_KEYSET_CURSOR_ATTRIBUTES1` (150).
pub const KEYSET_CURSOR_ATTRIBUTES1: &[Flag] = ca1_cursor_attributes!(false);

/// `SQL_STATIC_CURSOR_ATTRIBUTES1` (167).
pub const STATIC_CURSOR_ATTRIBUTES1: &[Flag] = ca1_cursor_attributes!(false);

/// `SQL_DYNAMIC_CURSOR_ATTRIBUTES1` (144).
pub const DYNAMIC_CURSOR_ATTRIBUTES1: &[Flag] = ca1_cursor_attributes!(false);

/// `SQL_CA2_*` layout shared by the cursor-attributes2 info types.
macro_rules! ca2_cursor_attributes {
    () => {
        &[
            Flag {
                name: "SQL_CA2_READ_ONLY_CONCURRENCY",
                enabled: false,
            }, // bit 0
            Flag {
                name: "SQL_CA2_LOCK_CONCURRENCY",
                enabled: false,
            }, // bit 1
            Flag {
                name: "SQL_CA2_OPT_ROWVER_CONCURRENCY",
                enabled: false,
            }, // bit 2
            Flag {
                name: "SQL_CA2_OPT_VALUES_CONCURRENCY",
                enabled: false,
            }, // bit 3
            Flag {
                name: "SQL_CA2_SENSITIVITY_ADDITIONS",
                enabled: false,
            }, // bit 4
            Flag {
                name: "SQL_CA2_SENSITIVITY_DELETIONS",
                enabled: false,
            }, // bit 5
            Flag {
                name: "SQL_CA2_SENSITIVITY_UPDATES",
                enabled: false,
            }, // bit 6
            Flag {
                name: "SQL_CA2_MAX_ROWS_SELECT",
                enabled: false,
            }, // bit 7
            Flag {
                name: "SQL_CA2_MAX_ROWS_INSERT",
                enabled: false,
            }, // bit 8
            Flag {
                name: "SQL_CA2_MAX_ROWS_DELETE",
                enabled: false,
            }, // bit 9
            Flag {
                name: "SQL_CA2_MAX_ROWS_UPDATE",
                enabled: false,
            }, // bit 10
            Flag {
                name: "SQL_CA2_MAX_ROWS_CATALOG",
                enabled: false,
            }, // bit 11
            Flag {
                name: "SQL_CA2_MAX_ROWS_AFFECTS_ALL",
                enabled: false,
            }, // bit 12
            Flag {
                name: "SQL_CA2_CRC_EXACT",
                enabled: false,
            }, // bit 13
            Flag {
                name: "SQL_CA2_CRC_APPROXIMATE",
                enabled: false,
            }, // bit 14
            Flag {
                name: "SQL_CA2_SIMULATE_NON_UNIQUE",
                enabled: false,
            }, // bit 15
            Flag {
                name: "SQL_CA2_SIMULATE_TRY_UNIQUE",
                enabled: false,
            }, // bit 16
            Flag {
                name: "SQL_CA2_SIMULATE_UNIQUE",
                enabled: false,
            }, // bit 17
        ]
    };
}

/// `SQL_FORWARD_ONLY_CURSOR_ATTRIBUTES2` (147).
pub const FORWARD_ONLY_CURSOR_ATTRIBUTES2: &[Flag] = ca2_cursor_attributes!();

/// `SQL_KEYSET_CURSOR_ATTRIBUTES2` (151).
pub const KEYSET_CURSOR_ATTRIBUTES2: &[Flag] = ca2_cursor_attributes!();

/// `SQL_STATIC_CURSOR_ATTRIBUTES2` (168).
pub const STATIC_CURSOR_ATTRIBUTES2: &[Flag] = ca2_cursor_attributes!();

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

/// `SQL_FETCH_DIRECTION` (8) — supported fetch directions (deprecated in ODBC 3.0).
pub const FETCH_DIRECTION: &[Flag] = &[
    Flag {
        name: "SQL_FD_FETCH_NEXT",
        enabled: true,
    }, // bit 0
    Flag {
        name: "SQL_FD_FETCH_FIRST",
        enabled: false,
    }, // bit 1
    Flag {
        name: "SQL_FD_FETCH_LAST",
        enabled: false,
    }, // bit 2
    Flag {
        name: "SQL_FD_FETCH_PRIOR",
        enabled: false,
    }, // bit 3
    Flag {
        name: "SQL_FD_FETCH_ABSOLUTE",
        enabled: false,
    }, // bit 4
    Flag {
        name: "SQL_FD_FETCH_RELATIVE",
        enabled: false,
    }, // bit 5
    Flag {
        name: "SQL_FD_FETCH_RESUME",
        enabled: false,
    }, // bit 6
    Flag {
        name: "SQL_FD_FETCH_BOOKMARK",
        enabled: false,
    }, // bit 7
];

/// `SQL_ALTER_TABLE` (86) — supported ALTER TABLE sub-clauses.
pub const ALTER_TABLE: &[Flag] = &[
    Flag {
        name: "SQL_AT_ADD_COLUMN",
        enabled: false,
    }, // bit 0  0x00001
    Flag {
        name: "SQL_AT_DROP_COLUMN",
        enabled: false,
    }, // bit 1  0x00002
    Flag {
        name: "(reserved)",
        enabled: false,
    }, // bit 2  0x00004
    Flag {
        name: "SQL_AT_ADD_CONSTRAINT",
        enabled: false,
    }, // bit 3  0x00008
    Flag {
        name: "(reserved)",
        enabled: false,
    }, // bit 4  0x00010
    Flag {
        name: "SQL_AT_ADD_COLUMN_SINGLE",
        enabled: true,
    }, // bit 5  0x00020
    Flag {
        name: "SQL_AT_ADD_COLUMN_DEFAULT",
        enabled: true,
    }, // bit 6  0x00040
    Flag {
        name: "SQL_AT_ADD_COLUMN_COLLATION",
        enabled: false,
    }, // bit 7  0x00080
    Flag {
        name: "SQL_AT_SET_COLUMN_DEFAULT",
        enabled: false,
    }, // bit 8  0x00100
    Flag {
        name: "SQL_AT_DROP_COLUMN_DEFAULT",
        enabled: true,
    }, // bit 9  0x00200
    Flag {
        name: "SQL_AT_DROP_COLUMN_CASCADE",
        enabled: true,
    }, // bit 10 0x00400
    Flag {
        name: "SQL_AT_DROP_COLUMN_RESTRICT",
        enabled: true,
    }, // bit 11 0x00800
    Flag {
        name: "SQL_AT_ADD_TABLE_CONSTRAINT",
        enabled: true,
    }, // bit 12 0x01000
    Flag {
        name: "SQL_AT_DROP_TABLE_CONSTRAINT_CASCADE",
        enabled: true,
    }, // bit 13 0x02000
    Flag {
        name: "SQL_AT_DROP_TABLE_CONSTRAINT_RESTRICT",
        enabled: true,
    }, // bit 14 0x04000
    Flag {
        name: "SQL_AT_CONSTRAINT_NAME_DEFINITION",
        enabled: true,
    }, // bit 15 0x08000
    Flag {
        name: "SQL_AT_CONSTRAINT_INITIALLY_DEFERRED",
        enabled: true,
    }, // bit 16 0x10000
    Flag {
        name: "SQL_AT_CONSTRAINT_INITIALLY_IMMEDIATE",
        enabled: true,
    }, // bit 17 0x20000
    Flag {
        name: "SQL_AT_CONSTRAINT_DEFERRABLE",
        enabled: true,
    }, // bit 18 0x40000
    Flag {
        name: "SQL_AT_CONSTRAINT_NON_DEFERRABLE",
        enabled: true,
    }, // bit 19 0x80000
];

/// `SQL_OJ_CAPABILITIES` (115) — supported outer join types.
pub const OJ_CAPABILITIES: &[Flag] = &[
    Flag {
        name: "SQL_OJ_LEFT",
        enabled: true,
    }, // bit 0
    Flag {
        name: "SQL_OJ_RIGHT",
        enabled: true,
    }, // bit 1
    Flag {
        name: "SQL_OJ_FULL",
        enabled: true,
    }, // bit 2
    Flag {
        name: "SQL_OJ_NESTED",
        enabled: false,
    }, // bit 3
    Flag {
        name: "SQL_OJ_NOT_ORDERED",
        enabled: false,
    }, // bit 4
    Flag {
        name: "SQL_OJ_INNER",
        enabled: false,
    }, // bit 5
    Flag {
        name: "SQL_OJ_ALL_COMPARISON_OPS",
        enabled: false,
    }, // bit 6
];

/// `SQL_ALTER_DOMAIN` (117) — supported ALTER DOMAIN sub-clauses.
pub const ALTER_DOMAIN: &[Flag] = &[
    Flag {
        name: "SQL_AD_CONSTRAINT_NAME_DEFINITION",
        enabled: false,
    }, // bit 0  0x001
    Flag {
        name: "(reserved)",
        enabled: false,
    }, // bit 1  0x002
    Flag {
        name: "(reserved)",
        enabled: false,
    }, // bit 2  0x004
    Flag {
        name: "(reserved)",
        enabled: false,
    }, // bit 3  0x008
    Flag {
        name: "(reserved)",
        enabled: false,
    }, // bit 4  0x010
    Flag {
        name: "SQL_AD_ADD_CONSTRAINT_INITIALLY_DEFERRED",
        enabled: false,
    }, // bit 5  0x020
    Flag {
        name: "SQL_AD_ADD_CONSTRAINT_INITIALLY_IMMEDIATE",
        enabled: false,
    }, // bit 6  0x040
    Flag {
        name: "SQL_AD_ADD_CONSTRAINT_DEFERRABLE",
        enabled: false,
    }, // bit 7  0x080
    Flag {
        name: "SQL_AD_ADD_CONSTRAINT_NON_DEFERRABLE",
        enabled: false,
    }, // bit 8  0x100
];

/// `SQL_DATETIME_LITERALS` (119) — supported SQL-92 datetime literal types.
pub const DATETIME_LITERALS: &[Flag] = &[
    Flag {
        name: "SQL_DL_SQL92_DATE",
        enabled: false,
    }, // bit 0
    Flag {
        name: "SQL_DL_SQL92_TIME",
        enabled: false,
    }, // bit 1
    Flag {
        name: "SQL_DL_SQL92_TIMESTAMP",
        enabled: false,
    }, // bit 2
    Flag {
        name: "SQL_DL_SQL92_INTERVAL_YEAR",
        enabled: false,
    }, // bit 3
    Flag {
        name: "SQL_DL_SQL92_INTERVAL_MONTH",
        enabled: false,
    }, // bit 4
    Flag {
        name: "SQL_DL_SQL92_INTERVAL_DAY",
        enabled: false,
    }, // bit 5
    Flag {
        name: "SQL_DL_SQL92_INTERVAL_HOUR",
        enabled: false,
    }, // bit 6
    Flag {
        name: "(reserved)",
        enabled: false,
    }, // bit 7
    Flag {
        name: "SQL_DL_SQL92_INTERVAL_SECOND",
        enabled: false,
    }, // bit 8
    Flag {
        name: "SQL_DL_SQL92_INTERVAL_YEAR_TO_MONTH",
        enabled: false,
    }, // bit 9
    Flag {
        name: "SQL_DL_SQL92_INTERVAL_DAY_TO_HOUR",
        enabled: false,
    }, // bit 10
    Flag {
        name: "SQL_DL_SQL92_INTERVAL_DAY_TO_MINUTE",
        enabled: false,
    }, // bit 11
    Flag {
        name: "SQL_DL_SQL92_INTERVAL_DAY_TO_SECOND",
        enabled: false,
    }, // bit 12
    Flag {
        name: "SQL_DL_SQL92_INTERVAL_HOUR_TO_MINUTE",
        enabled: false,
    }, // bit 13
    Flag {
        name: "SQL_DL_SQL92_INTERVAL_HOUR_TO_SECOND",
        enabled: false,
    }, // bit 14
    Flag {
        name: "SQL_DL_SQL92_INTERVAL_MINUTE_TO_SECOND",
        enabled: false,
    }, // bit 15
];

/// `SQL_BATCH_ROW_COUNT` (120) — batch row-count reporting behavior.
pub const BATCH_ROW_COUNT: &[Flag] = &[
    Flag {
        name: "SQL_BRC_PROCEDURES",
        enabled: false,
    }, // bit 0
    Flag {
        name: "SQL_BRC_EXPLICIT",
        enabled: false,
    }, // bit 1
    Flag {
        name: "SQL_BRC_ROLLED_UP",
        enabled: false,
    }, // bit 2
];

/// `SQL_BATCH_SUPPORT` (121) — driver support for batches.
pub const BATCH_SUPPORT: &[Flag] = &[
    Flag {
        name: "SQL_BS_SELECT_EXPLICIT",
        enabled: false,
    }, // bit 0
    Flag {
        name: "SQL_BS_ROW_COUNT_EXPLICIT",
        enabled: false,
    }, // bit 1
    Flag {
        name: "SQL_BS_SELECT_PROC",
        enabled: false,
    }, // bit 2
    Flag {
        name: "SQL_BS_ROW_COUNT_PROC",
        enabled: false,
    }, // bit 3
];

/// `SQL_CONVERT_INTERVAL_DAY_TIME` (123) — conversion targets from INTERVAL DAY-TIME source.
/// Layout mirrors the other SQL_CONVERT_* families (25 bits, 0..=24).
pub const CONVERT_INTERVAL_DAY_TIME: &[Flag] = &[
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
        enabled: false,
    }, // bit 19
    Flag {
        name: "SQL_CVT_INTERVAL_DAY_TIME",
        enabled: true,
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
        enabled: false,
    }, // bit 24
];

/// `SQL_CONVERT_INTERVAL_YEAR_MONTH` (124) — conversion targets from INTERVAL YEAR-MONTH source.
/// Layout mirrors the other SQL_CONVERT_* families (25 bits, 0..=24).
pub const CONVERT_INTERVAL_YEAR_MONTH: &[Flag] = &[
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
        enabled: false,
    }, // bit 24
];

/// `SQL_CREATE_ASSERTION` (127) — supported CREATE ASSERTION options.
pub const CREATE_ASSERTION: &[Flag] = &[
    Flag {
        name: "SQL_CA_CREATE_ASSERTION",
        enabled: false,
    }, // bit 0
    Flag {
        name: "SQL_CA_CONSTRAINT_INITIALLY_DEFERRED",
        enabled: false,
    }, // bit 1
    Flag {
        name: "SQL_CA_CONSTRAINT_INITIALLY_IMMEDIATE",
        enabled: false,
    }, // bit 2
    Flag {
        name: "SQL_CA_CONSTRAINT_DEFERRABLE",
        enabled: false,
    }, // bit 3
    Flag {
        name: "SQL_CA_CONSTRAINT_NON_DEFERRABLE",
        enabled: false,
    }, // bit 4
];

/// `SQL_CREATE_CHARACTER_SET` (128) — supported CREATE CHARACTER SET options.
pub const CREATE_CHARACTER_SET: &[Flag] = &[
    Flag {
        name: "SQL_CCS_CREATE_CHARACTER_SET",
        enabled: false,
    }, // bit 0
    Flag {
        name: "SQL_CCS_COLLATE_CLAUSE",
        enabled: false,
    }, // bit 1
    Flag {
        name: "SQL_CCS_LIMITED_COLLATION",
        enabled: false,
    }, // bit 2
];

/// `SQL_CREATE_COLLATION` (129) — supported CREATE COLLATION options.
pub const CREATE_COLLATION: &[Flag] = &[
    Flag {
        name: "SQL_CCO_CREATE_COLLATION",
        enabled: false,
    }, // bit 0
];

/// `SQL_CREATE_DOMAIN` (130) — supported CREATE DOMAIN options.
pub const CREATE_DOMAIN: &[Flag] = &[
    Flag {
        name: "SQL_CDO_CREATE_DOMAIN",
        enabled: false,
    }, // bit 0
    Flag {
        name: "SQL_CDO_DEFAULT",
        enabled: false,
    }, // bit 1
    Flag {
        name: "SQL_CDO_CONSTRAINT",
        enabled: false,
    }, // bit 2
    Flag {
        name: "SQL_CDO_COLLATE",
        enabled: false,
    }, // bit 3
    Flag {
        name: "SQL_CDO_CONSTRAINT_NAME_DEFINITION",
        enabled: false,
    }, // bit 4
    Flag {
        name: "SQL_CDO_CONSTRAINT_INITIALLY_DEFERRED",
        enabled: false,
    }, // bit 5
    Flag {
        name: "SQL_CDO_CONSTRAINT_INITIALLY_IMMEDIATE",
        enabled: false,
    }, // bit 6
    Flag {
        name: "SQL_CDO_CONSTRAINT_DEFERRABLE",
        enabled: false,
    }, // bit 7
    Flag {
        name: "SQL_CDO_CONSTRAINT_NON_DEFERRABLE",
        enabled: false,
    }, // bit 8
];

/// `SQL_CREATE_SCHEMA` (131) — supported CREATE SCHEMA options.
pub const CREATE_SCHEMA: &[Flag] = &[
    Flag {
        name: "SQL_CS_CREATE_SCHEMA",
        enabled: true,
    }, // bit 0
    Flag {
        name: "SQL_CS_AUTHORIZATION",
        enabled: true,
    }, // bit 1
    Flag {
        name: "SQL_CS_DEFAULT_CHARACTER_SET",
        enabled: false,
    }, // bit 2
];

/// `SQL_CREATE_TABLE` (132) — supported CREATE TABLE options.
pub const CREATE_TABLE: &[Flag] = &[
    Flag {
        name: "SQL_CT_CREATE_TABLE",
        enabled: true,
    }, // bit 0
    Flag {
        name: "SQL_CT_COMMIT_PRESERVE",
        enabled: false,
    }, // bit 1
    Flag {
        name: "SQL_CT_COMMIT_DELETE",
        enabled: false,
    }, // bit 2
    Flag {
        name: "SQL_CT_GLOBAL_TEMPORARY",
        enabled: false,
    }, // bit 3
    Flag {
        name: "SQL_CT_LOCAL_TEMPORARY",
        enabled: false,
    }, // bit 4
    Flag {
        name: "SQL_CT_CONSTRAINT_INITIALLY_DEFERRED",
        enabled: true,
    }, // bit 5
    Flag {
        name: "SQL_CT_CONSTRAINT_INITIALLY_IMMEDIATE",
        enabled: true,
    }, // bit 6
    Flag {
        name: "SQL_CT_CONSTRAINT_DEFERRABLE",
        enabled: true,
    }, // bit 7
    Flag {
        name: "SQL_CT_CONSTRAINT_NON_DEFERRABLE",
        enabled: true,
    }, // bit 8
    Flag {
        name: "SQL_CT_COLUMN_CONSTRAINT",
        enabled: true,
    }, // bit 9
    Flag {
        name: "SQL_CT_COLUMN_DEFAULT",
        enabled: false,
    }, // bit 10
    Flag {
        name: "SQL_CT_COLUMN_COLLATION",
        enabled: false,
    }, // bit 11
    Flag {
        name: "SQL_CT_TABLE_CONSTRAINT",
        enabled: true,
    }, // bit 12
    Flag {
        name: "SQL_CT_CONSTRAINT_NAME_DEFINITION",
        enabled: true,
    }, // bit 13
];

/// `SQL_CREATE_TRANSLATION` (133) — supported CREATE TRANSLATION options.
pub const CREATE_TRANSLATION: &[Flag] = &[
    Flag {
        name: "SQL_CTR_CREATE_TRANSLATION",
        enabled: false,
    }, // bit 0
];

/// `SQL_CREATE_VIEW` (134) — supported CREATE VIEW options.
pub const CREATE_VIEW: &[Flag] = &[
    Flag {
        name: "SQL_CV_CREATE_VIEW",
        enabled: true,
    }, // bit 0
    Flag {
        name: "SQL_CV_CHECK_OPTION",
        enabled: false,
    }, // bit 1
    Flag {
        name: "SQL_CV_CASCADED",
        enabled: false,
    }, // bit 2
    Flag {
        name: "SQL_CV_LOCAL",
        enabled: false,
    }, // bit 3
];

/// `SQL_DROP_ASSERTION` (136) — supported DROP ASSERTION options.
pub const DROP_ASSERTION: &[Flag] = &[
    Flag {
        name: "SQL_DA_DROP_ASSERTION",
        enabled: false,
    }, // bit 0
];

/// `SQL_DROP_CHARACTER_SET` (137) — supported DROP CHARACTER SET options.
pub const DROP_CHARACTER_SET: &[Flag] = &[
    Flag {
        name: "SQL_DCS_DROP_CHARACTER_SET",
        enabled: false,
    }, // bit 0
];

/// `SQL_DROP_COLLATION` (138) — supported DROP COLLATION options.
pub const DROP_COLLATION: &[Flag] = &[
    Flag {
        name: "SQL_DC_DROP_COLLATION",
        enabled: false,
    }, // bit 0
];

/// `SQL_DROP_DOMAIN` (139) — supported DROP DOMAIN options.
pub const DROP_DOMAIN: &[Flag] = &[
    Flag {
        name: "SQL_DD_DROP_DOMAIN",
        enabled: false,
    }, // bit 0
    Flag {
        name: "SQL_DD_CASCADE",
        enabled: false,
    }, // bit 1
    Flag {
        name: "SQL_DD_RESTRICT",
        enabled: false,
    }, // bit 2
];

/// `SQL_DROP_SCHEMA` (140) — supported DROP SCHEMA options.
pub const DROP_SCHEMA: &[Flag] = &[
    Flag {
        name: "SQL_DS_DROP_SCHEMA",
        enabled: true,
    }, // bit 0
    Flag {
        name: "SQL_DS_RESTRICT",
        enabled: true,
    }, // bit 1
    Flag {
        name: "SQL_DS_CASCADE",
        enabled: true,
    }, // bit 2
];

/// `SQL_DROP_TABLE` (141) — supported DROP TABLE options.
pub const DROP_TABLE: &[Flag] = &[
    Flag {
        name: "SQL_DT_DROP_TABLE",
        enabled: true,
    }, // bit 0
    Flag {
        name: "SQL_DT_RESTRICT",
        enabled: true,
    }, // bit 1
    Flag {
        name: "SQL_DT_CASCADE",
        enabled: true,
    }, // bit 2
];

/// `SQL_DROP_TRANSLATION` (142) — supported DROP TRANSLATION options.
pub const DROP_TRANSLATION: &[Flag] = &[
    Flag {
        name: "SQL_DTR_DROP_TRANSLATION",
        enabled: false,
    }, // bit 0
];

/// `SQL_DROP_VIEW` (143) — supported DROP VIEW options.
pub const DROP_VIEW: &[Flag] = &[
    Flag {
        name: "SQL_DV_DROP_VIEW",
        enabled: true,
    }, // bit 0
    Flag {
        name: "SQL_DV_RESTRICT",
        enabled: false,
    }, // bit 1
    Flag {
        name: "SQL_DV_CASCADE",
        enabled: false,
    }, // bit 2
];

/// `SQL_DYNAMIC_CURSOR_ATTRIBUTES2` (145).
pub const DYNAMIC_CURSOR_ATTRIBUTES2: &[Flag] = ca2_cursor_attributes!();

/// `SQL_INFO_SCHEMA_VIEWS` (149) — supported INFORMATION_SCHEMA views.
pub const INFO_SCHEMA_VIEWS: &[Flag] = &[
    Flag {
        name: "SQL_ISV_ASSERTIONS",
        enabled: false,
    }, // bit 0  0x000001
    Flag {
        name: "SQL_ISV_CHARACTER_SETS",
        enabled: false,
    }, // bit 1  0x000002
    Flag {
        name: "SQL_ISV_CHECK_CONSTRAINTS",
        enabled: false,
    }, // bit 2  0x000004
    Flag {
        name: "SQL_ISV_COLLATIONS",
        enabled: false,
    }, // bit 3  0x000008
    Flag {
        name: "SQL_ISV_COLUMN_DOMAIN_USAGE",
        enabled: false,
    }, // bit 4  0x000010
    Flag {
        name: "SQL_ISV_COLUMN_PRIVILEGES",
        enabled: false,
    }, // bit 5  0x000020
    Flag {
        name: "SQL_ISV_COLUMNS",
        enabled: true,
    }, // bit 6  0x000040
    Flag {
        name: "SQL_ISV_CONSTRAINT_COLUMN_USAGE",
        enabled: false,
    }, // bit 7  0x000080
    Flag {
        name: "SQL_ISV_CONSTRAINT_TABLE_USAGE",
        enabled: false,
    }, // bit 8  0x000100
    Flag {
        name: "SQL_ISV_DOMAIN_CONSTRAINTS",
        enabled: false,
    }, // bit 9  0x000200
    Flag {
        name: "SQL_ISV_DOMAINS",
        enabled: false,
    }, // bit 10 0x000400
    Flag {
        name: "SQL_ISV_KEY_COLUMN_USAGE",
        enabled: false,
    }, // bit 11 0x000800
    Flag {
        name: "SQL_ISV_REFERENTIAL_CONSTRAINTS",
        enabled: true,
    }, // bit 12 0x001000
    Flag {
        name: "SQL_ISV_SCHEMATA",
        enabled: true,
    }, // bit 13 0x002000
    Flag {
        name: "SQL_ISV_SQL_LANGUAGES",
        enabled: false,
    }, // bit 14 0x004000
    Flag {
        name: "SQL_ISV_TABLE_CONSTRAINTS",
        enabled: true,
    }, // bit 15 0x008000
    Flag {
        name: "SQL_ISV_TABLE_PRIVILEGES",
        enabled: true,
    }, // bit 16 0x010000
    Flag {
        name: "SQL_ISV_TABLES",
        enabled: true,
    }, // bit 17 0x020000
    Flag {
        name: "SQL_ISV_TRANSLATIONS",
        enabled: false,
    }, // bit 18 0x040000
    Flag {
        name: "SQL_ISV_USAGE_PRIVILEGES",
        enabled: true,
    }, // bit 19 0x080000
    Flag {
        name: "SQL_ISV_VIEW_COLUMN_USAGE",
        enabled: false,
    }, // bit 20 0x100000
    Flag {
        name: "SQL_ISV_VIEW_TABLE_USAGE",
        enabled: false,
    }, // bit 21 0x200000
    Flag {
        name: "SQL_ISV_VIEWS",
        enabled: true,
    }, // bit 22 0x400000
];

/// `SQL_SUBQUERIES` (95) — supported subquery types.
pub const SUBQUERIES: &[Flag] = &[
    Flag {
        name: "SQL_SQ_COMPARISON",
        enabled: true,
    }, // bit 0
    Flag {
        name: "SQL_SQ_EXISTS",
        enabled: true,
    }, // bit 1
    Flag {
        name: "SQL_SQ_IN",
        enabled: true,
    }, // bit 2
    Flag {
        name: "SQL_SQ_QUANTIFIED",
        enabled: false,
    }, // bit 3
    Flag {
        name: "SQL_SQ_CORRELATED_SUBQUERIES",
        enabled: true,
    }, // bit 4
];

/// `SQL_UNION` (96) — supported UNION clause types.
pub const UNION: &[Flag] = &[
    Flag {
        name: "SQL_U_UNION",
        enabled: true,
    }, // bit 0
    Flag {
        name: "SQL_U_UNION_ALL",
        enabled: true,
    }, // bit 1
];

/// `SQL_SQL92_DATETIME_FUNCTIONS` (155) — supported SQL-92 datetime scalar functions.
pub const SQL92_DATETIME_FUNCTIONS: &[Flag] = &[
    Flag {
        name: "SQL_SDF_CURRENT_DATE",
        enabled: true,
    }, // bit 0
    Flag {
        name: "SQL_SDF_CURRENT_TIME",
        enabled: true,
    }, // bit 1
    Flag {
        name: "SQL_SDF_CURRENT_TIMESTAMP",
        enabled: true,
    }, // bit 2
];

/// `SQL_SQL92_FOREIGN_KEY_DELETE_RULE` (156) — supported delete rules for foreign keys.
pub const SQL92_FOREIGN_KEY_DELETE_RULE: &[Flag] = &[
    Flag {
        name: "SQL_SFKD_CASCADE",
        enabled: true,
    }, // bit 0
    Flag {
        name: "SQL_SFKD_NO_ACTION",
        enabled: true,
    }, // bit 1
    Flag {
        name: "SQL_SFKD_SET_DEFAULT",
        enabled: true,
    }, // bit 2
    Flag {
        name: "SQL_SFKD_SET_NULL",
        enabled: true,
    }, // bit 3
];

/// `SQL_SQL92_FOREIGN_KEY_UPDATE_RULE` (157) — supported update rules for foreign keys.
pub const SQL92_FOREIGN_KEY_UPDATE_RULE: &[Flag] = &[
    Flag {
        name: "SQL_SFKU_CASCADE",
        enabled: true,
    }, // bit 0
    Flag {
        name: "SQL_SFKU_NO_ACTION",
        enabled: true,
    }, // bit 1
    Flag {
        name: "SQL_SFKU_SET_DEFAULT",
        enabled: true,
    }, // bit 2
    Flag {
        name: "SQL_SFKU_SET_NULL",
        enabled: true,
    }, // bit 3
];

/// `SQL_SQL92_GRANT` (158) — supported clauses in the SQL-92 GRANT statement.
pub const SQL92_GRANT: &[Flag] = &[
    Flag {
        name: "SQL_SG_USAGE_ON_DOMAIN",
        enabled: false,
    }, // bit 0  0x0001
    Flag {
        name: "SQL_SG_USAGE_ON_CHARACTER_SET",
        enabled: false,
    }, // bit 1  0x0002
    Flag {
        name: "SQL_SG_USAGE_ON_COLLATION",
        enabled: false,
    }, // bit 2  0x0004
    Flag {
        name: "SQL_SG_USAGE_ON_TRANSLATION",
        enabled: false,
    }, // bit 3  0x0008
    Flag {
        name: "SQL_SG_WITH_GRANT_OPTION",
        enabled: true,
    }, // bit 4  0x0010
    Flag {
        name: "SQL_SG_DELETE_TABLE",
        enabled: true,
    }, // bit 5  0x0020
    Flag {
        name: "SQL_SG_INSERT_TABLE",
        enabled: true,
    }, // bit 6  0x0040
    Flag {
        name: "SQL_SG_INSERT_COLUMN",
        enabled: false,
    }, // bit 7  0x0080
    Flag {
        name: "SQL_SG_REFERENCES_TABLE",
        enabled: true,
    }, // bit 8  0x0100
    Flag {
        name: "SQL_SG_REFERENCES_COLUMN",
        enabled: false,
    }, // bit 9  0x0200
    Flag {
        name: "SQL_SG_SELECT_TABLE",
        enabled: true,
    }, // bit 10 0x0400
    Flag {
        name: "SQL_SG_UPDATE_TABLE",
        enabled: true,
    }, // bit 11 0x0800
    Flag {
        name: "SQL_SG_UPDATE_COLUMN",
        enabled: false,
    }, // bit 12 0x1000
];

/// `SQL_SQL92_NUMERIC_VALUE_FUNCTIONS` (159) — supported SQL-92 numeric value scalar functions.
pub const SQL92_NUMERIC_VALUE_FUNCTIONS: &[Flag] = &[
    Flag {
        name: "SQL_SNVF_BIT_LENGTH",
        enabled: true,
    }, // bit 0
    Flag {
        name: "SQL_SNVF_CHAR_LENGTH",
        enabled: false,
    }, // bit 1
    Flag {
        name: "SQL_SNVF_CHARACTER_LENGTH",
        enabled: false,
    }, // bit 2
    Flag {
        name: "SQL_SNVF_EXTRACT",
        enabled: true,
    }, // bit 3
    Flag {
        name: "SQL_SNVF_OCTET_LENGTH",
        enabled: true,
    }, // bit 4
    Flag {
        name: "SQL_SNVF_POSITION",
        enabled: true,
    }, // bit 5
];

/// `SQL_SQL92_REVOKE` (162) — supported clauses in the SQL-92 REVOKE statement.
pub const SQL92_REVOKE: &[Flag] = &[
    Flag {
        name: "SQL_SR_USAGE_ON_DOMAIN",
        enabled: false,
    }, // bit 0  0x0001
    Flag {
        name: "SQL_SR_USAGE_ON_CHARACTER_SET",
        enabled: false,
    }, // bit 1  0x0002
    Flag {
        name: "SQL_SR_USAGE_ON_COLLATION",
        enabled: false,
    }, // bit 2  0x0004
    Flag {
        name: "SQL_SR_USAGE_ON_TRANSLATION",
        enabled: false,
    }, // bit 3  0x0008
    Flag {
        name: "SQL_SR_GRANT_OPTION_FOR",
        enabled: true,
    }, // bit 4  0x0010
    Flag {
        name: "SQL_SR_CASCADE",
        enabled: true,
    }, // bit 5  0x0020
    Flag {
        name: "SQL_SR_RESTRICT",
        enabled: true,
    }, // bit 6  0x0040
    Flag {
        name: "SQL_SR_DELETE_TABLE",
        enabled: true,
    }, // bit 7  0x0080
    Flag {
        name: "SQL_SR_INSERT_TABLE",
        enabled: true,
    }, // bit 8  0x0100
    Flag {
        name: "SQL_SR_INSERT_COLUMN",
        enabled: false,
    }, // bit 9  0x0200
    Flag {
        name: "SQL_SR_REFERENCES_TABLE",
        enabled: true,
    }, // bit 10 0x0400
    Flag {
        name: "SQL_SR_REFERENCES_COLUMN",
        enabled: false,
    }, // bit 11 0x0800
    Flag {
        name: "SQL_SR_SELECT_TABLE",
        enabled: true,
    }, // bit 12 0x1000
    Flag {
        name: "SQL_SR_UPDATE_TABLE",
        enabled: true,
    }, // bit 13 0x2000
    Flag {
        name: "SQL_SR_UPDATE_COLUMN",
        enabled: false,
    }, // bit 14 0x4000
];

/// `SQL_SQL92_ROW_VALUE_CONSTRUCTOR` (163) — supported row value constructor expressions.
pub const SQL92_ROW_VALUE_CONSTRUCTOR: &[Flag] = &[
    Flag {
        name: "SQL_SRVC_VALUE_EXPRESSION",
        enabled: true,
    }, // bit 0
    Flag {
        name: "SQL_SRVC_NULL",
        enabled: true,
    }, // bit 1
    Flag {
        name: "SQL_SRVC_DEFAULT",
        enabled: true,
    }, // bit 2
    Flag {
        name: "SQL_SRVC_ROW_SUBQUERY",
        enabled: true,
    }, // bit 3
];

/// `SQL_SQL92_STRING_FUNCTIONS` (164) — supported SQL-92 string scalar functions.
pub const SQL92_STRING_FUNCTIONS: &[Flag] = &[
    Flag {
        name: "SQL_SSF_CONVERT",
        enabled: true,
    }, // bit 0
    Flag {
        name: "SQL_SSF_LOWER",
        enabled: true,
    }, // bit 1
    Flag {
        name: "SQL_SSF_UPPER",
        enabled: true,
    }, // bit 2
    Flag {
        name: "SQL_SSF_SUBSTRING",
        enabled: true,
    }, // bit 3
    Flag {
        name: "SQL_SSF_TRANSLATE",
        enabled: false,
    }, // bit 4
    Flag {
        name: "SQL_SSF_TRIM_BOTH",
        enabled: true,
    }, // bit 5
    Flag {
        name: "SQL_SSF_TRIM_LEADING",
        enabled: true,
    }, // bit 6
    Flag {
        name: "SQL_SSF_TRIM_TRAILING",
        enabled: true,
    }, // bit 7
];

/// `SQL_POSITIONED_STATEMENTS` (80) — supported positioned statement types (deprecated).
pub const POSITIONED_STATEMENTS: &[Flag] = &[
    Flag {
        name: "SQL_PS_POSITIONED_DELETE",
        enabled: false,
    }, // bit 0
    Flag {
        name: "SQL_PS_POSITIONED_UPDATE",
        enabled: false,
    }, // bit 1
    Flag {
        name: "SQL_PS_SELECT_FOR_UPDATE",
        enabled: false,
    }, // bit 2
];

/// `SQL_DDL_INDEX` (170) — support for CREATE INDEX and DROP INDEX.
pub const DDL_INDEX: &[Flag] = &[
    Flag {
        name: "SQL_DI_CREATE_INDEX",
        enabled: false,
    }, // bit 0
    Flag {
        name: "SQL_DI_DROP_INDEX",
        enabled: false,
    }, // bit 1
];

/// `SQL_INSERT_STATEMENT` (172) — supported forms of INSERT statement.
pub const INSERT_STATEMENT: &[Flag] = &[
    Flag {
        name: "SQL_IS_INSERT_LITERALS",
        enabled: true,
    }, // bit 0
    Flag {
        name: "SQL_IS_INSERT_SEARCHED",
        enabled: true,
    }, // bit 1
    Flag {
        name: "SQL_IS_SELECT_INTO",
        enabled: false,
    }, // bit 2
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

        assert_eq!(synthesize(SCROLL_CONCURRENCY), 0x1);
        assert_eq!(synthesize(SCROLL_OPTIONS), 0x1);
        assert_eq!(synthesize(TXN_ISOLATION_OPTION), 0x2);
        assert_eq!(synthesize(LOCK_TYPES), 0x2);
        assert_eq!(synthesize(STATIC_SENSITIVITY), 0x3);
        assert_eq!(synthesize(FORWARD_ONLY_CURSOR_ATTRIBUTES1), 0x1);
        assert_eq!(synthesize(POS_OPERATIONS), 0x0);
        assert_eq!(synthesize(BOOKMARK_PERSISTENCE), 0x0);
        assert_eq!(synthesize(FORWARD_ONLY_CURSOR_ATTRIBUTES2), 0x0);
        assert_eq!(synthesize(KEYSET_CURSOR_ATTRIBUTES1), 0x0);
        assert_eq!(synthesize(KEYSET_CURSOR_ATTRIBUTES2), 0x0);
        assert_eq!(synthesize(STATIC_CURSOR_ATTRIBUTES1), 0x0);
        assert_eq!(synthesize(STATIC_CURSOR_ATTRIBUTES2), 0x0);
        assert_eq!(synthesize(DYNAMIC_CURSOR_ATTRIBUTES1), 0x0);

        // ---- New non-zero bitmask families ----------------------------------
        assert_eq!(synthesize(FETCH_DIRECTION), 0x1);
        assert_eq!(synthesize(OJ_CAPABILITIES), 0x7);
        assert_eq!(synthesize(SUBQUERIES), 0x17);
        assert_eq!(synthesize(UNION), 0x3);
        assert_eq!(synthesize(SQL92_DATETIME_FUNCTIONS), 0x7);
        assert_eq!(synthesize(SQL92_FOREIGN_KEY_DELETE_RULE), 0xF);
        assert_eq!(synthesize(SQL92_FOREIGN_KEY_UPDATE_RULE), 0xF);
        assert_eq!(synthesize(SQL92_GRANT), 0xD70);
        assert_eq!(synthesize(SQL92_NUMERIC_VALUE_FUNCTIONS), 0x39);
        assert_eq!(synthesize(SQL92_REVOKE), 0x35F0);
        assert_eq!(synthesize(SQL92_ROW_VALUE_CONSTRUCTOR), 0xF);
        assert_eq!(synthesize(SQL92_STRING_FUNCTIONS), 0xEF);
        assert_eq!(synthesize(CREATE_SCHEMA), 0x3);
        assert_eq!(synthesize(CREATE_TABLE), 0x33E1);
        assert_eq!(synthesize(CREATE_VIEW), 0x1);
        assert_eq!(synthesize(DROP_SCHEMA), 0x7);
        assert_eq!(synthesize(DROP_TABLE), 0x7);
        assert_eq!(synthesize(DROP_VIEW), 0x1);
        assert_eq!(synthesize(INSERT_STATEMENT), 0x3);

        // ---- New all-zero bitmask families ----------------------------------
        assert_eq!(synthesize(ALTER_TABLE), 0xFFE60);
        assert_eq!(synthesize(ALTER_DOMAIN), 0x0);
        assert_eq!(synthesize(DATETIME_LITERALS), 0x0);
        assert_eq!(synthesize(BATCH_ROW_COUNT), 0x0);
        assert_eq!(synthesize(BATCH_SUPPORT), 0x0);
        assert_eq!(synthesize(CREATE_ASSERTION), 0x0);
        assert_eq!(synthesize(CREATE_CHARACTER_SET), 0x0);
        assert_eq!(synthesize(CREATE_COLLATION), 0x0);
        assert_eq!(synthesize(CREATE_DOMAIN), 0x0);
        assert_eq!(synthesize(CREATE_TRANSLATION), 0x0);
        assert_eq!(synthesize(DROP_ASSERTION), 0x0);
        assert_eq!(synthesize(DROP_CHARACTER_SET), 0x0);
        assert_eq!(synthesize(DROP_COLLATION), 0x0);
        assert_eq!(synthesize(DROP_DOMAIN), 0x0);
        assert_eq!(synthesize(DROP_TRANSLATION), 0x0);
        assert_eq!(synthesize(POSITIONED_STATEMENTS), 0x0);
        assert_eq!(synthesize(DDL_INDEX), 0x0);
        assert_eq!(synthesize(DYNAMIC_CURSOR_ATTRIBUTES2), 0x0);
        assert_eq!(synthesize(INFO_SCHEMA_VIEWS), 0x4BB040);

        // ---- New CONVERT_INTERVAL families (25 bits each) -------------------
        for (name, slice) in [
            ("CONVERT_INTERVAL_DAY_TIME", CONVERT_INTERVAL_DAY_TIME),
            ("CONVERT_INTERVAL_YEAR_MONTH", CONVERT_INTERVAL_YEAR_MONTH),
        ] {
            assert_eq!(slice.len(), 25, "{name} slice should cover bits 0..=24");
            assert!(
                synthesize(slice) != 0,
                "{name}: CONVERT_INTERVAL family must advertise at least one target",
            );
        }
        assert_eq!(synthesize(CONVERT_INTERVAL_DAY_TIME), 0x106F1F);
        assert_eq!(synthesize(CONVERT_INTERVAL_YEAR_MONTH), 0x86F1F);

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
