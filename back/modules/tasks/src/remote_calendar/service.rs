use super::{availability, connect, http};
use aphel_sdk::{
    AppError, Ctx,
    tasks::remote_calendar::{CalendarApi, CalendarConnectUrl, CalendarStatus, RemoteCalendars},
};
use std::sync::Arc;
pub(crate) struct Calendars;
pub(crate) fn providers(ctx: &Ctx<'_>) -> Result<Vec<Arc<dyn CalendarApi>>, AppError> {
    let mut providers = Vec::new();
    let mut keys = std::collections::BTreeSet::new();
    for (owner, api) in ctx.app().all_contributions::<dyn CalendarApi>() {
        if !ctx.app().is_enabled(owner) {
            continue;
        }
        if api.key() != api.spec().key || !keys.insert(api.spec().key) {
            return Err(AppError::Configuration(format!(
                "invalid or ambiguous calendar provider {}",
                api.spec().key
            )));
        }
        providers.push(api);
    }
    Ok(providers)
}
pub(crate) fn provider(ctx: &Ctx<'_>, key: &str) -> Result<Option<Arc<dyn CalendarApi>>, AppError> {
    Ok(providers(ctx)?
        .into_iter()
        .find(|api| api.spec().key == key))
}
fn required(ctx: &Ctx<'_>, key: &str) -> Result<Arc<dyn CalendarApi>, AppError> {
    provider(ctx, key)?.ok_or_else(|| {
        AppError::unavailable_code(
            "calendar_provider_unavailable",
            "the calendar provider is not enabled",
        )
    })
}
#[async_trait::async_trait]
impl RemoteCalendars for Calendars {
    async fn status(&self, ctx: &Ctx<'_>, key: &str) -> Result<CalendarStatus, AppError> {
        connect::status(ctx, required(ctx, key)?.as_ref()).await
    }
    async fn connect_url(&self, ctx: &Ctx<'_>, key: &str) -> Result<CalendarConnectUrl, AppError> {
        connect::connect_url(ctx, required(ctx, key)?.as_ref()).await
    }
    async fn complete(
        &self,
        ctx: &Ctx<'_>,
        key: &str,
        state: &aphel_sdk::sso::StoredState,
        code: &str,
    ) -> axum::response::Response {
        match required(ctx, key) {
            Ok(api) => connect::complete_connection(ctx, api.as_ref(), state, code).await,
            Err(_) => aphel_sdk::sso::login::redirect_to(&format!(
                "/{key}?calendar_error=calendar_provider_unavailable"
            )),
        }
    }
    async fn disconnect(&self, ctx: &Ctx<'_>, key: &str) -> Result<(), AppError> {
        connect::disconnect(ctx, required(ctx, key)?.as_ref()).await
    }
    async fn sync_now(&self, ctx: &Ctx<'_>, key: &str) -> Result<(), AppError> {
        connect::sync_now(ctx, required(ctx, key)?.as_ref()).await
    }
    async fn busy(
        &self,
        ctx: &Ctx<'_>,
        key: &str,
        user: &aphel_kernel::domain::UserId,
        from: chrono::DateTime<chrono::Utc>,
        to: chrono::DateTime<chrono::Utc>,
    ) -> Result<Option<Vec<aphel_sdk::calendar::BusyInterval>>, AppError> {
        let Some(api) = provider(ctx, key)? else {
            return Ok(None);
        };
        availability::busy(ctx, api.as_ref(), user, from, to).await
    }
    async fn send(
        &self,
        request: reqwest::RequestBuilder,
        cursor_request: bool,
    ) -> Result<serde_json::Value, AppError> {
        http::send(request, cursor_request).await
    }
}
