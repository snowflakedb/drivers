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
        let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();
        let def_path = std::path::Path::new(&manifest_dir).join("exports.def");
        println!("cargo:rustc-cdylib-link-arg=/DEF:{}", def_path.display());
    }
}
