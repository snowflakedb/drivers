use std::env;
use std::path::PathBuf;

fn main() {
    // Always include an rpath relative to the loaded library location so colocated deps work
    if cfg!(target_os = "macos") {
        println!("cargo:rustc-link-arg=-Wl,-rpath,@loader_path");
    } else if cfg!(target_os = "linux") {
        println!("cargo:rustc-link-arg=-Wl,-rpath,$ORIGIN");
    }

    // Best-effort: add rpath to AWS-LC artifacts if present in target dir layout
    // target/debug/build/aws-lc-fips-sys-*/out/build/artifacts
    if let Ok(out_dir) = env::var("OUT_DIR") {
        // OUT_DIR like: target/debug/build/sf_core-<hash>/out
        let mut p = PathBuf::from(out_dir);
        // Walk up to target/debug
        for _ in 0..3 {
            let _ = p.pop();
        }
        // p now points to target/debug
        let build_dir = p.join("build");
        if let Ok(entries) = std::fs::read_dir(&build_dir) {
            for entry in entries.flatten() {
                let name = entry.file_name().to_string_lossy().into_owned();
                if name.starts_with("aws-lc-fips-sys-") {
                    let artifacts_abs = entry.path().join("out/build/artifacts");
                    if artifacts_abs.is_dir() {
                        // Compute relative path from the produced dylib location to artifacts dir.
                        println!(
                            "cargo:rustc-link-arg=-Wl,-rpath,{}",
                            artifacts_abs.display()
                        );
                        break;
                    }
                }
            }
        }
    }
}
