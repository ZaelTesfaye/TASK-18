use silverscreen_backend::middleware::rate_limit::RateLimiter;

#[test]
fn test_rate_limiter_allows_under_threshold() {
    let limiter = RateLimiter::new();

    // Record 5 attempts; limit is 10.
    for _ in 0..5 {
        limiter.record_attempt("user:alice");
    }

    assert!(limiter.check_rate_limit("user:alice", 10, 900).is_ok());
}

#[test]
fn test_rate_limiter_blocks_at_threshold() {
    let limiter = RateLimiter::new();

    // Record exactly 10 attempts; limit is 10.
    for _ in 0..10 {
        limiter.record_attempt("user:bob");
    }

    let result = limiter.check_rate_limit("user:bob", 10, 900);
    assert!(result.is_err());
}

#[test]
fn test_rate_limiter_blocks_over_threshold() {
    let limiter = RateLimiter::new();

    for _ in 0..15 {
        limiter.record_attempt("user:charlie");
    }

    let result = limiter.check_rate_limit("user:charlie", 10, 900);
    assert!(result.is_err());
}

#[test]
fn test_rate_limiter_separate_keys() {
    let limiter = RateLimiter::new();

    for _ in 0..10 {
        limiter.record_attempt("user:alice");
    }

    // Bob should still be under the limit.
    assert!(limiter.check_rate_limit("user:bob", 10, 900).is_ok());
    // Alice is at the limit.
    assert!(limiter.check_rate_limit("user:alice", 10, 900).is_err());
}

#[test]
fn test_rate_limiter_first_check_passes() {
    let limiter = RateLimiter::new();
    // No attempts recorded.
    assert!(limiter.check_rate_limit("user:new", 10, 900).is_ok());
}

#[test]
fn test_rate_limiter_ip_key() {
    let limiter = RateLimiter::new();

    for _ in 0..30 {
        limiter.record_attempt("ip:192.168.1.1");
    }

    let result = limiter.check_rate_limit("ip:192.168.1.1", 30, 900);
    assert!(result.is_err());
}

#[test]
fn test_rate_limiter_combo_key() {
    let limiter = RateLimiter::new();

    for _ in 0..10 {
        limiter.record_attempt("combo:alice:192.168.1.1");
    }

    let result = limiter.check_rate_limit("combo:alice:192.168.1.1", 10, 900);
    assert!(result.is_err());
}

#[test]
fn test_cleanup_does_not_panic() {
    let limiter = RateLimiter::new();
    limiter.record_attempt("user:test");
    limiter.cleanup_expired(900); // Should not panic.
}
