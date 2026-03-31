include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../build_common.rs"));

use proto_generator::{CodeGenerator, RustGenerator};
use std::path::PathBuf;

fn normalize_path(path: PathBuf) -> PathBuf {
    #[cfg(target_os = "windows")]
    {
        let path_str = path.to_string_lossy();
        if let Some(stripped) = path_str.strip_prefix(r"\\?\") {
            return PathBuf::from(stripped);
        }
    }
    path
}

fn generate_protobuf() {
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR not set");
    let out_dir = std::env::var("OUT_DIR").expect("OUT_DIR not set");
    let proto_file = normalize_path(
        PathBuf::from(&manifest_dir)
            .join(
                Path::new("..")
                    .join("protobuf")
                    .join("database_driver_v1.proto"),
            )
            .canonicalize()
            .expect("Proto file not found"),
    );

    println!("cargo:rerun-if-changed={}", proto_file.display());

    let context = proto_generator::GeneratorContext::new(proto_file, Vec::new());
    let generator = RustGenerator::new();
    let result = generator
        .generate(&context)
        .expect("Failed to generate protobuf Rust code");

    if result.has_errors() {
        for error in &result.errors {
            eprintln!("Proto generation error: {}", error);
        }
        panic!("Proto generation failed");
    }

    let out_path = PathBuf::from(&out_dir);
    for (path, file) in &result.files {
        let full_path = out_path.join(path);
        std::fs::write(&full_path, &file.content).expect("Failed to write generated file");
    }
}

fn main() {
    if !cfg!(target_os = "windows") {
        emit_loader_rpaths();
    }

    // On Windows, use a .def file to limit DLL exports to only C API functions.
    // This avoids the PE/COFF 65535 export symbol limit.
    // NOTE: rustc-link-arg applies to test executables too, but exports.def omits
    // the LIBRARY directive so MSVC does not set the DLL bit in test binaries.
    // Without the DLL bit, test executables remain valid Win32 applications.
    #[cfg(target_os = "windows")]
    {
        let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR not set");
        let def_path = std::path::Path::new(&manifest_dir).join("exports.def");
        // Rebuild when the export list changes so the DLL export table stays in sync.
        println!("cargo:rerun-if-changed={}", def_path.display());
        // No quoting: cargo passes rustc-link-arg as a single OS-level token
        // (via a response file), so the linker receives the path verbatim.
        // Adding quotes makes them literal characters in the path, which both
        // lld-link and MSVC link.exe reject.
        println!("cargo:rustc-link-arg=/DEF:{}", def_path.display());
    }

    generate_protobuf();
}
