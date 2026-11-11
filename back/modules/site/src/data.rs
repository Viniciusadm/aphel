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

pub(crate) struct RecordPage {
    pub records: Vec<Value>,
    pub total: u64,
}

fn site_scope() -> AccessScope {
    AccessScope::system().acting_as("site", "public")
}

fn publication_filter(exposure: &SiteExposure, extra: Option<Filter>) -> Option<Filter> {
    let mut conditions: Vec<Filter> = exposure
        .conditions
        .iter()
        .map(|(field, value)| Filter::condition(field.clone(), FilterOperator::Eq, value.clone()))
        .collect();
    conditions.extend(extra);
    match conditions.len() {
        0 => None,
        1 => conditions.pop(),
        _ => Some(Filter::and(conditions)),
    }
}

fn filled(value: &Value) -> bool {
    match value {
        Value::Null => false,
        Value::String(text) => !text.trim().is_empty(),
        _ => true,
    }
}

fn project(exposure: &SiteExposure, record: Value, suffix: Option<&str>) -> Value {
    let Value::Object(mut record) = record else {
        return Value::Null;
    };
    if let Some(Value::Object(custom)) = record.remove("custom_fields") {
        for (field, value) in custom {
            record.entry(field).or_insert(value);
        }
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
            .map(|record| project(exposure, record, suffix))
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
    )
    .await
}
