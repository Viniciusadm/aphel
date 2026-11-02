use chrono::{DateTime, Utc};
use serde_json::{Value, json};
use uuid::Uuid;

use crate::application::access::Authorizer;
use crate::application::auth::ClientMetadata;
use crate::application::fields::EntityDefinition;
use crate::application::files::UserReference;
use crate::application::record_sharing::{
    MAX_RECORD_SHARES, NewRecordShare, RECORD_NOT_FOUND, RECORD_SHARE_EXISTS,
    RECORD_SHARE_NOT_ALLOWED, RECORD_SHARE_NOT_FOUND, RECORD_SHARE_UPDATED, RECORD_SHARED,
    RECORD_SHARES_LIMIT_REACHED, RECORD_UNSHARED, RecordDirectory, RecordHeader, RecordShare,
    RecordShareRepository, RecordShares, RecordSharingPorts, RecordSharingTx, ShareAccess,
    ShareGrantee, ShareGranteeType,
};
use crate::application::security::{Clock, NewSecurityEvent, SecurityEventKind, record_quietly};
use crate::application::shared::{FieldChange, RecordRef, TransactionManager, ensure_version};
use crate::domain::UserId;
use crate::domain::access::{AccessScope, Permission};
use crate::errors::{AppError, FieldViolation};
use crate::records::{RecordMutation, publish_mutation};

const INVALID_SHARE: &str = "invalid record share";

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct CreateRecordShareCommand {
    pub grantee_type: String,
    pub grantee_id: String,
    pub access: String,
    pub client: ClientMetadata,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct UpdateRecordShareCommand {
    pub access: String,
    pub expected_updated_at: Option<DateTime<Utc>>,
    pub client: ClientMetadata,
}

pub fn record_not_found() -> AppError {
    AppError::not_found_code(
        RECORD_NOT_FOUND,
        "the record does not exist or you cannot read it",
    )
}

fn share_not_found() -> AppError {
    AppError::not_found_code(RECORD_SHARE_NOT_FOUND, "record share was not found")
}

fn share_not_allowed() -> AppError {
    AppError::forbidden_code(
        RECORD_SHARE_NOT_ALLOWED,
        "only the owner of the record or someone with records.share can manage its shares",
    )
}

fn parse_access(value: &str, violations: &mut Vec<FieldViolation>) -> Option<ShareAccess> {
    let access = ShareAccess::parse(value.trim());
    if access.is_none() {
        violations.push(FieldViolation::new("access", "must be read or edit"));
    }
    access
}

pub fn can_manage_shares(scope: &AccessScope, header: &RecordHeader) -> bool {
    header.owner_id.as_ref() == Some(scope.user_id())
        || scope.scope_for(&Permission::RECORDS_SHARE).is_some()
}

fn share_payload(share: &RecordShare) -> Value {
    json!({
        "share_id": share.id.to_string(),
        "record_type": share.record.record_type.as_str(),
        "record_id": share.record.record_id.to_string(),
        "grantee_type": share.grantee.kind.as_str(),
        "grantee_id": share.grantee.id,
        "grantee_name": share.grantee.name,
        "access": share.access.as_str(),
    })
}

fn share_changes(old: Option<&RecordShare>, new: Option<&RecordShare>) -> Vec<FieldChange> {
    let grantee = |share: Option<&RecordShare>| {
        share.map_or(Value::Null, |share| {
            Value::String(format!(
                "{}:{}",
                share.grantee.kind.as_str(),
                share.grantee.id
            ))
        })
    };
    let access = |share: Option<&RecordShare>| {
        share.map_or(Value::Null, |share| {
            Value::String(share.access.as_str().to_owned())
        })
    };
    [
        ("share.grantee", grantee(old), grantee(new)),
        ("share.access", access(old), access(new)),
    ]
    .into_iter()
    .filter(|(_, old, new)| old != new)
    .map(|(field, old, new)| FieldChange::new(field, old, new))
    .collect()
}

struct ShareTarget {
    record: RecordRef,
    header: RecordHeader,
}

pub struct RecordSharingUseCases<P> {
    ports: P,
}

impl<P> RecordSharingUseCases<P>
where
    P: RecordSharingPorts,
{
    pub fn new(ports: P) -> Self {
        Self { ports }
    }

    pub async fn list(
        &self,
        scope: &AccessScope,
        record_type: &str,
        record_id: &str,
    ) -> Result<RecordShares, AppError> {
        let target = self.target(scope, record_type, record_id).await?;
        let shares = self.ports.shares().list(&target.record).await?;
        Ok(RecordShares {
            can_manage: can_manage_shares(scope, &target.header),
            owner_id: target.header.owner_id,
            record: target.record,
            shares,
        })
    }

    pub async fn create(
        &self,
        scope: &AccessScope,
        record_type: &str,
        record_id: &str,
        command: CreateRecordShareCommand,
    ) -> Result<RecordShare, AppError> {
        let target = self.manageable(scope, record_type, record_id).await?;
        let mut violations = Vec::new();
        let grantee_type = ShareGranteeType::parse(command.grantee_type.trim());
        if grantee_type.is_none() {
            violations.push(FieldViolation::new("grantee_type", "must be user or team"));
        }
        let access = parse_access(&command.access, &mut violations);
        let grantee_id = Uuid::parse_str(command.grantee_id.trim())
            .ok()
            .map(|id| id.to_string());
        if grantee_id.is_none() {
            violations.push(FieldViolation::new("grantee_id", "must be a valid id"));
        }
        let mut grantee_name = None;
        if let (Some(kind), Some(id)) = (grantee_type, grantee_id.as_deref()) {
            grantee_name = self.ports.shares().grantee_name(kind, id).await?;
            if grantee_name.is_none() {
                violations.push(FieldViolation::new(
                    "grantee_id",
                    match kind {
                        ShareGranteeType::User => "must be an active member of the organization",
                        ShareGranteeType::Team => "must be a team of the organization",
                    },
                ));
            } else if kind == ShareGranteeType::User
                && target.header.owner_id.as_ref().map(UserId::as_str) == Some(id)
            {
                violations.push(FieldViolation::new(
                    "grantee_id",
                    "is the owner of the record",
                ));
            }
        }
        let (Some(kind), Some(access), Some(grantee_id), true) =
            (grantee_type, access, grantee_id, violations.is_empty())
        else {
            return Err(AppError::invalid_fields(INVALID_SHARE, violations));
        };
        let now = self.ports.clock().now();
        let mut tx = self.ports.transactions().begin().await?;
        let existing = self
            .ports
            .shares()
            .lock_list(&mut tx, &target.record)
            .await?;
        if existing
            .iter()
            .any(|share| share.grantee.kind == kind && share.grantee.id == grantee_id)
        {
            return Err(AppError::conflict_code(
                RECORD_SHARE_EXISTS,
                "the record is already shared with this user or team",
            ));
        }
        if existing.len() >= MAX_RECORD_SHARES {
            return Err(AppError::validation_code(
                RECORD_SHARES_LIMIT_REACHED,
                format!("a record can be shared at most {MAX_RECORD_SHARES} times"),
            ));
        }
        let share = RecordShare {
            id: Uuid::now_v7(),
            record: target.record.clone(),
            grantee: ShareGrantee {
                kind,
                id: grantee_id.clone(),
                name: grantee_name,
            },
            access,
            shared_by: Some(UserReference {
                id: scope.user_id().clone(),
                display_name: String::new(),
            }),
            created_at: now,
            updated_at: now,
        };
        self.ports
            .shares()
            .insert(
                &mut tx,
                &NewRecordShare {
                    id: share.id,
                    record: share.record.clone(),
                    grantee_type: kind,
                    grantee_id,
                    access,
                    shared_by: scope.user_id().clone(),
                    at: now,
                },
            )
            .await?;
        self.publish(
            &mut tx,
            scope,
            &share,
            RECORD_SHARED,
            share_changes(None, Some(&share)),
            command.client.ip.clone(),
        )
        .await?;
        self.ports.transactions().commit(tx).await?;
        self.security_event(scope, &share, "shared", &command.client)
            .await;
        Ok(self.find(&share.record, share.id).await?.unwrap_or(share))
    }

    pub async fn update(
        &self,
        scope: &AccessScope,
        record_type: &str,
        record_id: &str,
        share_id: &str,
        command: UpdateRecordShareCommand,
    ) -> Result<RecordShare, AppError> {
        let target = self.manageable(scope, record_type, record_id).await?;
        let share_id = Uuid::parse_str(share_id.trim()).map_err(|_| share_not_found())?;
        let mut violations = Vec::new();
        let Some(access) = parse_access(&command.access, &mut violations) else {
            return Err(AppError::invalid_fields(INVALID_SHARE, violations));
        };
        let now = self.ports.clock().now();
        let mut tx = self.ports.transactions().begin().await?;
        let current = self
            .ports
            .shares()
            .lock_list(&mut tx, &target.record)
            .await?
            .into_iter()
            .find(|share| share.id == share_id)
            .ok_or_else(share_not_found)?;
        ensure_version(command.expected_updated_at, current.updated_at)?;
        if current.access == access {
            return Ok(current);
        }
        let mut next = current.clone();
        next.access = access;
        next.updated_at = now;
        self.ports
            .shares()
            .update_access(&mut tx, share_id, access, now)
            .await?;
        self.publish(
            &mut tx,
            scope,
            &next,
            RECORD_SHARE_UPDATED,
            share_changes(Some(&current), Some(&next)),
            command.client.ip.clone(),
        )
        .await?;
        self.ports.transactions().commit(tx).await?;
        self.security_event(scope, &next, "updated", &command.client)
            .await;
        Ok(next)
    }

    pub async fn delete(
        &self,
        scope: &AccessScope,
        record_type: &str,
        record_id: &str,
        share_id: &str,
        client: ClientMetadata,
    ) -> Result<(), AppError> {
        let target = self.manageable(scope, record_type, record_id).await?;
        let share_id = Uuid::parse_str(share_id.trim()).map_err(|_| share_not_found())?;
        let mut tx = self.ports.transactions().begin().await?;
        let current = self
            .ports
            .shares()
            .lock_list(&mut tx, &target.record)
            .await?
            .into_iter()
            .find(|share| share.id == share_id)
            .ok_or_else(share_not_found)?;
        self.ports.shares().delete(&mut tx, share_id).await?;
        self.publish(
            &mut tx,
            scope,
            &current,
            RECORD_UNSHARED,
            share_changes(Some(&current), None),
            client.ip.clone(),
        )
        .await?;
        self.ports.transactions().commit(tx).await?;
        self.security_event(scope, &current, "unshared", &client)
            .await;
        Ok(())
    }

    async fn find(
        &self,
        record: &RecordRef,
        share_id: Uuid,
    ) -> Result<Option<RecordShare>, AppError> {
        Ok(self
            .ports
            .shares()
            .list(record)
            .await?
            .into_iter()
            .find(|share| share.id == share_id))
    }

    fn definition(&self, record_type: &str) -> Result<EntityDefinition, AppError> {
        self.ports.registry().require(record_type).cloned()
    }

    async fn target(
        &self,
        scope: &AccessScope,
        record_type: &str,
        record_id: &str,
    ) -> Result<ShareTarget, AppError> {
        let definition = self.definition(record_type)?;
        let visibility = scope.record_filter(&definition.read_permission)?;
        let id = Uuid::parse_str(record_id.trim()).map_err(|_| record_not_found())?;
        let header = self
            .ports
            .records()
            .header(scope, &definition, &visibility, id)
            .await?
            .ok_or_else(record_not_found)?;
        Ok(ShareTarget {
            record: RecordRef::new(definition.record_type.clone(), id),
            header,
        })
    }

    async fn manageable(
        &self,
        scope: &AccessScope,
        record_type: &str,
        record_id: &str,
    ) -> Result<ShareTarget, AppError> {
        let target = self.target(scope, record_type, record_id).await?;
        if !can_manage_shares(scope, &target.header) {
            return Err(share_not_allowed());
        }
        Ok(target)
    }

    async fn publish(
        &self,
        tx: &mut RecordSharingTx<P>,
        scope: &AccessScope,
        share: &RecordShare,
        event_type: &'static str,
        changes: Vec<FieldChange>,
        ip: Option<String>,
    ) -> Result<(), AppError> {
        let mut payload = share_payload(share);
        payload["changes"] = json!(
            changes
                .iter()
                .map(|change| json!({
                    "field": change.field,
                    "old": change.old,
                    "new": change.new,
                }))
                .collect::<Vec<_>>()
        );
        publish_mutation(
            self.ports.events(),
            self.ports.audit(),
            tx,
            scope,
            self.ports.clock().now(),
            RecordMutation {
                record: share.record.clone(),
                event_type,
                payload,
                changes,
                ip,
            },
        )
        .await
    }

    async fn security_event(
        &self,
        scope: &AccessScope,
        share: &RecordShare,
        action: &str,
        client: &ClientMetadata,
    ) {
        let subject_user = match share.grantee.kind {
            ShareGranteeType::User => UserId::new(share.grantee.id.clone()).ok(),
            ShareGranteeType::Team => None,
        }
        .unwrap_or_else(|| scope.user_id().clone());
        let event = NewSecurityEvent::new(
            SecurityEventKind::PermissionChanged,
            self.ports.clock().now(),
        )
        .user(&subject_user)
        .subject(format!(
            "{}:{}",
            share.record.record_type.as_str(),
            share.record.record_id
        ))
        .client(client.ip.as_deref(), client.user_agent.as_deref())
        .metadata(json!({
            "change": "record_share",
            "action": action,
            "record_type": share.record.record_type.as_str(),
            "record_id": share.record.record_id.to_string(),
            "share_id": share.id.to_string(),
            "grantee_type": share.grantee.kind.as_str(),
            "grantee_id": share.grantee.id,
            "grantee_name": share.grantee.name,
            "access": share.access.as_str(),
            "actor_user_id": scope.user_id().as_str(),
        }));
        record_quietly(self.ports.security_events(), event).await;
    }
}
