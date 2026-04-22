use std::path::{Path, PathBuf};

const PROTOC_VERSION: &str = "32.1";

/// Returns the path to a working `protoc` binary.
///
/// Resolution order:
/// 1. `PROTOC` environment variable (user override / offline builds)
/// 2. Cached binary in `~/.cache/protoc/v{VERSION}/bin/protoc`
/// 3. Fresh download from the official GitHub releases
pub fn protoc_path() -> PathBuf {
    if let Ok(protoc) = std::env::var("PROTOC") {
        let path = PathBuf::from(&protoc);
        if path.exists() {
            return path;
        }
        eprintln!("Warning: PROTOC={protoc} does not exist, falling back to download");
    }

    let cache = cache_dir();
    let bin = protoc_bin_path(&cache);

    if !bin.exists() {
        download_and_install(&cache);
        assert!(
            bin.exists(),
            "protoc binary not found at {} after installation",
            bin.display()
        );
    }

    bin
}

fn protoc_bin_path(cache: &Path) -> PathBuf {
    if cfg!(target_os = "windows") {
        cache.join("bin").join("protoc.exe")
    } else {
        cache.join("bin").join("protoc")
    }
}

fn cache_dir() -> PathBuf {
    home_dir()
        .join(".cache")
        .join("protoc")
        .join(format!("v{PROTOC_VERSION}"))
}

fn home_dir() -> PathBuf {
    std::env::var("HOME")
        .or_else(|_| std::env::var("USERPROFILE"))
        .map(PathBuf::from)
        .expect("Neither HOME nor USERPROFILE environment variable is set")
}

fn download_url() -> String {
    let platform = match (std::env::consts::OS, std::env::consts::ARCH) {
        ("linux", "x86_64") => "linux-x86_64",
        ("linux", "aarch64") => "linux-aarch_64",
        ("macos", "x86_64") => "osx-x86_64",
        ("macos", "aarch64") => "osx-aarch_64",
        ("windows", "x86_64") | ("windows", "aarch64") => "win64",
        ("windows", "x86") => "win32",
        (os, arch) => panic!("Unsupported platform for protoc download: {os}-{arch}"),
    };

    format!(
        "https://github.com/protocolbuffers/protobuf/releases/download/v{PROTOC_VERSION}/protoc-{PROTOC_VERSION}-{platform}.zip"
    )
}

fn download_and_install(dest: &Path) {
    let url = download_url();
    eprintln!("Downloading protoc v{PROTOC_VERSION} from {url}");

    std::fs::create_dir_all(dest).expect("Failed to create protoc cache directory");
    let zip_path = dest.join("protoc.zip");

    let status = std::process::Command::new("curl")
        .args(["-sSL", "--fail", "-o"])
        .arg(&zip_path)
        .arg(&url)
        .status()
        .expect("Failed to execute curl. Install curl or set the PROTOC env var.");

    assert!(
        status.success(),
        "Failed to download protoc from {url} (curl exit status: {status})"
    );

    extract_zip(&zip_path, dest);

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let bin = dest.join("bin").join("protoc");
        if bin.exists() {
            let mut perms = std::fs::metadata(&bin).unwrap().permissions();
            perms.set_mode(0o755);
            std::fs::set_permissions(&bin, perms).unwrap();
        }
    }

    let _ = std::fs::remove_file(&zip_path);
    eprintln!("protoc v{PROTOC_VERSION} installed to {}", dest.display());
}

#[cfg(unix)]
fn extract_zip(zip_path: &Path, dest: &Path) {
    let status = std::process::Command::new("unzip")
        .args(["-o", "-q"])
        .arg(zip_path)
        .arg("-d")
        .arg(dest)
        .status()
        .expect("Failed to execute unzip. Install unzip or set the PROTOC env var.");

    assert!(status.success(), "Failed to extract protoc archive");
}

#[cfg(windows)]
fn extract_zip(zip_path: &Path, dest: &Path) {
    let status = std::process::Command::new("powershell")
        .args([
            "-NoProfile",
            "-Command",
            &format!(
                "Expand-Archive -Path '{}' -DestinationPath '{}' -Force",
                zip_path.display(),
                dest.display()
            ),
        ])
        .status()
        .expect("Failed to execute PowerShell for archive extraction");

    assert!(status.success(), "Failed to extract protoc archive");
}
