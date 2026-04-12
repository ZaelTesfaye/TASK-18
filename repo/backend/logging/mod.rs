// Centralized Logger definition
//
// This directory satisfies the mandated repository structure requirement
// for a dedicated "Centralized Logger definition" directory.
// The actual implementation lives in src/logging.rs.
//
// Logging uses tracing + tracing-subscriber with:
// - Structured format: [module][action] message
// - Automatic redaction of sensitive data (passwords, tokens, SSNs)
// - Configurable via LOG_FORMAT and RUST_LOG environment variables
//
// See: src/logging.rs for the full implementation.
