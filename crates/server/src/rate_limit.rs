//! Lightweight per-IP throttling for unauthenticated account endpoints.

use axum::extract::{ConnectInfo, Request, State};
use axum::http::header::{HeaderValue, RETRY_AFTER};
use axum::http::StatusCode;
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde_json::json;
use std::collections::HashMap;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

const MAX_TRACKED_CLIENTS: usize = 10_000;

#[derive(Clone, Default)]
pub struct AuthRateLimiter {
    windows: Arc<Mutex<HashMap<(IpAddr, &'static str), FixedWindow>>>,
}

#[derive(Clone, Copy)]
struct FixedWindow {
    started_at: Instant,
    count: u32,
}

#[derive(Clone, Copy)]
struct Policy {
    name: &'static str,
    max_requests: u32,
    window: Duration,
}

pub async fn limit_auth_requests(
    State(limiter): State<AuthRateLimiter>,
    request: Request,
    next: Next,
) -> Response {
    let Some(policy) = policy_for(request.uri().path()) else {
        return next.run(request).await;
    };
    let ip = client_ip(&request);

    match limiter.check(ip, policy, Instant::now()) {
        Ok(()) => next.run(request).await,
        Err(retry_after) => {
            let mut response = (
                StatusCode::TOO_MANY_REQUESTS,
                Json(json!({ "error": "too many requests, please try again later" })),
            )
                .into_response();
            if let Ok(value) = HeaderValue::from_str(&retry_after.as_secs().max(1).to_string()) {
                response.headers_mut().insert(RETRY_AFTER, value);
            }
            response
        }
    }
}

impl AuthRateLimiter {
    fn check(&self, ip: IpAddr, policy: Policy, now: Instant) -> Result<(), Duration> {
        let mut windows = self
            .windows
            .lock()
            .unwrap_or_else(|error| error.into_inner());

        if windows.len() >= MAX_TRACKED_CLIENTS {
            windows.retain(|_, window| {
                now.duration_since(window.started_at) < Duration::from_secs(3600)
            });
        }
        if windows.len() >= MAX_TRACKED_CLIENTS && !windows.contains_key(&(ip, policy.name)) {
            return Err(policy.window);
        }

        let window = windows.entry((ip, policy.name)).or_insert(FixedWindow {
            started_at: now,
            count: 0,
        });
        let elapsed = now.duration_since(window.started_at);
        if elapsed >= policy.window {
            *window = FixedWindow {
                started_at: now,
                count: 0,
            };
        }

        if window.count >= policy.max_requests {
            return Err(policy.window.saturating_sub(elapsed));
        }
        window.count += 1;
        Ok(())
    }
}

fn policy_for(path: &str) -> Option<Policy> {
    match path {
        "/api/auth/login" => Some(Policy {
            name: "login",
            max_requests: 10,
            window: Duration::from_secs(5 * 60),
        }),
        "/api/auth/register" => Some(Policy {
            name: "register",
            max_requests: 5,
            window: Duration::from_secs(60 * 60),
        }),
        _ => None,
    }
}

fn client_ip(request: &Request) -> IpAddr {
    let peer = request
        .extensions()
        .get::<ConnectInfo<SocketAddr>>()
        .map(|connect| connect.0.ip());

    // Only trust forwarded addresses from a loopback or private reverse proxy.
    if peer.is_some_and(is_trusted_proxy) {
        if let Some(forwarded) = request
            .headers()
            .get("x-forwarded-for")
            .and_then(|value| value.to_str().ok())
            .and_then(|value| value.split(',').next())
            .and_then(|value| value.trim().parse().ok())
        {
            return forwarded;
        }
    }

    peer.unwrap_or(IpAddr::V4(Ipv4Addr::UNSPECIFIED))
}

fn is_trusted_proxy(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(ip) => ip.is_private() || ip.is_loopback(),
        IpAddr::V6(ip) => ip.is_loopback() || ip.is_unique_local(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn blocks_after_the_policy_limit_and_recovers_next_window() {
        let limiter = AuthRateLimiter::default();
        let policy = Policy {
            name: "test",
            max_requests: 2,
            window: Duration::from_secs(60),
        };
        let ip = IpAddr::V4(Ipv4Addr::new(203, 0, 113, 10));
        let now = Instant::now();

        assert!(limiter.check(ip, policy, now).is_ok());
        assert!(limiter.check(ip, policy, now).is_ok());
        assert!(limiter.check(ip, policy, now).is_err());
        assert!(limiter
            .check(ip, policy, now + Duration::from_secs(61))
            .is_ok());
    }
}
