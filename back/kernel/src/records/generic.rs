use async_trait::async_trait;
use serde_json::{Map, Value, json};
use sqlx::{MySql, QueryBuilder};
use uuid::Uuid;

use crate::Ctx;
use crate::application::access::Authorizer;
use crate::application::audit::{DiffOptions, diff_values};
use crate::application::fields::CustomFieldValueRepository;
use crate::application::fields::permissions::check_record_writes;
use crate::application::fields::{
    CustomFieldPorts, CustomFieldWriter, EntityDefinition, FieldSet, RecordDraft, RecordPipeline,
    RecordWrite, ReferenceChecker, WriteMode, raw_from_json, value_to_json,
};
use crate::application::query::{
    ArchivedFilter, Filter, FilterCondition, FilterOperator, RecordQuery, RecordQueryEngine,
};
use crate::application::shared::{RecordRef, RecordType};
use crate::database::transaction::MySqlTx;
use crate::domain::fields::{FieldValue, validate_value};
use crate::errors::{AppError, FieldViolation};

use super::{
    PendingCommit, RecordCtx, RecordMutation, RecordTypeHandler, lock_version,
    operation_not_supported, publish_mutation,
};

#[async_trait]
pub trait EntityWriteBehavior: Send + Sync {
    async fn create(
        &self,
        ctx: &RecordCtx<'_>,
        tx: &mut MySqlTx,
        values: Value,
    ) -> Result<PendingCommit, AppError>;
    async fn update(
        &self,
        ctx: &RecordCtx<'_>,
        tx: &mut MySqlTx,
        id: Uuid,
        values: Value,
    ) -> Result<Option<PendingCommit>, AppError>;
}

pub struct GenericRecordHandler {
    record_type: RecordType,
    behavior: Option<std::sync::Arc<dyn EntityWriteBehavior>>,
}

impl GenericRecordHandler {
    pub fn new(record_type: RecordType) -> Self {
        Self {
            record_type,
            behavior: None,
        }
    }

    pub fn with_behavior(mut self, behavior: std::sync::Arc<dyn EntityWriteBehavior>) -> Self {
        self.behavior = Some(behavior);
        self
    }

    async fn set_archived(
        &self,
        ctx: &RecordCtx<'_>,
        id: Uuid,
        archived: bool,
    ) -> Result<(), AppError> {
        let ctx = Ctx::from_state(ctx.state, ctx.scope).await?;
        let definition = ctx.registry().require(self.record_type.as_str())?;
        if !definition.generic_writes {
            return Err(operation_not_supported(
                self.record_type.as_str(),
                "generic archive",
            ));
        }
        let column = definition
            .archived_column
            .as_ref()
            .ok_or_else(|| operation_not_supported(self.record_type.as_str(), "archive"))?;
        let permission = definition
            .action_permission("archive")
            .ok_or_else(|| operation_not_supported(self.record_type.as_str(), "archive"))?;
        ctx.scope.require(&permission)?;
        let mut tx = ctx.begin().await?;
        self.authorize(&ctx, &mut tx, id, &permission).await?;
        let previous: Option<chrono::NaiveDateTime> =
            sqlx::query_scalar(sqlx::AssertSqlSafe(format!(
                "SELECT `{column}` FROM `{}` WHERE id = ? FOR UPDATE",
                definition.table
            )))
            .bind(id.to_string())
            .fetch_one(&mut *tx)
            .await?;
        if previous.is_some() == archived {
            return Ok(());
        }
        let now = ctx.now();
        lock_version(&mut tx, &definition.table, id, None).await?;
        let record = RecordRef::new(self.record_type.clone(), id);
        if archived {
            ctx.pipeline()
                .archiving(&mut tx, ctx.scope, &record)
                .await?;
        }
        sqlx::query(sqlx::AssertSqlSafe(format!(
            "UPDATE `{}` SET `{column}` = {}, updated_at = UTC_TIMESTAMP(6) WHERE id = ?",
            definition.table,
            if archived { "UTC_TIMESTAMP(6)" } else { "NULL" }
        )))
        .bind(id.to_string())
        .execute(&mut *tx)
        .await?;
        if !archived {
            ctx.pipeline().restored(&mut tx, ctx.scope, &record).await?;
        }
        publish_mutation(
            ctx.events(),
            ctx.audit(),
            &mut tx,
            ctx.scope,
            ctx.now(),
            RecordMutation {
                record,
                event_type: crate::modules::intern::intern(&format!(
                    "{}.{}",
                    definition.event_prefix,
                    if archived { "archived" } else { "unarchived" }
                )),
                payload: json!({}),
                changes: vec![crate::application::shared::FieldChange::new(
                    column.clone(),
                    previous
                        .map(|at| json!(at.and_utc()))
                        .unwrap_or(Value::Null),
                    if archived { json!(now) } else { Value::Null },
                )],
                ip: None,
            },
        )
        .await?;
        tx.commit().await?;
        Ok(())
    }

    async fn authorize(
        &self,
        ctx: &Ctx<'_>,
        tx: &mut MySqlTx,
        id: Uuid,
        permission: &crate::domain::access::Permission,
    ) -> Result<(), AppError> {
        let definition = ctx.registry().require(self.record_type.as_str())?;
        let visibility = ctx.scope.record_filter(permission)?;
        let mut query = QueryBuilder::<MySql>::new(format!(
            "SELECT r.id FROM `{}` r WHERE r.id = ",
            definition.table
        ));
        query.push_bind(id.to_string());
        crate::infrastructure::query::compiler::push_record_scope(
            &mut query,
            ctx.scope,
            definition,
            &visibility,
        );
        if let Some(column) = &definition.deleted_column {
            query.push(format!(" AND r.`{column}` IS NULL"));
        }
        query.push(" FOR UPDATE");
        if query
            .build_query_scalar::<String>()
            .fetch_optional(&mut **tx)
            .await?
            .is_none()
        {
            return Err(AppError::forbidden_code(
                "permission_denied",
                "record is outside the mutation scope",
            ));
        }
        Ok(())
    }

    async fn visible(&self, ctx: &Ctx<'_>, id: Uuid) -> Result<bool, AppError> {
        let query = RecordQuery {
            filter: Some(Filter::Condition(FilterCondition {
                field: "id".to_owned(),
                operator: FilterOperator::Eq,
                value: json!(id),
            })),
            archived: ArchivedFilter::Include,
            ..Default::default()
        };
        Ok(!RecordQueryEngine::new(&ctx.query())
            .query(ctx.scope, self.record_type.as_str(), query)
            .await?
            .items
            .is_empty())
    }

    async fn snapshot(&self, ctx: &Ctx<'_>, tx: &mut MySqlTx, id: Uuid) -> Result<Value, AppError> {
        let record = RecordRef::new(self.record_type.clone(), id);
        let snapshot = super::snapshot::capture(tx, ctx.pool(), ctx.registry(), &record).await?;
        let checked = tx
            .visible_record
            .as_ref()
            .is_some_and(|(record_type, record_id)| {
                record_type == self.record_type.as_str() && *record_id == id
            });
        if snapshot.is_none() || (!checked && !self.visible(ctx, id).await?) {
            return Err(record_not_found());
        }
        Ok(snapshot.expect("checked snapshot"))
    }

    async fn begin_visible(&self, ctx: &RecordCtx<'_>, id: Uuid) -> Result<MySqlTx, AppError> {
        let query_ctx = Ctx::from_state(ctx.state, ctx.scope).await?;
        if !self.visible(&query_ctx, id).await? {
            return Err(record_not_found());
        }
        let mut tx: MySqlTx = ctx.state.db()?.begin().await?.into();
        tx.visible_record = Some((self.record_type.as_str().to_owned(), id));
        Ok(tx)
    }

    async fn mutate(
        &self,
        ctx: &RecordCtx<'_>,
        tx: &mut MySqlTx,
        id: Uuid,
        body: Value,
        mode: WriteMode,
    ) -> Result<PendingCommit, AppError> {
        let ctx = Ctx::from_state(ctx.state, ctx.scope).await?;
        let definition = ctx.registry().require(self.record_type.as_str())?;
        if !definition.generic_writes {
            return Err(operation_not_supported(
                self.record_type.as_str(),
                "generic mutations require an entity migration",
            ));
        }
        let permission = if mode == WriteMode::Create {
            definition.create_permission.as_ref()
        } else {
            definition.update_permission.as_ref()
        }
        .ok_or_else(|| {
            operation_not_supported(
                self.record_type.as_str(),
                "generic writes without a declared permission",
            )
        })?;
        ctx.scope.require(permission)?;
        let mut input = body
            .as_object()
            .cloned()
            .ok_or_else(|| AppError::Validation("record values must be an object".to_owned()))?;
        let expected = input
            .remove("_expected_updated_at")
            .map(serde_json::from_value)
            .transpose()
            .map_err(|_| AppError::Validation("invalid record version".to_owned()))?;
        let custom = match input.remove("custom_fields") {
            None => Map::new(),
            Some(Value::Object(map)) => map,
            Some(_) => {
                return Err(AppError::Validation(
                    "custom_fields must be an object".to_owned(),
                ));
            }
        };
        let before = if mode == WriteMode::Create {
            json!({"values":{},"custom_fields":{}})
        } else {
            self.snapshot(&ctx, tx, id).await?
        };
        if mode != WriteMode::Create {
            self.authorize(&ctx, tx, id, permission).await?;
        }
        lock_version(tx, &definition.table, id, expected).await?;
        let fields = FieldSet::for_scope(ctx.fields().definitions(), definition, ctx.scope).await?;
        let mut values = before["values"].as_object().cloned().unwrap_or_default();
        let provided = input.keys().map(String::as_str).collect::<Vec<_>>();
        for key in &provided {
            if !definition
                .system_field_by_key(key)
                .is_some_and(|field| field.writable && !field.is_computed() && field.is_available())
            {
                return Err(AppError::invalid_fields(
                    "invalid record values",
                    vec![FieldViolation::new(
                        *key,
                        "unknown or read-only system field",
                    )],
                ));
            }
        }
        values.extend(input.clone());
        if mode == WriteMode::Create
            && !ctx.scope.is_system()
            && let Some(owner) = &definition.owner_column
            && let Some(field) = definition
                .system_fields
                .iter()
                .find(|field| &field.column == owner)
        {
            values
                .entry(field.key.to_string())
                .or_insert_with(|| json!(ctx.scope.user_id().as_str()));
        }
        let record = RecordRef::new(self.record_type.clone(), id);
        let draft = ctx
            .pipeline()
            .saving(
                tx,
                ctx.scope,
                RecordDraft::new(record.clone(), mode, &Value::Object(values), custom),
            )
            .await?;
        let mut normalized = draft.values.clone();
        let mut errors = Vec::new();
        for field in fields.all().iter().filter(|field| {
            field.system.as_ref().is_some_and(|system| {
                system.writable && !system.is_computed() && system.is_available()
            })
        }) {
            let value = draft
                .values
                .get(field.key())
                .cloned()
                .unwrap_or(Value::Null);
            let default = if mode == WriteMode::Create && !draft.values.contains_key(field.key()) {
                field.definition.config.default_value.as_ref()
            } else {
                None
            };
            let raw = default
                .cloned()
                .or_else(|| raw_from_json(&value))
                .ok_or_else(|| {
                    AppError::Validation(format!("invalid value for {}", field.key()))
                })?;
            match validate_value(&field.definition, &raw) {
                Ok(None) if field.definition.is_required => {
                    errors.push(FieldViolation::new(field.key(), "is required"))
                }
                Ok(None) => {
                    normalized.insert(field.key().to_owned(), Value::Null);
                }
                Ok(Some(value)) => {
                    self.check_reference(&ctx, tx, &field.definition, &value)
                        .await?;
                    normalized.insert(
                        field.key().to_owned(),
                        value_to_json(&field.definition, &value),
                    );
                }
                Err(error) => errors.push(FieldViolation::new(field.key(), error.message())),
            }
        }
        if !errors.is_empty() {
            return Err(AppError::invalid_fields("invalid record values", errors));
        }
        check_record_writes(
            ctx.fields().definitions(),
            ctx.scope,
            &self.record_type,
            &provided,
            &before["values"],
            &Value::Object(normalized.clone()),
        )
        .await?;
        self.persist(
            tx,
            definition,
            id,
            &normalized,
            mode,
            (!ctx.scope.is_system()).then(|| ctx.scope.user_id().as_str().to_owned()),
        )
        .await?;
        let custom_changes = CustomFieldWriter::new(&ctx.fields())
            .with_record_values(&Value::Object(normalized.clone()))
            .apply(tx, ctx.scope, &record, &draft.custom_fields, mode)
            .await?;
        let persisted = super::snapshot::capture(tx, ctx.pool(), ctx.registry(), &record)
            .await?
            .ok_or_else(|| AppError::Internal("saved entity record is missing".to_owned()))?;
        super::snapshot::remember(tx, &record, Some(persisted.clone()));
        let values = persisted["values"].clone();
        let sensitive = definition
            .system_fields
            .iter()
            .filter(|field| field.sensitive)
            .map(|field| field.key.as_str())
            .collect::<Vec<_>>();
        let mut changes = diff_values(
            if mode == WriteMode::Create {
                &Value::Null
            } else {
                &before["values"]
            },
            &values,
            &DiffOptions::default().redacting(&sensitive).ignoring(&[
                "created_at",
                "updated_at",
                "created_by",
            ]),
        )?;
        crate::application::fields::permissions::check_managed_writes(
            definition, &provided, &changes,
        )?;
        changes.extend(custom_changes);
        let write = RecordWrite {
            record: &record,
            mode,
            changes: &changes,
            values: &values,
        };
        ctx.pipeline().validate(tx, ctx.scope, &write).await?;
        ctx.pipeline().saved(tx, ctx.scope, &write).await?;
        let action = if mode == WriteMode::Create {
            "created"
        } else {
            "updated"
        };
        publish_mutation(
            ctx.events(),
            ctx.audit(),
            tx,
            ctx.scope,
            ctx.now(),
            RecordMutation {
                record: record.clone(),
                event_type: crate::modules::intern::intern(&format!(
                    "{}.{action}",
                    definition.event_prefix
                )),
                payload: super::changes_payload(&changes),
                changes: changes.clone(),
                ip: None,
            },
        )
        .await?;
        Ok(PendingCommit {
            record,
            mode,
            changes,
            values,
        })
    }

    async fn check_reference(
        &self,
        ctx: &Ctx<'_>,
        tx: &mut MySqlTx,
        field: &crate::domain::fields::FieldDefinition,
        value: &FieldValue,
    ) -> Result<(), AppError> {
        let fields = ctx.fields();
        let exists = match value {
            FieldValue::User(id) => !fields
                .references()
                .existing_users(std::slice::from_ref(id))
                .await?
                .is_empty(),
            FieldValue::Record(id) => {
                let key = field.config.target_record_type.as_deref().ok_or_else(|| {
                    AppError::Validation("reference target is required".to_owned())
                })?;
                let target = ctx.registry().require(key)?;
                let visibility = ctx.scope.record_filter(&target.read_permission)?;
                match fields
                    .values()
                    .existing_records_in(tx, ctx.scope, target, &visibility, &[*id])
                    .await?
                {
                    Some(found) => !found.is_empty(),
                    None => !fields
                        .references()
                        .existing_records(ctx.scope, target, &visibility, &[*id])
                        .await?
                        .is_empty(),
                }
            }
            _ => true,
        };
        if exists {
            Ok(())
        } else {
            Err(AppError::invalid_fields(
                "invalid reference",
                vec![FieldViolation::new(
                    field.key.as_str(),
                    "referenced record is unavailable",
                )],
            ))
        }
    }

    async fn persist(
        &self,
        tx: &mut MySqlTx,
        definition: &EntityDefinition,
        id: Uuid,
        values: &Map<String, Value>,
        mode: WriteMode,
        creator: Option<String>,
    ) -> Result<(), AppError> {
        let fields = definition
            .system_fields
            .iter()
            .filter(|field| {
                field.writable
                    && !field.is_computed()
                    && field.contributed_by.is_none()
                    && values.contains_key(field.key.as_str())
            })
            .collect::<Vec<_>>();
        let mut query = QueryBuilder::<MySql>::new(if mode == WriteMode::Create {
            format!("INSERT INTO `{}` (`id`", definition.table)
        } else {
            format!("UPDATE `{}` SET ", definition.table)
        });
        if mode == WriteMode::Create {
            for field in &fields {
                query.push(format!(", `{}`", field.column));
            }
            if definition.storage == crate::application::fields::EntityStorage::Managed {
                query.push(", `created_by`");
            }
            query.push(") VALUES (").push_bind(id.to_string());
            for field in &fields {
                query.push(", ");
                push_value(&mut query, &values[field.key.as_str()]);
            }
            if definition.storage == crate::application::fields::EntityStorage::Managed {
                query.push(", ").push_bind(creator);
            }
            query.push(")");
        } else {
            for (index, field) in fields.iter().enumerate() {
                if index > 0 {
                    query.push(", ");
                }
                query.push(format!("`{}` = ", field.column));
                push_value(&mut query, &values[field.key.as_str()]);
            }
            if definition.storage == crate::application::fields::EntityStorage::Managed
                || definition.system_field_by_key("updated_at").is_some()
            {
                if !fields.is_empty() {
                    query.push(", ");
                }
                query.push("updated_at = UTC_TIMESTAMP(6)");
            } else if fields.is_empty() {
                query.push("id = id");
            }

            query.push(" WHERE id = ").push_bind(id.to_string());
        }
        query.build().execute(&mut **tx).await.map_err(|error| {
            if error
                .as_database_error()
                .is_some_and(|error| error.is_unique_violation())
            {
                AppError::conflict_code(
                    "entity_unique_conflict",
                    "record violates a unique entity index",
                )
            } else {
                AppError::from(error)
            }
        })?;
        super::write_contributed(tx, definition, id, values).await
    }
}

fn push_value(query: &mut QueryBuilder<MySql>, value: &Value) {
    match value {
        Value::Null => {
            query.push_bind(None::<String>);
        }
        Value::Bool(value) => {
            query.push_bind(*value);
        }
        Value::Number(value) => {
            query.push_bind(value.to_string());
        }
        Value::String(value) => {
            query.push_bind(value.clone());
        }
        value => {
            query.push_bind(value.to_string());
        }
    }
}

#[async_trait]
impl RecordTypeHandler for GenericRecordHandler {
    fn record_type(&self) -> &str {
        self.record_type.as_str()
    }
    fn durable_mutations(&self) -> bool {
        true
    }

    async fn create(&self, ctx: &RecordCtx<'_>, body: Value) -> Result<Value, AppError> {
        let mut tx = ctx.state.db()?.begin().await?.into();
        let pending = self.create_in_tx(ctx, &mut tx, body).await?;
        tx.commit().await?;
        pending
            .committed(&Ctx::from_state(ctx.state, ctx.scope).await?)
            .await;
        Ok(self.load(ctx, pending.record.record_id).await?.unwrap_or_else(|| json!({"id": pending.record.record_id, "record_type": self.record_type.as_str()})))
    }

    async fn create_in_tx(
        &self,
        ctx: &RecordCtx<'_>,
        tx: &mut MySqlTx,
        body: Value,
    ) -> Result<PendingCommit, AppError> {
        if let Some(behavior) = &self.behavior {
            return behavior.create(ctx, tx, body).await;
        }
        self.mutate(ctx, tx, Uuid::now_v7(), body, WriteMode::Create)
            .await
    }

    async fn update(&self, ctx: &RecordCtx<'_>, id: Uuid, body: Value) -> Result<Value, AppError> {
        let mut tx = self.begin_visible(ctx, id).await?;
        let pending = self.update_in_tx(ctx, &mut tx, id, body).await?;
        tx.commit().await?;
        if let Some(pending) = pending {
            pending
                .committed(&Ctx::from_state(ctx.state, ctx.scope).await?)
                .await;
        }
        Ok(self
            .load(ctx, id)
            .await?
            .unwrap_or_else(|| json!({"id": id, "record_type": self.record_type.as_str()})))
    }

    async fn update_in_tx(
        &self,
        ctx: &RecordCtx<'_>,
        tx: &mut MySqlTx,
        id: Uuid,
        body: Value,
    ) -> Result<Option<PendingCommit>, AppError> {
        if let Some(behavior) = &self.behavior {
            return behavior.update(ctx, tx, id, body).await;
        }
        Ok(Some(
            self.mutate(ctx, tx, id, body, WriteMode::Update).await?,
        ))
    }

    async fn delete(&self, ctx: &RecordCtx<'_>, id: Uuid) -> Result<(), AppError> {
        let mut tx = self.begin_visible(ctx, id).await?;
        self.delete_in_tx(ctx, &mut tx, id).await?;
        tx.commit().await?;
        Ok(())
    }

    async fn delete_in_tx(
        &self,
        ctx: &RecordCtx<'_>,
        tx: &mut MySqlTx,
        id: Uuid,
    ) -> Result<(), AppError> {
        let ctx = Ctx::from_state(ctx.state, ctx.scope).await?;
        let definition = ctx.registry().require(self.record_type.as_str())?;
        if !definition.generic_writes {
            return Err(operation_not_supported(
                self.record_type.as_str(),
                "generic mutations require an entity migration",
            ));
        }
        ctx.scope
            .require(definition.delete_permission.as_ref().ok_or_else(|| {
                operation_not_supported(self.record_type.as_str(), "generic deletion")
            })?)?;
        self.authorize(
            &ctx,
            tx,
            id,
            definition
                .delete_permission
                .as_ref()
                .expect("checked deletion permission"),
        )
        .await?;
        let before = self.snapshot(&ctx, tx, id).await?;
        let record = RecordRef::new(self.record_type.clone(), id);
        ctx.pipeline().deleting(tx, ctx.scope, &record).await?;
        let mut query = QueryBuilder::<MySql>::new(match &definition.deleted_column {
            Some(column) => format!(
                "UPDATE `{}` SET `{column}` = UTC_TIMESTAMP(6) WHERE id = ",
                definition.table
            ),
            None => format!("DELETE FROM `{}` WHERE id = ", definition.table),
        });
        query
            .push_bind(id.to_string())
            .build()
            .execute(&mut **tx)
            .await?;
        ctx.pipeline().deleted(tx, ctx.scope, &record).await?;
        if definition.deleted_column.is_none() {
            ctx.pipeline().purged(tx, ctx.scope, &record).await?;
        }
        publish_mutation(
            ctx.events(),
            ctx.audit(),
            tx,
            ctx.scope,
            ctx.now(),
            RecordMutation {
                record,
                event_type: crate::modules::intern::intern(&format!(
                    "{}.deleted",
                    definition.event_prefix
                )),
                payload: json!({}),
                changes: diff_values(
                    &before["values"],
                    &Value::Null,
                    &DiffOptions::default().redacting(
                        &definition
                            .system_fields
                            .iter()
                            .filter(|field| field.sensitive)
                            .map(|field| field.key.as_str())
                            .collect::<Vec<_>>(),
                    ),
                )?,
                ip: None,
            },
        )
        .await?;
        Ok(())
    }

    async fn archive(&self, ctx: &RecordCtx<'_>, id: Uuid) -> Result<(), AppError> {
        self.set_archived(ctx, id, true).await
    }
    async fn restore(&self, ctx: &RecordCtx<'_>, id: Uuid) -> Result<(), AppError> {
        self.set_archived(ctx, id, false).await
    }

    async fn load(&self, ctx: &RecordCtx<'_>, id: Uuid) -> Result<Option<Value>, AppError> {
        let ctx = Ctx::from_state(ctx.state, ctx.scope).await?;
        if !self.visible(&ctx, id).await? {
            return Ok(None);
        }
        let record = RecordRef::new(self.record_type.clone(), id);
        let Some(snapshot) = super::snapshot::read(ctx.pool(), ctx.registry(), &record).await?
        else {
            return Ok(None);
        };
        let definition = ctx.registry().require(self.record_type.as_str())?;
        let mut values = snapshot["values"].as_object().cloned().unwrap_or_default();
        if let Some(column) = &definition.archived_column {
            let archived: Option<chrono::NaiveDateTime> = sqlx::query_scalar(sqlx::AssertSqlSafe(
                format!("SELECT `{column}` FROM `{}` WHERE id = ?", definition.table),
            ))
            .bind(id.to_string())
            .fetch_optional(ctx.pool())
            .await?
            .flatten();
            values.insert(
                "archived_at".to_owned(),
                archived
                    .map(|at| json!(at.and_utc()))
                    .unwrap_or(Value::Null),
            );
        }
        values.insert("id".to_owned(), json!(id));
        values.insert("record_type".to_owned(), json!(self.record_type.as_str()));
        values.insert("entity".to_owned(), json!(definition.key));
        values.insert(
            "custom_fields".to_owned(),
            snapshot["custom_fields"].clone(),
        );
        Ok(Some(
            ctx.redacted(ctx.scope, &self.record_type, Value::Object(values))
                .await?,
        ))
    }
}

fn record_not_found() -> AppError {
    AppError::not_found_code("record_not_found", "record was not found")
}
