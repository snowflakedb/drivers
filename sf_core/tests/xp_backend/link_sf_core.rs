//! XP hosts run on Linux only. This crate still has to link `sf_core` on other
//! targets: Windows applies `sf_core/exports.def` to every package executable,
//! and an empty test crate would leave those C exports unresolved.

#[test]
fn xp_host_tests_are_linux_only() {
    let _driver = sf_core::apis::database_driver_v1::DatabaseDriverV1::new();
}
