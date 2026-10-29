use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::application::access::{Authorizer, PERMISSION_DENIED};
use crate::application::auth::AuthSettings;
use crate::application::organization_admin::OrganizationAdminPorts;
use crate::application::security::Clock;
use crate::application::security::{NoopSecurityEventRecorder, SharedSecurityEventRecorder};
use crate::application::shared::{
    Actor, AuditEntry, AuditLog, DomainEvent, EventPublisher, FieldChange, RecordRef, RecordType,
    TransactionManager,
};
use crate::domain::access::{AccessScope, Permission};
use crate::errors::AppError;

pub const ALREADY_MEMBER: &str = "already_member";
pub const LAST_ADMIN: &str = "last_admin";
pub const ROLE_IN_USE: &str = "role_in_use";
pub const ROLE_KEY_TAKEN: &str = "role_key_taken";
pub const SYSTEM_ROLE_LOCKED: &str = "system_role_locked";
pub const TEAM_CYCLE: &str = "team_cycle";
pub const NOT_INVITED: &str = "not_invited";
pub const INVALID_MEMBER_STATUS: &str = "invalid_member_status";
pub const MEMBER_NOT_FOUND: &str = "member_not_found";
pub const ROLE_NOT_FOUND: &str = "role_not_found";
pub const TEAM_NOT_FOUND: &str = "team_not_found";
pub const MAX_NAME_LENGTH: usize = 140;
pub const MEMBER_ROLE_CHANGED: &str = "member.role_changed";
pub const MEMBER_INVITED: &str = "member.invited";
pub const MEMBER_INVITATION_RESENT: &str = "member.invitation_resent";
pub const MEMBER_INVITATION_CANCELLED: &str = "member.invitation_cancelled";
pub const MEMBER_SUSPENDED: &str = "member.suspended";
pub const MEMBER_REACTIVATED: &str = "member.reactivated";
pub const MEMBER_REMOVED: &str = "member.removed";
pub const ROLE_CREATED: &str = "role.created";
pub const ROLE_UPDATED: &str = "role.updated";
pub const ROLE_DELETED: &str = "role.deleted";
pub const TEAM_CREATED: &str = "team.created";
pub const TEAM_UPDATED: &str = "team.updated";
pub const TEAM_DELETED: &str = "team.deleted";

pub struct OrganizationAdminUseCases<P> {
    pub(super) ports: P,
    pub(super) settings: AuthSettings,
    pub(super) security_events: SharedSecurityEventRecorder,
}

impl<P> OrganizationAdminUseCases<P>
where
    P: OrganizationAdminPorts,
{
    pub fn new(ports: P, settings: AuthSettings) -> Self {
        Self {
            ports,
            settings,
            security_events: std::sync::Arc::new(NoopSecurityEventRecorder),
        }
    }

    pub fn with_security_events(mut self, recorder: SharedSecurityEventRecorder) -> Self {
        self.security_events = recorder;
        self
    }

    pub(super) fn now(&self) -> DateTime<Utc> {
        self.ports.clock().now()
    }

    pub(super) async fn record_admin_change(
        &self,
        tx: &mut <P::Transactions as TransactionManager>::Tx,
        scope: &AccessScope,
        change: AdminChange,
    ) -> Result<(), AppError> {
        let actor = Actor::from_scope(scope);
        let now = self.now();
        let record = RecordRef::new(change.record_type, change.record_id);
        self.ports
            .events()
            .publish(
                tx,
                DomainEvent::new(actor.clone(), change.event_type, now)
                    .with_record(record.clone())
                    .with_payload(change.payload),
            )
            .await?;
        self.ports
            .audit()
            .record(
                tx,
                AuditEntry::new(actor, change.event_type, now)
                    .with_record(record)
                    .with_changes(change.changes),
            )
            .await
    }
}

pub struct AdminChange {
    pub event_type: &'static str,
    pub record_type: RecordType,
    pub record_id: Uuid,
    pub payload: serde_json::Value,
    pub changes: Vec<FieldChange>,
}

impl AdminChange {
    pub fn new(
        event_type: &'static str,
        record_type: RecordType,
        id: &str,
        payload: serde_json::Value,
    ) -> Result<Self, AppError> {
        Ok(Self {
            event_type,
            record_type,
            record_id: Uuid::parse_str(id)
                .map_err(|_| AppError::Internal(format!("{id} is not a UUID")))?,
            payload,
            changes: Vec::new(),
        })
    }

    pub fn with_changes(mut self, changes: Vec<FieldChange>) -> Self {
        self.changes = changes;
        self
    }
}

pub(super) fn new_id() -> String {
    Uuid::now_v7().to_string()
}

pub(super) fn require_any(scope: &AccessScope, permissions: &[Permission]) -> Result<(), AppError> {
    if permissions
        .iter()
        .any(|permission| scope.require(permission).is_ok())
    {
        return Ok(());
    }
    let names: Vec<&str> = permissions.iter().map(Permission::as_str).collect();
    Err(AppError::forbidden_code(
        PERMISSION_DENIED,
        format!("missing one of the permissions {}", names.join(", ")),
    ))
}

pub(super) fn bounded_name(value: &str) -> Option<String> {
    let name = value.trim();
    (!name.is_empty() && name.chars().count() <= MAX_NAME_LENGTH).then(|| name.to_owned())
}

pub(super) fn name_message() -> String {
    format!("must have between 1 and {MAX_NAME_LENGTH} characters")
}
