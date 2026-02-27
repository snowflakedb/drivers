pub(crate) struct Redacted;

impl std::fmt::Debug for Redacted {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("****")
    }
}

pub(crate) fn redact(opt: &Option<String>) -> Option<Redacted> {
    opt.as_ref().map(|_| Redacted)
}
