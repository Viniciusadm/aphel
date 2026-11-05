pub(crate) mod email_blocklist;
pub(crate) mod email_capture;
pub(crate) mod email_templates;
pub(crate) mod email_threads;
pub(crate) mod email_tracking;
pub(crate) mod email_tracking_public;
pub(crate) mod emails;
pub(crate) mod mailboxes;

use utoipa_axum::router::OpenApiRouter;

use aphel_sdk::platform::api::state::AppState;

pub(crate) fn router() -> OpenApiRouter<AppState> {
    OpenApiRouter::new()
        .merge(mailboxes::router())
        .merge(email_threads::router())
        .merge(email_capture::router())
        .merge(email_blocklist::router())
        .merge(emails::router())
        .merge(email_templates::router())
        .merge(email_tracking::router())
        .merge(email_tracking_public::router())
}
