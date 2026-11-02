pub(crate) mod erasure;
pub(crate) mod package;
pub(crate) mod requests;
pub(crate) mod retention;
pub(crate) mod search;
pub(crate) mod subject;

use std::borrow::Cow;

use chrono::{DateTime, Utc};
use serde_json::Value;

use crate::domain::{FieldSource, PersonalField, PrivacyRequest, PrivacyRequestKind, RunReport};
use aphel_kernel::Ctx;
use aphel_kernel::application::access::Authorizer;
use aphel_kernel::application::data_subject::{
    DataSubjectAction, DataSubjectRequest, DataSubjectSource, affected_items,
};
use aphel_kernel::application::fields::{
    CustomFieldPorts, EntityDefinition, FieldDefinitionRepository,
};
use aphel_kernel::application::shared::{
    Actor, AuditEntry, AuditLog, DomainEvent, EventPublisher, FieldChange, FileStorage, RecordRef,
};
use aphel_kernel::database::transaction::MySqlTx;
use aphel_kernel::domain::UserId;
use aphel_kernel::domain::access::{AccessScope, Permission, PermissionSet, RoleKey};
use aphel_kernel::errors::AppError;

pub(crate) fn acting_scope(user_id: Option<UserId>) -> AccessScope {
    match user_id {
        Some(user_id) => AccessScope::new(
            user_id,
            RoleKey::system(),
            Vec::new(),
            PermissionSet::everything(),
        ),
        None => AccessScope::system(),
    }
}

pub(crate) fn require_privacy(scope: &AccessScope) -> Result<(), AppError> {
    scope.require(&Permission::PRIVACY_MANAGE).map(|_| ())
}

pub(crate) struct Mutation {
    pub(crate) actor: Actor,
    pub(crate) event_type: Cow<'static, str>,
    pub(crate) record: RecordRef,
    pub(crate) payload: Value,
    pub(crate) changes: Vec<FieldChange>,
    pub(crate) now: DateTime<Utc>,
    pub(crate) ip: Option<String>,
}

pub(crate) async fn run_external_sources(
    ctx: &Ctx<'_>,
    request: &PrivacyRequest,
    report: &mut RunReport,
) -> Vec<(String, Value)> {
    let action = match request.kind {
        PrivacyRequestKind::Export => DataSubjectAction::Export,
        PrivacyRequestKind::Anonymize => DataSubjectAction::Anonymize,
        PrivacyRequestKind::Delete => DataSubjectAction::Delete,
    };
    let subject = DataSubjectRequest {
        id: request.id,
        action,
        records: &request.targets.records,
    };
    let mut sections = Vec::new();
    for source in ctx.app().contributions::<dyn DataSubjectSource>() {
        for outcome in source.run(&subject).await {
            match outcome.result {
                Ok(data) => {
                    report.external_items += affected_items(&data);
                    sections.push((outcome.source, data));
                }
                Err(error) => {
                    tracing::warn!(
                        error = %error,
                        source = %outcome.source,
                        "an external source of a privacy request could not be processed"
                    );
                    report.errors += 1;
                }
            }
        }
    }
    sections
}

pub(crate) fn definition(ctx: &Ctx<'_>, record_type: &str) -> Result<EntityDefinition, AppError> {
    ctx.registry().require(record_type).cloned()
}

pub(crate) async fn personal_fields(
    ctx: &Ctx<'_>,
    definition: &EntityDefinition,
) -> Result<Vec<PersonalField>, AppError> {
    let definitions = ctx
        .fields()
        .definitions()
        .list(&definition.record_type)
        .await?;
    Ok(definitions
        .into_iter()
        .filter(|field| field.is_personal)
        .filter_map(|field| {
            let system = definition.system_field_by_key(field.key.as_str());
            let source = if field.is_system {
                let column = field
                    .column_name
                    .clone()
                    .or_else(|| system.map(|system| system.column.clone()))?;
                FieldSource::Column(column)
            } else {
                FieldSource::Custom(field.id)
            };
            Some(PersonalField {
                key: field.key.as_str().to_owned(),
                label: field.label.clone(),
                field_type: field.field_type,
                required: field.is_required || system.is_some_and(|system| system.required),
                sensitive: field.is_sensitive || system.is_some_and(|system| system.sensitive),
                source,
            })
        })
        .collect())
}

pub(crate) async fn publish(
    ctx: &Ctx<'_>,
    tx: &mut MySqlTx,
    mutation: Mutation,
) -> Result<(), AppError> {
    let Mutation {
        actor,
        event_type,
        record,
        payload,
        changes,
        now,
        ip,
    } = mutation;
    ctx.events()
        .publish(
            tx,
            DomainEvent::new(actor.clone(), event_type.clone(), now)
                .with_record(record.clone())
                .with_payload(payload),
        )
        .await?;
    ctx.audit()
        .record(
            tx,
            AuditEntry::new(actor, event_type, now)
                .with_record(record)
                .with_changes(changes)
                .with_ip(ip),
        )
        .await
}

pub(crate) async fn audit_only(
    ctx: &Ctx<'_>,
    tx: &mut MySqlTx,
    action: &'static str,
    record: RecordRef,
    changes: Vec<FieldChange>,
    ip: Option<String>,
) -> Result<(), AppError> {
    ctx.audit()
        .record(
            tx,
            AuditEntry::new(Actor::from_scope(ctx.scope), action, ctx.now())
                .with_record(record)
                .with_changes(changes)
                .with_ip(ip),
        )
        .await
}

pub(crate) async fn remove_stored_files(storage: &dyn FileStorage, keys: &[String]) -> u64 {
    let mut removed = 0;
    for key in keys {
        match storage.delete(key).await {
            Ok(()) => removed += 1,
            Err(error) => {
                tracing::warn!(error = %error, "failed to remove a file from the storage")
            }
        }
    }
    removed
}
