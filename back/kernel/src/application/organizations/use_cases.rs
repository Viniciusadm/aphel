use serde::Serialize;
use serde_json::json;
use uuid::Uuid;

use crate::application::access::Authorizer;
use crate::application::audit::{DiffOptions, diff_changes};
use crate::application::files::{FileDownload, FileRepository};
use crate::application::organizations::{
    OrganizationChanges, OrganizationPorts, OrganizationSettingsRepository,
    UpdateOrganizationCommand, normalize_primary_color, validate_organization_name,
    validate_preferences,
};
use crate::application::public_events::{
    PreviousValues, PublicEventDefinition, PublicResource, changed,
};
use crate::application::security::Clock;
use crate::application::shared::{
    Actor, AuditEntry, AuditLog, DomainEvent, EventPublisher, FileStorage, RecordRef,
    TransactionManager,
};
use crate::domain::NonEmptyString;
use crate::domain::access::{AccessScope, Permission};
use crate::domain::organizations::Organization;
use crate::errors::{AppError, FieldViolation};

pub const ORGANIZATION_UPDATED: &str = "organization.updated";
pub const ORGANIZATION_RESOURCE: PublicResource = PublicResource::payload("organization");
pub const ORGANIZATION_PUBLIC_EVENTS: &[PublicEventDefinition] = &[changed(
    ORGANIZATION_UPDATED,
    ORGANIZATION_RESOURCE,
    "The organization settings changed (`data` has `changed_fields` and the current `currency`; `previous_currency` when the base currency changed)",
    PreviousValues::Fields(&[("currency", "previous_currency")]),
)];
pub const LOGO_NOT_FOUND: &str = "logo_not_found";
pub const MAX_LOGO_BYTES: u64 = 2 * 1024 * 1024;
pub const LOGO_MIME_TYPES: [&str; 4] = ["image/png", "image/jpeg", "image/webp", "image/gif"];

pub struct OrganizationUseCases<P> {
    ports: P,
}

#[derive(Debug, Serialize)]
struct SettingsSnapshot<'a> {
    name: &'a str,
    primary_color: Option<&'a str>,
    locale: &'a str,
    timezone: &'a str,
    currency: &'a str,
    logo_file_id: Option<&'a str>,
}

impl<'a> SettingsSnapshot<'a> {
    fn of(organization: &'a Organization) -> Self {
        Self {
            name: organization.name.as_str(),
            primary_color: organization.primary_color.as_deref(),
            locale: &organization.locale,
            timezone: &organization.timezone,
            currency: &organization.currency,
            logo_file_id: organization.logo_file_id.as_deref(),
        }
    }

    fn changes(changes: &'a OrganizationChanges) -> Self {
        Self {
            name: changes.name.as_str(),
            primary_color: changes.primary_color.as_deref(),
            locale: &changes.locale,
            timezone: &changes.timezone,
            currency: &changes.currency,
            logo_file_id: changes.logo_file_id.as_deref(),
        }
    }
}

impl<P> OrganizationUseCases<P>
where
    P: OrganizationPorts,
{
    pub fn new(ports: P) -> Self {
        Self { ports }
    }

    pub async fn current(&self) -> Result<Organization, AppError> {
        self.ports
            .organizations()
            .find()
            .await?
            .ok_or_else(|| AppError::NotFound("organization was not found".to_owned()))
    }

    pub async fn update(
        &self,
        scope: &AccessScope,
        command: UpdateOrganizationCommand,
    ) -> Result<Organization, AppError> {
        scope.require(&Permission::SETTINGS_MANAGE)?;
        let current = self.current().await?;

        let mut violations = Vec::new();
        let name = validate_organization_name(&command.name, "name", &mut violations);
        let primary_color = normalize_primary_color(command.primary_color, &mut violations);
        let preferences = validate_preferences(
            &command.locale,
            &command.timezone,
            &command.currency,
            &mut violations,
        );
        let logo_file_id = match command.logo_file_id {
            None => current.logo_file_id.clone(),
            Some(None) => None,
            Some(Some(file_id)) => self.logo_file(&file_id, &mut violations).await?,
        };
        let (Some(name), Some(preferences), true) = (name, preferences, violations.is_empty())
        else {
            return Err(AppError::invalid_fields(
                "invalid organization settings",
                violations,
            ));
        };

        let changes = OrganizationChanges {
            name: NonEmptyString::new(name, "name")?,
            primary_color,
            locale: preferences.locale,
            timezone: preferences.timezone,
            currency: preferences.currency,
            logo_file_id,
        };
        let diff = diff_changes(
            &SettingsSnapshot::of(&current),
            &SettingsSnapshot::changes(&changes),
            &DiffOptions::default(),
        )?;
        if diff.is_empty() {
            return Ok(current);
        }

        let record = RecordRef::organization();
        let actor = Actor::from_scope(scope);
        let now = self.ports.clock().now();
        let changed_fields: Vec<&str> = diff.iter().map(|change| change.field.as_str()).collect();
        let mut payload = json!({
            "changed_fields": changed_fields,
            "currency": changes.currency,
        });
        if current.currency != changes.currency {
            payload["previous_currency"] = json!(current.currency);
        }
        let event = DomainEvent::new(actor.clone(), ORGANIZATION_UPDATED, now)
            .with_record(record.clone())
            .with_payload(payload);
        let entry = AuditEntry::new(actor, ORGANIZATION_UPDATED, now)
            .with_record(record)
            .with_changes(diff)
            .with_ip(command.ip);

        let transactions = self.ports.transactions();
        let mut tx = transactions.begin().await?;
        self.ports.organizations().update(&mut tx, &changes).await?;
        self.ports.events().publish(&mut tx, event).await?;
        self.ports.audit().record(&mut tx, entry).await?;
        transactions.commit(tx).await?;

        self.current().await
    }

    pub async fn logo(&self) -> Result<FileDownload, AppError> {
        let organization = self.current().await?;
        let file_id = organization
            .logo_file_id
            .as_deref()
            .and_then(|file_id| Uuid::parse_str(file_id).ok())
            .ok_or_else(logo_not_found)?;
        let file = self
            .ports
            .files()
            .find_file(file_id)
            .await?
            .ok_or_else(logo_not_found)?;
        let content = self.ports.storage().get_stream(&file.storage_key).await?;
        Ok(FileDownload { file, content })
    }

    async fn logo_file(
        &self,
        file_id: &str,
        violations: &mut Vec<FieldViolation>,
    ) -> Result<Option<String>, AppError> {
        let file = match Uuid::parse_str(file_id.trim()) {
            Ok(id) => self.ports.files().find_file(id).await?,
            Err(_) => None,
        };
        match file {
            None => violations.push(FieldViolation::new("logo_file_id", "file was not found")),
            Some(file) if !LOGO_MIME_TYPES.contains(&file.mime_type.as_str()) => violations.push(
                FieldViolation::new("logo_file_id", "must be a PNG, JPEG, WebP or GIF image"),
            ),
            Some(file) if file.size > MAX_LOGO_BYTES => violations.push(FieldViolation::new(
                "logo_file_id",
                format!("must have at most {MAX_LOGO_BYTES} bytes"),
            )),
            Some(file) => return Ok(Some(file.id.to_string())),
        }
        Ok(None)
    }
}

fn logo_not_found() -> AppError {
    AppError::not_found_code(LOGO_NOT_FOUND, "the organization has no logo")
}
