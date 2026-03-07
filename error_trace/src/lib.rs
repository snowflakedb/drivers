pub use error_trace_derive::ErrorTrace;

/// Source-code location that owns its file path.
///
/// Unlike `snafu::Location` (which stores `&'static str`), this type
/// can be constructed from a `String`, making it usable in contexts
/// where the file path is dynamically produced.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Location {
    pub file: String,
    pub line: u32,
    pub column: u32,
}

impl Location {
    pub fn new(file: impl Into<String>, line: u32, column: u32) -> Self {
        Self {
            file: file.into(),
            line,
            column,
        }
    }
}

impl From<snafu::Location> for Location {
    fn from(loc: snafu::Location) -> Self {
        Self {
            file: loc.file.to_string(),
            line: loc.line,
            column: loc.column,
        }
    }
}

impl std::fmt::Display for Location {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}:{}:{}", self.file, self.line, self.column)
    }
}

/// A single frame in an error trace.
#[derive(Debug, Clone)]
pub struct ErrorTraceEntry {
    pub location: Location,
    pub message: String,
}

/// A trait for collecting an "error trace" — the chain of nested error
/// messages together with the source-code location where each error was
/// constructed.  Analogous to a stack trace, but follows the logical
/// `source` chain of application-defined error types rather than the
/// call stack.
pub trait ErrorTrace {
    fn error_trace(&self) -> Vec<ErrorTraceEntry>;
}

impl<T: ErrorTrace + ?Sized> ErrorTrace for Box<T> {
    fn error_trace(&self) -> Vec<ErrorTraceEntry> {
        (**self).error_trace()
    }
}

// ---------------------------------------------------------------------------
// Autoref-based specialization hack
// ---------------------------------------------------------------------------
//
// `ErrorTraceResolver` lets derive-generated code recurse into a `source`
// field *without* requiring the source type to implement `ErrorTrace`.
//
// Method resolution order makes this work:
//
//   1. If `T: ErrorTrace`, the **inherent** method on `ErrorTraceResolver<&T>`
//      is found first and delegates to `error_trace()`.
//
//   2. Otherwise the **trait** method from `ErrorTraceFallback` (implemented
//      for all `ErrorTraceResolver<&T>`) is used, which returns an empty vec.
//
// Callers write:
//
//     ErrorTraceResolver(&source).resolve()
//

pub struct ErrorTraceResolver<T>(pub T);

impl<T: ErrorTrace> ErrorTraceResolver<&T> {
    pub fn resolve(&self) -> Vec<ErrorTraceEntry> {
        self.0.error_trace()
    }
}

pub trait ErrorTraceFallback {
    fn resolve(&self) -> Vec<ErrorTraceEntry>;
}

impl<T> ErrorTraceFallback for ErrorTraceResolver<&T> {
    fn resolve(&self) -> Vec<ErrorTraceEntry> {
        Vec::new()
    }
}

/// Format an error trace as a human-readable multi-line string.
pub fn format_error_trace(trace: &[ErrorTraceEntry]) -> String {
    let mut buf = String::new();
    for (i, entry) in trace.iter().enumerate() {
        if i > 0 {
            buf.push('\n');
        }
        buf.push_str(&format!("  {i}: {}", entry.message));
        buf.push_str(&format!("\n      at {}", entry.location));
    }
    buf
}
