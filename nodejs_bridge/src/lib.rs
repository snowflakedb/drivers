use napi_derive::napi;
use sf_core::apis::database_driver_v1::DatabaseDriverV1;

#[napi]
pub fn dummy_test_entrypoint() -> String {
    let _driver = DatabaseDriverV1::new();
    format!("nodejs_bridge {} ok", env!("CARGO_PKG_VERSION"))
}
