use std::sync::atomic::{AtomicU64, Ordering};

use crate::tenancy::TenantLocal;

static ACCESS_CATALOG_GENERATION: TenantLocal<AtomicU64> = TenantLocal::new(AtomicU64::default);

pub fn invalidate_access_catalog() {
    if let Some(generation) = ACCESS_CATALOG_GENERATION.get() {
        generation.fetch_add(1, Ordering::SeqCst);
    }
}

pub fn access_catalog_generation() -> u64 {
    ACCESS_CATALOG_GENERATION
        .get()
        .map_or(u64::MAX, |generation| generation.load(Ordering::SeqCst))
}
