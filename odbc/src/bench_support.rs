//! Benchmark-only re-exports.
//!
//! These items are otherwise private to the crate; they are surfaced here only
//! when the `bench` feature is enabled (and are `#[doc(hidden)]`) so the
//! criterion bench in `benches/conversion.rs` can drive the fetch-conversion
//! pipeline directly. This module is **not** part of the public API — do not
//! depend on it outside benchmarks.

pub use crate::api::CDataType;
pub use crate::conversion::error::ConversionError;
pub use crate::conversion::warning::Warnings;
pub use crate::conversion::{Binding, BindingStrides, ColumnConverter};

/// Build a column converter for `field` using default session settings.
///
/// Wraps the internal `make_converter` so the bench does not need to name the
/// crate-private `NumericSettings` type (keeping it off the re-export surface).
pub fn make_converter(field: &arrow::datatypes::Field) -> Box<dyn ColumnConverter> {
    crate::conversion::make_converter(field, &crate::conversion::NumericSettings::default())
        .expect("bench: make_converter should succeed for the benched fields")
}
