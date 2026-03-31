/// Where the error originated.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorOrigin {
    Core,
    Wrapper,
}

impl ErrorOrigin {
    pub fn as_str(&self) -> &'static str {
        match self {
            ErrorOrigin::Core => "core",
            ErrorOrigin::Wrapper => "wrapper",
        }
    }
}

/// Data provided by wrappers for session_init.
#[derive(Debug, Clone)]
pub struct SessionInitData {
    pub driver_name: String,
    pub driver_version: String,
    pub language_runtime: String,
    pub language_version: String,
    pub language_compiler: Option<String>,
    //pub release_date: Option<String>,
    //pub is_lts: Option<bool>,
    pub svn_revision: Option<String>,
    pub application: Option<String>,
    pub application_path: Option<String>,
    pub tracing_level: Option<i32>,
    pub login_timeout: Option<i32>,
    pub network_timeout: Option<i32>,
    pub socket_timeout: Option<i32>,
}

/// Data for driver_exception events reported by wrappers.
#[derive(Debug, Clone)]
pub struct WrapperErrorData {
    pub exception_type: String,
    pub error_source: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn error_origin_as_str() {
        assert_eq!(ErrorOrigin::Core.as_str(), "core");
        assert_eq!(ErrorOrigin::Wrapper.as_str(), "wrapper");
    }
}
