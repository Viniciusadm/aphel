use std::collections::BTreeMap;
use std::sync::Arc;

use aphel_contract_site::{PageTerm, SiteExposure, TERM_PARAM};
use aphel_contract_tagging::{CLASSIFIABLE, ClassifiableDefinition, PublicTerm, TermResolver};
use aphel_kernel::errors::AppError;
use aphel_sdk::platform::api::state::AppState;
use serde_json::{Value, json};

use crate::cache;

pub(crate) const SITE_TERMS_FIELD: &str = "site_terms";
pub(crate) const TERM_ENTITIES: [&str; 2] = ["tagging.tag", "tagging.vocabulary"];

pub(crate) struct TermMatch {
    pub term: PublicTerm,
    pub ids: Vec<String>,
}

fn resolver(state: &AppState) -> Option<Arc<dyn TermResolver>> {
    state
        .app
        .contributions::<dyn TermResolver>()
        .into_iter()
        .next()
}

pub(crate) fn cache_tags() -> Vec<String> {
    TERM_ENTITIES
        .iter()
        .map(|entity| cache::entity_tag(entity))
        .collect()
}

fn published_field(state: &AppState, entity: &str) -> Option<String> {
    let native = state
        .app
        .contributions::<ClassifiableDefinition>()
        .into_iter()
        .map(|definition| (*definition).clone());
    let runtime = state
        .app
        .runtime_contributions(CLASSIFIABLE)
        .into_iter()
        .filter_map(|contribution| {
            serde_json::from_value::<ClassifiableDefinition>(contribution.data.clone()).ok()
        });
    native
        .chain(runtime)
        .find(|definition| definition.entity == entity)
        .and_then(|definition| definition.published)
}

fn ids_in(value: Option<&Value>) -> Vec<String> {
    value
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .map(str::to_owned)
        .collect()
}

pub(crate) async fn annotate(
    state: &AppState,
    exposure: &SiteExposure,
    records: &mut [Value],
    tags: &mut Vec<String>,
) -> Result<(), AppError> {
    let Some(resolver) = resolver(state) else {
        return Ok(());
    };
    let Some(field) = published_field(state, &exposure.entity) else {
        return Ok(());
    };
    if !exposure.exposes(&field) {
        return Ok(());
    }
    let mut ids: Vec<String> = records
        .iter()
        .flat_map(|record| ids_in(record.get(&field)))
        .collect();
    ids.sort();
    ids.dedup();
    let terms: BTreeMap<String, PublicTerm> = if ids.is_empty() {
        BTreeMap::new()
    } else {
        resolver
            .resolve(&ids)
            .await?
            .into_iter()
            .map(|term| (term.id.clone(), term))
            .collect()
    };
    for record in records.iter_mut() {
        let selected: Vec<&PublicTerm> = ids_in(record.get(&field))
            .iter()
            .filter_map(|id| terms.get(id))
            .collect();
        if let Some(fields) = record.as_object_mut() {
            fields.insert(SITE_TERMS_FIELD.to_owned(), json!(selected));
        }
    }
    tags.extend(cache_tags());
    Ok(())
}

pub(crate) async fn find(
    state: &AppState,
    page_term: &PageTerm,
    slug: &str,
) -> Result<Option<TermMatch>, AppError> {
    let Some(resolver) = resolver(state) else {
        return Ok(None);
    };
    let Some(term) = resolver.find(&page_term.vocabulary, slug).await? else {
        return Ok(None);
    };
    let mut ids = vec![term.id.clone()];
    ids.extend(resolver.descendants(&term.id).await?);
    Ok(Some(TermMatch { term, ids }))
}

pub(crate) async fn slugs(state: &AppState, page_term: &PageTerm) -> Result<Vec<String>, AppError> {
    let Some(resolver) = resolver(state) else {
        return Ok(Vec::new());
    };
    Ok(resolver
        .terms(&page_term.vocabulary)
        .await?
        .into_iter()
        .map(|term| term.slug)
        .collect())
}

pub(crate) async fn vocabularies(
    state: &AppState,
    runtime: &crate::runtime::SiteRuntime,
    entity: &str,
) -> Result<BTreeMap<String, Vec<PublicTerm>>, AppError> {
    let Some(resolver) = resolver(state) else {
        return Ok(BTreeMap::new());
    };
    let mut found = BTreeMap::new();
    for route in runtime.routes.iter().chain(&runtime.prefixed) {
        let Some(page_term) = &route.page.term else {
            continue;
        };
        if route.page.source.entity() != Some(entity) || found.contains_key(&page_term.vocabulary) {
            continue;
        }
        found.insert(
            page_term.vocabulary.clone(),
            resolver.terms(&page_term.vocabulary).await?,
        );
    }
    Ok(found)
}

pub(crate) async fn route_path(
    state: &AppState,
    runtime: &crate::runtime::SiteRuntime,
    vocabulary: &str,
    slug: &str,
) -> Result<Option<String>, AppError> {
    let params = BTreeMap::from([(TERM_PARAM.to_owned(), slug.to_owned())]);
    let shows = |route: &&crate::runtime::Route| {
        route
            .page
            .term
            .as_ref()
            .is_some_and(|page_term| page_term.vocabulary == vocabulary)
    };
    if let Some(route) = runtime.routes.iter().find(shows) {
        return Ok(route.pattern.build(&params));
    }
    let Some(route) = runtime.prefixed.iter().find(shows) else {
        return Ok(None);
    };
    let Some(prefix) = &route.page.prefix else {
        return Ok(None);
    };
    let candidates = crate::prefixes::candidates(state, runtime, &route.owner, prefix).await?;
    let Some(segment) = candidates
        .iter()
        .find_map(|candidate| candidate.get(&prefix.field).and_then(Value::as_str))
    else {
        return Ok(None);
    };
    Ok(route
        .pattern
        .build(&params)
        .map(|detail| format!("/{segment}{}", detail.trim_end_matches('/'))))
}

pub(crate) async fn find_public(
    state: &AppState,
    vocabulary: &str,
    slug: &str,
) -> Result<Option<PublicTerm>, AppError> {
    match resolver(state) {
        Some(resolver) => resolver.find(vocabulary, slug).await,
        None => Ok(None),
    }
}

pub(crate) async fn linkable(
    state: &AppState,
    runtime: &crate::runtime::SiteRuntime,
) -> Result<Vec<(String, String, Vec<PublicTerm>)>, AppError> {
    let Some(resolver) = resolver(state) else {
        return Ok(Vec::new());
    };
    let mut found: Vec<(String, String, Vec<PublicTerm>)> = Vec::new();
    for route in runtime.routes.iter().chain(&runtime.prefixed) {
        let Some(page_term) = &route.page.term else {
            continue;
        };
        if found
            .iter()
            .any(|(vocabulary, ..)| vocabulary == &page_term.vocabulary)
        {
            continue;
        }
        let name = resolver
            .vocabulary_name(&page_term.vocabulary)
            .await?
            .unwrap_or_else(|| page_term.vocabulary.clone());
        let group = format!("{} · {name}", runtime.owner_name(&route.owner));
        found.push((
            page_term.vocabulary.clone(),
            group,
            resolver.terms(&page_term.vocabulary).await?,
        ));
    }
    Ok(found)
}
