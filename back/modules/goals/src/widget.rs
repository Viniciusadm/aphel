use async_trait::async_trait;
use serde_json::{Map, Value, json};

use crate::domain::{DATE_FORMAT, GoalListQuery, GoalOwnerKind};
use crate::http::GoalResponse;
use crate::service;
use aphel_kernel::Ctx;
use aphel_kernel::errors::{AppError, FieldViolation};
use aphel_sdk::dashboard::local_today;
use aphel_sdk::dashboard::{DashboardPorts, SettingsReader, WidgetContext, WidgetProvider};
use aphel_sdk::platform::application::organization_admin::ListWindow;
use aphel_sdk::platform::domain::access::Permission;
use aphel_sdk::platform::state::runtime_state;

pub(crate) const GOALS_PROGRESS: &str = "goals_progress";

pub(crate) struct GoalsProgressWidget;

struct GoalsWidgetSettings {
    limit: i64,
    only_mine: bool,
}

fn widget_settings(settings: &Value) -> Result<GoalsWidgetSettings, Vec<FieldViolation>> {
    let mut reader = SettingsReader::new(settings)?;
    let parsed = GoalsWidgetSettings {
        limit: reader.integer("limit", 5, 1, 20),
        only_mine: reader.boolean("only_mine", false),
    };
    let mut normalized = Map::new();
    normalized.insert("limit".to_owned(), json!(parsed.limit));
    normalized.insert("only_mine".to_owned(), json!(parsed.only_mine));
    reader.finish(normalized)?;
    Ok(parsed)
}

#[async_trait]
impl<P> WidgetProvider<P> for GoalsProgressWidget
where
    P: DashboardPorts,
{
    fn key(&self) -> &str {
        GOALS_PROGRESS
    }

    fn title(&self) -> &str {
        "Metas"
    }

    fn category(&self) -> &str {
        "sales"
    }

    fn required_permissions(&self) -> Vec<Permission> {
        Vec::new()
    }

    fn uses_period(&self) -> bool {
        false
    }

    fn default_settings(&self) -> Value {
        json!({ "limit": 5, "only_mine": false })
    }

    fn normalize_settings(&self, settings: &Value) -> Result<Value, Vec<FieldViolation>> {
        let parsed = widget_settings(settings)?;
        Ok(json!({ "limit": parsed.limit, "only_mine": parsed.only_mine }))
    }

    async fn data(
        &self,
        context: &WidgetContext<'_, P>,
        settings: &Value,
    ) -> Result<Value, AppError> {
        let settings = widget_settings(settings).map_err(|violations| {
            AppError::invalid_fields("invalid widget settings", violations)
        })?;
        let today = local_today(context.now, context.timezone);
        let state = runtime_state()?;
        let ctx = Ctx::from_state(&state, context.scope).await?;
        let user_id = context.scope.user_id().as_str().to_owned();
        let listing = service::list(
            &ctx,
            GoalListQuery {
                owner_type: settings.only_mine.then_some(GoalOwnerKind::User),
                user_id: settings.only_mine.then(|| user_id.clone()),
                current: true,
                with_progress: true,
                ..GoalListQuery::default()
            },
            ListWindow {
                offset: 0,
                limit: u64::try_from(settings.limit).unwrap_or(1),
            },
        )
        .await?;
        let items = listing
            .items
            .into_iter()
            .map(|view| serde_json::to_value(GoalResponse::from(view)))
            .collect::<Result<Vec<_>, _>>()
            .map_err(|error| AppError::Internal(error.to_string()))?;
        Ok(json!({
            "active_on": today.format(DATE_FORMAT).to_string(),
            "limit": settings.limit,
            "only_mine": settings.only_mine,
            "user_id": user_id,
            "total": listing.total,
            "items": items,
        }))
    }
}
