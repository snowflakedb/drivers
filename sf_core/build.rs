include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../build_common.rs"));

fn main() {
    if !cfg!(target_os = "windows") {
        emit_loader_rpaths();
    }

    // On Windows, use a .def file to limit DLL exports to only C API functions.
    // This avoids the PE/COFF 65535 export symbol limit.
    #[cfg(target_os = "windows")]
    {
        let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();
        let def_path = std::path::Path::new(&manifest_dir).join("exports.def");
        // Use rustc-link-arg for dylib targets (not cdylib)
        println!("cargo:rustc-link-arg=/DEF:{}", def_path.display());
    }
}
