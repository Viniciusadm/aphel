use std::collections::BTreeSet;

use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use uuid::Uuid;

use crate::application::{
    ApplyContext, ConfigInstallRepository, ConfigPackage, ConfigPackageInstall, ConfigPackagePorts,
    ConflictDecision, Decisions, PackageIssue, PackageMetadata, PlanAction, PlanItem,
    TemplateCatalog,
};
use aphel_kernel::errors::{AppError, FieldViolation};
use aphel_sdk::platform::application::access::Authorizer;
use aphel_sdk::platform::application::organization_admin::{ListWindow, Listing};
use aphel_sdk::platform::application::security::Clock;
use aphel_sdk::platform::application::shared::{
    Actor, AuditEntry, AuditLog, DomainEvent, EventPublisher, FieldChange, RecordRef, RecordType,
    TransactionManager,
};
use aphel_sdk::platform::domain::access::{AccessScope, Permission};
use aphel_sdk::platform::domain::organization_profile::DEFAULT_LOCALE;

pub const CONFIG_PACKAGE_APPLIED: &str = "config_package.applied";
pub const CONFIG_PACKAGE_INSTALL_RECORD_TYPE: RecordType =
    RecordType::from_static("config_package_install");

pub const TEMPLATE_NOT_FOUND: &str = "template_not_found";
pub const INSTALL_NOT_FOUND: &str = "config_package_install_not_found";
pub use aphel_sdk::configuration::INVALID_PACKAGE;
pub const PLAN_OUTDATED: &str = "plan_outdated";
pub const INVALID_DECISION: &str = "invalid_decision";

pub use aphel_sdk::configuration::{
    ApplyCommand, ExportCommand, InstallResult, PackagePlan, PackageSource, PlanSummary,
    TemplateSummary,
};

pub struct ConfigPackageUseCases<P> {
    ports: P,
}

impl<P> ConfigPackageUseCases<P>
where
    P: ConfigPackagePorts,
{
    pub fn new(ports: P) -> Self {
        Self { ports }
    }

    pub fn section_keys(&self, scope: &AccessScope) -> Result<Vec<String>, AppError> {
        scope.require(&Permission::SETTINGS_MANAGE)?;
        Ok(self
            .ports
            .sections()
            .keys()
            .into_iter()
            .map(str::to_owned)
            .collect())
    }

    pub async fn export(
        &self,
        scope: &AccessScope,
        command: ExportCommand,
        default_locale: &str,
    ) -> Result<ConfigPackage, AppError> {
        scope.require(&Permission::SETTINGS_MANAGE)?;
        let available = self.ports.sections().keys();
        let selected: Vec<&'static str> = match &command.sections {
            None => available.clone(),
            Some(requested) => {
                let unknown: Vec<&String> = requested
                    .iter()
                    .filter(|key| !available.contains(&key.as_str()))
                    .collect();
                if !unknown.is_empty() {
                    return Err(AppError::invalid_fields(
                        "invalid export",
                        unknown
                            .into_iter()
                            .map(|key| {
                                FieldViolation::new(
                                    "sections",
                                    format!("{key} is not a known section"),
                                )
                            })
                            .collect(),
                    ));
                }
                available
                    .iter()
                    .copied()
                    .filter(|key| requested.iter().any(|requested| requested == key))
                    .collect()
            }
        };
        let mut package = ConfigPackage::new(PackageMetadata {
            name: command
                .name
                .map(|name| name.trim().to_owned())
                .filter(|name| !name.is_empty())
                .unwrap_or_else(|| "Configuração exportada".to_owned()),
            description: command
                .description
                .map(|description| description.trim().to_owned())
                .filter(|description| !description.is_empty()),
            author: command
                .author
                .map(|author| author.trim().to_owned())
                .filter(|author| !author.is_empty()),
            locale: command.locale.unwrap_or_else(|| default_locale.to_owned()),
        });
        let issues = package.header_issues(
            &aphel_sdk::platform::application::organization_profile::SUPPORTED_LOCALES,
        );
        if !issues.is_empty() {
            return Err(AppError::invalid_fields(
                "invalid export",
                issues
                    .into_iter()
                    .map(|issue| FieldViolation::new(issue.path, issue.message))
                    .collect(),
            ));
        }
        for section in self.ports.sections().sections() {
            if !selected.contains(&section.key()) {
                continue;
            }
            if let Some(content) = section.export(scope).await? {
                package.sections.insert(section.key().to_owned(), content);
            }
        }
        Ok(package)
    }

    pub async fn templates(&self, scope: &AccessScope) -> Result<Vec<TemplateSummary>, AppError> {
        scope.require(&Permission::SETTINGS_MANAGE)?;
        let locale = self.template_locale().await?;
        let mut templates: Vec<TemplateSummary> = self
            .ports
            .templates()
            .templates(&locale)
            .into_iter()
            .filter_map(|template| {
                let package = ConfigPackage::parse(&template.package).ok()?;
                Some(TemplateSummary {
                    key: template.key,
                    name: package.name,
                    description: package.description,
                    author: package.author,
                    locale: package.locale,
                    sections: package.sections.keys().cloned().collect(),
                })
            })
            .collect();
        templates.sort_by(|left, right| left.key.cmp(&right.key));
        Ok(templates)
    }

    pub async fn template(&self, scope: &AccessScope, key: &str) -> Result<Value, AppError> {
        scope.require(&Permission::SETTINGS_MANAGE)?;
        let locale = self.template_locale().await?;
        self.ports
            .templates()
            .template(key, &locale)
            .map(|template| template.package)
            .ok_or_else(template_not_found)
    }

    async fn template_locale(&self) -> Result<String, AppError> {
        Ok(self
            .ports
            .installs()
            .organization_locale()
            .await?
            .unwrap_or_else(|| DEFAULT_LOCALE.to_owned()))
    }

    pub async fn plan(
        &self,
        scope: &AccessScope,
        source: PackageSource,
    ) -> Result<PackagePlan, AppError> {
        scope.require(&Permission::SETTINGS_MANAGE)?;
        let (template_key, value) = self.resolve_source(source).await?;
        let (package, issues, skipped_fields) = self.inspect(scope, &value).await?;
        let Some(package) = package.filter(|_| issues.is_empty()) else {
            return Ok(PackagePlan {
                package: ConfigPackage::parse(&value)
                    .ok()
                    .map(|package| metadata(&package)),
                template_key,
                sections: Vec::new(),
                skipped_sections: Vec::new(),
                skipped_fields,
                valid: false,
                issues,
                items: Vec::new(),
                fingerprint: String::new(),
                summary: PlanSummary::default(),
            });
        };
        let items = self.plan_items(scope, &package).await?;
        Ok(PackagePlan {
            package: Some(metadata(&package)),
            template_key,
            sections: self.present_sections(&package),
            skipped_sections: self.skipped_sections(&package),
            skipped_fields,
            valid: true,
            issues,
            fingerprint: fingerprint(&items)?,
            summary: PlanSummary::of(&items),
            items,
        })
    }

    pub async fn apply(
        &self,
        scope: &AccessScope,
        command: ApplyCommand,
    ) -> Result<InstallResult, AppError> {
        scope.require(&Permission::SETTINGS_MANAGE)?;
        let (template_key, value) = self.resolve_source(command.source).await?;
        let (package, issues, _) = self.inspect(scope, &value).await?;
        let Some(package) = package.filter(|_| issues.is_empty()) else {
            return Err(AppError::invalid_fields_code(
                INVALID_PACKAGE,
                "the package is not valid",
                issues
                    .into_iter()
                    .map(|issue| FieldViolation::new(issue.path, issue.message))
                    .collect(),
            ));
        };
        let items = self.plan_items(scope, &package).await?;
        if fingerprint(&items)? != command.fingerprint {
            return Err(AppError::conflict_code(
                PLAN_OUTDATED,
                "the organization changed since the plan was made; review the plan again",
            ));
        }
        let decisions = Decisions::new(command.decisions);
        validate_decisions(&items, &decisions)?;

        let now = self.ports.clock().now();
        let mut context = ApplyContext::new(now);
        let mut tx = self.ports.transactions().begin().await?;
        for section in self.ports.sections().sections() {
            if !self.ports.sections().is_available(section.key()) {
                continue;
            }
            let Some(content) = package.section(section.key()) else {
                continue;
            };
            section
                .apply(&mut tx, scope, &package, content, &decisions, &mut context)
                .await?;
        }
        let applied: Vec<PlanItem> = items
            .iter()
            .filter(|item| decisions.executes(item))
            .cloned()
            .collect();
        let summary = PlanSummary {
            create: count(&applied, PlanAction::Create),
            update: applied.len() as u64 - count(&applied, PlanAction::Create),
            skip_conflict: items
                .iter()
                .filter(|item| item.action == PlanAction::SkipConflict && !decisions.executes(item))
                .count() as u64,
            unchanged: count(&items, PlanAction::Unchanged),
        };
        let install = ConfigPackageInstall {
            id: Uuid::now_v7(),
            name: package.name.clone(),
            format_version: package.format_version,
            template_key: template_key.clone(),
            package: value,
            summary: json!({
                "create": summary.create,
                "update": summary.update,
                "skip_conflict": summary.skip_conflict,
                "unchanged": summary.unchanged,
                "no_op": applied.is_empty(),
                "sections": self.present_sections(&package),
            }),
            installed_by: Some(scope.user_id().clone()),
            installed_by_name: None,
            installed_at: now,
        };
        self.ports.installs().insert(&mut tx, &install).await?;
        let record = RecordRef::new(CONFIG_PACKAGE_INSTALL_RECORD_TYPE, install.id);
        let actor = Actor::from_scope(scope);
        self.ports
            .events()
            .publish(
                &mut tx,
                DomainEvent::new(actor.clone(), CONFIG_PACKAGE_APPLIED, now)
                    .with_record(record.clone())
                    .with_payload(json!({
                        "install_id": install.id.to_string(),
                        "name": install.name,
                        "template_key": template_key,
                        "summary": install.summary,
                    })),
            )
            .await?;
        self.ports
            .audit()
            .record(
                &mut tx,
                AuditEntry::new(actor, CONFIG_PACKAGE_APPLIED, now)
                    .with_record(record)
                    .with_changes(
                        applied
                            .iter()
                            .map(|item| {
                                FieldChange::new(
                                    item.id.clone(),
                                    Value::Null,
                                    Value::from(item.action.as_str()),
                                )
                            })
                            .collect(),
                    )
                    .with_ip(command.ip),
            )
            .await?;
        self.ports.transactions().commit(tx).await?;
        Ok(InstallResult {
            install,
            applied,
            summary,
        })
    }

    pub async fn installs(
        &self,
        scope: &AccessScope,
        window: ListWindow,
    ) -> Result<Listing<ConfigPackageInstall>, AppError> {
        scope.require(&Permission::SETTINGS_MANAGE)?;
        self.ports.installs().list(window).await
    }

    pub async fn install(
        &self,
        scope: &AccessScope,
        install_id: &str,
    ) -> Result<ConfigPackageInstall, AppError> {
        scope.require(&Permission::SETTINGS_MANAGE)?;
        let id = Uuid::parse_str(install_id).map_err(|_| install_not_found())?;
        self.ports
            .installs()
            .find(id)
            .await?
            .ok_or_else(install_not_found)
    }

    async fn resolve_source(
        &self,
        source: PackageSource,
    ) -> Result<(Option<String>, Value), AppError> {
        match source {
            PackageSource::Package(value) => Ok((None, value)),
            PackageSource::Template(key) => {
                let locale = self.template_locale().await?;
                let template = self
                    .ports
                    .templates()
                    .template(&key, &locale)
                    .ok_or_else(template_not_found)?;
                Ok((Some(template.key), template.package))
            }
        }
    }

    async fn inspect(
        &self,
        scope: &AccessScope,
        value: &Value,
    ) -> Result<(Option<ConfigPackage>, Vec<PackageIssue>, Vec<String>), AppError> {
        let mut package = match ConfigPackage::parse(value) {
            Ok(package) => package,
            Err(issues) => return Ok((None, issues, Vec::new())),
        };
        let skipped_fields = self.prune(&mut package);
        let mut issues = package.header_issues(
            &aphel_sdk::platform::application::organization_profile::SUPPORTED_LOCALES,
        );
        let known: BTreeSet<&str> = self.ports.sections().keys().into_iter().collect();
        for key in package.sections.keys() {
            if !known.contains(key.as_str()) {
                issues.push(PackageIssue::new(
                    format!("sections.{key}"),
                    "unknown_section",
                    "is not a section known by this Aphel",
                ));
            }
        }
        for section in self.ports.sections().sections() {
            if !self.ports.sections().is_available(section.key()) {
                continue;
            }
            let Some(content) = package.section(section.key()) else {
                continue;
            };
            let prefix = format!("sections.{}", section.key());
            issues.extend(
                section
                    .validate(scope, &package, content)
                    .await?
                    .into_iter()
                    .map(|issue| issue.under(&prefix)),
            );
        }
        Ok((Some(package), issues, skipped_fields))
    }

    async fn plan_items(
        &self,
        scope: &AccessScope,
        package: &ConfigPackage,
    ) -> Result<Vec<PlanItem>, AppError> {
        let mut items = Vec::new();
        for section in self.ports.sections().sections() {
            if !self.ports.sections().is_available(section.key()) {
                continue;
            }
            let Some(content) = package.section(section.key()) else {
                continue;
            };
            items.extend(section.plan(scope, package, content).await?);
        }
        Ok(items)
    }

    fn prune(&self, package: &mut ConfigPackage) -> Vec<String> {
        let registry = self.ports.sections();
        let unavailable = registry.unavailable_fields();
        if unavailable.is_empty() {
            return Vec::new();
        }
        let mut removed = BTreeSet::new();
        for section in registry.sections() {
            if !registry.is_available(section.key()) {
                continue;
            }
            if let Some(content) = package.sections.get_mut(section.key()) {
                removed.extend(section.prune(content, unavailable));
            }
        }
        removed.into_iter().collect()
    }

    fn skipped_sections(&self, package: &ConfigPackage) -> Vec<String> {
        let registry = self.ports.sections();
        registry
            .keys()
            .into_iter()
            .filter(|key| !registry.is_available(key) && package.section(key).is_some())
            .map(str::to_owned)
            .collect()
    }

    fn present_sections(&self, package: &ConfigPackage) -> Vec<String> {
        let registry = self.ports.sections();
        registry
            .keys()
            .into_iter()
            .filter(|key| registry.is_available(key) && package.section(key).is_some())
            .map(str::to_owned)
            .collect()
    }
}

fn metadata(package: &ConfigPackage) -> PackageMetadata {
    PackageMetadata {
        name: package.name.clone(),
        description: package.description.clone(),
        author: package.author.clone(),
        locale: package.locale.clone(),
    }
}

fn count(items: &[PlanItem], action: PlanAction) -> u64 {
    items.iter().filter(|item| item.action == action).count() as u64
}

fn fingerprint(items: &[PlanItem]) -> Result<String, AppError> {
    let bytes = serde_json::to_vec(items).map_err(|error| AppError::Internal(error.to_string()))?;
    Ok(Sha256::digest(&bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect())
}

fn validate_decisions(items: &[PlanItem], decisions: &Decisions) -> Result<(), AppError> {
    let mut violations = Vec::new();
    for (item_id, decision) in decisions.iter() {
        let path = format!("decisions.{item_id}");
        match items.iter().find(|item| &item.id == item_id) {
            None => violations.push(FieldViolation::new(path, "is not an item of the plan")),
            Some(item) if item.action != PlanAction::SkipConflict => {
                violations.push(FieldViolation::new(path, "is not a conflict"))
            }
            Some(item)
                if *decision == ConflictDecision::Apply
                    && !item
                        .conflict
                        .as_ref()
                        .is_some_and(|conflict| conflict.resolvable) =>
            {
                violations.push(FieldViolation::new(
                    path,
                    "this conflict cannot be applied, only skipped",
                ))
            }
            Some(_) => {}
        }
    }
    if violations.is_empty() {
        Ok(())
    } else {
        violations.sort_by(|left, right| left.field.cmp(&right.field));
        Err(AppError::invalid_fields_code(
            INVALID_DECISION,
            "invalid conflict decisions",
            violations,
        ))
    }
}

fn template_not_found() -> AppError {
    AppError::not_found_code(TEMPLATE_NOT_FOUND, "template was not found")
}

fn install_not_found() -> AppError {
    AppError::not_found_code(INSTALL_NOT_FOUND, "installation was not found")
}
