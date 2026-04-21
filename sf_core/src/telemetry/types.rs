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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn error_origin_as_str() {
        assert_eq!(ErrorOrigin::Core.as_str(), "core");
        assert_eq!(ErrorOrigin::Wrapper.as_str(), "wrapper");
    }
}
