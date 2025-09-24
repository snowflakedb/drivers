use std::env;
use std::path::PathBuf;

fn main() {
    // Loader-relative rpath so colocated deps resolve at runtime
    if cfg!(target_os = "macos") {
        println!("cargo:rustc-link-arg=-Wl,-rpath,@loader_path");
    } else if cfg!(target_os = "linux") {
        println!("cargo:rustc-link-arg=-Wl,-rpath,$ORIGIN");
    }

    // Best-effort: add absolute rpath to aws-lc artifacts dir if present
    if let Ok(out_dir) = env::var("OUT_DIR") {
        // OUT_DIR typically looks like: target/debug/build/odbc-<hash>/out
        let mut p = PathBuf::from(out_dir);
        // Walk up the directory tree until we find the "build" directory
        while let Some(component) = p.file_name() {
            if component == "build" {
                break;
            }
            let _ = p.pop();
            if p.as_os_str().is_empty() {
                break;
            }
        }
        let build_dir = p.clone();
        if let Ok(entries) = std::fs::read_dir(&build_dir) {
            for entry in entries.flatten() {
                let name = entry.file_name().to_string_lossy().into_owned();
                if name.starts_with("aws-lc-fips-sys-") {
                    let artifacts_abs = entry.path().join("out/build/artifacts");
                    if artifacts_abs.is_dir() {
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
