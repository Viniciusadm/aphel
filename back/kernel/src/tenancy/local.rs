use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, OnceLock};

use crate::errors::AppError;
use crate::tenancy::{current, is_multi, outside_scope};

static NEXT_ID: AtomicU64 = AtomicU64::new(1);

pub struct TenantLocal<T> {
    id: OnceLock<u64>,
    init: fn() -> T,
    single: OnceLock<Arc<T>>,
}

impl<T: Send + Sync + 'static> TenantLocal<T> {
    pub const fn new(init: fn() -> T) -> Self {
        Self {
            id: OnceLock::new(),
            init,
            single: OnceLock::new(),
        }
    }

    fn id(&self) -> u64 {
        *self
            .id
            .get_or_init(|| NEXT_ID.fetch_add(1, Ordering::Relaxed))
    }

    pub fn get(&self) -> Option<Arc<T>> {
        if let Some(tenant) = current() {
            return Some(tenant.local(self.id(), self.init));
        }
        if is_multi() {
            return None;
        }
        Some(self.single.get_or_init(|| Arc::new((self.init)())).clone())
    }

    pub fn try_get(&self) -> Result<Arc<T>, AppError> {
        self.get().ok_or_else(outside_scope)
    }
}

impl<T: Default + Send + Sync + 'static> Default for TenantLocal<T> {
    fn default() -> Self {
        Self::new(T::default)
    }
}
