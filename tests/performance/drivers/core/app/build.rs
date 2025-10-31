use std::env;
use std::fs;
use std::path::Path;

fn main() {
    // Set rpath to @loader_path (macOS) or $ORIGIN (Linux)
    let loader_prefix = if cfg!(target_os = "linux") {
        "$ORIGIN"
    } else {
        "@loader_path"
    };

    println!("cargo:rustc-link-arg=-Wl,-rpath,{}", loader_prefix);

    // Find and add rpaths for artifact directories (aws-lc-fips, etc.)
    if let Ok(out_dir) = env::var("OUT_DIR") {
        if let Some(target_dir) = Path::new(&out_dir)
            .ancestors()
            .find(|p| p.file_name().and_then(|n| n.to_str()) == Some("target"))
        {
            let build_dir = target_dir.join("release").join("build");
            if build_dir.exists() {
                if let Ok(entries) = fs::read_dir(&build_dir) {
                    for entry in entries.flatten() {
                        let name = entry.file_name();
                        if name.to_string_lossy().contains("-sys") {
                            let artifacts = entry.path().join("out/build/artifacts");
                            if artifacts.is_dir() {
                                if let Ok(rel) = artifacts.strip_prefix(target_dir.join("release"))
                                {
                                    println!(
                                        "cargo:rustc-link-arg=-Wl,-rpath,{}/{}",
                                        loader_prefix,
                                        rel.display()
                                    );
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
