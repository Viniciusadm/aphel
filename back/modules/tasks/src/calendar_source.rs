use aphel_contract_crm::{CrmPermissions, CrmRecordTypes};
use aphel_kernel::application::access::Authorizer;
use aphel_kernel::application::fields::{CustomFieldPorts, ReferenceChecker};
use aphel_kernel::application::shared::RecordType;
use aphel_kernel::domain::access::Permission;
use aphel_sdk::calendar::{
    CalendarBatch, CalendarCategory, CalendarOwner, CalendarQuery, CalendarRecord, CalendarSource,
};
use aphel_sdk::{AppError, Ctx};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde_json::Value;

pub(crate) struct TaskCalendar;
#[async_trait]
impl CalendarSource for TaskCalendar {
    fn key(&self) -> &'static str {
        "tasks"
    }
    async fn categories(&self, ctx: &Ctx<'_>) -> Result<Vec<CalendarCategory>, AppError> {
        if ctx.scope.require(&Permission::TASKS_READ).is_err() {
            return Ok(Vec::new());
        }
        let fields = ctx
            .field_set(ctx.scope, RecordType::ACTIVITY.as_str())
            .await?;
        let visible = |key: &str| {
            fields
                .find_active(key)
                .is_some_and(|field| !field.is_hidden())
        };
        let categories = if visible("activity_type_id") {
            crate::types::list(
                ctx,
                false,
                aphel_kernel::application::organization_admin::ListWindow {
                    offset: 0,
                    limit: 200,
                },
            )
            .await?
            .items
            .into_iter()
            .map(|category| CalendarCategory {
                source: "tasks".into(),
                id: category.id.to_string(),
                name: category.name,
                icon: category.icon,
                color: category.color,
            })
            .collect()
        } else {
            Vec::new()
        };
        Ok(categories)
    }
    async fn entries(
        &self,
        ctx: &Ctx<'_>,
        query: &CalendarQuery,
    ) -> Result<CalendarBatch, AppError> {
        if ctx.scope.require(&Permission::TASKS_READ).is_err() {
            return Ok(CalendarBatch::default());
        }
        let fields = ctx
            .field_set(ctx.scope, RecordType::ACTIVITY.as_str())
            .await?;
        let visible = |key: &str| {
            fields
                .find_active(key)
                .is_some_and(|field| !field.is_hidden())
        };
        if !visible("title") || !visible("due_at") {
            return Ok(CalendarBatch::default());
        }
        let listing = crate::service::calendar(
            ctx,
            crate::domain::CalendarQuery {
                from: query.from.clone(),
                to: query.to.clone(),
                owner_id: query.owner_id.clone(),
                team_id: query.team_id.clone(),
                activity_type_id: query.category_id.clone(),
                status: query.status.clone(),
            },
        )
        .await?;
        let ids: Vec<_> = listing
            .items
            .iter()
            .map(|record| record.activity.id)
            .collect();
        let editable = match ctx.scope.record_filter(&Permission::TASKS_UPDATE) {
            Ok(filter) => {
                ctx.fields()
                    .references()
                    .existing_records(
                        ctx.scope,
                        ctx.registry().require(RecordType::ACTIVITY.as_str())?,
                        &filter,
                        &ids,
                    )
                    .await?
            }
            Err(_) => Vec::new(),
        };
        let editable: std::collections::BTreeSet<_> = editable.into_iter().collect();
        let mut items = Vec::new();
        for record in listing.items {
            let record_id = record.activity.id;
            let value = serde_json::to_value(crate::http::ActivityResponse::from(record))
                .map_err(|error| AppError::Internal(error.to_string()))?;
            let text = |key: &str| {
                value
                    .get(key)
                    .and_then(Value::as_str)
                    .unwrap_or_default()
                    .to_owned()
            };
            let id = text("id");
            let status = if visible("status") {
                text("status")
            } else {
                String::new()
            };
            let owner = if visible("owner_id") {
                value.get("owner").and_then(|owner| {
                    Some(CalendarOwner {
                        id: owner.get("id")?.as_str()?.into(),
                        display_name: owner.get("display_name")?.as_str()?.into(),
                    })
                })
            } else {
                None
            };
            let category = if visible("activity_type_id") {
                value.get("activity_type").and_then(|category| {
                    serde_json::from_value::<CalendarCategory>(category.clone()).ok()
                })
            } else {
                None
            };
            items.push(CalendarRecord {
                id: format!("tasks:{id}"),
                source: "tasks".into(),
                record_id: id.clone(),
                title: text("title"),
                due_at: text("due_at"),
                duration_minutes: if visible("duration_minutes") {
                    value.get("duration_minutes").and_then(Value::as_i64)
                } else {
                    None
                },
                finished: matches!(status.as_str(), "done" | "canceled"),
                status,
                category: category
                    .map(|mut category| {
                        category.source = "tasks".into();
                        category
                    })
                    .unwrap_or(CalendarCategory {
                        source: "tasks".into(),
                        id: String::new(),
                        name: String::new(),
                        icon: None,
                        color: None,
                    }),
                owner,
                href: format!("/tasks/{id}"),
                updated_at: text("updated_at"),
                can_update: editable.contains(&record_id)
                    && fields
                        .find_active("due_at")
                        .is_some_and(|field| field.is_editable()),
            });
        }
        Ok(CalendarBatch {
            items,
            total: listing.total,
            truncated: listing.truncated,
        })
    }
    async fn reschedule(
        &self,
        ctx: &Ctx<'_>,
        id: &str,
        due_at: &str,
        expected_updated_at: Option<DateTime<Utc>>,
        ip: Option<String>,
    ) -> Result<(), AppError> {
        let request: crate::http::UpdateActivityRequest =
            serde_json::from_value(serde_json::json!({"due_at":due_at}))
                .map_err(|error| AppError::Validation(error.to_string()))?;
        crate::service::update(ctx, id, request.into_command(ip, expected_updated_at)).await?;
        Ok(())
    }
}
