use aphel_kernel::{Ctx, errors::AppError};
use aphel_sdk::platform::application::organization_admin::ListWindow;
use aphel_sdk::platform::application::query::{
    Filter, FilterOperator, RecordQuery, RecordQueryEngine, SortSpec,
};
use aphel_sdk::platform::infrastructure::query::MySqlRecordQueryPorts;
use serde_json::{Map, Value};
use uuid::Uuid;

pub(crate) async fn find_record(
    ctx: &Ctx<'_>,
    entity: &str,
    key: &str,
    value: Value,
) -> Result<Option<Uuid>, AppError> {
    let ports = MySqlRecordQueryPorts::new(ctx.pool().clone(), ctx.registry().clone());
    let result = RecordQueryEngine::new(&ports)
        .query(
            ctx.scope,
            entity,
            RecordQuery {
                filter: Some(Filter::condition(key, FilterOperator::Eq, value)),
                sort: vec![SortSpec::asc("id")],
                window: ListWindow {
                    offset: 0,
                    limit: 2,
                },
                ..RecordQuery::default()
            },
        )
        .await?;
    if result.items.len() > 1 {
        return Err(AppError::Conflict(
            "the match field identified multiple visible records".into(),
        ));
    }
    Ok(result.items.into_iter().next())
}
fn selector(
    ctx: &Ctx<'_>,
) -> Result<Option<std::sync::Arc<dyn aphel_sdk::assignment::AssignmentSelector>>, AppError> {
    let sources = ctx
        .app()
        .contributions::<dyn aphel_sdk::assignment::AssignmentSelector>();
    if sources.len() > 1 {
        return Err(AppError::Configuration(
            "multiple assignment selectors".into(),
        ));
    }
    Ok(sources.into_iter().next())
}
pub(crate) async fn distribution_rule_usable(
    ctx: &Ctx<'_>,
    rule: &str,
    entity: &str,
) -> Result<bool, AppError> {
    match selector(ctx)? {
        Some(provider) => provider.usable(ctx, rule, entity).await,
        None => Ok(false),
    }
}
pub(crate) async fn distribute_owner(
    ctx: &Ctx<'_>,
    rule: &str,
    entity: &str,
    values: &Map<String, Value>,
) -> Result<Option<String>, AppError> {
    let provider =
        selector(ctx)?.ok_or_else(|| AppError::Validation("assignment is unavailable".into()))?;
    if !provider.usable(ctx, rule, entity).await? {
        return Err(AppError::Validation(
            "assignment rule is unavailable".into(),
        ));
    }
    Ok(provider
        .select(
            ctx,
            rule,
            entity,
            values,
            "incoming_webhook",
            Uuid::now_v7().to_string(),
        )
        .await?
        .map(|id| id.as_str().to_owned()))
}
