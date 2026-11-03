use serde_json::{Map, Value, json};
use uuid::Uuid;

use aphel_kernel::Ctx;
use aphel_kernel::application::fields::FieldDefinitionRepository;
use aphel_kernel::application::organization_admin::ListWindow;
use aphel_kernel::application::query::{Filter, FilterOperator, RecordQuery};
use aphel_kernel::application::shared::RecordType;
use aphel_kernel::domain::access::AccessScope;
use aphel_kernel::domain::fields::FieldDefinition;
use aphel_kernel::errors::AppError;
use aphel_kernel::infrastructure::fields::MySqlFieldDefinitionRepository;
use deals::domain::DealInput;
use lead_distribution::{AssignRequest, AssignmentSource, RULE_NOT_FOUND};
use people::domain::SaveContactCommand;
use tasks::domain::CreateActivityCommand;

fn missing_is_false(result: Result<bool, AppError>) -> Result<bool, AppError> {
    match result {
        Err(AppError::NotFound(_)) => Ok(false),
        Err(AppError::Coded { code, .. }) if code.ends_with("_not_found") => Ok(false),
        other => other,
    }
}

pub(crate) async fn find_contact_by_email(
    ctx: &Ctx<'_>,
    email: &str,
) -> Result<Option<Uuid>, AppError> {
    first_contact(
        ctx,
        Filter::condition("email", FilterOperator::Eq, json!(email)),
    )
    .await
}

pub(crate) async fn find_contact_by_phone(
    ctx: &Ctx<'_>,
    numbers: &[String],
) -> Result<Option<Uuid>, AppError> {
    let conditions: Vec<Filter> = numbers
        .iter()
        .flat_map(|number| {
            ["phone", "mobile"]
                .into_iter()
                .map(move |key| Filter::condition(key, FilterOperator::Eq, json!(number)))
        })
        .collect();
    if conditions.is_empty() {
        return Ok(None);
    }
    first_contact(ctx, Filter::or(conditions)).await
}

pub(crate) async fn find_contact_by_field(
    ctx: &Ctx<'_>,
    key: &str,
    value: Value,
) -> Result<Option<Uuid>, AppError> {
    first_contact(ctx, Filter::condition(key, FilterOperator::Eq, value)).await
}

async fn first_contact(ctx: &Ctx<'_>, filter: Filter) -> Result<Option<Uuid>, AppError> {
    let query = RecordQuery {
        filter: Some(filter),
        window: ListWindow {
            offset: 0,
            limit: 1,
        },
        ..RecordQuery::default()
    };
    Ok(people::service::list(ctx, query)
        .await?
        .items
        .first()
        .map(|details| details.view.contact.id))
}

pub(crate) async fn create_contact(
    ctx: &Ctx<'_>,
    command: SaveContactCommand,
) -> Result<Uuid, AppError> {
    Ok(people::service::create(ctx, command).await?.view.contact.id)
}

pub(crate) async fn update_contact(
    ctx: &Ctx<'_>,
    contact_id: Uuid,
    command: SaveContactCommand,
) -> Result<Uuid, AppError> {
    Ok(
        people::service::update(ctx, &contact_id.to_string(), command)
            .await?
            .view
            .contact
            .id,
    )
}

pub(crate) async fn create_deal(ctx: &Ctx<'_>, input: DealInput) -> Result<Uuid, AppError> {
    Ok(deals::service::create(ctx, input).await?.record.deal.id)
}

pub(crate) async fn create_activity(
    ctx: &Ctx<'_>,
    command: CreateActivityCommand,
) -> Result<Uuid, AppError> {
    Ok(tasks::service::create(ctx, command).await?.activity.id)
}

pub(crate) async fn stage_in_pipeline(
    ctx: &Ctx<'_>,
    pipeline_id: &str,
    stage_id: &str,
) -> Result<bool, AppError> {
    let Ok(pipeline_id) = Uuid::parse_str(pipeline_id.trim()) else {
        return Ok(false);
    };
    if Uuid::parse_str(stage_id.trim()).is_err() {
        return Ok(false);
    }
    let system = AccessScope::system();
    missing_is_false(
        workflow::service::get_stage(&ctx.with_scope(&system), stage_id)
            .await
            .map(|stage| stage.pipeline_id == pipeline_id && stage.is_active),
    )
}

pub(crate) async fn activity_type_active(
    ctx: &Ctx<'_>,
    activity_type_id: &str,
) -> Result<bool, AppError> {
    if Uuid::parse_str(activity_type_id.trim()).is_err() {
        return Ok(false);
    }
    let system = AccessScope::system();
    missing_is_false(
        tasks::types::get(&ctx.with_scope(&system), activity_type_id)
            .await
            .map(|activity_type| activity_type.is_active),
    )
}

pub(crate) async fn field_definitions(
    ctx: &Ctx<'_>,
    record_type: &RecordType,
) -> Result<Vec<FieldDefinition>, AppError> {
    MySqlFieldDefinitionRepository::new(ctx.pool().clone())
        .list(record_type)
        .await
}

pub(crate) async fn distribution_rule_usable(
    ctx: &Ctx<'_>,
    rule_id: &str,
    record_type: &RecordType,
) -> Result<bool, AppError> {
    Ok(lead_distribution::service::check(ctx, rule_id)
        .await?
        .is_some_and(|rule| rule.is_active && rule.record_type == record_type.as_str()))
}

pub(crate) async fn distribute_owner(
    ctx: &Ctx<'_>,
    rule_id: &str,
    record_type: &RecordType,
    values: &Map<String, Value>,
) -> Result<Option<String>, AppError> {
    let request = AssignRequest {
        rule_id,
        record_type: record_type.as_str(),
        record: None,
        values,
        source: AssignmentSource::IncomingWebhook,
        idempotency_key: None,
    };
    match lead_distribution::service::assign(ctx, request, &lead_distribution::service::PickOnly)
        .await
    {
        Ok(outcome) => Ok(outcome.user_id().map(|user_id| user_id.as_str().to_owned())),
        Err(AppError::Coded { code, .. }) if code == RULE_NOT_FOUND => Ok(None),
        Err(error) => Err(error),
    }
}
