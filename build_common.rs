use std::path::Path;
use std::{env, fs};

#[inline]
fn loader_prefix() -> &'static str {
    if cfg!(target_os = "linux") {
        return "$ORIGIN";
    }
    "@loader_path"
}

// Finds all artifact directories (*-sys*) under the build directory.
fn artifact_dirs_under_build(build_dir: &Path) -> Vec<std::path::PathBuf> {
    let mut found = Vec::new();
    if let Ok(mut entries) = fs::read_dir(build_dir) {
        while let Some(Ok(entry)) = entries.next() {
            let name = entry.file_name().to_string_lossy().into_owned();
            if name.contains("-sys") {
                let artifacts = entry.path().join("out/build/artifacts");
                if artifacts.is_dir() {
                    found.push(artifacts);
                }
            }
        }
    }
    found
}

pub fn emit_loader_rpaths() {
    println!("cargo:rustc-link-arg=-Wl,-rpath,{}", loader_prefix());

    if let Ok(out_dir) = env::var("OUT_DIR") {
        let build_dir_opt = Path::new(&out_dir)
            .ancestors()
            .find(|p| p.file_name().map(|n| n == "build").unwrap_or(false))
            .map(|p| p.to_path_buf());

        if let Some(build_dir) = build_dir_opt
            && let Some(target_debug_dir) = build_dir.parent()
        {
            for artifacts_abs in artifact_dirs_under_build(&build_dir) {
                if let Ok(rel) = artifacts_abs.strip_prefix(target_debug_dir) {
                    println!(
                        "cargo:rustc-link-arg=-Wl,-rpath,{}/{}",
                        loader_prefix(),
                        rel.display()
                    );
                }
            }
        }
    }
}
