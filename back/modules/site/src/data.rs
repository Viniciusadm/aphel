use aphel_contract_site::SiteExposure;
use aphel_kernel::errors::AppError;
use aphel_kernel::records::{RecordCtx, query_ports};
use aphel_sdk::platform::api::state::AppState;
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
}

fn equalities(conditions: &BTreeMap<String, Value>) -> Vec<Filter> {
    conditions
        .iter()
        .map(|(field, value)| Filter::condition(field.clone(), FilterOperator::Eq, value.clone()))
        .collect()
}

fn combined(mut conditions: Vec<Filter>) -> Option<Filter> {
    match conditions.len() {
        0 => None,
        1 => conditions.pop(),
        _ => Some(Filter::and(conditions)),
    }
}

fn publication_filter(exposure: &SiteExposure, extra: Option<Filter>) -> Option<Filter> {
    let mut conditions = equalities(&exposure.conditions);
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

fn swap_drafts(
    record: &mut Map<String, Value>,
    fields: &BTreeMap<String, String>,
    suffix: Option<&str>,
) {
    for (target, source) in fields {
        let pairs = std::iter::once((target.clone(), source.clone())).chain(
            suffix.map(|suffix| (format!("{target}_{suffix}"), format!("{source}_{suffix}"))),
        );
        for (target, source) in pairs {
            let value = record.get(&source).cloned().unwrap_or(Value::Null);
            record.insert(target, value);
        }
    }
}

fn project(
    exposure: &SiteExposure,
    record: Value,
    suffix: Option<&str>,
    drafts: Option<&BTreeMap<String, String>>,
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
        swap_drafts(&mut record, drafts, suffix);
    }
    if let Some(suffix) = suffix {
        for field in &exposure.translated {
            if let Some(translation) = record
                .get(&format!("{field}_{suffix}"))
                .filter(|value| filled(value))
                .cloned()
            {
                record.insert(field.clone(), translation);
            }
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
    suffix: Option<&str>,
    drafts: Option<&BTreeMap<String, String>>,
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
            .map(|record| project(exposure, record, suffix, drafts))
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
    suffix: Option<&str>,
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
        suffix,
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
    suffix: Option<&str>,
) -> Result<RecordPage, AppError> {
    query(
        state,
        exposure,
        RecordQuery {
            filter: publication_filter(exposure, restricted(None, restriction)),
            sort: sort.map(parse_sort).unwrap_or_default(),
            search: None,
            archived: ArchivedFilter::Exclude,
            window: ListWindow {
                offset: page.saturating_sub(1).saturating_mul(per_page),
                limit: per_page,
            },
        },
        suffix,
        None,
    )
    .await
}

async fn first(
    state: &AppState,
    exposure: &SiteExposure,
    filter: Option<Filter>,
    suffix: Option<&str>,
    drafts: Option<&BTreeMap<String, String>>,
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
        suffix,
        drafts,
    )
    .await?;
    Ok(found.records.into_iter().next())
}

pub(crate) async fn single(
    state: &AppState,
    exposure: &SiteExposure,
    conditions: &BTreeMap<String, Value>,
    suffix: Option<&str>,
) -> Result<Option<Value>, AppError> {
    let page_conditions = combined(equalities(conditions));
    first(
        state,
        exposure,
        publication_filter(exposure, page_conditions),
        suffix,
        None,
    )
    .await
}

pub(crate) async fn draft(
    state: &AppState,
    exposure: &SiteExposure,
    draft: &Draft<'_>,
    conditions: &BTreeMap<String, Value>,
    suffix: Option<&str>,
) -> Result<Option<Value>, AppError> {
    let mut filters = equalities(conditions);
    filters.push(Filter::condition(
        "id",
        FilterOperator::Eq,
        Value::String(draft.record_id.to_owned()),
    ));
    first(
        state,
        exposure,
        combined(filters),
        suffix,
        Some(draft.fields),
    )
    .await
}

pub(crate) async fn matching(
    state: &AppState,
    exposure: &SiteExposure,
    conditions: &BTreeMap<String, Value>,
    sort: Option<&str>,
    limit: u64,
    suffix: Option<&str>,
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
        suffix,
        None,
    )
    .await?
    .records)
}
