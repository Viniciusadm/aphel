use crate::engine::{DashboardPorts, SettingsReader, WidgetContext, WidgetProvider, instant_value};
use crate::queries::{self, ReadableRecords, RecentEvent, RecentEventsRequest, RecordLink};
use aphel_kernel::application::files::UserReference;
use aphel_kernel::application::query::RecordQueryPorts;
use aphel_kernel::domain::access::Permission;
use aphel_kernel::errors::{AppError, FieldViolation};
use async_trait::async_trait;
use serde_json::{Map, Value, json};
pub(crate) const RECENT_ACTIVITY: &str = "recent_activity";

fn user_json(user: &Option<UserReference>) -> Value {
    user.as_ref()
        .map(|user| json!({ "id": user.id.as_str(), "display_name": user.display_name }))
        .unwrap_or(Value::Null)
}

fn record_json(record: &Option<RecordLink>) -> Value {
    record
        .as_ref()
        .map(|record| {
            json!({
                "record_type": record.record_type,
                "record_id": record.record_id.to_string(),
                "title": record.title,
            })
        })
        .unwrap_or(Value::Null)
}

pub(crate) struct RecentActivityWidget;

fn recent_limit(settings: &Value) -> Result<i64, Vec<FieldViolation>> {
    let mut reader = SettingsReader::new(settings)?;
    let limit = reader.integer("limit", 15, 5, 50);
    let mut normalized = Map::new();
    normalized.insert("limit".to_owned(), json!(limit));
    reader.finish(normalized)?;
    Ok(limit)
}

fn event_json(event: &RecentEvent) -> Value {
    json!({
        "id": event.id.to_string(),
        "event_type": event.event_type,
        "occurred_at": instant_value(event.occurred_at),
        "actor": user_json(&event.actor),
        "record": record_json(&event.record),
        "payload": event.payload,
    })
}

pub(crate) fn recent_events_request<P: RecordQueryPorts>(
    query: &P,
    scope: &aphel_kernel::domain::access::AccessScope,
    limit: u32,
) -> RecentEventsRequest {
    let records = query
        .registry()
        .definitions()
        .filter_map(|definition| {
            let visibility = scope.filter_for(&definition.read_permission)?;
            Some(ReadableRecords {
                definition: definition.clone(),
                visibility,
            })
        })
        .collect();
    RecentEventsRequest {
        records,
        actor_ids: None,
        limit,
    }
}

#[async_trait]
impl<P> WidgetProvider<P> for RecentActivityWidget
where
    P: DashboardPorts,
{
    fn default_order(&self) -> Option<i32> {
        Some(30)
    }
    fn key(&self) -> &str {
        RECENT_ACTIVITY
    }

    fn title(&self) -> &str {
        "Recent activity"
    }

    fn category(&self) -> &str {
        "history"
    }

    fn required_permissions(&self) -> Vec<Permission> {
        Vec::new()
    }
    fn is_available(&self, scope: &aphel_kernel::domain::access::AccessScope) -> bool {
        aphel_kernel::modules::app()
            .record_types()
            .definitions()
            .any(|definition| scope.filter_for(&definition.read_permission).is_some())
    }
    fn uses_period(&self) -> bool {
        false
    }

    fn default_settings(&self) -> Value {
        json!({ "limit": 15 })
    }

    fn normalize_settings(&self, settings: &Value) -> Result<Value, Vec<FieldViolation>> {
        Ok(json!({ "limit": recent_limit(settings)? }))
    }

    async fn data(
        &self,
        context: &WidgetContext<'_, P>,
        settings: &Value,
    ) -> Result<Value, AppError> {
        let limit = recent_limit(settings).map_err(|violations| {
            AppError::invalid_fields("invalid widget settings", violations)
        })?;
        let mut request = recent_events_request(
            context.ports.query(),
            context.scope,
            u32::try_from(limit).unwrap_or(15),
        );
        request.actor_ids = context.subject.as_ref().map(|subject| {
            subject
                .member_ids
                .iter()
                .map(|id| id.as_str().to_owned())
                .collect()
        });
        let events = if request.records.is_empty() {
            Vec::new()
        } else {
            queries::recent_events(
                context.ports.pool(),
                context.scope,
                context.ports.query(),
                &request,
            )
            .await?
        };
        Ok(json!({ "items": events.iter().map(event_json).collect::<Vec<_>>() }))
    }
}

async fn source_sections(
    context: &WidgetContext<'_, aphel_sdk::dashboard::WidgetPorts>,
    collection: &str,
    settings: &Value,
) -> Result<Value, AppError> {
    let mut sources =
        aphel_kernel::modules::app().contributions::<dyn aphel_sdk::dashboard::DashboardSource>();
    sources.sort_by(|left, right| left.key().cmp(right.key()));
    let mut keys = std::collections::HashSet::new();
    let mut sections = Vec::new();
    for source in sources
        .into_iter()
        .filter(|source| source.collection() == collection)
    {
        if source.key().is_empty()
            || source.key().len() > 64
            || !source.key().bytes().all(|byte| {
                byte.is_ascii_lowercase()
                    || byte.is_ascii_digit()
                    || matches!(byte, b'_' | b'.' | b'-')
            })
            || !keys.insert(source.key().to_owned())
            || keys.len() > 64
        {
            return Err(AppError::Configuration(
                "invalid dashboard source identity".into(),
            ));
        }
        if source.available(context.scope) {
            sections
                .push(json!({"key": source.key(), "data": source.data(context, settings).await?}));
        }
    }
    Ok(json!({"sections": sections}))
}

pub(crate) struct PendingCenterWidget;
pub(crate) struct ReportTotalsWidget;
#[async_trait]
impl WidgetProvider<aphel_sdk::dashboard::WidgetPorts> for PendingCenterWidget {
    fn key(&self) -> &str {
        "pending_center"
    }
    fn title(&self) -> &str {
        "Pending work"
    }
    fn category(&self) -> &str {
        "overview"
    }
    fn default_order(&self) -> Option<i32> {
        Some(20)
    }
    fn uses_period(&self) -> bool {
        false
    }
    fn required_permissions(&self) -> Vec<Permission> {
        Vec::new()
    }
    fn is_available(&self, scope: &aphel_kernel::domain::access::AccessScope) -> bool {
        aphel_kernel::modules::app()
            .contributions::<dyn aphel_sdk::dashboard::DashboardSource>()
            .iter()
            .any(|source| source.collection() == "pending" && source.available(scope))
    }
    fn default_settings(&self) -> Value {
        json!({"stale_days":14,"only_mine":true,"limit":5})
    }
    fn normalize_settings(&self, settings: &Value) -> Result<Value, Vec<FieldViolation>> {
        let mut reader = SettingsReader::new(settings)?;
        let days = reader.integer("stale_days", 14, 1, 365);
        let mine = reader.boolean("only_mine", true);
        let limit = reader.integer("limit", 5, 1, 20);
        reader.finish(
            serde_json::from_value(json!({"stale_days":days,"only_mine":mine,"limit":limit}))
                .expect("object"),
        )
    }
    async fn data(
        &self,
        context: &WidgetContext<'_, aphel_sdk::dashboard::WidgetPorts>,
        settings: &Value,
    ) -> Result<Value, AppError> {
        source_sections(context, "pending", settings).await
    }
}
#[async_trait]
impl WidgetProvider<aphel_sdk::dashboard::WidgetPorts> for ReportTotalsWidget {
    fn key(&self) -> &str {
        "report_totals"
    }
    fn title(&self) -> &str {
        "Summary"
    }
    fn category(&self) -> &str {
        "overview"
    }
    fn default_order(&self) -> Option<i32> {
        Some(40)
    }
    fn uses_period(&self) -> bool {
        true
    }
    fn required_permissions(&self) -> Vec<Permission> {
        Vec::new()
    }
    fn is_available(&self, scope: &aphel_kernel::domain::access::AccessScope) -> bool {
        aphel_kernel::modules::app()
            .contributions::<dyn aphel_sdk::dashboard::DashboardSource>()
            .iter()
            .any(|source| source.collection() == "summary" && source.available(scope))
    }
    async fn data(
        &self,
        context: &WidgetContext<'_, aphel_sdk::dashboard::WidgetPorts>,
        settings: &Value,
    ) -> Result<Value, AppError> {
        source_sections(context, "summary", settings).await
    }
}
