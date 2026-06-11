//! Snowflake `statementTypeId` taxonomy — the single source of truth shared by
//! the native `sf_core` result path and the ODBC layer.
//!
//! Specific ids are explicit constants; hierarchical matching uses level-2
//! (`0xff00`) and level-3 (`0xf000`) bit-masks so subtypes (e.g. `COPY = 0x3600`)
//! `belongs_to` their parent family (`DML = 0x3000`).

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct QueryType(i64);

/// Result-set behavior a statement produces, independent of any one wrapper.
///
/// The driver maps this to a concrete representation (ODBC `SQLRowCount`,
/// native `rows_affected`, etc.).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ResultKind {
    /// Opens a cursor / browsable result set (e.g. `SELECT`, `SHOW`, `CALL`).
    Cursor,
    /// No cursor; reports an update count. DML only (`INSERT`/`UPDATE`/...).
    UpdateCount,
    /// No cursor and no meaningful update count (DDL / TCL / acks / unknown).
    NoResult,
}

// Full taxonomy is shared API; not every id is referenced by every consumer.
#[allow(dead_code)]
impl QueryType {
    pub const UNKNOWN: Self = Self(0x0000);
    pub const SELECT: Self = Self(0x1000);
    pub const EXPLAIN: Self = Self(0x2000);
    pub const DML: Self = Self(0x3000);
    pub const INSERT: Self = Self(0x3100);
    pub const UPDATE: Self = Self(0x3200);
    pub const DELETE: Self = Self(0x3300);
    pub const MERGE: Self = Self(0x3400);
    pub const MULTI_TABLE_INSERT: Self = Self(0x3500);
    pub const COPY: Self = Self(0x3600);
    pub const SYSCMD: Self = Self(0x4000);
    pub const SHOW: Self = Self(0x4400);
    pub const DESCRIBE: Self = Self(0x4500);
    pub const LIST_FILES: Self = Self(0x4701);
    pub const DDL: Self = Self(0x6000);
    /// `MANAGE_PATS` — DDL-family statement that returns a browsable result set.
    /// Preserved as an explicit exception because it doesn't fit the DDL default.
    pub const MANAGE_PATS: Self = Self(0x6244);
    pub const STAGE_FILE_OPERATIONS: Self = Self(0x7000);
    pub const GET_FILES: Self = Self(0x7101);
    pub const PUT_FILES: Self = Self(0x7102);
    pub const REMOVE_FILES: Self = Self(0x7103);
    pub const MISC_QUERY_TYPES: Self = Self(0x8000);
    pub const BEGIN: Self = Self(0x8101);
    pub const END: Self = Self(0x8102);
    pub const COMMIT: Self = Self(0x8103);
    pub const SET: Self = Self(0x8104);
    pub const CALL: Self = Self(0x9000);
    pub const MULTI_STMT: Self = Self(0xA000);

    const LEVEL_2_MASK: i64 = 0xff00;
    const LEVEL_3_MASK: i64 = 0xf000;

    pub fn from_raw(id: Option<i64>) -> Self {
        Self(id.unwrap_or(0))
    }

    pub fn raw(self) -> i64 {
        self.0
    }

    pub fn belongs_to(self, family: Self) -> bool {
        family.0 == self.0
            || family.0 == (self.0 & Self::LEVEL_2_MASK)
            || family.0 == (self.0 & Self::LEVEL_3_MASK)
    }

    pub fn is_dml(self) -> bool {
        self.belongs_to(Self::DML)
    }

    /// Whether this statement produces a browsable result set (cursor).
    pub fn has_result_set(self) -> bool {
        self == Self::SELECT
            || self == Self::EXPLAIN
            || self == Self::CALL
            || self == Self::COPY
            || self == Self::MANAGE_PATS
            || self == Self::SHOW
            || self == Self::DESCRIBE
            || self == Self::LIST_FILES
            || self.belongs_to(Self::STAGE_FILE_OPERATIONS)
    }

    /// Classify the statement's result behavior.
    ///
    /// Precedence: `has_result_set` → `is_dml` → default `NoResult`.
    /// Matches the reference driver (`snowflake-odbc` `SFResults.cpp`), which
    /// whitelists cursor-producing types and DML, and treats everything else
    /// (DDL, TCL / MISC, MULTI_STMT parents, SYSCMD subtypes we haven't
    /// whitelisted, and unknown / future ids) as "no cursor, unknown row count".
    pub fn result_kind(self) -> ResultKind {
        if self.has_result_set() {
            ResultKind::Cursor
        } else if self.is_dml() {
            ResultKind::UpdateCount
        } else {
            ResultKind::NoResult
        }
    }

    /// Whether this DML statement reports its affected-row count as columns in
    /// the result rowset (the "number of rows inserted/updated/deleted" cells)
    /// rather than via a server-side total. Excludes `COPY`, which is DML by
    /// family but returns a browsable result set instead of an update count.
    pub fn reports_affected_rows_in_rowset(self) -> bool {
        self.is_dml() && self.result_kind() == ResultKind::UpdateCount
    }
}

/// Result-rowset column names whose integer cell values sum to a DML
/// statement's affected-row count. Matched case-insensitively against the
/// server's column names; prefix entries match `starts_with`.
pub const DML_AFFECTED_ROWS_COLUMNS: &[&str] = &[
    "number of rows updated",
    "number of multi-joined rows updated",
    "number of rows deleted",
];
pub const DML_AFFECTED_ROWS_COLUMN_PREFIXES: &[&str] = &["number of rows inserted"];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn copy_belongs_to_dml_via_level3_mask() {
        assert!(QueryType::COPY.belongs_to(QueryType::DML));
        assert!(QueryType::COPY.is_dml());
    }

    #[test]
    fn select_explain_call_have_result_sets() {
        assert!(QueryType::SELECT.has_result_set());
        assert!(QueryType::EXPLAIN.has_result_set());
        assert!(QueryType::CALL.has_result_set());
    }

    #[test]
    fn syscmd_subtypes_are_cursors_parent_is_not() {
        assert!(QueryType::SHOW.has_result_set());
        assert!(QueryType::DESCRIBE.has_result_set());
        assert!(QueryType::LIST_FILES.has_result_set());
        assert!(!QueryType::SYSCMD.has_result_set());
    }

    #[test]
    fn stage_operations_have_result_sets() {
        assert!(QueryType::GET_FILES.has_result_set());
        assert!(QueryType::PUT_FILES.has_result_set());
        assert!(QueryType::REMOVE_FILES.has_result_set());
        assert!(QueryType::STAGE_FILE_OPERATIONS.has_result_set());
    }

    #[test]
    fn manage_pats_has_result_set() {
        assert_eq!(QueryType::MANAGE_PATS.result_kind(), ResultKind::Cursor);
    }

    #[test]
    fn result_kind_update_count_for_plain_dml() {
        assert_eq!(QueryType::DML.result_kind(), ResultKind::UpdateCount);
        assert_eq!(QueryType::INSERT.result_kind(), ResultKind::UpdateCount);
        assert_eq!(QueryType::UPDATE.result_kind(), ResultKind::UpdateCount);
        assert_eq!(QueryType::DELETE.result_kind(), ResultKind::UpdateCount);
        assert_eq!(QueryType::MERGE.result_kind(), ResultKind::UpdateCount);
        assert_eq!(
            QueryType::MULTI_TABLE_INSERT.result_kind(),
            ResultKind::UpdateCount
        );
    }

    #[test]
    fn result_kind_cursor_for_copy_even_though_its_dml() {
        assert!(QueryType::COPY.is_dml());
        assert_eq!(QueryType::COPY.result_kind(), ResultKind::Cursor);
    }

    #[test]
    fn result_kind_cursor_for_select_and_friends() {
        assert_eq!(QueryType::SELECT.result_kind(), ResultKind::Cursor);
        assert_eq!(QueryType::EXPLAIN.result_kind(), ResultKind::Cursor);
        assert_eq!(QueryType::CALL.result_kind(), ResultKind::Cursor);
        assert_eq!(QueryType::SHOW.result_kind(), ResultKind::Cursor);
        assert_eq!(QueryType::DESCRIBE.result_kind(), ResultKind::Cursor);
        assert_eq!(QueryType::LIST_FILES.result_kind(), ResultKind::Cursor);
        assert_eq!(QueryType::GET_FILES.result_kind(), ResultKind::Cursor);
        assert_eq!(QueryType::PUT_FILES.result_kind(), ResultKind::Cursor);
    }

    #[test]
    fn result_kind_no_result_for_ddl_and_tcl() {
        assert_eq!(QueryType::DDL.result_kind(), ResultKind::NoResult);
        assert_eq!(QueryType::BEGIN.result_kind(), ResultKind::NoResult);
        assert_eq!(QueryType::COMMIT.result_kind(), ResultKind::NoResult);
        assert_eq!(QueryType::END.result_kind(), ResultKind::NoResult);
        assert_eq!(QueryType::SET.result_kind(), ResultKind::NoResult);
    }

    #[test]
    fn result_kind_no_result_for_multi_stmt_parent() {
        assert_eq!(QueryType::MULTI_STMT.result_kind(), ResultKind::NoResult);
        assert_eq!(
            QueryType::from_raw(Some(0xA100)).result_kind(),
            ResultKind::NoResult
        );
    }

    #[test]
    fn result_kind_no_result_for_unknown_or_absent_id() {
        assert_eq!(QueryType::UNKNOWN.result_kind(), ResultKind::NoResult);
        assert_eq!(
            QueryType::from_raw(None).result_kind(),
            ResultKind::NoResult
        );
        assert_eq!(
            QueryType::from_raw(Some(0xBEEF)).result_kind(),
            ResultKind::NoResult
        );
    }

    #[test]
    fn result_kind_no_result_for_syscmd_subtypes_not_whitelisted() {
        assert_eq!(QueryType::SYSCMD.result_kind(), ResultKind::NoResult);
        assert_eq!(
            QueryType::from_raw(Some(0x4104)).result_kind(),
            ResultKind::NoResult
        );
    }

    #[test]
    fn reports_affected_rows_in_rowset_for_plain_dml_only() {
        assert!(QueryType::INSERT.reports_affected_rows_in_rowset());
        assert!(QueryType::DML.reports_affected_rows_in_rowset());
        // COPY is DML-family but produces a cursor, not an update count.
        assert!(!QueryType::COPY.reports_affected_rows_in_rowset());
        // DDL / SELECT never report affected rows this way.
        assert!(!QueryType::DDL.reports_affected_rows_in_rowset());
        assert!(!QueryType::SELECT.reports_affected_rows_in_rowset());
    }
}
