use aphel_contract_crm::CrmRecordTypes;
use serde_json::{Value, json};
use uuid::Uuid;

use aphel_kernel::context::Ctx;
use aphel_kernel::errors::{AppError, CodedKind};
use aphel_sdk::platform::application::organization_admin::ListWindow;
use aphel_sdk::platform::application::query::{
    ArchivedFilter, Filter, FilterOperator, RecordQuery, RecordQueryEngine,
};
use aphel_sdk::platform::application::shared::{RecordRef, RecordType};

pub(crate) struct FieldOption {
    pub(crate) value: String,
    pub(crate) label: String,
}

pub(crate) struct FieldInfo {
    pub(crate) key: String,
    pub(crate) label: String,
    pub(crate) field_type: String,
    pub(crate) custom: bool,
    pub(crate) filterable: bool,
    pub(crate) personal: bool,
    pub(crate) sensitive: bool,
    pub(crate) options: Vec<FieldOption>,
}

pub(crate) async fn fields(ctx: &Ctx<'_>, record_type: &str) -> Result<Vec<FieldInfo>, AppError> {
    let fields = ctx.field_set(ctx.scope, record_type).await?;
    Ok(fields
        .active()
        .map(|field| FieldInfo {
            key: field.key().to_owned(),
            label: field.definition.label.clone(),
            field_type: field.definition.type_name().to_owned(),
            custom: !field.is_system(),
            filterable: field.filterable(),
            personal: field.definition.is_personal,
            sensitive: field.definition.is_sensitive,
            options: field
                .definition
                .options
                .iter()
                .filter(|option| option.is_active)
                .map(|option| FieldOption {
                    value: option.value.clone(),
                    label: option.label.clone(),
                })
                .collect(),
        })
        .collect())
}

fn is_missing(error: &AppError) -> bool {
    matches!(
        error,
        AppError::NotFound(_)
            | AppError::Forbidden(_)
            | AppError::Coded {
                kind: CodedKind::NotFound | CodedKind::Forbidden,
                ..
            }
    )
}

pub(crate) async fn load(
    ctx: &Ctx<'_>,
    record_type: &str,
    id: Uuid,
) -> Result<Option<Value>, AppError> {
    match ctx.load_record(record_type, id).await {
        Ok(value) => Ok(value),
        Err(error) if is_missing(&error) => Ok(None),
        Err(error) => Err(error),
    }
}

pub(crate) async fn count(
    ctx: &Ctx<'_>,
    record_type: &str,
    filter: Option<&Value>,
) -> Result<u64, AppError> {
    let filter = filter
        .cloned()
        .map(serde_json::from_value::<Filter>)
        .transpose()
        .map_err(|error| AppError::validation_code("invalid_filter", error.to_string()))?;
    let listing = RecordQueryEngine::new(&ctx.query())
        .query(
            ctx.scope,
            record_type,
            RecordQuery {
                filter,
                sort: Vec::new(),
                search: None,
                archived: ArchivedFilter::Exclude,
                window: ListWindow {
                    offset: 0,
                    limit: 1,
                },
            },
        )
        .await?;
    Ok(listing.total)
}

fn window(limit: u64) -> ListWindow {
    ListWindow { offset: 0, limit }
}

fn listing_query(filter: Option<Filter>, limit: u64) -> RecordQuery {
    RecordQuery {
        filter,
        window: window(limit),
        ..RecordQuery::default()
    }
}

pub(crate) async fn notes(
    ctx: &Ctx<'_>,
    record_type: &str,
    record_id: Uuid,
    limit: u64,
) -> Result<Vec<Value>, AppError> {
    aphel_sdk::record_context::related(
        ctx,
        aphel_contract_crm::keys::NOTE,
        &RecordRef::new(RecordType::parse(record_type)?, record_id),
        limit,
    )
    .await
}

pub(crate) async fn tasks(
    ctx: &Ctx<'_>,
    record_type: &str,
    record_id: Uuid,
    limit: u64,
) -> Result<Vec<Value>, AppError> {
    aphel_sdk::record_context::related(
        ctx,
        RecordType::ACTIVITY.as_str(),
        &RecordRef::new(RecordType::parse(record_type)?, record_id),
        limit,
    )
    .await
}

fn link_filter(field: &str, record_id: Uuid) -> Option<Filter> {
    Some(Filter::condition(
        field,
        FilterOperator::Eq,
        json!(record_id.to_string()),
    ))
}

pub(crate) async fn deals(
    ctx: &Ctx<'_>,
    link_field: &str,
    record_id: Uuid,
    limit: u64,
) -> Result<Vec<Value>, AppError> {
    Ok(aphel_sdk::record_context::list(
        ctx,
        RecordType::DEAL.as_str(),
        listing_query(link_filter(link_field, record_id), limit),
    )
    .await?
    .items)
}

pub(crate) async fn people(
    ctx: &Ctx<'_>,
    link_field: &str,
    record_id: Uuid,
    limit: u64,
) -> Result<Vec<Value>, AppError> {
    Ok(aphel_sdk::record_context::list(
        ctx,
        RecordType::CONTACT.as_str(),
        listing_query(link_filter(link_field, record_id), limit),
    )
    .await?
    .items)
}
