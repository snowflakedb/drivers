pub mod generator;
pub mod generators;
pub mod protobuf;
mod protoc_installer;

pub use generator::*;
pub use generators::*;

/// Returns the absolute path to the `protoc` binary, downloading it from
/// the official GitHub releases if not already cached.
///
/// Override by setting the `PROTOC` environment variable.
pub fn vendored_protoc_path() -> std::path::PathBuf {
    protoc_installer::protoc_path()
}
