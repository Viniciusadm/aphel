use std::collections::HashMap;

use aphel_contract_tagging::{PublicTerm, TermResolver};
use aphel_kernel::errors::AppError;
use async_trait::async_trait;
use uuid::Uuid;

use crate::domain::{Tag, Vocabulary};
use crate::store::TagStore;

pub(crate) struct PublicTerms;

fn store() -> Result<TagStore, AppError> {
    let state = aphel_sdk::runtime()?;
    Ok(TagStore::new(state.as_ref().db()?))
}

async fn public_vocabularies(store: &TagStore) -> Result<HashMap<Uuid, Vocabulary>, AppError> {
    Ok(store
        .vocabularies()
        .await?
        .into_iter()
        .filter(|vocabulary| vocabulary.public)
        .map(|vocabulary| (vocabulary.id, vocabulary))
        .collect())
}

fn public_term(tag: Tag, vocabularies: &HashMap<Uuid, Vocabulary>) -> Option<PublicTerm> {
    let vocabulary = vocabularies.get(&tag.vocabulary_id)?;
    Some(PublicTerm {
        id: tag.id.to_string(),
        vocabulary: vocabulary.key.clone(),
        name: tag.name,
        slug: tag.slug?,
        parent_id: tag.parent_id.map(|id| id.to_string()),
    })
}

#[async_trait]
impl TermResolver for PublicTerms {
    async fn resolve(&self, ids: &[String]) -> Result<Vec<PublicTerm>, AppError> {
        let ids: Vec<Uuid> = ids
            .iter()
            .filter_map(|id| Uuid::parse_str(id.trim()).ok())
            .collect();
        let store = store()?;
        let vocabularies = public_vocabularies(&store).await?;
        let mut found: HashMap<Uuid, Tag> = store
            .find_many(&ids)
            .await?
            .into_iter()
            .map(|tag| (tag.id, tag))
            .collect();
        Ok(ids
            .iter()
            .filter_map(|id| found.remove(id))
            .filter_map(|tag| public_term(tag, &vocabularies))
            .collect())
    }

    async fn find(&self, vocabulary: &str, slug: &str) -> Result<Option<PublicTerm>, AppError> {
        let store = store()?;
        let Some(found) = store
            .vocabulary_by_key(vocabulary)
            .await?
            .filter(|found| found.public)
        else {
            return Ok(None);
        };
        let vocabularies = HashMap::from([(found.id, found.clone())]);
        Ok(store
            .find_by_slug(found.id, slug)
            .await?
            .and_then(|tag| public_term(tag, &vocabularies)))
    }

    async fn descendants(&self, id: &str) -> Result<Vec<String>, AppError> {
        let Ok(root) = Uuid::parse_str(id.trim()) else {
            return Ok(Vec::new());
        };
        let tags = store()?.list().await?;
        let mut found: Vec<Uuid> = Vec::new();
        let mut level = vec![root];
        while !level.is_empty() {
            level = tags
                .iter()
                .filter(|tag| tag.parent_id.is_some_and(|parent| level.contains(&parent)))
                .map(|tag| tag.id)
                .filter(|id| !found.contains(id) && *id != root)
                .collect();
            found.extend(level.iter().copied());
        }
        Ok(found.into_iter().map(|id| id.to_string()).collect())
    }

    async fn vocabulary_name(&self, vocabulary: &str) -> Result<Option<String>, AppError> {
        Ok(public_vocabularies(&store()?)
            .await?
            .into_values()
            .find(|found| found.key == vocabulary)
            .map(|found| found.name))
    }

    async fn terms(&self, vocabulary: &str) -> Result<Vec<PublicTerm>, AppError> {
        let store = store()?;
        let vocabularies = public_vocabularies(&store).await?;
        let Some(found) = vocabularies.values().find(|found| found.key == vocabulary) else {
            return Ok(Vec::new());
        };
        let id = found.id;
        Ok(store
            .list()
            .await?
            .into_iter()
            .filter(|tag| tag.vocabulary_id == id)
            .filter_map(|tag| public_term(tag, &vocabularies))
            .collect())
    }
}
