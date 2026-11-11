use std::sync::LazyLock;

use tokio::sync::Notify;

static JOBS_AVAILABLE: LazyLock<Notify> = LazyLock::new(Notify::new);

pub(crate) fn wake_workers() {
    JOBS_AVAILABLE.notify_one();
}

pub(crate) fn jobs_available() -> &'static Notify {
    &JOBS_AVAILABLE
}
