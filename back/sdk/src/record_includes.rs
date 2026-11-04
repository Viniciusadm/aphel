use crate::{AppError, Ctx};
use aphel_kernel::api::extractors::Includes;
use async_trait::async_trait;
use serde_json::Value;
use std::collections::{BTreeSet, HashMap};
use std::sync::Arc;

#[async_trait]
pub trait RecordIncludeSource: Send + Sync {
    fn key(&self) -> &'static str;
    fn supports(&self, ctx: &Ctx<'_>, record_type: &str) -> bool;
    async fn load(
        &self,
        ctx: &Ctx<'_>,
        record_type: &str,
        ids: &[String],
    ) -> Result<HashMap<String, Value>, AppError>;
}

pub struct RecordIncludes {
    record_type: String,
    sources: Vec<Arc<dyn RecordIncludeSource>>,
}

pub fn record_includes(
    ctx: &Ctx<'_>,
    record_type: &str,
    raw: Option<&str>,
) -> Result<RecordIncludes, AppError> {
    let sources = ctx
        .app()
        .contributions::<dyn RecordIncludeSource>()
        .into_iter()
        .filter(|source| source.supports(ctx, record_type))
        .collect::<Vec<_>>();
    let mut keys = BTreeSet::new();
    for source in &sources {
        if !keys.insert(source.key()) {
            return Err(AppError::Internal(format!(
                "duplicate record include {}",
                source.key()
            )));
        }
    }
    let allowed = keys.into_iter().collect::<Vec<_>>();
    let includes = Includes::parse(raw, &allowed)?;
    Ok(RecordIncludes {
        record_type: record_type.to_owned(),
        sources: sources
            .into_iter()
            .filter(|source| includes.contains(source.key()))
            .collect(),
    })
}

impl RecordIncludes {
    pub async fn enrich(&self, ctx: &Ctx<'_>, records: &mut [Value]) -> Result<(), AppError> {
        if self.sources.is_empty() {
            return Ok(());
        }
        if records.len() > 1000 {
            return Err(AppError::Validation(
                "record includes accept at most 1000 records".to_owned(),
            ));
        }
        let mut ids = Vec::new();
        for record in records.iter() {
            let id = record.get("id").and_then(Value::as_str).ok_or_else(|| {
                AppError::Internal("included record must have a string id".to_owned())
            })?;
            if !ids.iter().any(|known| known == id) {
                ids.push(id.to_owned());
            }
        }
        if ids.is_empty() {
            return Ok(());
        }
        for source in &self.sources {
            let values = source.load(ctx, &self.record_type, &ids).await?;
            if values.keys().any(|id| !ids.contains(id)) {
                return Err(AppError::Internal(format!(
                    "record include {} returned an unrequested id",
                    source.key()
                )));
            }
            for record in records.iter_mut() {
                let id = record.get("id").and_then(Value::as_str).unwrap_or_default();
                let Some(value) = values.get(id) else {
                    continue;
                };
                let object = record.as_object_mut().ok_or_else(|| {
                    AppError::Internal("included record must be an object".to_owned())
                })?;
                if object.contains_key(source.key()) {
                    return Err(AppError::Internal(format!(
                        "record include {} conflicts with a response field",
                        source.key()
                    )));
                }
                object.insert(source.key().to_owned(), value.clone());
            }
        }
        Ok(())
    }
}
