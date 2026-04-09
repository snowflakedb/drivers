use crate::apis::database_driver_v1::connection::WrapperIdentity;

/// Auto-detected system environment information for telemetry.
#[derive(Debug, Clone)]
pub struct EnvironmentInfo {
    pub os_name: String,
    pub os_version: String,
    pub os_architecture: String,
    pub driver_name: String,
    pub driver_version: String,
    pub language_runtime: String,
    pub language_version: String,
    /// `None` means compiler info is not applicable for this language.
    pub language_compiler: Option<String>,
}

impl EnvironmentInfo {
    /// Detect OS-level fields and merge wrapper identity (from `ConnectionInit`).
    pub fn with_wrapper(identity: &WrapperIdentity) -> Self {
        Self {
            os_name: std::env::consts::OS.to_string(),
            os_version: detect_os_version(),
            os_architecture: std::env::consts::ARCH.to_string(),
            driver_name: identity.driver_name.clone(),
            driver_version: identity.driver_version.clone(),
            language_runtime: identity.language_runtime.clone(),
            language_version: identity.language_version.clone(),
            language_compiler: identity.language_compiler.clone(),
        }
    }

    /// Detect OS-level fields only; wrapper fields remain empty.
    pub fn detect() -> Self {
        Self {
            os_name: std::env::consts::OS.to_string(),
            os_version: detect_os_version(),
            os_architecture: std::env::consts::ARCH.to_string(),
            driver_name: String::new(),
            driver_version: String::new(),
            language_runtime: String::new(),
            language_version: String::new(),
            language_compiler: None,
        }
    }
}

fn detect_os_version() -> String {
    #[cfg(target_os = "linux")]
    {
        if let Ok(content) = std::fs::read_to_string("/etc/os-release") {
            for line in content.lines() {
                if let Some(version) = line.strip_prefix("VERSION_ID=") {
                    return version.trim_matches('"').to_string();
                }
            }
        }
    }
    #[cfg(target_os = "macos")]
    {
        if let Ok(output) = std::process::Command::new("sw_vers")
            .arg("-productVersion")
            .output()
        {
            if output.status.success() {
                return String::from_utf8_lossy(&output.stdout).trim().to_string();
            }
        }
    }
    #[cfg(target_os = "windows")]
    {
        // `cmd /C ver` outputs e.g. "Microsoft Windows [Version 10.0.22631.5039]"
        if let Ok(output) = std::process::Command::new("cmd")
            .args(["/C", "ver"])
            .output()
        {
            if output.status.success() {
                let ver = String::from_utf8_lossy(&output.stdout);
                if let Some(start) = ver.find('[') {
                    if let Some(end) = ver.find(']') {
                        return ver[start + 1..end]
                            .strip_prefix("Version ")
                            .unwrap_or(&ver[start + 1..end])
                            .to_string();
                    }
                }
            }
        }
    }
    "unknown".to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detect_returns_non_empty_os_fields() {
        let info = EnvironmentInfo::detect();
        assert!(!info.os_name.is_empty());
        assert!(!info.os_architecture.is_empty());
    }

    #[test]
    fn wrapper_fields_are_empty_stubs() {
        let info = EnvironmentInfo::detect();
        assert!(info.driver_name.is_empty());
        assert!(info.driver_version.is_empty());
        assert!(info.language_runtime.is_empty());
        assert!(info.language_version.is_empty());
        assert!(info.language_compiler.is_none());
    }

    #[test]
    fn with_wrapper_merges_identity_and_os_fields() {
        let identity = WrapperIdentity {
            driver_name: "snowflake-connector-python".to_string(),
            driver_version: "3.0.0".to_string(),
            language_runtime: "CPython".to_string(),
            language_version: "3.12.0".to_string(),
            language_compiler: Some("GCC 13.2.0".to_string()),
        };
        let info = EnvironmentInfo::with_wrapper(&identity);
        assert!(!info.os_name.is_empty());
        assert_eq!(info.driver_name, "snowflake-connector-python");
        assert_eq!(info.driver_version, "3.0.0");
        assert_eq!(info.language_runtime, "CPython");
        assert_eq!(info.language_version, "3.12.0");
        assert_eq!(info.language_compiler, Some("GCC 13.2.0".to_string()));
    }

    #[test]
    fn with_wrapper_none_compiler() {
        let identity = WrapperIdentity {
            driver_name: "test".to_string(),
            driver_version: "1.0".to_string(),
            language_runtime: "node".to_string(),
            language_version: "20.0".to_string(),
            language_compiler: None,
        };
        let info = EnvironmentInfo::with_wrapper(&identity);
        assert!(info.language_compiler.is_none());
    }
}
