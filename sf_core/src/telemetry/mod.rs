//! Telemetry signals attached to Snowflake login payloads.
//!
//! Currently this module exposes platform detection helpers whose results are
//! serialized into `CLIENT_ENVIRONMENT.PLATFORM` on every login.

pub mod platform_detection;
