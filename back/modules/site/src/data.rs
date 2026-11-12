use aphel_contract_site::SiteExposure;
use aphel_kernel::errors::AppError;
use aphel_kernel::records::{RecordCtx, query_ports};
use aphel_sdk::platform::api::state::AppState;
use aphel_sdk::platform::application::fields::translations_field;
use aphel_sdk::platform::application::organization_admin::ListWindow;
use aphel_sdk::platform::application::query::{
    ArchivedFilter, Filter, FilterOperator, RecordQuery, RecordQueryEngine, parse_sort,
};
use aphel_sdk::platform::domain::access::AccessScope;
use serde_json::{Map, Value};
use std::collections::BTreeMap;

const SINGLE_SORT: &str = "-updated_at";

pub(crate) struct RecordPage {
    pub records: Vec<Value>,
    pub total: u64,
}

fn site_scope() -> AccessScope {
    AccessScope::system().acting_as("site", "public")
}

pub(crate) struct Draft<'a> {
    pub record_id: &'a str,
    pub fields: &'a BTreeMap<String, String>,
    pub values: &'a BTreeMap<String, Value>,
}

fn equalities(conditions: &BTreeMap<String, Value>) -> Vec<Filter> {
    conditions
        .iter()
        .map(|(field, value)| {
            let operator = if value.is_array() {
                FilterOperator::In
            } else {
                FilterOperator::Eq
            };
            Filter::condition(field.clone(), operator, value.clone())
        })
        .collect()
}

fn combined(mut conditions: Vec<Filter>) -> Option<Filter> {
    match conditions.len() {
        0 => None,
        1 => conditions.pop(),
        _ => Some(Filter::and(conditions)),
    }
}

fn nothing() -> Filter {
    Filter::condition("id".to_owned(), FilterOperator::IsEmpty, Value::Null)
}

fn publication_filter(exposure: &SiteExposure, extra: Option<Filter>) -> Option<Filter> {
    let mut conditions = equalities(&exposure.conditions);
    if let Some(filter) = &exposure.filter {
        match serde_json::from_value::<Filter>(filter.clone()) {
            Ok(filter) => conditions.push(filter),
            Err(error) => {
                tracing::warn!(entity = %exposure.entity, %error, "invalid site exposure filter; nothing is public");
                conditions.push(nothing());
            }
        }
    }
    conditions.extend(extra);
    combined(conditions)
}

fn filled(value: &Value) -> bool {
    match value {
        Value::Null => false,
        Value::String(text) => !text.trim().is_empty(),
        _ => true,
    }
}

fn translated(record: &Map<String, Value>, field: &str, language: &str) -> Option<Value> {
    record
        .get(&translations_field(field))
        .and_then(Value::as_str)
        .and_then(|text| serde_json::from_str::<Map<String, Value>>(text).ok())
        .and_then(|map| map.get(language).cloned())
        .filter(filled)
}

fn project(
    exposure: &SiteExposure,
    record: Value,
    translation: Option<&str>,
    drafts: Option<&Draft<'_>>,
) -> Value {
    let Value::Object(mut record) = record else {
        return Value::Null;
    };
    if let Some(Value::Object(custom)) = record.remove("custom_fields") {
        for (field, value) in custom {
            record.entry(field).or_insert(value);
        }
    }
    if let Some(drafts) = drafts {
        for (target, source) in drafts.fields {
            let value = record.get(source).cloned().unwrap_or(Value::Null);
            record.insert(target.clone(), value);
        }
    }
    if let Some(language) = translation {
        for field in &exposure.fields {
            if let Some(value) = translated(&record, field, language) {
                record.insert(field.clone(), value);
            }
        }
    }
    if let Some(drafts) = drafts {
        for (target, value) in drafts.values {
            record.insert(target.clone(), value.clone());
        }
    }
    Value::Object(
        record
            .into_iter()
            .filter(|(field, _)| exposure.exposes(field))
            .collect::<Map<String, Value>>(),
    )
}

async fn query(
    state: &AppState,
    exposure: &SiteExposure,
    query: RecordQuery,
    translation: Option<&str>,
    drafts: Option<&Draft<'_>>,
) -> Result<RecordPage, AppError> {
    let scope = site_scope();
    let ports = query_ports(state).await?;
    let listing = RecordQueryEngine::new(&ports)
        .query(&scope, &exposure.entity, query)
        .await?;
    let loaded = state
        .registry()
        .await?
        .handler(&exposure.entity)?
        .load_listed(&RecordCtx::new(&scope, state), &listing.items)
        .await?;
    Ok(RecordPage {
        records: loaded
            .into_iter()
            .map(|record| project(exposure, record, translation, drafts))
            .collect(),
        total: listing.total,
    })
}

pub(crate) type Restriction<'a> = Option<(&'a str, &'a str)>;

fn restricted(extra: Option<Filter>, restriction: Restriction<'_>) -> Option<Filter> {
    let owned = restriction.map(|(field, value)| {
        Filter::condition(field, FilterOperator::Eq, Value::String(value.to_owned()))
    });
    match (extra, owned) {
        (Some(extra), Some(owned)) => Some(Filter::and(vec![extra, owned])),
        (extra, owned) => extra.or(owned),
    }
}

pub(crate) async fn record(
    state: &AppState,
    exposure: &SiteExposure,
    field: &str,
    value: &str,
    restriction: Restriction<'_>,
    translation: Option<&str>,
) -> Result<Option<Value>, AppError> {
    let found = query(
        state,
        exposure,
        RecordQuery {
            filter: publication_filter(
                exposure,
                restricted(
                    Some(Filter::condition(
                        field,
                        FilterOperator::Eq,
                        Value::String(value.to_owned()),
                    )),
                    restriction,
                ),
            ),
            sort: Vec::new(),
            search: None,
            archived: ArchivedFilter::Exclude,
            window: ListWindow {
                offset: 0,
                limit: 1,
            },
        },
        translation,
        None,
    )
    .await?;
    Ok(found.records.into_iter().next())
}

pub(crate) async fn list(
    state: &AppState,
    exposure: &SiteExposure,
    sort: Option<&str>,
    page: u64,
    per_page: u64,
    restriction: Restriction<'_>,
    translation: Option<&str>,
) -> Result<RecordPage, AppError> {
    list_in(
        state,
        exposure,
        (sort, page, per_page),
        restriction,
        None,
        translation,
    )
    .await
}

pub(crate) fn any_of(field: &str, ids: &[String]) -> Filter {
    Filter::condition(
        field,
        FilterOperator::In,
        Value::Array(ids.iter().cloned().map(Value::String).collect()),
    )
}

pub(crate) async fn list_in(
    state: &AppState,
    exposure: &SiteExposure,
    (sort, page, per_page): (Option<&str>, u64, u64),
    restriction: Restriction<'_>,
    extra: Option<Filter>,
    translation: Option<&str>,
) -> Result<RecordPage, AppError> {
    query(
        state,
        exposure,
        RecordQuery {
            filter: publication_filter(exposure, restricted(extra, restriction)),
            sort: sort.map(parse_sort).unwrap_or_default(),
            search: None,
            archived: ArchivedFilter::Exclude,
            window: ListWindow {
                offset: page.saturating_sub(1).saturating_mul(per_page),
                limit: per_page,
            },
        },
        translation,
        None,
    )
    .await
}

async fn first(
    state: &AppState,
    exposure: &SiteExposure,
    filter: Option<Filter>,
    translation: Option<&str>,
    drafts: Option<&Draft<'_>>,
) -> Result<Option<Value>, AppError> {
    let found = query(
        state,
        exposure,
        RecordQuery {
            filter,
            sort: parse_sort(SINGLE_SORT),
            search: None,
            archived: ArchivedFilter::Exclude,
            window: ListWindow {
                offset: 0,
                limit: 1,
            },
        },
        translation,
        drafts,
    )
    .await?;
    Ok(found.records.into_iter().next())
}

pub(crate) async fn single(
    state: &AppState,
    exposure: &SiteExposure,
    conditions: &BTreeMap<String, Value>,
    translation: Option<&str>,
) -> Result<Option<Value>, AppError> {
    let page_conditions = combined(equalities(conditions));
    first(
        state,
        exposure,
        publication_filter(exposure, page_conditions),
        translation,
        None,
    )
    .await
}

pub(crate) async fn draft(
    state: &AppState,
    exposure: &SiteExposure,
    draft: &Draft<'_>,
    conditions: &BTreeMap<String, Value>,
    translation: Option<&str>,
) -> Result<Option<Value>, AppError> {
    let mut filters = equalities(conditions);
    filters.push(Filter::condition(
        "id",
        FilterOperator::Eq,
        Value::String(draft.record_id.to_owned()),
    ));
    first(state, exposure, combined(filters), translation, Some(draft)).await
}

pub(crate) async fn matching(
    state: &AppState,
    exposure: &SiteExposure,
    conditions: &BTreeMap<String, Value>,
    sort: Option<&str>,
    limit: u64,
    translation: Option<&str>,
) -> Result<Vec<Value>, AppError> {
    let page_conditions = combined(equalities(conditions));
    Ok(query(
        state,
        exposure,
        RecordQuery {
            filter: publication_filter(exposure, page_conditions),
            sort: sort.map(parse_sort).unwrap_or_default(),
            search: None,
            archived: ArchivedFilter::Exclude,
            window: ListWindow { offset: 0, limit },
        },
        translation,
        None,
    )
    .await?
    .records)
}

pub(crate) async fn count(
    state: &AppState,
    exposure: &SiteExposure,
    filter: Filter,
) -> Result<u64, AppError> {
    let scope = site_scope();
    let ports = query_ports(state).await?;
    let listing = RecordQueryEngine::new(&ports)
        .query(
            &scope,
            &exposure.entity,
            RecordQuery {
                filter: Some(filter),
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

pub(crate) async fn neighbour(
    state: &AppState,
    exposure: &SiteExposure,
    (published_at, id): (&str, &str),
    older: bool,
    restriction: Restriction<'_>,
    translation: Option<&str>,
) -> Result<Option<Value>, AppError> {
    let (operator, sort) = match older {
        true => (FilterOperator::Lt, "-published_at"),
        false => (FilterOperator::Gt, "published_at"),
    };
    let extra = Filter::and(vec![
        Filter::condition(
            "published_at",
            operator,
            Value::String(published_at.to_owned()),
        ),
        Filter::condition("id", FilterOperator::Neq, Value::String(id.to_owned())),
    ]);
    let page = list_in(
        state,
        exposure,
        (Some(sort), 1, 1),
        restriction,
        Some(extra),
        translation,
    )
    .await?;
    Ok(page.records.into_iter().next())
}
