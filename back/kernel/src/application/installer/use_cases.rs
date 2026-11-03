use serde_json::json;
use uuid::Uuid;

use crate::application::installer::{
    FirstOrganization, InstallCommand, Installation, InstallationRepository, InstallerPorts,
};
use crate::application::organization_profile::{
    NewOrganization, NewOwner, OrganizationConfig, normalize_public_base_url,
    organization_registration, validate_organization_name, validate_preferences,
};
use crate::application::security::{Clock, PasswordHasher, password_violation};
use crate::application::shared::{
    Actor, AuditEntry, AuditLog, DomainEvent, EventPublisher, FieldChange, RecordRef,
    TransactionManager,
};
use crate::domain::{Email, NonEmptyString, UserId};
use crate::errors::{AppError, FieldViolation};

pub const INSTALLATION_COMPLETED: &str = "installation.completed";
pub const EMAIL_TAKEN: &str = "email_taken";
const MAX_ADMIN_NAME_LENGTH: usize = 140;

pub struct InstallerUseCases<P> {
    ports: P,
}

impl<P> InstallerUseCases<P>
where
    P: InstallerPorts,
{
    pub fn new(ports: P) -> Self {
        Self { ports }
    }

    pub async fn status(&self) -> Result<(), AppError> {
        if self.ports.installation().is_available().await? {
            Ok(())
        } else {
            Err(unavailable())
        }
    }

    pub async fn install(&self, command: InstallCommand) -> Result<Installation, AppError> {
        self.status().await?;
        let modules =
            super::modules::prepare(self.ports.app(), &command.modules, &command.module_settings)?;

        let mut violations = Vec::new();
        let organization_name = validate_organization_name(
            &command.organization_name,
            "organization_name",
            &mut violations,
        );
        let preferences = validate_preferences(
            &command.locale,
            &command.timezone,
            &command.currency,
            &mut violations,
        );
        let admin_name = command.admin_name.trim();
        if admin_name.is_empty() || admin_name.chars().count() > MAX_ADMIN_NAME_LENGTH {
            violations.push(FieldViolation::new(
                "admin_name",
                format!("must have between 1 and {MAX_ADMIN_NAME_LENGTH} characters"),
            ));
        }
        let email = Email::new(command.admin_email.clone()).ok();
        if email.is_none() {
            violations.push(FieldViolation::new(
                "admin_email",
                "must be a valid email address",
            ));
        }
        if let Some(message) = password_violation(command.admin_password.expose()) {
            violations.push(FieldViolation::new("admin_password", message));
        }
        let public_base_url = match command.public_base_url.as_deref().map(str::trim) {
            None | Some("") => None,
            Some(raw) => {
                let normalized = normalize_public_base_url(raw);
                if normalized.is_none() {
                    violations.push(FieldViolation::new(
                        "public_base_url",
                        "must be an http(s) address",
                    ));
                }
                normalized
            }
        };
        let business = super::blueprint_input(command.business, &mut violations);
        let (Some(organization_name), Some(preferences), Some(email), true) =
            (organization_name, preferences, email, violations.is_empty())
        else {
            return Err(AppError::invalid_fields("invalid installation", violations));
        };

        let now = self.ports.clock().now();
        let user_id = UserId::new(Uuid::now_v7().to_string())?;
        let registration = organization_registration(
            NewOrganization {
                name: NonEmptyString::new(organization_name.clone(), "organization_name")?,
                locale: preferences.locale.clone(),
                timezone: preferences.timezone.clone(),
                currency: preferences.currency.clone(),
            },
            NewOwner {
                id: user_id.clone(),
                email: email.clone(),
                display_name: NonEmptyString::new(admin_name, "admin_name")?,
                password_hash: self
                    .ports
                    .hasher()
                    .hash_password(command.admin_password.expose())?,
            },
            now,
        )?;

        let record = RecordRef::organization();
        let actor = Actor::User(user_id.clone());
        let event = DomainEvent::new(actor.clone(), INSTALLATION_COMPLETED, now)
            .with_record(record.clone())
            .with_payload(json!({
                "modules": modules.iter().map(|module| &module.key).collect::<Vec<_>>(),
                "business": business,
                "admin_user_id": user_id.as_str(),
                "locale": preferences.locale,
                "timezone": preferences.timezone,
                "currency": preferences.currency,
            }));
        let mut changes = vec![
            FieldChange::new(
                "modules",
                json!([]),
                json!(modules.iter().map(|module| &module.key).collect::<Vec<_>>()),
            ),
            FieldChange::new("name", json!(null), json!(organization_name)),
            FieldChange::new("admin_email", json!(null), json!(email.as_str())),
            FieldChange::new("locale", json!(null), json!(preferences.locale)),
            FieldChange::new("timezone", json!(null), json!(preferences.timezone)),
            FieldChange::new("currency", json!(null), json!(preferences.currency)),
        ];
        if let Some(business) = &business {
            changes.push(FieldChange::new("business", json!(null), json!(business)));
        }
        let entry = AuditEntry::new(actor, INSTALLATION_COMPLETED, now)
            .with_record(record)
            .with_changes(changes)
            .with_ip(command.ip);
        let first = FirstOrganization {
            registration,
            configuration: OrganizationConfig {
                public_base_url,
                ..OrganizationConfig::default()
            },
            business,
            email_verified_at: now,
        };

        let transactions = self.ports.transactions();
        let mut tx = transactions.begin().await?;
        if !self.ports.installation().claim(&mut tx).await? {
            return Err(unavailable());
        }
        self.ports
            .installation()
            .create_first_organization(&mut tx, &first)
            .await
            .map_err(|error| match error {
                AppError::Conflict(_) => {
                    AppError::conflict_code(EMAIL_TAKEN, "email is already registered")
                }
                other => other,
            })?;
        self.ports
            .installation()
            .configure_modules(&mut tx, &modules)
            .await?;
        for module in modules.iter().filter(|module| {
            self.ports
                .app()
                .module(&module.key)
                .is_some_and(|entry| !entry.manifest.required)
        }) {
            let actor = Actor::User(user_id.clone());
            let record = RecordRef::new(
                crate::application::shared::RecordType::from_static("module"),
                crate::application::module_admin::ModuleState::new(&module.key, "", now)
                    .record_id(),
            );
            self.ports
                .events()
                .publish(
                    &mut tx,
                    DomainEvent::new(actor.clone(), "module.enabled", now)
                        .with_record(record.clone())
                        .with_payload(json!({ "key": module.key })),
                )
                .await?;
            let mut changes = vec![FieldChange::new("enabled", json!(false), json!(true))];
            changes.extend(module.values.iter().map(|(key, value)| {
                FieldChange::new(format!("settings.{key}"), json!(null), value.clone())
            }));
            changes.extend(module.secrets.iter().map(|(key, _)| {
                FieldChange::new(format!("settings.{key}"), json!(null), json!("[redacted]"))
            }));
            self.ports
                .audit()
                .record(
                    &mut tx,
                    AuditEntry::new(actor, "module.enabled", now)
                        .with_record(record)
                        .with_changes(changes),
                )
                .await?;
        }
        self.ports.events().publish(&mut tx, event).await?;
        self.ports.audit().record(&mut tx, entry).await?;
        transactions.commit(tx).await?;

        Ok(Installation {
            user_id,
            admin_email: email.as_str().to_owned(),
        })
    }
}

fn unavailable() -> AppError {
    AppError::NotFound("the installer is not available".to_owned())
}
