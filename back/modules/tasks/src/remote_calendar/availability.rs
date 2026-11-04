use super::engine::{access_token, usable_scope};
use super::{model::CalendarApi, store};
use aphel_kernel::{Ctx, domain::UserId, errors::AppError};
use aphel_sdk::calendar::BusyInterval;
use chrono::{DateTime, Utc};
use uuid::Uuid;
pub(crate) async fn busy(
    ctx: &Ctx<'_>,
    api: &dyn CalendarApi,
    user_id: &UserId,
    from: DateTime<Utc>,
    to: DateTime<Utc>,
) -> Result<Option<Vec<BusyInterval>>, AppError> {
    let user_id = Uuid::parse_str(user_id.as_str())
        .map_err(|_| AppError::Internal(format!("{} is not a UUID", user_id.as_str())))?;
    let connected = store::find_connection(ctx.pool(), api.spec().key, user_id)
        .await?
        .is_some_and(|connection| connection.syncable());
    if !connected {
        return Ok(None);
    }
    let Ok(scope) = usable_scope(ctx, api, user_id).await? else {
        return Ok(Some(Vec::new()));
    };
    let token = access_token(ctx, api, &scope).await?;
    let mut intervals = api.busy(&token, from, to).await?;
    intervals.retain(|interval| {
        interval.ends_at > interval.starts_at && interval.ends_at > from && interval.starts_at < to
    });
    Ok(Some(intervals))
}
