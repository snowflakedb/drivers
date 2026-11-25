// `azure_multipart` async/type depth exceeds the default after TransferOp plumbing.
#![recursion_limit = "256"]

#[path = "../common/mod.rs"]
pub mod common;

pub mod authentication;
pub mod config;
pub mod http;
pub mod multi_statement;
pub mod put_get;
pub mod query;
pub mod session;
pub mod telemetry;
