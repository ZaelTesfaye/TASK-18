use chrono::{DateTime, Utc};
use dashmap::DashMap;

use crate::config::Config;
use crate::errors::AppError;

// ---------------------------------------------------------------------------
// RateLimiter
// ---------------------------------------------------------------------------

/// In-memory, thread-safe sliding-window rate limiter backed by [`DashMap`].
///
/// Each key maps to a `Vec` of timestamps representing recorded attempts.
/// [`check_rate_limit`] counts how many of those timestamps fall within the
/// current window and rejects the request when the limit is exceeded.
pub struct RateLimiter {
    attempts: DashMap<String, Vec<DateTime<Utc>>>,
}

impl RateLimiter {
    /// Creates a new, empty rate limiter.
    pub fn new() -> Self {
        Self {
            attempts: DashMap::new(),
        }
    }

    /// Checks whether `key` has exceeded `max_attempts` within the last
    /// `window_seconds`. Returns `Ok(())` when the request is allowed or
    /// `Err(AppError::RateLimited)` when the caller should back off.
    ///
    /// Stale entries older than the window are pruned during the check.
    pub fn check_rate_limit(
        &self,
        key: &str,
        max_attempts: u32,
        window_seconds: u64,
    ) -> Result<(), AppError> {
        let window_start =
            Utc::now() - chrono::Duration::seconds(window_seconds as i64);

        let count = {
            let mut entry = self
                .attempts
                .entry(key.to_string())
                .or_insert_with(Vec::new);

            // Prune timestamps that have fallen out of the window.
            entry.retain(|ts| *ts > window_start);
            entry.len()
        };

        if count >= max_attempts as usize {
            tracing::warn!(
                key = %key,
                count = count,
                max = max_attempts,
                window_seconds = window_seconds,
                "Rate limit exceeded"
            );
            return Err(AppError::RateLimited(format!(
                "Too many attempts. Try again in {} seconds.",
                window_seconds
            )));
        }

        Ok(())
    }

    /// Records a new attempt for the given key at the current time.
    pub fn record_attempt(&self, key: &str) {
        self.attempts
            .entry(key.to_string())
            .or_insert_with(Vec::new)
            .push(Utc::now());
    }

    /// Removes all recorded timestamps older than `window_seconds` across
    /// **every** key. Empty keys are removed entirely to reclaim memory.
    ///
    /// Call this periodically (e.g. from a background task) to prevent
    /// unbounded growth.
    pub fn cleanup_expired(&self, window_seconds: u64) {
        let window_start =
            Utc::now() - chrono::Duration::seconds(window_seconds as i64);

        // Collect keys that become empty so we can remove them outside the
        // iterator to avoid holding shard locks longer than necessary.
        let empty_keys: Vec<String> = self
            .attempts
            .iter_mut()
            .filter_map(|mut entry| {
                entry.value_mut().retain(|ts| *ts > window_start);
                if entry.value().is_empty() {
                    Some(entry.key().clone())
                } else {
                    None
                }
            })
            .collect();

        for key in empty_keys {
            // Re-check under the write lock in case a new attempt was recorded
            // between the iterator and this removal.
            self.attempts.remove_if(&key, |_, v| v.is_empty());
        }
    }
}

impl Default for RateLimiter {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// Higher-level login rate limiting
// ---------------------------------------------------------------------------

/// Checks three rate-limiting dimensions for a login attempt:
///
/// 1. **Per-username** -- prevents brute-forcing a single account.
/// 2. **Per-IP** -- prevents credential-stuffing from one source (hard-capped
///    at 30 attempts per window).
/// 3. **Per-username+IP** -- tighter per-pair limit using the configured max.
///
/// If **any** dimension exceeds its limit, returns `Err(AppError::RateLimited)`
/// with a Retry-After hint. The caller is responsible for calling
/// [`RateLimiter::record_attempt`] on all three keys **after** this check
/// passes.
pub fn check_login_rate_limits(
    limiter: &RateLimiter,
    username: &str,
    ip: &str,
    config: &Config,
) -> Result<(), AppError> {
    let window = config.rate_limit_login_window_seconds;
    let max = config.rate_limit_login_max;

    // Dimension 1: per-username
    let user_key = format!("login:user:{}", username);
    limiter.check_rate_limit(&user_key, max, window)?;

    // Dimension 2: per-IP (configurable, default 10 to match spec)
    let ip_key = format!("login:ip:{}", ip);
    limiter.check_rate_limit(&ip_key, config.rate_limit_login_ip_max, window)?;

    // Dimension 3: per-username+IP combination
    let combo_key = format!("login:combo:{}:{}", username, ip);
    limiter.check_rate_limit(&combo_key, max, window)?;

    Ok(())
}

/// Convenience helper: records an attempt across all three login rate-limit
/// dimensions so that callers don't have to repeat the key-formatting logic.
pub fn record_login_attempt(limiter: &RateLimiter, username: &str, ip: &str) {
    limiter.record_attempt(&format!("login:user:{}", username));
    limiter.record_attempt(&format!("login:ip:{}", ip));
    limiter.record_attempt(&format!("login:combo:{}:{}", username, ip));
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn allows_requests_within_limit() {
        let rl = RateLimiter::new();
        for _ in 0..5 {
            rl.record_attempt("key");
        }
        assert!(rl.check_rate_limit("key", 10, 60).is_ok());
    }

    #[test]
    fn blocks_requests_exceeding_limit() {
        let rl = RateLimiter::new();
        for _ in 0..10 {
            rl.record_attempt("key");
        }
        assert!(rl.check_rate_limit("key", 10, 60).is_err());
    }

    #[test]
    fn cleanup_removes_stale_entries() {
        let rl = RateLimiter::new();
        // Insert an entry with a manually backdated timestamp.
        let old = Utc::now() - chrono::Duration::seconds(120);
        rl.attempts
            .entry("stale".to_string())
            .or_insert_with(Vec::new)
            .push(old);

        rl.cleanup_expired(60);
        assert!(
            rl.attempts.get("stale").is_none(),
            "Stale key should have been removed"
        );
    }
}
