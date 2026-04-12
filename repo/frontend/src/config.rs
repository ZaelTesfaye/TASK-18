/// Frontend configuration constants.

/// Base URL for all API requests. Uses a relative path so that
/// nginx (or Trunk's dev-proxy) forwards `/api/*` to the backend.
pub const API_BASE_URL: &str = "/api";
