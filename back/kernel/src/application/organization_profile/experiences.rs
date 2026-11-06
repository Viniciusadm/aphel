use std::collections::BTreeSet;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Deserializer, Serialize};
use serde_json::json;
use utoipa::ToSchema;

use crate::application::access::Authorizer;
use crate::application::audit::{DiffOptions, diff_changes};
use crate::application::fields::RecordTypeRegistry;
use crate::application::security::Clock;
use crate::application::shared::{
    Actor, AuditEntry, AuditLog, DomainEvent, EventPublisher, RecordRef, TransactionManager,
    VERSION_CONFLICT,
};
use crate::domain::access::{AccessScope, Permission};
use crate::errors::{AppError, FieldViolation};

pub const EXPERIENCES_UPDATED: &str = "organization.experiences_updated";
const MAX_PROFILES: usize = 32;
const MAX_REFERENCES: usize = 32;
const MAX_PROFILE_BYTES: usize = 16 * 1024;
const MAX_CATALOG_BYTES: usize = 256 * 1024;

#[derive(Debug, Clone)]
pub struct PersonallyOmittableRecordSection {
    pub key: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct ExperienceProvider {
    pub module: String,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct ExperienceSection {
    pub module: String,
    pub key: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct ExperienceRegions {
    pub main: Vec<ExperienceSection>,
}

fn regions<'de, D: Deserializer<'de>>(
    deserializer: D,
) -> Result<Option<ExperienceRegions>, D::Error> {
    ExperienceRegions::deserialize(deserializer).map(Some)
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct ExperienceRecordDetail {
    pub record_type: String,
    pub presentation: ExperienceProvider,
    #[serde(
        default,
        deserialize_with = "regions",
        skip_serializing_if = "Option::is_none"
    )]
    #[schema(nullable = false)]
    pub regions: Option<ExperienceRegions>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct ExperienceProfile {
    pub format_version: u32,
    pub key: String,
    pub label: String,
    pub frame: ExperienceProvider,
    pub record_detail: ExperienceRecordDetail,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct ExperienceEntry {
    pub enabled: bool,
    pub profile: ExperienceProfile,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct ExperienceCustomization {
    pub profile_selection: bool,
    pub navigation: bool,
    pub section_order: bool,
    pub section_hidden: Vec<ExperienceSection>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct ExperiencePolicy {
    pub allowed_profiles: Vec<String>,
    pub customization: ExperienceCustomization,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct OrganizationExperiences {
    pub profiles: Vec<ExperienceEntry>,
    pub default_profile: Option<String>,
    pub policy: ExperiencePolicy,
}

fn identifier(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value
            .bytes()
            .all(|c| c.is_ascii_alphanumeric() || b"._-".contains(&c))
}

fn profile_key(value: &str) -> bool {
    value.len() <= 128
        && value.contains('.')
        && value.split('.').all(|part| {
            part.as_bytes().first().is_some_and(u8::is_ascii_lowercase)
                && part
                    .bytes()
                    .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || b"_-".contains(&c))
        })
}

fn check_identifier(value: &str, path: &str, errors: &mut Vec<FieldViolation>) {
    if !identifier(value) {
        errors.push(FieldViolation::new(
            path,
            "must be an ASCII identifier of 1 to 128 characters",
        ));
    }
}

pub(crate) fn check_sections(
    sources: &[ExperienceSection],
    path: &str,
    errors: &mut Vec<FieldViolation>,
) {
    if sources.len() > MAX_REFERENCES {
        errors.push(FieldViolation::new(
            path,
            "cannot contain more than 32 sections",
        ));
    }
    let mut seen = BTreeSet::new();
    for (index, source) in sources.iter().enumerate() {
        check_identifier(&source.module, &format!("{path}.{index}.module"), errors);
        check_identifier(&source.key, &format!("{path}.{index}.key"), errors);
        if !seen.insert(source) {
            errors.push(FieldViolation::new(
                format!("{path}.{index}"),
                "is duplicated",
            ));
        }
    }
}

pub fn validate_experience_profile(profile: &ExperienceProfile, path: &str) -> Vec<FieldViolation> {
    let mut errors = Vec::new();
    if profile.format_version != 1 {
        errors.push(FieldViolation::new(
            format!("{path}.format_version"),
            "only format version 1 is supported",
        ));
    }
    if !profile_key(&profile.key) {
        errors.push(FieldViolation::new(
            format!("{path}.key"),
            "must be a namespaced lower-case profile key of at most 128 characters",
        ));
    }
    if profile.label.trim().is_empty() || profile.label.chars().count() > 120 {
        errors.push(FieldViolation::new(
            format!("{path}.label"),
            "must have between 1 and 120 characters",
        ));
    }
    check_identifier(
        &profile.frame.module,
        &format!("{path}.frame.module"),
        &mut errors,
    );
    check_identifier(
        &profile.record_detail.record_type,
        &format!("{path}.record_detail.record_type"),
        &mut errors,
    );
    check_identifier(
        &profile.record_detail.presentation.module,
        &format!("{path}.record_detail.presentation.module"),
        &mut errors,
    );
    if let Some(regions) = &profile.record_detail.regions {
        if profile.record_detail.presentation.module == "kernel" {
            errors.push(FieldViolation::new(
                format!("{path}.record_detail.regions"),
                "the standard presentation does not accept region overrides",
            ));
        }
        check_sections(
            &regions.main,
            &format!("{path}.record_detail.regions.main"),
            &mut errors,
        );
    }
    if serde_json::to_vec(profile).map_or(true, |bytes| bytes.len() > MAX_PROFILE_BYTES) {
        errors.push(FieldViolation::new(
            path,
            "the canonical profile JSON cannot exceed 16 KiB",
        ));
    }
    errors
}

pub fn validate_organization_experiences(config: &OrganizationExperiences) -> Vec<FieldViolation> {
    if serde_json::to_vec(config).map_or(true, |bytes| bytes.len() > MAX_CATALOG_BYTES) {
        return vec![FieldViolation::new(
            "profiles",
            "the canonical catalog JSON cannot exceed 256 KiB",
        )];
    }
    let mut errors = Vec::new();
    if config.profiles.len() > MAX_PROFILES {
        errors.push(FieldViolation::new(
            "profiles",
            "cannot contain more than 32 profiles",
        ));
    }
    let mut keys = BTreeSet::new();
    let mut enabled = BTreeSet::new();
    let mut sections = BTreeSet::new();
    for (index, entry) in config.profiles.iter().enumerate() {
        errors.extend(validate_experience_profile(
            &entry.profile,
            &format!("profiles.{index}.profile"),
        ));
        if !keys.insert(&entry.profile.key) {
            errors.push(FieldViolation::new(
                format!("profiles.{index}.profile.key"),
                "is duplicated",
            ));
        }
        if entry.enabled {
            enabled.insert(&entry.profile.key);
        }
    }
    let mut allowed = BTreeSet::new();
    for (index, key) in config.policy.allowed_profiles.iter().enumerate() {
        if !enabled.contains(key) {
            errors.push(FieldViolation::new(
                format!("policy.allowed_profiles.{index}"),
                "must reference an enabled catalog profile",
            ));
        }
        if !allowed.insert(key) {
            errors.push(FieldViolation::new(
                format!("policy.allowed_profiles.{index}"),
                "is duplicated",
            ));
        }
    }
    if let Some(key) = &config.default_profile
        && !allowed.contains(key)
    {
        errors.push(FieldViolation::new(
            "default_profile",
            "must reference an enabled, allowed catalog profile",
        ));
    }
    for entry in &config.profiles {
        if entry.enabled
            && allowed.contains(&entry.profile.key)
            && let Some(regions) = &entry.profile.record_detail.regions
        {
            sections.extend(regions.main.iter());
        }
    }
    let hidden = &config.policy.customization.section_hidden;
    check_sections(hidden, "policy.customization.section_hidden", &mut errors);
    for (index, source) in hidden.iter().enumerate() {
        if !sections.contains(source) {
            errors.push(FieldViolation::new(
                format!("policy.customization.section_hidden.{index}"),
                "must reference a main section of an enabled, allowed profile",
            ));
        }
    }

    errors
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct ExperienceDiagnostic {
    pub profile_key: String,
    pub path: String,
    pub code: String,
    pub required: bool,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct OrganizationExperiencesView {
    pub configuration: Option<OrganizationExperiences>,
    #[schema(value_type = String, format = DateTime)]
    pub updated_at: DateTime<Utc>,
    pub requires_host_resolution: bool,
    pub diagnostics: Vec<ExperienceDiagnostic>,
}

#[derive(Debug, Clone)]
pub struct StoredOrganizationExperiences {
    pub configuration: Option<OrganizationExperiences>,
    pub updated_at: DateTime<Utc>,
}

#[async_trait]
pub trait OrganizationExperiencesRepository: Send + Sync {
    type Tx: Send;
    async fn find(&self) -> Result<StoredOrganizationExperiences, AppError>;
    async fn lock(&self, tx: &mut Self::Tx) -> Result<StoredOrganizationExperiences, AppError>;
    async fn lock_shared(
        &self,
        tx: &mut Self::Tx,
    ) -> Result<StoredOrganizationExperiences, AppError>;
    async fn save(
        &self,
        tx: &mut Self::Tx,
        configuration: &OrganizationExperiences,
    ) -> Result<DateTime<Utc>, AppError>;
}

pub type ExperiencesTx<P> =
    <<P as OrganizationExperiencesPorts>::Transactions as TransactionManager>::Tx;

pub trait OrganizationExperiencesPorts: Send + Sync {
    type Transactions: TransactionManager;
    type Repository: OrganizationExperiencesRepository<Tx = ExperiencesTx<Self>>;
    type Events: EventPublisher<ExperiencesTx<Self>>;
    type Audit: AuditLog<ExperiencesTx<Self>>;
    type Clock: Clock;
    fn transactions(&self) -> &Self::Transactions;
    fn repository(&self) -> &Self::Repository;
    fn events(&self) -> &Self::Events;
    fn audit(&self) -> &Self::Audit;
    fn clock(&self) -> &Self::Clock;
    fn registry(&self) -> &RecordTypeRegistry;
}

pub struct OrganizationExperiencesUseCases<P> {
    ports: P,
}

impl<P: OrganizationExperiencesPorts> OrganizationExperiencesUseCases<P> {
    pub fn new(ports: P) -> Self {
        Self { ports }
    }

    fn view(&self, stored: StoredOrganizationExperiences) -> OrganizationExperiencesView {
        let mut diagnostics = Vec::new();
        if let Some(config) = &stored.configuration {
            for (index, entry) in config
                .profiles
                .iter()
                .enumerate()
                .filter(|(_, entry)| entry.enabled)
            {
                let profile = &entry.profile;
                let path = format!("profiles.{index}.profile");
                let mut diagnostic = |suffix: &str, code: &str, required: bool| {
                    diagnostics.push(ExperienceDiagnostic {
                        profile_key: profile.key.clone(),
                        path: format!("{path}.{suffix}"),
                        code: code.to_owned(),
                        required,
                    })
                };
                if self
                    .ports
                    .registry()
                    .require(&profile.record_detail.record_type)
                    .is_err()
                {
                    diagnostic("record_detail.record_type", "record_type_unavailable", true);
                }
                diagnostic("frame", "provider_verification_required", true);
                diagnostic(
                    "record_detail.presentation",
                    "provider_verification_required",
                    true,
                );
                if let Some(regions) = &profile.record_detail.regions {
                    for index in 0..regions.main.len() {
                        diagnostic(
                            &format!("record_detail.regions.main.{index}"),
                            "section_verification_required",
                            false,
                        );
                    }
                }
            }
        }
        OrganizationExperiencesView {
            requires_host_resolution: stored.configuration.is_some(),
            configuration: stored.configuration,
            updated_at: stored.updated_at,
            diagnostics,
        }
    }

    pub async fn get(&self, _scope: &AccessScope) -> Result<OrganizationExperiencesView, AppError> {
        Ok(self.view(self.ports.repository().find().await?))
    }

    pub async fn replace(
        &self,
        scope: &AccessScope,
        config: OrganizationExperiences,
        expected: Option<DateTime<Utc>>,
        ip: Option<String>,
    ) -> Result<OrganizationExperiencesView, AppError> {
        scope.require(&Permission::SETTINGS_MANAGE)?;
        let expected = expected.ok_or_else(|| {
            AppError::invalid_fields(
                "If-Match is required",
                vec![FieldViolation::new(
                    "If-Match",
                    "must contain the updated_at returned by GET",
                )],
            )
        })?;
        let errors = validate_organization_experiences(&config);
        if !errors.is_empty() {
            return Err(AppError::invalid_fields(
                "invalid organization experiences",
                errors,
            ));
        }
        let mut tx = self.ports.transactions().begin().await?;
        let current = self.ports.repository().lock(&mut tx).await?;
        if expected.timestamp_micros() != current.updated_at.timestamp_micros() {
            return Err(AppError::conflict_code(
                VERSION_CONFLICT,
                "the organization changed; reload it before updating experiences",
            ));
        }
        if current.configuration.as_ref() == Some(&config) {
            return Ok(self.view(current));
        }
        let changes = diff_changes(
            &current.configuration,
            &Some(&config),
            &DiffOptions::default(),
        )?;
        let updated_at = self.ports.repository().save(&mut tx, &config).await?;
        let now = self.ports.clock().now();
        let actor = Actor::from_scope(scope);
        let record = RecordRef::organization();
        self.ports
            .events()
            .publish(
                &mut tx,
                DomainEvent::new(actor.clone(), EXPERIENCES_UPDATED, now)
                    .with_record(record.clone())
                    .with_payload(json!({ "changed_fields": ["experiences"] })),
            )
            .await?;
        self.ports
            .audit()
            .record(
                &mut tx,
                AuditEntry::new(actor, EXPERIENCES_UPDATED, now)
                    .with_record(record)
                    .with_changes(changes)
                    .with_ip(ip),
            )
            .await?;
        self.ports.transactions().commit(tx).await?;
        Ok(self.view(StoredOrganizationExperiences {
            configuration: Some(config),
            updated_at,
        }))
    }
}
