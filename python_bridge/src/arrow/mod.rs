//! Arrow C stream → Python row materialization.

mod batch_converter;
mod converters;
mod error;
mod iterator;
mod odbc_decode;
mod plan;
mod stream;

#[cfg(test)]
pub(crate) mod test_support;

pub use iterator::ArrowStreamIterator;
