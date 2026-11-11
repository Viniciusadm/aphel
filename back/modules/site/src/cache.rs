use std::collections::{HashMap, VecDeque};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use aphel_kernel::errors::AppError;
use sha2::{Digest, Sha256};
use sqlx::{MySqlPool, Row};

pub(crate) const ALL_TAG: &str = "*";
const CHECK_INTERVAL: Duration = Duration::from_secs(2);
const ENTRY_LIFETIME: Duration = Duration::from_secs(600);
const MAX_BYTES: usize = 32 * 1024 * 1024;
const INVALIDATION_BATCH: i64 = 1000;

#[derive(Clone)]
pub(crate) struct CachedPage {
    pub html: Arc<str>,
    pub etag: String,
}

struct Entry {
    page: CachedPage,
    tags: Vec<String>,
    stored_at: Instant,
}

#[derive(Default)]
struct PageCache {
    entries: HashMap<String, Entry>,
    order: VecDeque<String>,
    bytes: usize,
    last_invalidation: Option<u64>,
    checked_at: Option<Instant>,
}

impl PageCache {
    fn remove(&mut self, key: &str) {
        if let Some(entry) = self.entries.remove(key) {
            self.bytes = self.bytes.saturating_sub(entry.page.html.len());
        }
    }

    fn purge(&mut self, tag: &str) {
        if tag == ALL_TAG {
            self.entries.clear();
            self.order.clear();
            self.bytes = 0;
            return;
        }
        let keys: Vec<String> = self
            .entries
            .iter()
            .filter(|(_, entry)| entry.tags.iter().any(|candidate| candidate == tag))
            .map(|(key, _)| key.clone())
            .collect();
        for key in keys {
            self.remove(&key);
        }
    }

    fn evict(&mut self) {
        while self.bytes > MAX_BYTES {
            let Some(key) = self.order.pop_front() else {
                break;
            };
            self.remove(&key);
        }
    }
}

static CACHE: Mutex<Option<PageCache>> = Mutex::new(None);

fn with_cache<T>(apply: impl FnOnce(&mut PageCache) -> T) -> Option<T> {
    let mut guard = CACHE.lock().ok()?;
    Some(apply(guard.get_or_insert_with(PageCache::default)))
}

pub(crate) fn etag_of(html: &str) -> String {
    let digest = Sha256::digest(html.as_bytes());
    let hex: String = digest[..12]
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect();
    format!("\"{hex}\"")
}

pub(crate) async fn sync(pool: &MySqlPool) -> Result<(), AppError> {
    let due = with_cache(|cache| {
        let fresh = cache
            .checked_at
            .is_some_and(|checked| checked.elapsed() < CHECK_INTERVAL);
        (!fresh).then_some(cache.last_invalidation)
    })
    .flatten();
    let Some(last) = due else {
        return Ok(());
    };
    let Some(last) = last else {
        let newest: Option<u64> =
            sqlx::query_scalar("SELECT MAX(id) FROM site_cache_invalidations")
                .fetch_one(pool)
                .await?;
        with_cache(|cache| {
            cache.last_invalidation = Some(newest.unwrap_or_default());
            cache.checked_at = Some(Instant::now());
        });
        return Ok(());
    };
    let rows = sqlx::query(
        "SELECT id, tag FROM site_cache_invalidations WHERE id > ? ORDER BY id LIMIT ?",
    )
    .bind(last)
    .bind(INVALIDATION_BATCH)
    .fetch_all(pool)
    .await?;
    let mut invalidations = Vec::with_capacity(rows.len());
    for row in rows {
        invalidations.push((
            row.try_get::<u64, _>("id")?,
            row.try_get::<String, _>("tag")?,
        ));
    }
    with_cache(|cache| {
        if invalidations.len() as i64 == INVALIDATION_BATCH {
            cache.purge(ALL_TAG);
        } else {
            for (_, tag) in &invalidations {
                cache.purge(tag);
            }
        }
        if let Some((id, _)) = invalidations.last() {
            cache.last_invalidation = Some(*id);
        }
        cache.checked_at = Some(Instant::now());
    });
    Ok(())
}

pub(crate) fn get(key: &str) -> Option<CachedPage> {
    with_cache(|cache| {
        let expired = cache
            .entries
            .get(key)
            .is_some_and(|entry| entry.stored_at.elapsed() > ENTRY_LIFETIME);
        if expired {
            cache.remove(key);
            return None;
        }
        cache.entries.get(key).map(|entry| entry.page.clone())
    })
    .flatten()
}

pub(crate) fn put(key: String, html: String, tags: Vec<String>) -> CachedPage {
    let page = CachedPage {
        etag: etag_of(&html),
        html: Arc::from(html),
    };
    with_cache(|cache| {
        cache.remove(&key);
        cache.bytes += page.html.len();
        cache.order.retain(|candidate| candidate != &key);
        cache.order.push_back(key.clone());
        cache.entries.insert(
            key,
            Entry {
                page: page.clone(),
                tags,
                stored_at: Instant::now(),
            },
        );
        cache.evict();
    });
    page
}

pub(crate) async fn invalidate(pool: &MySqlPool, tags: &[String]) -> Result<(), AppError> {
    for tag in tags {
        sqlx::query("INSERT INTO site_cache_invalidations (tag) VALUES (?)")
            .bind(tag)
            .execute(pool)
            .await?;
    }
    sqlx::query(
        "DELETE FROM site_cache_invalidations WHERE created_at < NOW(6) - INTERVAL 1 DAY LIMIT 1000",
    )
    .execute(pool)
    .await?;
    Ok(())
}

pub(crate) fn record_tag(record_type: &str, record_id: &str) -> String {
    format!("record:{record_type}:{record_id}")
}

pub(crate) fn entity_tag(record_type: &str) -> String {
    format!("entity:{record_type}")
}
