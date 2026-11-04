pub mod domain;
pub(crate) mod http;
pub(crate) mod notifications;
pub(crate) mod section;
pub mod service;
pub mod store;

use aphel_kernel::errors::AppError;
use aphel_kernel::modules::AppBuilder;

pub(crate) fn register(app: &mut AppBuilder) -> Result<(), AppError> {
    app.openapi_tag(
        http::TAG,
        "Team of a deal (members, roles and revenue credit split) and the configurable member roles",
    )
    .routes(|_| http::router())
    .contribute(std::sync::Arc::new(notifications::DEAL_TEAM_NOTIFICATION_KIND))
    .consumer(|env| notifications::DealTeamNotificationConsumer::shared(env.pool.clone()));
    Ok(())
}

pub(crate) mod credit_reader;
