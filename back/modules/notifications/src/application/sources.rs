pub use aphel_sdk::notifications::{
    DEFAULT_SOURCE_ORDER, NotificationSource, NotificationSourceContext, event_dedupe,
    payload_text, payload_user, requested_job_intents,
};
use std::sync::Arc;

pub fn notification_sources() -> Vec<Arc<dyn NotificationSource>> {
    let mut sources = aphel_kernel::modules::try_app()
        .map(|app| app.contributions::<dyn NotificationSource>())
        .unwrap_or_default();
    sources.sort_by_key(|source| source.order());
    sources
}
