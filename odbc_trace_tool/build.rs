fn main() {
    // Provide library search paths for the ODBC driver manager.
    // The actual linking is done via #[link] attributes in replayer/mod.rs.

    if cfg!(target_os = "macos") {
        let iodbc_path = "/usr/local/iODBC/lib";
        if std::path::Path::new(iodbc_path)
            .join("libiodbc.dylib")
            .exists()
        {
            println!("cargo:rustc-link-search=native={iodbc_path}");
        }

        let homebrew_path = "/opt/homebrew/lib";
        if std::path::Path::new(homebrew_path).exists() {
            println!("cargo:rustc-link-search=native={homebrew_path}");
        }
    }
}
