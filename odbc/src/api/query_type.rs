//! Snowflake `statementTypeId` taxonomy.
//!
//! Specific ids are explicit constants; hierarchical matching uses level-2 (`0xff00`) and level-3
//! (`0xf000`) bit-masks so subtypes (e.g. `COPY = 0x3600`) `belongs_to`
//! their parent family (`DML = 0x3000`).

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct QueryType(i64);

/// Result-set behavior that the ODBC layer should produce for a statement.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ResultKind {
    /// Opens a cursor. `SQLNumResultCols > 0`, `SQLFetch` yields rows.
    Cursor,
    /// No cursor; `SQLRowCount` reports an update count. DML only.
    UpdateCount,
    /// No cursor, no update count (`SQLRowCount` returns -1). DDL/TCL/acks.
    NoResult,
}

// Full taxonomy is public API; not every id is referenced internally.
#[allow(dead_code)]
impl QueryType {
    pub const UNKNOWN: Self = Self(0x0000);
    pub const SELECT: Self = Self(0x1000);
    pub const EXPLAIN: Self = Self(0x2000);
    pub const DML: Self = Self(0x3000);
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

    pub fn belongs_to(self, family: Self) -> bool {
        family.0 == self.0
            || family.0 == (self.0 & Self::LEVEL_2_MASK)
            || family.0 == (self.0 & Self::LEVEL_3_MASK)
    }

    pub fn is_dml(self) -> bool {
        self.belongs_to(Self::DML)
    }

    fn is_stage_operation(self) -> bool {
        self == Self::LIST_FILES || self.belongs_to(Self::STAGE_FILE_OPERATIONS)
    }

    fn is_syscmd_with_result_set(self) -> bool {
        self == Self::SHOW || self == Self::DESCRIBE || self == Self::LIST_FILES
    }

    /// Whether this statement produces a browsable result set (cursor).
    pub fn has_result_set(self) -> bool {
        self == Self::SELECT
            || self == Self::EXPLAIN
            || self == Self::CALL
            || self == Self::COPY
            || self == Self::MANAGE_PATS
            || self.is_syscmd_with_result_set()
            || self.is_stage_operation()
    }

    /// Families we explicitly know produce no cursor and no update count.
    /// Used as the middle step in [`Self::result_kind`] so that ids the driver
    /// hasn't catalogued yet default to `Cursor` rather than silently dropping
    /// result data.
    fn is_known_no_result(self) -> bool {
        self.belongs_to(Self::DDL)
            || self.belongs_to(Self::MISC_QUERY_TYPES)
            || self.belongs_to(Self::MULTI_STMT)
    }

    /// Which ODBC state should the driver produce for a statement of this type.
    ///
    /// Precedence: `has_result_set` → `is_dml` → `is_known_no_result`.  Any id
    /// that falls out the bottom (including `UNKNOWN` and ids the driver
    /// doesn't recognise yet) defaults to `Cursor` so new Snowflake statement
    /// types don't silently lose their result data.
    pub fn result_kind(self) -> ResultKind {
        if self.has_result_set() {
            ResultKind::Cursor
        } else if self.is_dml() {
            ResultKind::UpdateCount
        } else if self.is_known_no_result() {
            ResultKind::NoResult
        } else {
            ResultKind::Cursor
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unknown_is_neither_dml_nor_cursor() {
        let qt = QueryType::from_raw(None);
        assert_eq!(qt, QueryType::UNKNOWN);
        assert!(!qt.is_dml());
        assert!(!qt.has_result_set());
    }

    #[test]
    fn copy_belongs_to_dml_via_level3_mask() {
        assert!(QueryType::COPY.belongs_to(QueryType::DML));
        assert!(QueryType::COPY.is_dml());
    }

    #[test]
    fn tcl_subtypes_belong_to_misc_via_level3_mask() {
        assert!(QueryType::BEGIN.belongs_to(QueryType::MISC_QUERY_TYPES));
        assert!(QueryType::COMMIT.belongs_to(QueryType::MISC_QUERY_TYPES));
        assert!(QueryType::SET.belongs_to(QueryType::MISC_QUERY_TYPES));
    }

    #[test]
    fn tcl_has_no_result_set() {
        assert!(!QueryType::BEGIN.has_result_set());
        assert!(!QueryType::COMMIT.has_result_set());
        assert!(!QueryType::END.has_result_set());
        assert!(!QueryType::SET.has_result_set());
    }

    #[test]
    fn ddl_is_neither_dml_nor_cursor() {
        let qt = QueryType::DDL;
        assert!(!qt.is_dml());
        assert!(!qt.has_result_set());
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
        // The SYSCMD family parent itself is not whitelisted by old ODBC.
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
    fn copy_is_cursor_and_dml() {
        // COPY is classified as DML by family but also opens a cursor.
        assert!(QueryType::COPY.is_dml());
        assert!(QueryType::COPY.has_result_set());
    }

    #[test]
    fn multi_stmt_and_unrecognised_ids_classify_as_no_cursor() {
        assert!(!QueryType::MULTI_STMT.is_dml());
        assert!(!QueryType::MULTI_STMT.has_result_set());
        // A synthetic future id outside every known family.
        let future = QueryType::from_raw(Some(0xBEEF));
        assert!(!future.is_dml());
        assert!(!future.has_result_set());
    }

    #[test]
    fn manage_pats_has_result_set() {
        // 0x6244 belongs to the DDL family via the level-3 mask but produces a
        // browsable result set (e.g. SHOW PATS).  Verified as an explicit
        // exception in `has_result_set()`.
        assert!(QueryType::MANAGE_PATS.belongs_to(QueryType::DDL));
        assert!(QueryType::MANAGE_PATS.has_result_set());
        assert_eq!(QueryType::MANAGE_PATS.result_kind(), ResultKind::Cursor);
    }

    #[test]
    fn result_kind_update_count_for_plain_dml() {
        // DML family excluding COPY — plain INSERT/UPDATE/DELETE/MERGE land
        // on the `UpdateCount` branch.
        assert_eq!(QueryType::DML.result_kind(), ResultKind::UpdateCount);
    }

    #[test]
    fn result_kind_cursor_for_copy_even_though_its_dml() {
        // COPY belongs to DML but also opens a cursor; `Cursor` must win.
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
        assert_eq!(QueryType::MULTI_STMT.result_kind(), ResultKind::NoResult);
    }

    #[test]
    fn result_kind_cursor_for_unknown_or_absent_id() {
        // Defensive default: an id the driver doesn't recognise yet (or a
        // missing `statementTypeId` on the wire) should open a cursor so
        // future statement types don't silently drop their result data.
        assert_eq!(QueryType::UNKNOWN.result_kind(), ResultKind::Cursor);
        assert_eq!(QueryType::from_raw(None).result_kind(), ResultKind::Cursor);
        assert_eq!(
            QueryType::from_raw(Some(0xBEEF)).result_kind(),
            ResultKind::Cursor
        );
    }
}
