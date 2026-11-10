use std::sync::atomic::{AtomicU64, Ordering};

static ACCESS_CATALOG_GENERATION: AtomicU64 = AtomicU64::new(0);

pub fn invalidate_access_catalog() {
    ACCESS_CATALOG_GENERATION.fetch_add(1, Ordering::SeqCst);
}

pub fn access_catalog_generation() -> u64 {
    ACCESS_CATALOG_GENERATION.load(Ordering::SeqCst)
}
