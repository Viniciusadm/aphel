use aphel_contract_crm::CrmPermissions;
use async_trait::async_trait;
use serde_json::{Value, json};

use crate::CATEGORY;
use crate::aggregate::{combine, condition, count_at, period, run, with_subject};
use crate::settings::{SettingsReader, invalid_settings};
use ::dashboard::engine::{DashboardPorts, WidgetContext, WidgetProvider, WidgetSize};
use aphel_kernel::domain::access::Permission;
use aphel_kernel::errors::{AppError, FieldViolation};

const OWNER_FIELD: &str = "owner_id";
const TYPE_FIELD: &str = "activity_type_id";
const PERIOD_FIELD: &str = "completed_at";

struct RankingSettings {
    limit: i64,
    by_type: bool,
}

fn parse(settings: &Value) -> Result<(RankingSettings, Value), Vec<FieldViolation>> {
    let mut reader = SettingsReader::new(settings)?;
    let parsed = RankingSettings {
        limit: reader.integer("limit", 5, 1, 20),
        by_type: reader.boolean("by_type", false),
    };
    Ok((parsed, reader.finish()?))
}

fn done_conditions() -> Vec<Value> {
    vec![condition("status", "eq", json!("done"))]
}

pub(crate) struct TeamActivityRankingWidget;

impl TeamActivityRankingWidget {
    pub(crate) const KEY: &'static str = "insights.team_activity_ranking";
}

#[async_trait]
impl<P> WidgetProvider<P> for TeamActivityRankingWidget
where
    P: DashboardPorts,
{
    fn key(&self) -> &str {
        Self::KEY
    }

    fn title(&self) -> &str {
        "Ranking de atividades da equipe"
    }

    fn category(&self) -> &str {
        CATEGORY
    }

    fn required_permissions(&self) -> Vec<Permission> {
        vec![Permission::TASKS_READ]
    }

    fn default_size(&self) -> WidgetSize {
        WidgetSize::Medium
    }

    fn default_settings(&self) -> Value {
        json!({ "limit": 5, "by_type": false })
    }

    fn normalize_settings(&self, settings: &Value) -> Result<Value, Vec<FieldViolation>> {
        parse(settings).map(|(_, normalized)| normalized)
    }

    async fn data(
        &self,
        context: &WidgetContext<'_, P>,
        settings: &Value,
    ) -> Result<Value, AppError> {
        let (settings, _) = parse(settings).map_err(invalid_settings)?;
        let ranking = run(
            context,
            json!({
                "record_type": "tasks.task",
                "filter": with_subject(context, done_conditions())?,
                "group_by": [{ "field": OWNER_FIELD }],
                "metrics": [{ "fn": "count" }],
                "period": period(context, PERIOD_FIELD),
                "sort": [{ "key": "count", "direction": "desc" }],
                "limit": settings.limit,
            }),
        )
        .await?;
        let owners: Vec<Option<String>> = ranking
            .rows
            .iter()
            .map(|row| row.groups.first().and_then(|group| group.value.clone()))
            .collect();
        let breakdown = if settings.by_type && !ranking.rows.is_empty() {
            let owner_ids: Vec<&String> = owners.iter().flatten().collect();
            let mut owner_conditions = Vec::new();
            if !owner_ids.is_empty() {
                owner_conditions.push(condition(OWNER_FIELD, "in", json!(owner_ids)));
            }
            if owners.iter().any(Option::is_none) {
                owner_conditions.push(condition(OWNER_FIELD, "is_empty", Value::Null));
            }
            let mut conditions = done_conditions();
            conditions.extend(combine("or", owner_conditions));
            Some(
                run(
                    context,
                    json!({
                        "record_type": "tasks.task",
                        "filter": with_subject(context, conditions)?,
                        "group_by": [{ "field": OWNER_FIELD }, { "field": TYPE_FIELD }],
                        "metrics": [{ "fn": "count" }],
                        "period": period(context, PERIOD_FIELD),
                        "sort": [{ "key": "count", "direction": "desc" }],
                        "limit": 1000,
                    }),
                )
                .await?,
            )
        } else {
            None
        };
        let rows: Vec<Value> = ranking
            .rows
            .iter()
            .zip(owners.iter())
            .enumerate()
            .map(|(index, (row, owner))| {
                let name = row.groups.first().and_then(|group| group.label.clone());
                let types: Vec<Value> = breakdown
                    .as_ref()
                    .map(|breakdown| {
                        breakdown
                            .rows
                            .iter()
                            .filter(|candidate| {
                                candidate
                                    .groups
                                    .first()
                                    .and_then(|group| group.value.as_ref())
                                    == owner.as_ref()
                            })
                            .map(|candidate| {
                                let activity_type = candidate.groups.get(1);
                                json!({
                                    "id": activity_type.and_then(|group| group.value.clone()),
                                    "name": activity_type.and_then(|group| group.label.clone()),
                                    "count": count_at(&candidate.values, 0),
                                })
                            })
                            .collect()
                    })
                    .unwrap_or_default();
                json!({
                    "position": index + 1,
                    "user_id": owner,
                    "name": name,
                    "count": count_at(&row.values, 0),
                    "types": types,
                })
            })
            .collect();
        Ok(json!({
            "by_type": settings.by_type,
            "rows": rows,
            "total": count_at(&ranking.totals, 0),
        }))
    }
}
