include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../build_common.rs"));

fn main() {
    #[cfg(not(target_os = "windows"))]
    {
        emit_loader_rpaths();
    }

    // Export JNI entry points explicitly on Windows so Java can resolve the
    // native methods reliably across MSVC targets, including ARM64.
    #[cfg(target_os = "windows")]
    {
        let manifest_dir =
            std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR must be set by Cargo");
        let def_path = std::path::Path::new(&manifest_dir).join("exports.def");
        // Rebuild when the export list changes so the DLL export table stays in sync.
        println!("cargo:rerun-if-changed={}", def_path.display());
        // No quoting: cargo passes rustc-cdylib-link-arg as a single OS-level
        // token (via a response file), so the linker receives the path verbatim.
        println!("cargo:rustc-cdylib-link-arg=/DEF:{}", def_path.display());
    }
}
