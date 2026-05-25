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

    std::fs::create_dir_all(dest).expect("Failed to create protoc cache directory");
    let zip_path = dest.join("protoc.zip");

    // Two layers of retry guard the build against transient GitHub Releases failures
    // (recurring HTTP 5xx, slow connect, partial download):
    //
    //   1. `curl --retry 5` already covers the failure mode we actually see —
    //      curl's documented default treats HTTP 408/429/500/502/503/504 as
    //      transient and retries them with its own backoff, so a 504 from
    //      releases.githubusercontent.com is absorbed inside a single curl
    //      invocation without any extra flags. We intentionally do NOT pass
    //      `--retry-all-errors` (added in curl 7.71): the manylinux build
    //      container ships an older curl that hard-errors on unknown options
    //      (exit 2), which would push every attempt into the outer loop and
    //      effectively disable retries. Sticking to flags supported back to
    //      ~7.52 (`--retry`, `--retry-delay`, `--retry-max-time`, plus the
    //      connect/total timeouts) keeps every CI runner (macOS-15, ubuntu-24,
    //      Windows-2022, manylinux_2_*) on the same code path.
    //
    //   2. An outer exponential-backoff loop (1s → 2s → 4s → 8s, 5 attempts total)
    //      catches the failures curl's own retry doesn't: partial downloads that
    //      leave a corrupt zip on disk, transient DNS/SSL handshake errors that
    //      curl exits on without retrying, and any future failure class we
    //      haven't enumerated. Each outer attempt starts by deleting the
    //      previous zip so extract_zip never sees a half-written file.
    //
    // Total worst-case stall ≈ 1+2+4+8 = 15s of outer backoff plus curl's
    // internal retry budget (capped by --retry-max-time at 120s) — well under
    // any CI job budget, and far cheaper than rerunning the whole release
    // pipeline on a 504.
    const MAX_ATTEMPTS: u32 = 5;
    let mut last_status = None;
    for attempt in 1..=MAX_ATTEMPTS {
        eprintln!(
            "Downloading protoc v{PROTOC_VERSION} from {url} (attempt {attempt}/{MAX_ATTEMPTS})"
        );

        // Best-effort cleanup of any partial download from the previous attempt.
        // A corrupt zip would otherwise survive into extract_zip() and fail there.
        let _ = std::fs::remove_file(&zip_path);

        let status = std::process::Command::new("curl")
            .args([
                "-sSL",
                "--fail",
                "--retry",
                "5",
                "--retry-delay",
                "2",
                "--retry-max-time",
                "120",
                "--connect-timeout",
                "30",
                "--max-time",
                "300",
                "-o",
            ])
            .arg(&zip_path)
            .arg(&url)
            .status()
            .expect("Failed to execute curl. Install curl or set the PROTOC env var.");

        if status.success() {
            last_status = Some(status);
            break;
        }
        last_status = Some(status);

        if attempt < MAX_ATTEMPTS {
            let backoff_ms = 1_000u64 * (1u64 << (attempt - 1));
            eprintln!("curl failed (status: {status}); retrying in {backoff_ms} ms");
            std::thread::sleep(std::time::Duration::from_millis(backoff_ms));
        }
    }

    let status = last_status.expect("curl loop must produce at least one status");
    assert!(
        status.success(),
        "Failed to download protoc from {url} after {MAX_ATTEMPTS} attempts \
         (last curl exit status: {status}). \
         Set the PROTOC env var to a local protoc binary to bypass the download \
         (useful in offline or locked-down environments).",
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
