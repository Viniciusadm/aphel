use std::collections::{HashMap, VecDeque};
use std::sync::Mutex;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};

use chrono::{DateTime, Utc};
use sha2::{Digest, Sha256};
use sqlx::{MySqlPool, Row};

use crate::errors::AppError;

const MAX_TRACKED_KEYS: usize = 10_000;
const CLEANUP_EVERY_HITS: u64 = 512;
const CLEANUP_BATCH: i64 = 1_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WindowDecision {
    pub allowed: bool,
    pub limit: u32,
    pub remaining: u32,
    pub reset_after: Duration,
}

impl WindowDecision {
    pub fn rejection(&self, message: &str) -> AppError {
        AppError::rate_limited(message, self.reset_after, Some(self.limit))
    }
}

#[derive(Debug)]
pub struct SlidingWindowLimiter {
    limit: u32,
    window: Duration,
    hits: Mutex<HashMap<String, VecDeque<Instant>>>,
}

impl SlidingWindowLimiter {
    pub fn new(limit: u32, window: Duration) -> Self {
        Self {
            limit: limit.max(1),
            window,
            hits: Mutex::new(HashMap::new()),
        }
    }

    pub fn check(&self, key: &str, message: &str) -> Result<(), AppError> {
        let decision = self.hit(key)?;
        if decision.allowed {
            Ok(())
        } else {
            Err(decision.rejection(message))
        }
    }

    pub fn hit(&self, key: &str) -> Result<WindowDecision, AppError> {
        self.hit_at(key, Instant::now())
    }

    pub fn hit_at(&self, key: &str, now: Instant) -> Result<WindowDecision, AppError> {
        let mut hits = self
            .hits
            .lock()
            .map_err(|_| AppError::Internal("rate limiter lock poisoned".to_owned()))?;
        let window = self.window;
        if hits.len() >= MAX_TRACKED_KEYS {
            hits.retain(|_, entries| {
                entries
                    .back()
                    .is_some_and(|last| now.duration_since(*last) < window)
            });
        }
        let entries = hits.entry(tenant_key(key)).or_default();
        while entries
            .front()
            .is_some_and(|first| now.duration_since(*first) >= window)
        {
            entries.pop_front();
        }
        let allowed = (entries.len() as u32) < self.limit;
        if allowed {
            entries.push_back(now);
        }
        let reset_after = entries
            .front()
            .map(|first| window.saturating_sub(now.duration_since(*first)))
            .unwrap_or_default();
        Ok(WindowDecision {
            allowed,
            limit: self.limit,
            remaining: self.limit.saturating_sub(entries.len() as u32),
            reset_after,
        })
    }
}

fn tenant_key(key: &str) -> String {
    match crate::tenancy::current() {
        Some(tenant) => format!("{}\u{1f}{key}", tenant.tenant().id),
        None => key.to_owned(),
    }
}

#[derive(Debug)]
pub struct SharedRateLimiter {
    scope: &'static str,
    limit: u32,
    window: Duration,
    fallback: SlidingWindowLimiter,
}

impl SharedRateLimiter {
    pub fn new(scope: &'static str, limit: u32, window: Duration) -> Self {
        Self {
            scope,
            limit: limit.max(1),
            window,
            fallback: SlidingWindowLimiter::new(limit, window),
        }
    }

    pub async fn check(&self, key: &str, message: &str) -> Result<(), AppError> {
        let decision = self.hit(key).await?;
        if decision.allowed {
            Ok(())
        } else {
            Err(decision.rejection(message))
        }
    }

    pub async fn hit(&self, key: &str) -> Result<WindowDecision, AppError> {
        shared_hit(self.scope, key, self.limit, self.window, &self.fallback).await
    }
}

pub async fn shared_hit(
    scope: &str,
    key: &str,
    limit: u32,
    window: Duration,
    fallback: &SlidingWindowLimiter,
) -> Result<WindowDecision, AppError> {
    let Some(pool) = shared_pool() else {
        return fallback.hit(key);
    };
    match hit_counter(&pool, &bucket(scope, key), limit.max(1), window, Utc::now()).await {
        Ok(decision) => Ok(decision),
        Err(error) => {
            tracing::warn!(scope, %error, "shared rate limit unavailable, using the process limiter");
            fallback.hit(key)
        }
    }
}

fn shared_pool() -> Option<MySqlPool> {
    crate::state::runtime_state().ok()?.db
}

fn bucket(scope: &str, key: &str) -> String {
    let digest = Sha256::digest(format!("{scope}\u{1f}{key}").as_bytes());
    digest.iter().map(|byte| format!("{byte:02x}")).collect()
}

async fn hit_counter(
    pool: &MySqlPool,
    bucket: &str,
    limit: u32,
    window: Duration,
    now: DateTime<Utc>,
) -> Result<WindowDecision, AppError> {
    let window_ms = i64::try_from(window.as_millis()).unwrap_or(i64::MAX).max(1);
    let now_ms = now.timestamp_millis();
    let current_start = now_ms - now_ms.rem_euclid(window_ms);
    let previous_start = current_start - window_ms;
    let expires_at = DateTime::<Utc>::from_timestamp_millis(
        current_start.saturating_add(window_ms.saturating_mul(2)),
    )
    .unwrap_or(now)
    .naive_utc();

    sqlx::query(
        "INSERT INTO rate_limit_counters (bucket, window_start, hits, expires_at) VALUES (?, ?, 1, ?) \
         ON DUPLICATE KEY UPDATE hits = hits + 1",
    )
    .bind(bucket)
    .bind(current_start)
    .bind(expires_at)
    .execute(pool)
    .await?;
    let rows = sqlx::query(
        "SELECT window_start, hits FROM rate_limit_counters WHERE bucket = ? AND window_start IN (?, ?)",
    )
    .bind(bucket)
    .bind(current_start)
    .bind(previous_start)
    .fetch_all(pool)
    .await?;
    let mut previous = 0.0;
    let mut current = 0.0;
    for row in &rows {
        let start: i64 = row.try_get("window_start")?;
        let hits: u32 = row.try_get("hits")?;
        if start == current_start {
            current = f64::from(hits);
        } else {
            previous = f64::from(hits);
        }
    }

    let counter = SlidingCounter {
        previous,
        current,
        elapsed: (now_ms - current_start) as f64,
        window: window_ms as f64,
    };
    let limit_value = f64::from(limit);
    let allowed = counter.estimate() - 1.0 < limit_value;
    let counter = if allowed {
        counter
    } else {
        sqlx::query(
            "UPDATE rate_limit_counters SET hits = hits - 1 WHERE bucket = ? AND window_start = ? AND hits > 0",
        )
        .bind(bucket)
        .bind(current_start)
        .execute(pool)
        .await?;
        SlidingCounter {
            current: (counter.current - 1.0).max(0.0),
            ..counter
        }
    };
    let estimate = counter.estimate();
    let threshold = if allowed {
        (estimate - 1.0).max(0.0)
    } else {
        limit_value - 1.0
    };
    cleanup_expired(pool, now).await;
    Ok(WindowDecision {
        allowed,
        limit,
        remaining: (limit_value - estimate).floor().clamp(0.0, limit_value) as u32,
        reset_after: Duration::from_millis(counter.wait_until(threshold).ceil() as u64),
    })
}

async fn cleanup_expired(pool: &MySqlPool, now: DateTime<Utc>) {
    static HITS: AtomicU64 = AtomicU64::new(0);
    if !HITS
        .fetch_add(1, Ordering::Relaxed)
        .is_multiple_of(CLEANUP_EVERY_HITS)
    {
        return;
    }
    if let Err(error) = sqlx::query("DELETE FROM rate_limit_counters WHERE expires_at < ? LIMIT ?")
        .bind(now.naive_utc())
        .bind(CLEANUP_BATCH)
        .execute(pool)
        .await
    {
        tracing::debug!(%error, "could not clean expired rate limit counters");
    }
}

#[derive(Debug, Clone, Copy)]
struct SlidingCounter {
    previous: f64,
    current: f64,
    elapsed: f64,
    window: f64,
}

impl SlidingCounter {
    fn estimate(&self) -> f64 {
        self.previous * (self.window - self.elapsed) / self.window + self.current
    }

    fn wait_until(&self, threshold: f64) -> f64 {
        if self.estimate() <= threshold {
            return 0.0;
        }
        let rest_of_window = self.window - self.elapsed;
        if self.current <= threshold && self.previous > 0.0 {
            let wait = rest_of_window - (threshold - self.current) * self.window / self.previous;
            return wait.clamp(0.0, rest_of_window);
        }
        let next_window = if self.current <= threshold {
            0.0
        } else {
            self.window * (1.0 - threshold.max(0.0) / self.current)
        };
        rest_of_window + next_window
    }
}

#[derive(Debug)]
pub struct RateLimits {
    pub login: SharedRateLimiter,
    pub password_reset: SharedRateLimiter,
    pub two_factor: SharedRateLimiter,
    pub password_change: SharedRateLimiter,
    pub sso: SharedRateLimiter,
}

impl Default for RateLimits {
    fn default() -> Self {
        Self {
            login: SharedRateLimiter::new("auth.login", 10, Duration::from_secs(5 * 60)),
            password_reset: SharedRateLimiter::new(
                "auth.password_reset",
                5,
                Duration::from_secs(15 * 60),
            ),
            two_factor: SharedRateLimiter::new("auth.two_factor", 10, Duration::from_secs(15 * 60)),
            sso: SharedRateLimiter::new("auth.sso", 30, Duration::from_secs(5 * 60)),
            password_change: SharedRateLimiter::new(
                "auth.password_change",
                5,
                Duration::from_secs(15 * 60),
            ),
        }
    }
}
