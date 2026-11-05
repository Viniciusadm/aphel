use crate::domain::DealInput;
use aphel_sdk::platform::application::shared::money_text;
use aphel_sdk::{
    AppError, Ctx,
    deals::{DealRevenue, RenewalDealCreation, RenewalDealWriter, date},
};
use uuid::Uuid;
pub(crate) struct Writer;
#[async_trait::async_trait]
impl RenewalDealWriter for Writer {
    async fn create(&self, ctx: &Ctx<'_>, creation: RenewalDealCreation) -> Result<Uuid, AppError> {
        let record = crate::service::create_with_revenue(
            ctx,
            DealInput {
                title: Some(creation.title),
                amount: Some(creation.amount.map(money_text)),
                currency: Some(creation.currency),
                pipeline_id: Some(creation.pipeline_id.to_string()),
                stage_id: Some(creation.stage_id.to_string()),
                expected_close_date: Some(creation.expected_close_date.map(date)),
                contact_id: Some(creation.contact_id.map(|id| id.to_string())),
                company_id: Some(creation.company_id.map(|id| id.to_string())),
                owner_id: Some(creation.owner_id),
                source: Some(creation.source),
                contract_start: Some(creation.contract_start.map(date)),
                ..DealInput::default()
            },
            DealRevenue {
                renewal_of_deal_id: Some(creation.original_id),
                ..DealRevenue::default()
            },
        )
        .await?;
        Ok(record.record.deal.id)
    }
}

#[async_trait::async_trait]
impl aphel_sdk::deals::DealRenewalReader for Writer {
    async fn notification_target(
        &self,
        pool: &sqlx::MySqlPool,
        id: Uuid,
    ) -> Result<Option<(String, Option<String>)>, AppError> {
        Ok(
            sqlx::query_as("SELECT title, owner_id FROM deals WHERE id = ? AND deleted_at IS NULL")
                .bind(id.to_string())
                .fetch_optional(pool)
                .await?,
        )
    }

    async fn widget_rows(
        &self,
        context: &aphel_sdk::dashboard::WidgetContext<'_, aphel_sdk::dashboard::WidgetPorts>,
        plan: &aphel_sdk::platform::application::query::QueryPlan,
    ) -> Result<Vec<aphel_sdk::deals::RenewalDealRow>, AppError> {
        widget_rows(context, plan).await
    }

    async fn due(
        &self,
        ctx: &Ctx<'_>,
        query: aphel_sdk::deals::RenewalDueQuery,
    ) -> Result<Vec<aphel_sdk::deals::RenewalCandidate>, AppError> {
        use aphel_sdk::platform::application::query::{
            ArchivedFilter, Filter, FilterOperator, RecordQuery, SortSpec,
        };
        let until = query
            .today
            .checked_add_days(chrono::Days::new(u64::from(query.days_before)))
            .ok_or_else(|| AppError::Validation("renewal date is out of range".into()))?;
        let mut filters = vec![
            Filter::condition(
                "pipeline_id",
                FilterOperator::Eq,
                serde_json::json!(query.pipeline_id),
            ),
            Filter::condition("status", FilterOperator::Eq, serde_json::json!("won")),
            Filter::condition(
                "contract_end",
                FilterOperator::Gte,
                serde_json::json!(date(query.today)),
            ),
            Filter::condition(
                "contract_end",
                FilterOperator::Lte,
                serde_json::json!(date(until)),
            ),
        ];
        if let Some(after) = query.after {
            filters.push(Filter::or(vec![
                Filter::condition(
                    "contract_end",
                    FilterOperator::Gt,
                    serde_json::json!(date(after.contract_end)),
                ),
                Filter::and(vec![
                    Filter::condition(
                        "contract_end",
                        FilterOperator::Eq,
                        serde_json::json!(date(after.contract_end)),
                    ),
                    Filter::condition("id", FilterOperator::Gt, serde_json::json!(after.id)),
                ]),
            ]));
        }
        let rows = aphel_sdk::record_context::list(
            ctx,
            aphel_sdk::deals::RECORD_TYPE,
            RecordQuery {
                filter: Some(Filter::and(filters)),
                sort: vec![SortSpec::asc("contract_end"), SortSpec::asc("id")],
                archived: ArchivedFilter::Include,
                window: aphel_sdk::platform::application::organization_admin::ListWindow {
                    offset: 0,
                    limit: 100,
                },
                ..RecordQuery::default()
            },
        )
        .await?;
        rows.items
            .into_iter()
            .map(|value| {
                let id = value
                    .get("id")
                    .and_then(|value| value.as_str())
                    .and_then(|value| Uuid::parse_str(value).ok())
                    .ok_or_else(|| AppError::Internal("deal snapshot has no identity".into()))?;
                let contract_end = value
                    .get("contract_end")
                    .and_then(|value| value.as_str())
                    .and_then(|value| chrono::NaiveDate::parse_from_str(value, "%Y-%m-%d").ok())
                    .ok_or_else(|| {
                        AppError::Forbidden("deal contract end is not readable".into())
                    })?;
                Ok(aphel_sdk::deals::RenewalCandidate { id, contract_end })
            })
            .collect()
    }
    async fn existing(
        &self,
        ctx: &Ctx<'_>,
        original_id: Uuid,
    ) -> Result<Option<aphel_sdk::deals::ExistingDealRenewal>, AppError> {
        if ctx
            .load_record(aphel_sdk::deals::RECORD_TYPE, original_id)
            .await?
            .is_none()
        {
            return Ok(None);
        }
        let row: Option<(String, Option<chrono::NaiveDateTime>)> =
            sqlx::query_as("SELECT id, deleted_at FROM deals WHERE renewal_of_deal_id = ?")
                .bind(original_id.to_string())
                .fetch_optional(ctx.pool())
                .await?;
        row.map(|(id, deleted)| {
            Ok(aphel_sdk::deals::ExistingDealRenewal {
                deal_id: aphel_sdk::platform::infrastructure::repositories::mapper::parse_uuid(
                    &id, "deals.id",
                )?,
                deleted: deleted.is_some(),
            })
        })
        .transpose()
    }
}

async fn widget_rows(
    context: &aphel_sdk::dashboard::WidgetContext<'_, aphel_sdk::dashboard::WidgetPorts>,
    plan: &aphel_sdk::platform::application::query::QueryPlan,
) -> Result<Vec<aphel_sdk::deals::RenewalDealRow>, AppError> {
    use aphel_sdk::dashboard::DashboardPorts;
    use aphel_sdk::platform::application::fields::FieldSet;
    use aphel_sdk::platform::application::query::RecordQueryPorts;
    use aphel_sdk::platform::infrastructure::query::compiler::{Projection, build_query};
    use sqlx::Row;
    if plan.record_type.record_type.as_str() != aphel_sdk::deals::RECORD_TYPE
        || !(1..=1000).contains(&plan.window.limit)
    {
        return Err(AppError::Validation(
            "a bounded deal query is required".into(),
        ));
    }
    let pool = context.ports.pool();
    let constrained =
        aphel_sdk::platform::records::constrain_query(pool, context.scope, plan).await?;
    let plan = constrained.as_ref().unwrap_or(plan);
    let ids: Vec<String> = build_query(context.scope, plan, Projection::Ids)?
        .build_query_scalar()
        .fetch_all(pool)
        .await?;
    if ids.is_empty() {
        return Ok(Vec::new());
    }
    let fields = FieldSet::for_scope(
        context.ports.query().definitions(),
        &plan.record_type,
        context.scope,
    )
    .await?;
    let mut query = sqlx::QueryBuilder::<sqlx::MySql>::new(
        "SELECT d.id, d.title, d.amount, d.currency, d.stage_id, d.contract_end, d.owner_id, u.display_name AS owner_name FROM deals d LEFT JOIN users u ON u.id = d.owner_id WHERE d.id IN (",
    );
    let mut list = query.separated(", ");
    for id in &ids {
        list.push_bind(id.clone());
    }
    query.push(")");
    let rows = query.build().fetch_all(pool).await?;
    let mut stages = std::collections::HashMap::new();
    if !fields.is_hidden("stage_id")
        && let Some(source) = context
            .ports
            .services()
            .get::<dyn aphel_sdk::workflow::WorkflowCatalog>()
    {
        let stage_ids = rows
            .iter()
            .map(|row| {
                let id: String = row.try_get("stage_id")?;
                aphel_sdk::platform::infrastructure::repositories::mapper::parse_uuid(
                    &id,
                    "deals.stage_id",
                )
            })
            .collect::<Result<Vec<_>, AppError>>()?;
        stages = source.stage_names(pool, &stage_ids).await?;
    }
    let mut values = std::collections::HashMap::new();
    for row in rows {
        let raw_id: String = row.try_get("id")?;
        let stage_id: String = row.try_get("stage_id")?;
        let stage_id = aphel_sdk::platform::infrastructure::repositories::mapper::parse_uuid(
            &stage_id,
            "deals.stage_id",
        )?;
        let id = aphel_sdk::platform::infrastructure::repositories::mapper::parse_uuid(
            &raw_id, "deals.id",
        )?;
        values.insert(
            raw_id,
            aphel_sdk::deals::RenewalDealRow {
                id,
                title: if fields.is_hidden("title") {
                    String::new()
                } else {
                    row.try_get("title")?
                },
                amount: if fields.is_hidden("amount") {
                    None
                } else {
                    row.try_get("amount")?
                },
                currency: if fields.is_hidden("currency") {
                    String::new()
                } else {
                    row.try_get("currency")?
                },
                stage_name: stages.get(&stage_id).cloned().unwrap_or_default(),
                contract_end: if fields.is_hidden("contract_end") {
                    None
                } else {
                    row.try_get("contract_end")?
                },
                owner_id: if fields.is_hidden("owner_id") {
                    None
                } else {
                    row.try_get("owner_id")?
                },
                owner_name: if fields.is_hidden("owner_id") {
                    None
                } else {
                    row.try_get("owner_name")?
                },
            },
        );
    }
    Ok(ids.iter().filter_map(|id| values.remove(id)).collect())
}
