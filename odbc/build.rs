include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../build_common.rs"));

fn main() {
    // Tell rustc not to link against ODBC manager libraries
    // ODBC drivers should export functions, not import them from a manager
    println!("cargo:rustc-link-arg=-Wl,-undefined,dynamic_lookup");

    emit_loader_rpaths();
}
