use super::{Mutation, publish, remove_stored_files};
use crate::{PrivacyRequest, PrivacyRequestKind, PrivacyTargets, RunReport};
use aphel_kernel::{
    Ctx,
    application::shared::{Actor, FieldChange, FileStorage, RecordRef, RecordType},
    errors::{AppError, FieldViolation},
};
use aphel_sdk::privacy::{
    DataSubjectAction, PrivacyItem, PrivacySource, Redaction, SourceRequest, SourceTarget,
};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, HashSet},
    sync::Arc,
};
use uuid::Uuid;
pub(crate) async fn available(ctx: &Ctx<'_>) -> Result<Vec<Arc<dyn PrivacySource>>, AppError> {
    let mut result = Vec::new();
    let mut keys = HashSet::new();
    for (_, source) in ctx.app().all_contributions::<dyn PrivacySource>() {
        let key = source.key();
        if key.is_empty()
            || key.len() > 64
            || !key.bytes().all(|byte| {
                byte.is_ascii_lowercase()
                    || byte.is_ascii_digit()
                    || matches!(byte, b'_' | b'.' | b'-')
            })
            || !keys.insert(key)
        {
            return Err(AppError::Configuration(
                "invalid or duplicate privacy source".to_owned(),
            ));
        }
        if source.available(ctx).await? {
            result.push(source);
        }
    }
    Ok(result)
}
pub(crate) fn item_ids(targets: &PrivacyTargets, key: &str) -> Vec<String> {
    targets
        .items
        .iter()
        .filter(|item| item.source == key)
        .map(|item| item.id.clone())
        .collect()
}
pub(crate) async fn validate(
    ctx: &Ctx<'_>,
    targets: &PrivacyTargets,
) -> Result<PrivacyTargets, AppError> {
    let sources = available(ctx).await?;
    let mut result = PrivacyTargets {
        records: targets.records.clone(),
        items: Vec::new(),
    };
    let mut grouped: BTreeMap<&str, Vec<String>> = BTreeMap::new();
    for item in &targets.items {
        if item.id.is_empty() || item.id.len() > 200 || item.id.contains(char::is_control) {
            return Err(AppError::invalid_fields(
                "invalid privacy targets",
                vec![FieldViolation::new(
                    "targets.items",
                    "invalid source item identity",
                )],
            ));
        }
        grouped
            .entry(&item.source)
            .or_default()
            .push(item.id.clone());
    }
    for (key, ids) in grouped {
        let source = sources
            .iter()
            .find(|source| source.key() == key)
            .ok_or_else(|| {
                AppError::invalid_fields(
                    "invalid privacy targets",
                    vec![FieldViolation::new(
                        "targets.items",
                        "the source is not available",
                    )],
                )
            })?;
        for id in source.existing(ctx, &ids).await? {
            if !ids.contains(&id) {
                return Err(AppError::Configuration(
                    "privacy source returned an unrequested identity".to_owned(),
                ));
            }
            if let Some(record_type) = source.entity_type() {
                let record = RecordRef::parse(record_type.as_str(), &id)?;
                if !ctx
                    .registry()
                    .require(record_type.as_str())?
                    .capabilities
                    .has_trait("privacy.personal_data")
                {
                    return Err(AppError::Configuration(
                        "privacy source entity does not declare personal data".to_owned(),
                    ));
                }
                result.records.push(record);
            } else {
                result.items.push(SourceTarget {
                    source: key.to_owned(),
                    id,
                });
            }
        }
    }
    result.records.sort_by(|left, right| {
        (&left.record_type, left.record_id).cmp(&(&right.record_type, right.record_id))
    });
    result.records.dedup();
    result.items.sort();
    result.items.dedup();
    Ok(result)
}
pub(crate) fn validate_items(
    source: &dyn PrivacySource,
    items: &[PrivacyItem],
    limit: u32,
) -> Result<(), AppError> {
    if items.len() > limit as usize
        || items.iter().any(|item| {
            item.target.source != source.key()
                || item.target.id.is_empty()
                || item.target.id.len() > 200
        })
    {
        return Err(AppError::Configuration(
            "privacy source returned invalid discovery items".to_owned(),
        ));
    }
    Ok(())
}
pub(crate) async fn erase(
    ctx: &Ctx<'_>,
    storage: &dyn FileStorage,
    request: &PrivacyRequest,
    redaction: &Redaction,
    report: &mut RunReport,
) -> Result<(), AppError> {
    let sources = available(ctx).await?;
    if request
        .targets
        .items
        .iter()
        .any(|item| !sources.iter().any(|source| source.key() == item.source))
    {
        return Err(AppError::Configuration(
            "a requested privacy source is unavailable".to_owned(),
        ));
    }
    for source in sources {
        let ids = item_ids(&request.targets, source.key());
        let action = match request.kind {
            PrivacyRequestKind::Export => DataSubjectAction::Export,
            PrivacyRequestKind::Anonymize => DataSubjectAction::Anonymize,
            PrivacyRequestKind::Delete => DataSubjectAction::Delete,
        };
        let mut bytes = [0; 16];
        bytes.copy_from_slice(&Sha256::digest(source.key().as_bytes())[..16]);
        let marker = RecordRef::new(
            RecordType::from_static("privacy.source"),
            Uuid::from_bytes(bytes),
        );
        let result=async{
            let mut tx=ctx.begin().await?;
            if !crate::store::PrivacyStore::of(ctx).claim_item(&mut tx,request.id,&marker).await?{return Ok::<_,AppError>(None);}
            let operation=SourceRequest{id:request.id,action,records:&request.targets.records,items:&ids,redaction,at:ctx.now()};
            let outcome=source.erase(ctx,&mut tx,&operation).await?;
            let mentions=source.redact(ctx,&mut tx,redaction).await?;
            publish(ctx,&mut tx,Mutation{actor:Actor::System,event_type:"privacy_request.source_erased".into(),record:RecordRef::new(crate::domain::PRIVACY_REQUEST_RECORD_TYPE,request.id),payload:json!({"source":source.key(),"affected":outcome.affected,"mentions":mentions}),changes:vec![FieldChange::new("source",Value::Null,json!(source.key())),FieldChange::new("affected",Value::Null,json!(outcome.affected))],now:ctx.now(),ip:None}).await?;
            tx.commit().await?;Ok(Some((outcome,mentions)))
        }.await;
        match result {
            Ok(Some((outcome, mentions))) => {
                *report.source_items.entry(source.key().into()).or_default() +=
                    outcome.affected + mentions;
                report.files += remove_stored_files(storage, &outcome.storage_keys).await;
            }
            Ok(None) => report.skipped += 1,
            Err(error) => {
                tracing::warn!(source=source.key(),error=%error,"privacy source erasure failed");
                report.errors += 1;
            }
        }
    }
    Ok(())
}
