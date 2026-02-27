pub(crate) struct Redacted;

impl std::fmt::Debug for Redacted {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("****")
    }
}

pub(crate) fn redact(opt: &Option<String>) -> Option<Redacted> {
    opt.as_ref().map(|_| Redacted)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn redacted_debug_displays_mask() {
        let value = Redacted;
        assert_eq!(format!("{:?}", value), "****");
    }

    #[test]
    fn redact_some_returns_redacted_some() {
        let secret = Some("super-secret".to_string());
        let redacted = redact(&secret);

        assert!(redacted.is_some());
        assert_eq!(format!("{:?}", redacted), "Some(****)");
    }

    #[test]
    fn redact_none_returns_none() {
        let secret: Option<String> = None;
        let redacted = redact(&secret);

        assert!(redacted.is_none());
    }
}
