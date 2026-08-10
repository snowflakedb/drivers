//! Re-export shim for the canonical parameter registry.
//!
//! The registry itself now lives in the standalone [`sf_params_spec`] crate so
//! that both `sf_core` and the wrapper code generator (`sf_params_codegen`) can
//! depend on it without a cycle. This module re-exports the full public surface
//! under the historical `crate::config::param_registry` path so existing
//! callers (and the `config` module re-exports) keep working unchanged.
//!
//! Defaults are carried across the crate boundary as
//! [`sf_params_spec::DefaultValue`]; the conversion into `sf_core`'s
//! [`crate::config::settings::Setting`] lives in
//! [`crate::config::settings`].

pub use sf_params_spec::*;
