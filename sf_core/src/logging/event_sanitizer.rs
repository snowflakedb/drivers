use std::borrow::Cow;
use std::cell::RefCell;
use std::collections::HashSet;
use std::fmt;

use tracing::field::{Field, Visit};
use tracing::{Event, Subscriber};
use tracing_subscriber::Layer;
use tracing_subscriber::layer::Context;

/// Placeholder text used to replace redacted content.
pub const REDACTED: &str = "****";

const SENSITIVE_FIELD_NAMES: &[&str] = &[
    "password",
    "pwd",
    "token",
    "passcode",
    "private_key",
    "priv_key",
    "secret",
    "authorization",
];

/// Detects and masks sensitive data in log field names and values.
///
/// Field names are matched against a case-insensitive blocklist. String values
/// are scanned for known sensitive patterns (JWT tokens, PEM-encoded keys).
#[derive(Clone, Debug)]
pub struct Sanitizer {
    sensitive_names: HashSet<String>,
}

impl Default for Sanitizer {
    fn default() -> Self {
        Self::new()
    }
}

impl Sanitizer {
    pub fn new() -> Self {
        Self {
            sensitive_names: SENSITIVE_FIELD_NAMES
                .iter()
                .map(|s| (*s).to_string())
                .collect(),
        }
    }

    /// Returns `true` if the field name matches the sensitive-name blocklist
    /// (case-insensitive).
    pub fn is_sensitive_field(&self, name: &str) -> bool {
        self.sensitive_names.contains(&name.to_ascii_lowercase())
    }

    /// Returns `true` if the value contains a known sensitive pattern
    /// (JWT tokens, PEM keys). Also handles Debug-formatted strings that may
    /// be wrapped in quotes.
    pub fn has_sensitive_pattern(&self, value: &str) -> bool {
        if check_patterns(value) {
            return true;
        }
        if let Some(unquoted) = value.strip_prefix('"').and_then(|s| s.strip_suffix('"')) {
            return check_patterns(unquoted);
        }
        false
    }

    /// Returns [`REDACTED`] if the field name or value is sensitive, or the
    /// original value otherwise.
    pub fn sanitize<'a>(&self, field_name: &str, value: &'a str) -> Cow<'a, str> {
        if self.is_sensitive_field(field_name) || self.has_sensitive_pattern(value) {
            Cow::Borrowed(REDACTED)
        } else {
            Cow::Borrowed(value)
        }
    }
}

fn check_patterns(value: &str) -> bool {
    looks_like_jwt(value) || looks_like_pem_key(value)
}

/// JWT: three dot-separated, non-empty base64url segments where the header
/// starts with `eyJ` (base64url encoding of `{"`).
fn looks_like_jwt(s: &str) -> bool {
    let s = s.trim();
    if !s.starts_with("eyJ") {
        return false;
    }
    let mut parts = s.splitn(4, '.');
    parts.next().is_some_and(|p| !p.is_empty())
        && parts.next().is_some_and(|p| !p.is_empty())
        && parts.next().is_some_and(|p| !p.is_empty())
        && parts.next().is_none()
}

/// PEM-encoded key material: `-----BEGIN ... KEY-----`.
fn looks_like_pem_key(s: &str) -> bool {
    s.contains("-----BEGIN") && s.contains("KEY-----")
}

// ---------------------------------------------------------------------------
// SanitizingVisitor: wraps another Visit impl, redacting sensitive values
// ---------------------------------------------------------------------------

struct RedactedValue;

impl fmt::Debug for RedactedValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(REDACTED)
    }
}

/// A [`Visit`] adapter that interposes sanitization before forwarding values
/// to an inner visitor. Use this to transparently redact sensitive fields in
/// any layer that consumes events via the `Visit` trait.
pub struct SanitizingVisitor<'a, V> {
    sanitizer: &'a Sanitizer,
    inner: &'a mut V,
}

impl<'a, V> SanitizingVisitor<'a, V> {
    pub fn new(sanitizer: &'a Sanitizer, inner: &'a mut V) -> Self {
        Self { sanitizer, inner }
    }
}

impl<V: Visit> Visit for SanitizingVisitor<'_, V> {
    fn record_str(&mut self, field: &Field, value: &str) {
        if self.sanitizer.is_sensitive_field(field.name())
            || self.sanitizer.has_sensitive_pattern(value)
        {
            self.inner.record_str(field, REDACTED);
        } else {
            self.inner.record_str(field, value);
        }
    }

    fn record_debug(&mut self, field: &Field, value: &dyn fmt::Debug) {
        if self.sanitizer.is_sensitive_field(field.name()) {
            self.inner.record_debug(field, &RedactedValue);
            return;
        }
        let formatted = format!("{value:?}");
        if self.sanitizer.has_sensitive_pattern(&formatted) {
            self.inner.record_debug(field, &RedactedValue);
        } else {
            self.inner.record_debug(field, value);
        }
    }

    fn record_f64(&mut self, field: &Field, value: f64) {
        self.inner.record_f64(field, value);
    }

    fn record_i64(&mut self, field: &Field, value: i64) {
        self.inner.record_i64(field, value);
    }

    fn record_u64(&mut self, field: &Field, value: u64) {
        self.inner.record_u64(field, value);
    }

    fn record_bool(&mut self, field: &Field, value: bool) {
        self.inner.record_bool(field, value);
    }
}

// ---------------------------------------------------------------------------
// EventSanitizerLayer: outermost tracing-subscriber layer for sanitization
// ---------------------------------------------------------------------------

thread_local! {
    static CURRENT_SANITIZED_EVENT: RefCell<Option<SanitizedEvent>> = const { RefCell::new(None) };
}

/// Sanitized snapshot of a tracing event's fields, produced by
/// [`EventSanitizerLayer`].
#[derive(Debug, Clone)]
pub struct SanitizedEvent {
    fields: Vec<(String, String)>,
}

impl SanitizedEvent {
    /// Returns the sanitized fields as `(name, value)` pairs.
    pub fn fields(&self) -> &[(String, String)] {
        &self.fields
    }

    /// Formats the sanitized fields in the standard tracing style:
    /// `message field1=value1 field2=value2`.
    pub fn format_message(&self) -> String {
        let mut out = String::new();
        for (name, value) in &self.fields {
            if name == "message" {
                out.push_str(value);
            } else {
                if !out.is_empty() {
                    out.push(' ');
                }
                out.push_str(name);
                out.push('=');
                out.push_str(value);
            }
        }
        out
    }
}

/// A [`tracing_subscriber::Layer`] that intercepts events and stores sanitized
/// field data in thread-local storage before downstream layers process them.
///
/// Register this as the **outermost** layer (last `.with()` call) so that its
/// `on_event` executes before all other layers. Downstream layers can then
/// call [`with_sanitized_event`] to retrieve the redacted fields instead of
/// reading the raw event directly.
#[derive(Clone, Debug)]
pub struct EventSanitizerLayer {
    sanitizer: Sanitizer,
}

impl EventSanitizerLayer {
    pub fn new() -> Self {
        Self {
            sanitizer: Sanitizer::new(),
        }
    }

    pub fn sanitizer(&self) -> &Sanitizer {
        &self.sanitizer
    }
}

impl Default for EventSanitizerLayer {
    fn default() -> Self {
        Self::new()
    }
}

impl<S: Subscriber> Layer<S> for EventSanitizerLayer {
    fn on_event(&self, event: &Event<'_>, _ctx: Context<'_, S>) {
        let mut fields = Vec::new();
        let mut collector = FieldCollector {
            sanitizer: &self.sanitizer,
            fields: &mut fields,
        };
        event.record(&mut collector);
        CURRENT_SANITIZED_EVENT.with(|cell| {
            *cell.borrow_mut() = Some(SanitizedEvent { fields });
        });
    }
}

/// Retrieves the [`SanitizedEvent`] stored by the most recent
/// [`EventSanitizerLayer::on_event`] call on the current thread.
pub fn with_sanitized_event<R>(f: impl FnOnce(&SanitizedEvent) -> R) -> Option<R> {
    CURRENT_SANITIZED_EVENT.with(|cell| cell.borrow().as_ref().map(f))
}

/// Internal visitor that collects and sanitizes event fields.
struct FieldCollector<'a> {
    sanitizer: &'a Sanitizer,
    fields: &'a mut Vec<(String, String)>,
}

impl Visit for FieldCollector<'_> {
    fn record_str(&mut self, field: &Field, value: &str) {
        let sanitized = self.sanitizer.sanitize(field.name(), value);
        self.fields
            .push((field.name().to_string(), sanitized.into_owned()));
    }

    fn record_debug(&mut self, field: &Field, value: &dyn fmt::Debug) {
        if self.sanitizer.is_sensitive_field(field.name()) {
            self.fields
                .push((field.name().to_string(), REDACTED.to_string()));
            return;
        }
        let formatted = format!("{value:?}");
        if self.sanitizer.has_sensitive_pattern(&formatted) {
            self.fields
                .push((field.name().to_string(), REDACTED.to_string()));
        } else {
            self.fields.push((field.name().to_string(), formatted));
        }
    }

    fn record_f64(&mut self, field: &Field, value: f64) {
        self.fields
            .push((field.name().to_string(), value.to_string()));
    }

    fn record_i64(&mut self, field: &Field, value: i64) {
        self.fields
            .push((field.name().to_string(), value.to_string()));
    }

    fn record_u64(&mut self, field: &Field, value: u64) {
        self.fields
            .push((field.name().to_string(), value.to_string()));
    }

    fn record_bool(&mut self, field: &Field, value: bool) {
        self.fields
            .push((field.name().to_string(), value.to_string()));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tracing_subscriber::layer::SubscriberExt;

    // -- Sanitizer: field-name detection -------------------------------------

    #[test]
    fn sensitive_field_names_are_detected() {
        let s = Sanitizer::new();
        for name in SENSITIVE_FIELD_NAMES {
            assert!(
                s.is_sensitive_field(name),
                "expected '{name}' to be sensitive"
            );
        }
    }

    #[test]
    fn sensitive_field_names_are_case_insensitive() {
        let s = Sanitizer::new();
        assert!(s.is_sensitive_field("PASSWORD"));
        assert!(s.is_sensitive_field("Token"));
        assert!(s.is_sensitive_field("PRIVATE_KEY"));
        assert!(s.is_sensitive_field("Authorization"));
    }

    #[test]
    fn non_sensitive_field_names_pass_through() {
        let s = Sanitizer::new();
        assert!(!s.is_sensitive_field("username"));
        assert!(!s.is_sensitive_field("host"));
        assert!(!s.is_sensitive_field("query_id"));
    }

    // -- Sanitizer: value-pattern detection ----------------------------------

    #[test]
    fn jwt_tokens_are_detected() {
        let s = Sanitizer::new();
        let jwt = "eyJhbGciOiJIUzI1NiJ9.eyJpc3MiOiJ0ZXN0In0.abc123signature";
        assert!(s.has_sensitive_pattern(jwt));
    }

    #[test]
    fn jwt_detection_requires_exactly_three_parts() {
        let s = Sanitizer::new();
        assert!(!s.has_sensitive_pattern("eyJhbGci"));
        assert!(!s.has_sensitive_pattern("eyJhbGci.part2"));
        assert!(!s.has_sensitive_pattern("eyJhbGci.part2.part3.part4"));
    }

    #[test]
    fn quoted_jwt_is_detected() {
        let s = Sanitizer::new();
        let quoted = "\"eyJhbGciOiJIUzI1NiJ9.eyJpc3MiOiJ0ZXN0In0.abc123\"";
        assert!(s.has_sensitive_pattern(quoted));
    }

    #[test]
    fn pem_keys_are_detected() {
        let s = Sanitizer::new();
        assert!(s.has_sensitive_pattern("-----BEGIN PRIVATE KEY-----\nMIIEvgIBA..."));
        assert!(s.has_sensitive_pattern("-----BEGIN RSA PRIVATE KEY-----\nMIIBogIB..."));
        assert!(s.has_sensitive_pattern("-----BEGIN EC PRIVATE KEY-----\nMHQCAQEE..."));
        assert!(s.has_sensitive_pattern("-----BEGIN ENCRYPTED PRIVATE KEY-----\nMIIE..."));
        assert!(s.has_sensitive_pattern("-----BEGIN PUBLIC KEY-----\nMIIBIjAN..."));
    }

    #[test]
    fn normal_values_are_not_flagged() {
        let s = Sanitizer::new();
        assert!(!s.has_sensitive_pattern("hello world"));
        assert!(!s.has_sensitive_pattern("SELECT 1"));
        assert!(!s.has_sensitive_pattern("192.168.1.1"));
        assert!(!s.has_sensitive_pattern("eyJ")); // too short, no dots
    }

    // -- Sanitizer::sanitize -------------------------------------------------

    #[test]
    fn sanitize_redacts_sensitive_field() {
        let s = Sanitizer::new();
        assert_eq!(s.sanitize("password", "hunter2").as_ref(), REDACTED);
    }

    #[test]
    fn sanitize_redacts_jwt_in_non_sensitive_field() {
        let s = Sanitizer::new();
        let jwt = "eyJhbGciOiJIUzI1NiJ9.eyJpc3MiOiJ0ZXN0In0.sig";
        assert_eq!(s.sanitize("auth_header", jwt).as_ref(), REDACTED);
    }

    #[test]
    fn sanitize_passes_through_normal_values() {
        let s = Sanitizer::new();
        let result = s.sanitize("host", "example.com");
        assert_eq!(result.as_ref(), "example.com");
    }

    // -- SanitizingVisitor (tested via a custom collecting layer) ------------

    /// Helper layer that uses [`SanitizingVisitor`] to collect fields into
    /// shared storage, letting us verify the wrapper independently.
    struct SanitizingCollectorLayer {
        sanitizer: Sanitizer,
        collected: std::sync::Arc<std::sync::Mutex<Vec<(String, String)>>>,
    }

    impl<S: Subscriber> Layer<S> for SanitizingCollectorLayer {
        fn on_event(&self, event: &Event<'_>, _ctx: Context<'_, S>) {
            let mut inner = TestVisitor { values: Vec::new() };
            let mut visitor = SanitizingVisitor::new(&self.sanitizer, &mut inner);
            event.record(&mut visitor);
            self.collected.lock().unwrap().extend(inner.values);
        }
    }

    struct TestVisitor {
        values: Vec<(String, String)>,
    }

    impl Visit for TestVisitor {
        fn record_str(&mut self, field: &Field, value: &str) {
            self.values
                .push((field.name().to_string(), value.to_string()));
        }

        fn record_debug(&mut self, field: &Field, value: &dyn fmt::Debug) {
            self.values
                .push((field.name().to_string(), format!("{value:?}")));
        }
    }

    #[test]
    fn sanitizing_visitor_redacts_sensitive_str_field() {
        let collected = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
        let layer = SanitizingCollectorLayer {
            sanitizer: Sanitizer::new(),
            collected: collected.clone(),
        };
        let subscriber = tracing_subscriber::Registry::default().with(layer);

        tracing::subscriber::with_default(subscriber, || {
            tracing::info!(password = "hunter2", "login attempt");
        });

        let fields = collected.lock().unwrap();
        let pw = fields.iter().find(|(k, _)| k == "password");
        assert_eq!(pw.unwrap().1, REDACTED);
    }

    #[test]
    fn sanitizing_visitor_passes_through_normal_str_field() {
        let collected = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
        let layer = SanitizingCollectorLayer {
            sanitizer: Sanitizer::new(),
            collected: collected.clone(),
        };
        let subscriber = tracing_subscriber::Registry::default().with(layer);

        tracing::subscriber::with_default(subscriber, || {
            tracing::info!(host = "example.com", "connecting");
        });

        let fields = collected.lock().unwrap();
        let host = fields.iter().find(|(k, _)| k == "host");
        assert_eq!(host.unwrap().1, "example.com");
    }

    // -- EventSanitizerLayer integration ----------------------------------------

    #[test]
    fn event_sanitizer_layer_redacts_sensitive_fields() {
        let layer = EventSanitizerLayer::new();
        let subscriber = tracing_subscriber::Registry::default().with(layer);

        tracing::subscriber::with_default(subscriber, || {
            tracing::info!(password = "hunter2", host = "example.com", "connecting");

            let event =
                with_sanitized_event(|e| e.clone()).expect("sanitized event should be present");

            let pw = event.fields().iter().find(|(k, _)| k == "password");
            assert_eq!(pw.unwrap().1, REDACTED);

            let host = event.fields().iter().find(|(k, _)| k == "host");
            assert_ne!(host.unwrap().1, REDACTED);
        });
    }

    #[test]
    fn event_sanitizer_layer_redacts_jwt_in_values() {
        let layer = EventSanitizerLayer::new();
        let subscriber = tracing_subscriber::Registry::default().with(layer);

        tracing::subscriber::with_default(subscriber, || {
            let jwt = "eyJhbGciOiJIUzI1NiJ9.eyJpc3MiOiJ0ZXN0In0.signature";
            tracing::info!(auth_value = jwt, "auth event");

            let event =
                with_sanitized_event(|e| e.clone()).expect("sanitized event should be present");

            let auth = event.fields().iter().find(|(k, _)| k == "auth_value");
            assert_eq!(auth.unwrap().1, REDACTED);
        });
    }

    #[test]
    fn event_sanitizer_layer_preserves_numeric_fields() {
        let layer = EventSanitizerLayer::new();
        let subscriber = tracing_subscriber::Registry::default().with(layer);

        tracing::subscriber::with_default(subscriber, || {
            tracing::info!(count = 42u64, ratio = 2.72f64, flag = true, "metrics");

            let event =
                with_sanitized_event(|e| e.clone()).expect("sanitized event should be present");

            let count = event.fields().iter().find(|(k, _)| k == "count");
            assert_eq!(count.unwrap().1, "42");

            let ratio = event.fields().iter().find(|(k, _)| k == "ratio");
            assert_eq!(ratio.unwrap().1, "2.72");

            let flag = event.fields().iter().find(|(k, _)| k == "flag");
            assert_eq!(flag.unwrap().1, "true");
        });
    }

    // -- SanitizedEvent::format_message --------------------------------------

    #[test]
    fn sanitized_event_format_message() {
        let event = SanitizedEvent {
            fields: vec![
                ("message".to_string(), "hello".to_string()),
                ("user".to_string(), "alice".to_string()),
                ("password".to_string(), REDACTED.to_string()),
            ],
        };
        assert_eq!(event.format_message(), "hello user=alice password=****");
    }

    #[test]
    fn sanitized_event_format_message_without_message_field() {
        let event = SanitizedEvent {
            fields: vec![
                ("host".to_string(), "example.com".to_string()),
                ("port".to_string(), "443".to_string()),
            ],
        };
        assert_eq!(event.format_message(), "host=example.com port=443");
    }
}
