//! Snowflake `statementTypeId` taxonomy.
//!
//! This is now a thin re-export of the single source of truth in
//! `sf_core::query_types::statement_type`, so the ODBC layer and the native
//! `sf_core` result path classify statements identically. The taxonomy, its
//! tests, and the reference-driver (`SFResults.cpp`) semantics live there.

pub use sf_core::query_types::statement_type::{QueryType, ResultKind};
