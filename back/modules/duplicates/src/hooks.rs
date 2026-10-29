use async_trait::async_trait;
use sqlx::{MySql, QueryBuilder};
use uuid::Uuid;

use crate::KEY;
use crate::service;
use crate::store::DuplicateStore;
use aphel_kernel::Ctx;
use aphel_kernel::application::fields::RecordTypeDefinition;
use aphel_kernel::application::shared::RecordType;
use aphel_kernel::errors::AppError;
use aphel_kernel::modules::HookCtx;
use aphel_kernel::records::{DuplicateDetector, PossibleDuplicate, RemovedRecord};

pub(crate) struct RuleDuplicateDetector;

#[async_trait]
impl DuplicateDetector for RuleDuplicateDetector {
    async fn detect(
        &self,
        ctx: &Ctx<'_>,
        record_type: &RecordType,
        record_id: &str,
    ) -> Vec<PossibleDuplicate> {
        if !ctx.app().is_enabled(KEY) {
            return Vec::new();
        }
        let Ok(id) = Uuid::parse_str(record_id) else {
            return Vec::new();
        };
        match service::detect_for_record(ctx, record_type, id).await {
            Ok(matches) => matches.into_iter().map(Into::into).collect(),
            Err(error) => {
                tracing::warn!("duplicate check after creation failed: {error}");
                Vec::new()
            }
        }
    }

    async fn merged_into(
        &self,
        ctx: &Ctx<'_>,
        record_type: &RecordType,
        record_id: &str,
    ) -> Result<Option<String>, AppError> {
        if !ctx.app().is_enabled(KEY) {
            return Ok(None);
        }
        Ok(service::redirect(ctx, record_type, record_id)
            .await?
            .map(|id| id.to_string()))
    }
}

pub(crate) async fn forget_purged_record(
    ctx: &mut HookCtx<'_>,
    removed: &RemovedRecord,
) -> Result<(), AppError> {
    DuplicateStore::forget_record(
        ctx.tx,
        removed.record_type.as_str(),
        removed.record.record_id,
    )
    .await
}

pub(crate) struct MergedRecords;

impl trash::TrashExclusion for MergedRecords {
    fn push_hidden(
        &self,
        builder: &mut QueryBuilder<MySql>,
        definition: &RecordTypeDefinition,
        alias: &str,
    ) {
        builder
            .push(" AND NOT EXISTS (SELECT 1 FROM record_merges m WHERE m.record_type = ")
            .push_bind(definition.record_type.as_str().to_owned())
            .push(format!(" AND m.merged_id = {alias}.id)"));
    }
}
