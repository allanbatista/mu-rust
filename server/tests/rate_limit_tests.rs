use server::middleware::RateLimiter;
use std::net::{IpAddr, Ipv4Addr};
use std::thread;
use std::time::Duration;

#[test]
fn test_rate_limiter_creation() {
    let limiter = RateLimiter::new();
    let ip = IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1));

    assert!(limiter.check_rate_limit(ip));
}

#[test]
fn test_rate_limit_allows_under_limit() {
    let limiter = RateLimiter::new();
    let ip = IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1));

    // Send 10 requests (the limit)
    for _ in 0..10 {
        assert!(
            limiter.check_rate_limit(ip),
            "Should allow requests under limit"
        );
    }
}

#[test]
fn test_rate_limit_blocks_over_limit() {
    let limiter = RateLimiter::new();
    let ip = IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1));

    // Send 10 requests (the limit)
    for _ in 0..10 {
        limiter.check_rate_limit(ip);
    }

    // 11th request should be blocked
    assert!(
        !limiter.check_rate_limit(ip),
        "Should block requests over limit"
    );
}

#[test]
fn test_rate_limit_per_ip() {
    let limiter = RateLimiter::new();
    let ip1 = IpAddr::V4(Ipv4Addr::new(192, 168, 1, 1));
    let ip2 = IpAddr::V4(Ipv4Addr::new(192, 168, 1, 2));

    // Exhaust limit for IP1
    for _ in 0..10 {
        limiter.check_rate_limit(ip1);
    }

    // IP1 should be blocked
    assert!(!limiter.check_rate_limit(ip1));

    // IP2 should still be allowed
    assert!(limiter.check_rate_limit(ip2));
}

#[test]
fn test_cleanup_old_entries() {
    let limiter = RateLimiter::new();
    let ip = IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1));

    // Make some requests
    for _ in 0..5 {
        limiter.check_rate_limit(ip);
    }

    // Cleanup (this won't remove anything since they're recent)
    limiter.cleanup_old_entries();

    // Should still have requests tracked
    assert!(limiter.check_rate_limit(ip));
}

#[test]
fn test_window_expiration() {
    let limiter = RateLimiter::new();
    let ip = IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1));

    // Fill the limit
    for _ in 0..10 {
        limiter.check_rate_limit(ip);
    }

    // Should be blocked
    assert!(!limiter.check_rate_limit(ip));

    // Note: Can't actually test expiration without waiting 60s or mocking time
    // This test just verifies the mechanism exists
}

#[test]
fn test_concurrent_rate_limiting() {
    use std::sync::Arc;

    let limiter = Arc::new(RateLimiter::new());
    let ip = IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1));
    let mut handles = vec![];

    // Spawn multiple threads making requests
    for _ in 0..5 {
        let limiter_clone = Arc::clone(&limiter);
        let handle = thread::spawn(move || {
            for _ in 0..3 {
                limiter_clone.check_rate_limit(ip);
                thread::sleep(Duration::from_millis(1));
            }
        });
        handles.push(handle);
    }

    for handle in handles {
        handle.join().unwrap();
    }

    // After 15 requests (5 threads * 3 requests), should be over limit
    assert!(!limiter.check_rate_limit(ip));
}

#[test]
fn test_default_implementation() {
    let limiter = RateLimiter::default();
    let ip = IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1));

    assert!(limiter.check_rate_limit(ip));
}

#[test]
fn test_multiple_ips_independent_limits() {
    let limiter = RateLimiter::new();

    // Create 5 different IPs
    for i in 1..=5 {
        let ip = IpAddr::V4(Ipv4Addr::new(192, 168, 1, i));

        // Each should get their own quota of 10
        for _ in 0..10 {
            assert!(
                limiter.check_rate_limit(ip),
                "IP {}.{}.{}.{} should have independent limit",
                192,
                168,
                1,
                i
            );
        }

        // Each should be blocked after 10
        assert!(
            !limiter.check_rate_limit(ip),
            "IP {}.{}.{}.{} should be blocked after limit",
            192,
            168,
            1,
            i
        );
    }
}
