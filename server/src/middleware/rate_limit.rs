use actix_web::{
    body::MessageBody,
    dev::{ServiceRequest, ServiceResponse},
    error::ErrorTooManyRequests,
    middleware::Next,
};
use dashmap::DashMap;
use std::net::IpAddr;
use std::sync::Arc;
use std::time::{Duration, Instant};

const MAX_REQUESTS: usize = 10;
const WINDOW_DURATION: Duration = Duration::from_secs(60);

#[derive(Clone)]
pub struct RateLimiter {
    requests: Arc<DashMap<IpAddr, Vec<Instant>>>,
}

impl RateLimiter {
    pub fn new() -> Self {
        Self {
            requests: Arc::new(DashMap::new()),
        }
    }

    pub fn check_rate_limit(&self, ip: IpAddr) -> bool {
        let now = Instant::now();
        let cutoff = now - WINDOW_DURATION;

        let mut entry = self.requests.entry(ip).or_insert_with(Vec::new);

        // Remove old entries
        entry.retain(|&timestamp| timestamp > cutoff);

        // Check if under limit
        if entry.len() >= MAX_REQUESTS {
            return false;
        }

        // Add current request
        entry.push(now);
        true
    }

    pub fn cleanup_old_entries(&self) {
        let cutoff = Instant::now() - WINDOW_DURATION;

        self.requests.retain(|_, timestamps| {
            timestamps.retain(|&timestamp| timestamp > cutoff);
            !timestamps.is_empty()
        });
    }
}

impl Default for RateLimiter {
    fn default() -> Self {
        Self::new()
    }
}

pub async fn rate_limit_middleware(
    req: ServiceRequest,
    next: Next<impl MessageBody>,
) -> Result<ServiceResponse<impl MessageBody>, actix_web::Error> {
    // Get client IP
    let peer_addr = req
        .peer_addr()
        .ok_or_else(|| ErrorTooManyRequests("Unable to determine client IP"))?;

    let ip = peer_addr.ip();

    // Get RateLimiter from app data
    let rate_limiter = req
        .app_data::<actix_web::web::Data<RateLimiter>>()
        .ok_or_else(|| ErrorTooManyRequests("Rate limiter not available"))?;

    // Check rate limit
    if !rate_limiter.check_rate_limit(ip) {
        log::warn!("Rate limit exceeded for IP: {}", ip);
        return Err(ErrorTooManyRequests("Too many requests"));
    }

    next.call(req).await
}
