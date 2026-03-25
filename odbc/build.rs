include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../build_common.rs"));

fn main() {
    #[cfg(not(target_os = "windows"))]
    {
        emit_loader_rpaths();
    }

    // On Windows, use a .def file to limit DLL exports to only ODBC API functions.
    // This avoids the PE/COFF 65535 export symbol limit.
    #[cfg(target_os = "windows")]
    {
        let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR not set");
        let def_path = std::path::Path::new(&manifest_dir).join("exports.def");
        // No quoting: cargo passes rustc-cdylib-link-arg as a single OS-level
        // token (via a response file), so the linker receives the path verbatim.
        println!("cargo:rustc-cdylib-link-arg=/DEF:{}", def_path.display());
    }
}
