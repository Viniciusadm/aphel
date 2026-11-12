use std::collections::BTreeSet;

use aphel_kernel::errors::AppError;
use aphel_sdk::platform::api::state::AppState;
use aphel_sdk::platform::application::fields::translations_field;
use aphel_sdk::platform::application::query::{Filter, FilterOperator};
use serde_json::{Value, json};

use aphel_contract_site::SEARCH_PATHS;

use crate::data;
use crate::runtime::SiteRuntime;

pub(crate) const SEARCH_TEMPLATE: &str = "aphel/search.html";
const SEARCHABLE_FIELDS: [&str; 5] = ["title", "name", "summary", "description", "seo_description"];
const TITLE_FIELDS: [&str; 2] = ["title", "name"];
const SUMMARY_FIELDS: [&str; 3] = ["seo_description", "summary", "description"];
const MIN_QUERY: usize = 2;
const MAX_QUERY: usize = 100;
const RESULTS_PER_SOURCE: u64 = 10;
const MAX_RESULTS: usize = 30;

pub(crate) fn search_path(locale: &str) -> &'static str {
    match locale.starts_with("pt") {
        true => SEARCH_PATHS[0],
        false => SEARCH_PATHS[1],
    }
}

pub(crate) fn is_search_path(path: &str) -> bool {
    SEARCH_PATHS.contains(&path.trim_end_matches('/'))
}

pub(crate) fn requested_query(query: Option<&str>) -> String {
    form_urlencoded::parse(query.unwrap_or_default().as_bytes())
        .find(|(name, _)| name == "q")
        .map(|(_, value)| value.trim().chars().take(MAX_QUERY).collect())
        .unwrap_or_default()
}

fn first_text(record: &Value, fields: &[&str]) -> Option<String> {
    fields.iter().find_map(|field| {
        record
            .get(*field)
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|text| !text.is_empty())
            .map(str::to_owned)
    })
}

pub(crate) async fn results(
    state: &AppState,
    runtime: &SiteRuntime,
    query: &str,
    language_prefix: &str,
    translation: Option<&str>,
) -> Result<Vec<Value>, AppError> {
    if query.chars().count() < MIN_QUERY {
        return Ok(Vec::new());
    }
    let registry = state.registry().await?;
    let mut seen = BTreeSet::new();
    let mut found = Vec::new();
    for (_, exposure) in runtime.exposures() {
        if !seen.insert(exposure.entity.clone()) {
            continue;
        }
        let definition = registry.require(&exposure.entity).ok();
        let conditions: Vec<Filter> = SEARCHABLE_FIELDS
            .iter()
            .filter(|field| exposure.fields.iter().any(|exposed| exposed == *field))
            .flat_map(|field| {
                let translated = translation
                    .map(|_| translations_field(field))
                    .filter(|key| {
                        definition
                            .is_some_and(|definition| definition.system_field_by_key(key).is_some())
                    });
                std::iter::once((*field).to_owned()).chain(translated)
            })
            .map(|field| {
                Filter::condition(
                    field,
                    FilterOperator::Contains,
                    Value::String(query.to_owned()),
                )
            })
            .collect();
        if conditions.is_empty() {
            continue;
        }
        let mut listed = data::list_in(
            state,
            exposure,
            (None, 1, RESULTS_PER_SOURCE),
            None,
            Some(Filter::or(conditions)),
            translation,
        )
        .await?;
        crate::prefixes::annotate(state, runtime, &exposure.entity, &mut listed.records).await?;
        for record in &listed.records {
            let Some(path) = crate::targets::record_path(runtime, &exposure.entity, record) else {
                continue;
            };
            let Some(title) = first_text(record, &TITLE_FIELDS) else {
                continue;
            };
            found.push(json!({
                "title": title,
                "summary": first_text(record, &SUMMARY_FIELDS),
                "url": format!("{language_prefix}{path}"),
                "published_at": record.get("published_at"),
            }));
        }
        if found.len() >= MAX_RESULTS {
            break;
        }
    }
    found.truncate(MAX_RESULTS);
    Ok(found)
}
