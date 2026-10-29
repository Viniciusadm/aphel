use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::application::access::Authorizer;
use crate::application::audit::{AuditLogFilter, AuditLogView, AuditRepository};
use crate::application::organization_admin::{ListWindow, Listing};
use crate::application::shared::RecordType;
use crate::domain::access::{AccessScope, Permission};
use crate::errors::{AppError, FieldViolation};

pub const MAX_ACTOR_LENGTH: usize = 64;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct AuditLogQuery {
    pub record_type: Option<String>,
    pub record_id: Option<String>,
    pub actor: Option<String>,
    pub from: Option<String>,
    pub to: Option<String>,
}

pub struct AuditUseCases<R> {
    repository: R,
}

impl<R> AuditUseCases<R>
where
    R: AuditRepository,
{
    pub fn new(repository: R) -> Self {
        Self { repository }
    }

    pub async fn list(
        &self,
        scope: &AccessScope,
        query: AuditLogQuery,
        window: ListWindow,
    ) -> Result<Listing<AuditLogView>, AppError> {
        scope.require(&Permission::AUDIT_READ)?;
        let filter = parse_filter(query)?;
        self.repository.list(&filter, window).await
    }
}

fn present(value: Option<String>) -> Option<String> {
    value
        .map(|value| value.trim().to_owned())
        .filter(|value| !value.is_empty())
}

fn parse_filter(query: AuditLogQuery) -> Result<AuditLogFilter, AppError> {
    let mut violations = Vec::new();
    let mut filter = AuditLogFilter::default();

    if let Some(record_type) = present(query.record_type) {
        match RecordType::parse(record_type) {
            Ok(record_type) => filter.record_type = Some(record_type),
            Err(_) => violations.push(FieldViolation::new(
                "record_type",
                "must be lowercase letters, digits or underscores",
            )),
        }
    }
    if let Some(record_id) = present(query.record_id) {
        match Uuid::parse_str(&record_id) {
            Ok(record_id) => filter.record_id = Some(record_id),
            Err(_) => violations.push(FieldViolation::new("record_id", "must be a UUID")),
        }
    }
    if let Some(actor) = present(query.actor) {
        if actor.chars().count() > MAX_ACTOR_LENGTH {
            violations.push(FieldViolation::new(
                "actor",
                format!("must have at most {MAX_ACTOR_LENGTH} characters"),
            ));
        } else {
            filter.actor_id = Some(actor);
        }
    }
    filter.from = parse_instant(query.from, "from", &mut violations);
    filter.to = parse_instant(query.to, "to", &mut violations);
    if let (Some(from), Some(to)) = (filter.from, filter.to)
        && from > to
    {
        violations.push(FieldViolation::new("to", "must not be before from"));
    }

    if !violations.is_empty() {
        return Err(AppError::invalid_fields(
            "invalid audit log filters",
            violations,
        ));
    }
    Ok(filter)
}

fn parse_instant(
    value: Option<String>,
    field: &str,
    violations: &mut Vec<FieldViolation>,
) -> Option<DateTime<Utc>> {
    let value = present(value)?;
    match DateTime::parse_from_rfc3339(&value) {
        Ok(instant) => Some(instant.with_timezone(&Utc)),
        Err(_) => {
            violations.push(FieldViolation::new(
                field,
                "must be an RFC 3339 date and time",
            ));
            None
        }
    }
}
