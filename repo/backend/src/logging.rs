use std::io;
use tracing_subscriber::{
    fmt, layer::SubscriberExt, util::SubscriberInitExt, EnvFilter,
};

use crate::config::Config;

/// Initializes the global tracing subscriber with structured logging,
/// environment-based filtering, and automatic redaction of sensitive values.
pub fn init_logging() {
    let config = Config::get();

    let env_filter = EnvFilter::try_new(&config.rust_log)
        .unwrap_or_else(|_| EnvFilter::new("info"));

    let registry = tracing_subscriber::registry().with(env_filter);

    let make_writer = || RedactingWriter {
        inner: io::stdout(),
    };

    if config.log_format == "json" {
        let layer = fmt::layer()
            .json()
            .with_target(true)
            .with_thread_ids(false)
            .with_thread_names(false)
            .with_writer(make_writer);
        registry.with(layer).init();
    } else {
        let layer = fmt::layer()
            .with_target(true)
            .with_thread_ids(false)
            .with_thread_names(false)
            .with_writer(make_writer);
        registry.with(layer).init();
    }
}

/// A writer that intercepts log output and scrubs sensitive patterns
/// before forwarding to the underlying writer.
struct RedactingWriter<W: io::Write> {
    inner: W,
}

impl<W: io::Write> io::Write for RedactingWriter<W> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let original_len = buf.len();
        let input = String::from_utf8_lossy(buf);
        let redacted = redact_sensitive(&input);
        self.inner.write_all(redacted.as_bytes())?;
        // Return original length so tracing-subscriber thinks all bytes were consumed.
        Ok(original_len)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.inner.flush()
    }
}

/// Redacts common sensitive patterns from log output.
///
/// Patterns handled:
/// - `"password":"..."` / `"password_hash":"..."` / `"secret":"..."` / `"token":"..."`
/// - Bearer tokens in Authorization headers
/// - SSN-like patterns (###-##-####)
/// - JWT-shaped strings (three base64url segments separated by dots)
fn redact_sensitive(input: &str) -> String {
    use once_cell::sync::Lazy;
    use regex::Regex;

    // Password / secret fields in JSON-like output
    static RE_PASSWORD: Lazy<Regex> = Lazy::new(|| {
        Regex::new(r#"(?i)("(?:password|password_hash|secret|token)")\s*:\s*"[^"]*""#).unwrap()
    });

    // Bearer tokens
    static RE_BEARER: Lazy<Regex> =
        Lazy::new(|| Regex::new(r"(?i)Bearer\s+[A-Za-z0-9\-_\.]+").unwrap());

    // US Social Security Numbers
    static RE_SSN: Lazy<Regex> =
        Lazy::new(|| Regex::new(r"\b\d{3}-\d{2}-\d{4}\b").unwrap());

    // JWT-shaped tokens (header.payload.signature, all base64url)
    static RE_JWT: Lazy<Regex> = Lazy::new(|| {
        Regex::new(r"\beyJ[A-Za-z0-9\-_]+\.[A-Za-z0-9\-_]+\.[A-Za-z0-9\-_]+\b").unwrap()
    });

    let out = RE_PASSWORD.replace_all(input, r#"$1:"[REDACTED]""#);
    let out = RE_BEARER.replace_all(&out, "Bearer [REDACTED]");
    let out = RE_SSN.replace_all(&out, "[SSN REDACTED]");
    let out = RE_JWT.replace_all(&out, "[JWT REDACTED]");

    out.into_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_redact_password() {
        let input = r#"{"password":"mysecret123"}"#;
        let result = redact_sensitive(input);
        assert!(!result.contains("mysecret123"));
        assert!(result.contains("[REDACTED]"));
    }

    #[test]
    fn test_redact_bearer() {
        let input = "Authorization: Bearer eyJhbGciOiJIUzI1NiJ9.payload.sig";
        let result = redact_sensitive(input);
        assert!(!result.contains("eyJ"));
        assert!(result.contains("[REDACTED]"));
    }

    #[test]
    fn test_redact_ssn() {
        let input = "SSN is 123-45-6789 in the record";
        let result = redact_sensitive(input);
        assert!(!result.contains("123-45-6789"));
        assert!(result.contains("[SSN REDACTED]"));
    }
}
