use std::collections::BTreeMap;

use aphel_contract_site::{PREFIX_PARAM, PagePrefix};
use aphel_kernel::errors::AppError;
use aphel_sdk::platform::api::state::AppState;
use serde_json::Value;

use crate::cache;
use crate::data;
use crate::redirects;
use crate::runtime::{Route, SiteRuntime};

const MAX_PREFIXES: u64 = 500;

pub(crate) struct PrefixMatch {
    pub entity: String,
    pub scope: String,
    pub id: String,
    pub record: Value,
}

pub(crate) struct Found<'r> {
    pub route: &'r Route,
    pub params: BTreeMap<String, String>,
    pub prefix: Option<PrefixMatch>,
}

fn starts_with_literal(route: &Route) -> bool {
    !route.pattern.shape().starts_with("/{}")
}

pub(crate) async fn candidates(
    state: &AppState,
    runtime: &SiteRuntime,
    owner: &str,
    prefix: &PagePrefix,
) -> Result<Vec<Value>, AppError> {
    let key = format!("prefix|{}|{owner}|{}", runtime.revision(), prefix.entity);
    if let Some(cached) = cache::get(&key)
        && let Ok(records) = serde_json::from_str::<Vec<Value>>(&cached.html)
    {
        return Ok(records);
    }
    let Some(exposure) = runtime.exposure(owner, &prefix.entity) else {
        return Ok(Vec::new());
    };
    let records = data::matching(
        state,
        exposure,
        &prefix.conditions,
        None,
        MAX_PREFIXES,
        None,
    )
    .await?;
    cache::put(
        key,
        cache::page(
            serde_json::to_string(&records).unwrap_or_default(),
            String::new(),
        ),
        vec![cache::entity_tag(&prefix.entity)],
        None,
    );
    Ok(records)
}

pub(crate) const SITE_PREFIX_FIELD: &str = "site_prefix";

pub(crate) async fn annotate(
    state: &AppState,
    runtime: &SiteRuntime,
    record_type: &str,
    records: &mut [Value],
) -> Result<(), AppError> {
    let Some(route) = runtime
        .prefixed
        .iter()
        .find(|route| route.page.source.entity() == Some(record_type))
    else {
        return Ok(());
    };
    let Some(prefix) = &route.page.prefix else {
        return Ok(());
    };
    let candidates = candidates(state, runtime, &route.owner, prefix).await?;
    for record in records.iter_mut() {
        let segment = record
            .get(&prefix.scope)
            .and_then(Value::as_str)
            .and_then(|id| {
                candidates
                    .iter()
                    .find(|candidate| candidate.get("id").and_then(Value::as_str) == Some(id))
            })
            .and_then(|candidate| candidate.get(&prefix.field))
            .cloned();
        if let (Some(segment), Some(fields)) = (segment, record.as_object_mut()) {
            fields.insert(SITE_PREFIX_FIELD.to_owned(), segment);
        }
    }
    Ok(())
}

fn split(path: &str) -> (&str, String) {
    let trimmed = path.trim_start_matches('/');
    match trimmed.split_once('/') {
        Some((segment, rest)) => (segment, format!("/{rest}")),
        None => (trimmed, "/".to_owned()),
    }
}

pub(crate) async fn moved(
    state: &AppState,
    runtime: &SiteRuntime,
    path: &str,
) -> Result<Option<String>, AppError> {
    let (segment, rest) = split(path);
    if segment.is_empty() || runtime.prefixed.is_empty() {
        return Ok(None);
    }
    let pool = state.db()?;
    for route in &runtime.prefixed {
        let Some(prefix) = &route.page.prefix else {
            continue;
        };
        if route.pattern.matches(&rest).is_none() {
            continue;
        }
        if let Some(target) =
            redirects::lookup(&pool, &prefix.entity, &prefix.field, segment).await?
        {
            return Ok(Some(format!("/{target}{}", rest.trim_end_matches('/'))));
        }
    }
    Ok(None)
}

pub(crate) async fn find<'r>(
    state: &AppState,
    runtime: &'r SiteRuntime,
    path: &str,
) -> Result<Option<Found<'r>>, AppError> {
    let fixed = runtime.find(path);
    if let Some((route, _)) = &fixed
        && starts_with_literal(route)
    {
        return Ok(fixed.map(|(route, params)| Found {
            route,
            params,
            prefix: None,
        }));
    }
    let (segment, rest) = split(path);
    if !segment.is_empty() && !runtime.prefixed.is_empty() {
        if let Ok(pool) = state.db()
            && let Err(error) = cache::sync(&pool).await
        {
            tracing::warn!(%error, "could not read the site cache invalidations");
            cache::forget_all();
        }
        for route in &runtime.prefixed {
            let Some(prefix) = &route.page.prefix else {
                continue;
            };
            let Some(mut params) = route.pattern.matches(&rest) else {
                continue;
            };
            let found = candidates(state, runtime, &route.owner, prefix)
                .await?
                .into_iter()
                .find(|record| record.get(&prefix.field).and_then(Value::as_str) == Some(segment));
            let Some(record) = found else {
                continue;
            };
            let id = record
                .get("id")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_owned();
            params.insert(PREFIX_PARAM.to_owned(), segment.to_owned());
            return Ok(Some(Found {
                route,
                params,
                prefix: Some(PrefixMatch {
                    entity: prefix.entity.clone(),
                    scope: prefix.scope.clone(),
                    id,
                    record,
                }),
            }));
        }
    }
    Ok(fixed.map(|(route, params)| Found {
        route,
        params,
        prefix: None,
    }))
}
