use async_trait::async_trait;
use serde::Deserialize;
use serde_json::{Value, json};

use crate::domain::Vocabulary;
use crate::store::TagStore;
use crate::vocabularies::{self, UpdateVocabularyRequest, VocabularyInput};
use aphel_contract_tagging::VOCABULARY_PROVISION;
use aphel_kernel::Ctx;
use aphel_kernel::errors::AppError;
use aphel_sdk::provisioning::{ProvisionAction, ProvisionOutcome, ProvisionRequest, Provisioner};

pub(crate) struct VocabularyProvisioner;

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct VocabularyData {
    key: String,
    name: String,
    #[serde(default)]
    hierarchical: bool,
    #[serde(default)]
    public: bool,
}

fn data_of(request: &ProvisionRequest) -> Result<VocabularyData, AppError> {
    serde_json::from_value(request.data.clone()).map_err(|_| {
        AppError::validation_code(
            "plugin_pack_invalid",
            format!(
                "{}: a vocabulary needs key and name, and optionally hierarchical and public",
                request.key
            ),
        )
    })
}

fn applied(vocabulary: &Vocabulary) -> Value {
    json!({
        "key": vocabulary.key,
        "name": vocabulary.name,
        "hierarchical": vocabulary.hierarchical,
        "public": vocabulary.public,
    })
}

#[async_trait]
impl Provisioner for VocabularyProvisioner {
    fn kind(&self) -> &'static str {
        VOCABULARY_PROVISION
    }

    async fn apply(
        &self,
        ctx: &Ctx<'_>,
        request: &ProvisionRequest,
    ) -> Result<ProvisionOutcome, AppError> {
        let wanted = data_of(request)?;
        let current = TagStore::of(ctx).vocabulary_by_key(&wanted.key).await?;
        if let Some(existing) = &request.existing {
            let Some(current) = current else {
                return Ok(ProvisionOutcome {
                    action: ProvisionAction::Removed,
                    id: existing.id.clone(),
                    applied: existing.applied.clone(),
                });
            };
            if applied(&current) != existing.applied {
                return Ok(ProvisionOutcome {
                    action: ProvisionAction::UserChanged,
                    id: existing.id.clone(),
                    applied: existing.applied.clone(),
                });
            }
            let updated = vocabularies::update(
                ctx,
                &wanted.key,
                UpdateVocabularyRequest {
                    name: Some(wanted.name),
                    hierarchical: Some(wanted.hierarchical),
                    public: Some(wanted.public),
                },
                None,
            )
            .await?;
            return Ok(ProvisionOutcome {
                action: ProvisionAction::Updated,
                id: updated.key.clone(),
                applied: applied(&updated),
            });
        }
        if let Some(current) = current {
            return Ok(ProvisionOutcome {
                action: ProvisionAction::Kept,
                id: current.key.clone(),
                applied: applied(&current),
            });
        }
        let created = vocabularies::create(
            ctx,
            VocabularyInput {
                key: wanted.key,
                name: wanted.name,
                hierarchical: wanted.hierarchical,
                public: wanted.public,
            },
            None,
        )
        .await?;
        Ok(ProvisionOutcome {
            action: ProvisionAction::Created,
            id: created.key.clone(),
            applied: applied(&created),
        })
    }
}
