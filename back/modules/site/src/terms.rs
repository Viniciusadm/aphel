use std::collections::BTreeMap;
use std::sync::Arc;

use aphel_contract_site::{PageTerm, SiteExposure};
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
