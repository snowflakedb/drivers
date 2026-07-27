use std::fmt::Display;

/// Convert any error that implements [`Display`] (e.g. `sf_core`'s `ApiError`)
/// into a `napi::Error`, preserving the error's own message.
pub fn to_napi_err<E: Display>(err: E) -> napi::Error {
    napi::Error::from_reason(err.to_string())
}
