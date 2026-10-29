use std::sync::Arc;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use uuid::Uuid;

use super::engine::{access_token, usable_scope};
use super::model::CalendarApi;
use super::store;
use aphel_kernel::Ctx;
use aphel_kernel::domain::UserId;
use aphel_kernel::errors::AppError;
use scheduling::{BusyInterval, CalendarAvailability};

pub(crate) const AVAILABILITY_KEY: &str = "calendar";

pub struct CalendarBusy<A: CalendarApi> {
    api: Arc<A>,
    key: String,
}

impl<A: CalendarApi> CalendarBusy<A> {
    pub fn new(api: Arc<A>) -> Self {
        Self {
            key: format!("{}.{AVAILABILITY_KEY}", api.spec().key),
            api,
        }
    }
}

#[async_trait]
impl<A: CalendarApi + 'static> CalendarAvailability for CalendarBusy<A> {
    fn key(&self) -> &str {
        &self.key
    }

    async fn busy(
        &self,
        ctx: &Ctx<'_>,
        user_id: &UserId,
        from: DateTime<Utc>,
        to: DateTime<Utc>,
    ) -> Result<Option<Vec<BusyInterval>>, AppError> {
        let user_id = Uuid::parse_str(user_id.as_str())
            .map_err(|_| AppError::Internal(format!("{} is not a UUID", user_id.as_str())))?;
        let connected = store::find_connection(ctx.pool(), self.api.spec().key, user_id)
            .await?
            .is_some_and(|connection| connection.syncable());
        if !connected {
            return Ok(None);
        }
        let Ok(scope) = usable_scope(ctx, self.api.as_ref(), user_id).await? else {
            return Ok(Some(Vec::new()));
        };
        let token = access_token(ctx, self.api.as_ref(), &scope).await?;
        let mut intervals = self.api.busy(&token, from, to).await?;
        intervals.retain(|interval| {
            interval.ends_at > interval.starts_at
                && interval.ends_at > from
                && interval.starts_at < to
        });
        Ok(Some(intervals))
    }
}
