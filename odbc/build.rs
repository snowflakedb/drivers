include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../build_common.rs"));

fn main() {
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    let odbc_api_version = read_odbc_metadata(&manifest_dir, "odbc_api_version");
    println!("cargo:rustc-env=SF_ODBC_API_VER={odbc_api_version}");
    println!("cargo:rerun-if-changed=Cargo.toml");

    #[cfg(not(target_os = "windows"))]
    {
        emit_loader_rpaths();
    }

    // On Windows, use a .def file to limit DLL exports to only ODBC API functions.
    // This avoids the PE/COFF 65535 export symbol limit.
    #[cfg(target_os = "windows")]
    {
        let manifest_path = std::path::Path::new(&manifest_dir);
        let def_path = manifest_path.join("exports.def");
        println!("cargo:rustc-cdylib-link-arg=/DEF:{}", def_path.display());

        let (major, minor, patch) = parse_odbc_preview_version(&manifest_dir);
        let commit_hash = resolve_commit_hash();

        let target_arch =
            std::env::var("CARGO_CFG_TARGET_ARCH").unwrap_or_else(|_| "x86_64".to_string());
        let original_filename = if target_arch == "x86" {
            "sfodbc32.dll"
        } else {
            "sfodbc.dll"
        };

        let version_header = manifest_path.join("src/setup/version_generated.h");
        std::fs::write(
            &version_header,
            format!(
                "#define SF_VERSION_MAJOR {major}\n\
                 #define SF_VERSION_MINOR {minor}\n\
                 #define SF_VERSION_PATCH {patch}\n\
                 #define SF_VERSION_STR \"{major}.{minor}.{patch}-{commit_hash}\\0\"\n\
                 #define SF_VERSION_CSV {major},{minor},{patch},0\n\
                 #define SF_ORIGINAL_FILENAME \"{original_filename}\\0\"\n"
            ),
        )
        .expect("failed to write version_generated.h");

        println!("cargo:rerun-if-changed=exports.def");
        println!("cargo:rerun-if-changed=src/setup/resource.rc");
        println!("cargo:rerun-if-changed=src/setup/resource.h");
        println!("cargo:rerun-if-env-changed=COMMIT_HASH");

        emit_git_head_rerun_hints();

        let rc_path = manifest_path.join("src/setup/resource.rc");
        if let embed_resource::CompilationResult::Failed(error) =
            embed_resource::compile(&rc_path, embed_resource::NONE)
        {
            panic!(
                "failed to compile Windows resource file `{}`: {error}",
                rc_path.display()
            );
        }
    }
}

fn read_odbc_metadata(manifest_dir: &str, key: &str) -> String {
    const SECTION: &str = "[package.metadata.odbc]";

    let cargo_toml_path = std::path::Path::new(manifest_dir).join("Cargo.toml");
    let cargo_toml = std::fs::read_to_string(&cargo_toml_path)
        .unwrap_or_else(|err| panic!("failed to read {}: {err}", cargo_toml_path.display()));

    let mut in_section = false;
    for line in cargo_toml.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with('[') && trimmed.ends_with(']') {
            in_section = trimmed == SECTION;
            continue;
        }
        if !in_section {
            continue;
        }
        let Some(rhs) = trimmed.strip_prefix(key) else {
            continue;
        };
        let rhs = rhs.trim_start();
        let Some(rhs) = rhs.strip_prefix('=') else {
            continue;
        };
        let value = rhs.trim();
        let value = value.split('#').next().unwrap_or(value).trim();
        if let Some(stripped) = value
            .strip_prefix('"')
            .and_then(|s| s.strip_suffix('"'))
            .or_else(|| value.strip_prefix('\'').and_then(|s| s.strip_suffix('\'')))
        {
            return stripped.to_string();
        }
        panic!("`{key}` in {SECTION} must be a quoted string, got `{value}`");
    }
    panic!(
        "`{key}` not found in {SECTION} of {}",
        cargo_toml_path.display()
    );
}

#[cfg(target_os = "windows")]
fn parse_odbc_preview_version(manifest_dir: &str) -> (u32, u32, u32) {
    let raw = read_odbc_metadata(manifest_dir, "odbc_preview_version");
    let parts: Vec<&str> = raw.split('.').collect();
    assert!(
        parts.len() == 3,
        "invalid odbc_preview_version: expected <major>.<minor>.<patch>, got `{raw}`"
    );
    (
        parts[0]
            .parse()
            .unwrap_or_else(|_| panic!("invalid major version in `{raw}`")),
        parts[1]
            .parse()
            .unwrap_or_else(|_| panic!("invalid minor version in `{raw}`")),
        parts[2]
            .parse()
            .unwrap_or_else(|_| panic!("invalid patch version in `{raw}`")),
    )
}

#[cfg(target_os = "windows")]
fn emit_git_head_rerun_hints() {
    let git_dir =
        std::path::Path::new(&std::env::var("CARGO_MANIFEST_DIR").unwrap()).join("../.git");
    let head_path = git_dir.join("HEAD");
    if head_path.exists() {
        println!("cargo:rerun-if-changed={}", head_path.display());
        if let Ok(contents) = std::fs::read_to_string(&head_path) {
            if let Some(ref_path) = contents.trim().strip_prefix("ref: ") {
                let full_ref = git_dir.join(ref_path);
                if full_ref.exists() {
                    println!("cargo:rerun-if-changed={}", full_ref.display());
                }
            }
        }
    }
}

#[cfg(target_os = "windows")]
fn resolve_commit_hash() -> String {
    if let Ok(hash) = std::env::var("COMMIT_HASH") {
        if !hash.is_empty() {
            return hash;
        }
    }
    std::process::Command::new("git")
        .args(["rev-parse", "--short", "HEAD"])
        .output()
        .ok()
        .and_then(|o| {
            if o.status.success() {
                Some(String::from_utf8_lossy(&o.stdout).trim().to_string())
            } else {
                None
            }
        })
        .unwrap_or_else(|| "unknown".to_string())
}
