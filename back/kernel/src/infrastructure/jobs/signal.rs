use std::sync::Arc;

use tokio::sync::Notify;

use crate::tenancy::TenantLocal;

static JOBS_AVAILABLE: TenantLocal<Notify> = TenantLocal::new(Notify::new);

pub(crate) fn wake_workers() {
    if let Some(signal) = JOBS_AVAILABLE.get() {
        signal.notify_one();
    }
}

pub(crate) fn jobs_available() -> Option<Arc<Notify>> {
    JOBS_AVAILABLE.get()
}
