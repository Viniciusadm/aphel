use std::sync::Arc;

use async_trait::async_trait;
use serde_json::Value;
use sqlx::MySqlPool;
use uuid::Uuid;

use crate::application::custom_objects::RecordLinkRepository;
use crate::application::fields::CustomFieldValueRepository;
use crate::application::fields::{
    FieldSchema, PurgedRecord, RecordDraft, RecordPipeline, RecordTypeRegistry, RecordWrite,
};
use crate::application::query::{PlanNode, QueryPlan};
use crate::application::shared::{RecordRef, RecordType};
use crate::database::transaction::MySqlTx;
use crate::domain::access::AccessScope;
use crate::errors::AppError;
use crate::infrastructure::custom_objects::MySqlRecordLinkRepository;
use crate::infrastructure::fields::MySqlCustomFieldValueRepository;
use crate::infrastructure::storage::references::delete_unreferenced_files;
use crate::modules::builder::KERNEL_MODULE;
use crate::modules::{App, HookCtx, HookRef};
use crate::records::hooks::{
    ChangedRecord, QueryConstraints, RecordArchiving, RecordCommitted, RecordDeleted,
    RecordDeleting, RecordPurged, RecordQueryFilter, RecordRestored, RecordSaved, RecordSaving,
    RecordSchema, RecordSchemaFilter, RecordSerializing, RecordValidating, RemovedRecord,
    SavingRecord, SearchPlanFilter, SerializedRecords,
};

#[derive(Clone)]
pub struct MySqlRecordPipeline {
    app: Arc<App>,
    pool: MySqlPool,
    registry: Arc<RecordTypeRegistry>,
}

impl MySqlRecordPipeline {
    pub fn new(pool: MySqlPool, registry: Arc<RecordTypeRegistry>) -> Self {
        Self {
            app: crate::modules::app().clone(),
            pool,
            registry,
        }
    }

    async fn run<K: HookRef>(
        &self,
        hook: K,
        tx: &mut MySqlTx,
        scope: &AccessScope,
        args: &K::Args,
    ) -> Result<(), AppError> {
        let hooks = self.app.hooks();
        if !hooks.has(hook) {
            return Ok(());
        }
        let mut ctx = HookCtx::new(tx, &self.pool, scope, self.app.services(), KERNEL_MODULE);
        hooks.run(hook, &mut ctx, args).await
    }

    fn changed(&self, write: &RecordWrite<'_>) -> ChangedRecord {
        ChangedRecord::new(write, self.registry.clone())
    }

    fn removed(&self, record: &RecordRef) -> RemovedRecord {
        RemovedRecord {
            record_type: record.record_type.clone(),
            record: record.clone(),
            registry: self.registry.clone(),
        }
    }
}

#[async_trait]
impl RecordPipeline for MySqlRecordPipeline {
    type Tx = MySqlTx;

    async fn saving(
        &self,
        tx: &mut MySqlTx,
        scope: &AccessScope,
        draft: RecordDraft,
    ) -> Result<RecordDraft, AppError> {
        let key = (
            draft.record.record_type.as_str().to_owned(),
            draft.record.record_id,
        );
        let before = if matches!(draft.mode, crate::application::fields::WriteMode::Create) {
            None
        } else {
            crate::records::snapshot::capture(tx, &self.pool, &self.registry, &draft.record).await?
        };
        let proposed = crate::records::snapshot::proposed(
            before.as_ref(),
            &draft.values,
            &draft.custom_fields,
        );
        tx.record_states.insert(
            key,
            serde_json::json!({
                "operation_id": tx.operation_id, "record_type": draft.record.record_type.as_str(),
                "record_id": draft.record.record_id, "phase": "record.saving", "before": before,
                "proposed": proposed, "before_available": true, "persisted": null,
            }),
        );
        let hooks = self.app.hooks();
        if !hooks.has(RecordSaving) {
            return Ok(draft);
        }
        let mut ctx = HookCtx::new(tx, &self.pool, scope, self.app.services(), KERNEL_MODULE);
        let saving = SavingRecord {
            record_type: draft.record.record_type.clone(),
            draft,
            registry: self.registry.clone(),
        };
        Ok(hooks.apply(RecordSaving, &mut ctx, saving).await?.draft)
    }

    async fn validate(
        &self,
        tx: &mut MySqlTx,
        scope: &AccessScope,
        write: &RecordWrite<'_>,
    ) -> Result<(), AppError> {
        let stored =
            crate::records::snapshot::capture(tx, &self.pool, &self.registry, write.record).await?;
        let proposed = serde_json::json!({
            "values": write.values,
            "custom_fields": stored.as_ref().and_then(|value| value.get("custom_fields")).cloned().unwrap_or_else(|| serde_json::json!({})),
        });
        let key = (
            write.record.record_type.as_str().to_owned(),
            write.record.record_id,
        );
        let operation_id = tx.operation_id;
        let state = tx.record_states.entry(key).or_insert_with(|| serde_json::json!({
            "operation_id": operation_id, "record_type": write.record.record_type.as_str(),
            "record_id": write.record.record_id, "before": null, "before_available": false, "persisted": null,
        }));
        state["proposed"] = proposed;
        state["phase"] = serde_json::json!("record.validating");
        let mut reference_values = stored
            .as_ref()
            .and_then(|value| value.get("custom_fields"))
            .and_then(Value::as_object)
            .cloned()
            .unwrap_or_default();
        if let Some(values) = write.values.as_object() {
            reference_values.extend(values.clone());
        }
        crate::records::references::validate(
            tx,
            scope,
            &self.registry,
            write.record,
            &Value::Object(reference_values),
        )
        .await?;
        self.run(RecordValidating, tx, scope, &self.changed(write))
            .await
    }

    async fn saved(
        &self,
        tx: &mut MySqlTx,
        scope: &AccessScope,
        write: &RecordWrite<'_>,
    ) -> Result<(), AppError> {
        let proposed =
            crate::records::snapshot::capture(tx, &self.pool, &self.registry, write.record).await?;
        if let Some(state) = tx.record_states.get_mut(&(
            write.record.record_type.as_str().to_owned(),
            write.record.record_id,
        )) {
            state["proposed"] = proposed.unwrap_or(serde_json::Value::Null);
            state["phase"] = serde_json::json!("record.saved");
        }
        self.run(RecordSaved, tx, scope, &self.changed(write)).await
    }

    async fn committed(&self, scope: &AccessScope, write: &RecordWrite<'_>) {
        if !self.app.hooks().has(RecordCommitted) {
            return;
        }
        let args = self.changed(write);
        let outcome = async {
            let mut tx = MySqlTx::from(self.pool.begin().await?);
            self.run(RecordCommitted, &mut tx, scope, &args).await?;
            tx.commit().await?;
            Ok::<(), AppError>(())
        }
        .await;
        if let Err(error) = outcome {
            tracing::warn!(
                record_type = %args.record_type,
                record_id = %args.record.record_id,
                "record committed hook failed: {error}"
            );
        }
    }

    async fn deleting(
        &self,
        tx: &mut MySqlTx,
        scope: &AccessScope,
        record: &RecordRef,
    ) -> Result<(), AppError> {
        let before =
            crate::records::snapshot::capture(tx, &self.pool, &self.registry, record).await?;
        tx.record_states.insert(
            (record.record_type.as_str().to_owned(), record.record_id),
            serde_json::json!({
                "operation_id": tx.operation_id, "record_type": record.record_type.as_str(),
                "record_id": record.record_id, "phase": "record.deleting", "before": before,
                "before_available": true, "proposed": null, "persisted": null,
            }),
        );
        crate::records::references::deleting(tx, &self.pool, &self.registry, record).await?;
        self.run(RecordDeleting, tx, scope, &self.removed(record))
            .await
    }

    async fn archiving(
        &self,
        tx: &mut MySqlTx,
        scope: &AccessScope,
        record: &RecordRef,
    ) -> Result<(), AppError> {
        let before =
            crate::records::snapshot::capture(tx, &self.pool, &self.registry, record).await?;
        tx.record_states.insert(
            (record.record_type.as_str().to_owned(), record.record_id),
            serde_json::json!({
                "operation_id": tx.operation_id, "record_type": record.record_type.as_str(),
                "record_id": record.record_id, "phase": "record.archiving", "before": before,
                "before_available": true, "proposed": null, "persisted": null,
            }),
        );
        self.run(RecordArchiving, tx, scope, &self.removed(record))
            .await
    }

    async fn deleted(
        &self,
        tx: &mut MySqlTx,
        scope: &AccessScope,
        record: &RecordRef,
    ) -> Result<(), AppError> {
        self.run(RecordDeleted, tx, scope, &self.removed(record))
            .await?;
        crate::infrastructure::fields::unique_values::release_record(tx, record).await
    }

    async fn restored(
        &self,
        tx: &mut MySqlTx,
        scope: &AccessScope,
        record: &RecordRef,
    ) -> Result<(), AppError> {
        if let Some(snapshot) =
            super::snapshot::capture(tx, &self.pool, &self.registry, record).await?
        {
            let mut values = snapshot["custom_fields"]
                .as_object()
                .cloned()
                .unwrap_or_default();
            values.extend(snapshot["values"].as_object().cloned().unwrap_or_default());
            super::references::validate(tx, scope, &self.registry, record, &Value::Object(values))
                .await?;
        }
        if let Ok(definition) = self.registry.require(record.record_type.as_str()) {
            crate::infrastructure::fields::unique_values::reserve_record(tx, definition, record)
                .await?;
        }
        self.run(RecordRestored, tx, scope, &self.removed(record))
            .await
    }

    async fn purged(
        &self,
        tx: &mut MySqlTx,
        scope: &AccessScope,
        record: &RecordRef,
    ) -> Result<PurgedRecord, AppError> {
        crate::records::references::deleting(tx, &self.pool, &self.registry, record).await?;
        if let Ok(definition) = self.registry.require(record.record_type.as_str()) {
            let mut removed = std::collections::BTreeSet::new();
            for field in &definition.system_fields {
                if let Some(extension) = &field.extension
                    && field.is_available()
                    && removed.insert(&extension.table)
                {
                    sqlx::query(sqlx::AssertSqlSafe(format!(
                        "DELETE FROM `{}` WHERE `{}` = ?",
                        extension.table, extension.record_column
                    )))
                    .bind(record.record_id.to_string())
                    .execute(&mut **tx)
                    .await?;
                }
            }
        }
        self.run(RecordPurged, tx, scope, &self.removed(record))
            .await?;
        MySqlCustomFieldValueRepository::new(self.pool.clone())
            .delete_record(tx, record)
            .await?;
        MySqlRecordLinkRepository::new(self.pool.clone(), self.registry.clone())
            .delete_record_links(tx, record.record_id)
            .await?;
        clear_record_references(tx, record).await?;
        let attached = delete_record_rows(tx, record).await?;
        Ok(PurgedRecord {
            orphan_storage_keys: delete_unreferenced_files(tx, &attached).await?,
        })
    }
}

async fn clear_record_references(tx: &mut MySqlTx, record: &RecordRef) -> Result<(), AppError> {
    sqlx::query(
        "DELETE v FROM custom_field_values v JOIN field_definitions f ON f.id = v.field_id \
         WHERE f.field_type = 'record' \
         AND JSON_UNQUOTE(JSON_EXTRACT(f.config, '$.target_record_type')) = ? \
         AND v.value_text = ?",
    )
    .bind(record.record_type.as_str())
    .bind(record.record_id.to_string())
    .execute(&mut **tx)
    .await?;
    Ok(())
}

async fn delete_record_rows(tx: &mut MySqlTx, record: &RecordRef) -> Result<Vec<Uuid>, AppError> {
    let kind = record.record_type.as_str();
    let id = record.record_id.to_string();
    let attached: Vec<String> = sqlx::query_scalar(
        "SELECT file_id FROM attachments WHERE record_type = ? AND record_id = ?",
    )
    .bind(kind)
    .bind(&id)
    .fetch_all(&mut **tx)
    .await?;
    for table in ["attachments", "record_shares"] {
        sqlx::query(sqlx::AssertSqlSafe(format!(
            "DELETE FROM {table} WHERE record_type = ? AND record_id = ?"
        )))
        .bind(kind)
        .bind(&id)
        .execute(&mut **tx)
        .await?;
    }
    Ok(attached
        .iter()
        .filter_map(|file_id| Uuid::parse_str(file_id).ok())
        .collect())
}

async fn read_only<K: HookRef>(
    hook: K,
    pool: &MySqlPool,
    scope: &AccessScope,
    value: K::Args,
) -> Result<K::Args, AppError> {
    let app = crate::modules::app();
    let hooks = app.hooks();
    if !hooks.has(hook) {
        return Ok(value);
    }
    let mut tx = MySqlTx::from(pool.begin().await?);
    let value = {
        let mut ctx = HookCtx::new(&mut tx, pool, scope, app.services(), KERNEL_MODULE);
        hooks.apply(hook, &mut ctx, value).await?
    };
    tx.rollback().await?;
    Ok(value)
}

pub async fn filter_search_plan(
    pool: &MySqlPool,
    scope: &AccessScope,
    plan: QueryPlan,
) -> Result<QueryPlan, AppError> {
    read_only(SearchPlanFilter, pool, scope, plan).await
}

pub async fn constrain_query(
    pool: &MySqlPool,
    scope: &AccessScope,
    plan: &QueryPlan,
) -> Result<Option<QueryPlan>, AppError> {
    let constraints = QueryConstraints {
        record_type: plan.record_type.record_type.clone(),
        conditions: Vec::new(),
        sort: Vec::new(),
    };
    let constraints = read_only(RecordQueryFilter, pool, scope, constraints).await?;
    if constraints.conditions.is_empty() && constraints.sort.is_empty() {
        return Ok(None);
    }
    let mut constrained = plan.clone();
    constrained.filter = match constrained.filter.take() {
        Some(filter) => Some(PlanNode::And(
            std::iter::once(filter)
                .chain(constraints.conditions)
                .collect(),
        )),
        None if constraints.conditions.is_empty() => None,
        None => Some(PlanNode::And(constraints.conditions)),
    };
    constrained.sort.extend(constraints.sort);
    Ok(Some(constrained))
}

pub async fn serialize_records(
    pool: &MySqlPool,
    scope: &AccessScope,
    record_type: &RecordType,
    items: Vec<Value>,
) -> Result<Vec<Value>, AppError> {
    let records = SerializedRecords {
        record_type: record_type.clone(),
        items,
    };
    Ok(read_only(RecordSerializing, pool, scope, records)
        .await?
        .items)
}

pub async fn serialize_nested_records(
    pool: &MySqlPool,
    scope: &AccessScope,
    record_type: &RecordType,
    key: &str,
    mut items: Vec<Value>,
) -> Result<Vec<Value>, AppError> {
    let (positions, nested): (Vec<usize>, Vec<Value>) = items
        .iter_mut()
        .enumerate()
        .filter_map(|(index, item)| {
            item.get_mut(key)
                .filter(|nested| nested.is_object())
                .map(|nested| (index, nested.take()))
        })
        .unzip();
    if nested.is_empty() {
        return Ok(items);
    }
    let serialized = serialize_records(pool, scope, record_type, nested).await?;
    if serialized.len() != positions.len() {
        return Err(AppError::Internal(format!(
            "the {record_type} serializing hooks changed the number of nested records"
        )));
    }
    for (index, nested) in positions.into_iter().zip(serialized) {
        if let Some(item) = items[index].as_object_mut() {
            item.insert(key.to_owned(), nested);
        }
    }
    Ok(items)
}

pub async fn filter_schema(
    pool: &MySqlPool,
    scope: &AccessScope,
    record_type: &RecordType,
    schema: FieldSchema,
) -> Result<FieldSchema, AppError> {
    let schema = RecordSchema {
        record_type: record_type.clone(),
        schema,
    };
    Ok(read_only(RecordSchemaFilter, pool, scope, schema)
        .await?
        .schema)
}
