use std::cmp::Ordering;
use std::collections::{BTreeMap, BTreeSet, HashMap};

use serde_json::{Value, json};
use uuid::Uuid;

use crate::application::versioning::{
    ConfigVersionPorts, ConfigVersionRepository, InstalledVersion, ItemResolution, PackageFragment,
    PackageIdentity, VersionItem, canonical_hash, compare_versions, fragment_hash, fragments,
    identity_issues, package_changes, replace_fragment, sha256_hex,
};
use crate::application::{
    ApplyContext, ConfigInstallRepository, ConfigPackage, ConfigPackageInstall, ConfigPackagePorts,
    ConflictDecision, Decisions, INVALID_PACKAGE, PLAN_OUTDATED, PackageIssue, PlanAction,
    PlanItem, PlanSummary,
};
use aphel_kernel::application::access::Authorizer;
use aphel_kernel::application::security::Clock;
use aphel_kernel::application::shared::{
    Actor, AuditEntry, AuditLog, DomainEvent, EventPublisher, FieldChange, RecordRef, RecordType,
    TransactionManager,
};
use aphel_kernel::domain::access::{AccessScope, Permission};
use aphel_kernel::errors::{AppError, FieldViolation};

pub const CONFIG_PACKAGE_VERSION_APPLIED: &str = "config_package.version_applied";
pub const CONFIG_PACKAGE_VERSION_RECORD_TYPE: RecordType =
    RecordType::from_static("config_package_version");
pub const INVALID_RESOLUTION: &str = "invalid_resolution";

pub use aphel_sdk::configuration::versioning::{
    ApplyVersionCommand, ComparisonSummary, PackageStatus, Resolution, VersionApplyResult,
    VersionComparison, VersionItemComparison, VersionRelation,
};

struct Analysis {
    comparison: VersionComparison,
    package: Option<ConfigPackage>,
}

struct LocalState {
    changed: HashMap<String, bool>,
    unknown_sections: BTreeSet<String>,
}

pub struct ConfigVersionUseCases<P> {
    ports: P,
}

impl<P> ConfigVersionUseCases<P>
where
    P: ConfigVersionPorts,
{
    pub fn new(ports: P) -> Self {
        Self { ports }
    }

    pub async fn installed(&self, scope: &AccessScope) -> Result<Vec<InstalledVersion>, AppError> {
        scope.require(&Permission::SETTINGS_MANAGE)?;
        self.ports.versions().list().await
    }

    pub async fn latest(
        &self,
        scope: &AccessScope,
        package_id: &str,
    ) -> Result<Option<InstalledVersion>, AppError> {
        scope.require(&Permission::SETTINGS_MANAGE)?;
        self.ports.versions().latest(package_id).await
    }

    pub async fn compare(
        &self,
        scope: &AccessScope,
        package: &Value,
    ) -> Result<VersionComparison, AppError> {
        scope.require(&Permission::SETTINGS_MANAGE)?;
        Ok(self.analyze(scope, package).await?.comparison)
    }

    pub async fn apply(
        &self,
        scope: &AccessScope,
        command: ApplyVersionCommand,
    ) -> Result<VersionApplyResult, AppError> {
        scope.require(&Permission::SETTINGS_MANAGE)?;
        let analysis = self.analyze(scope, &command.package).await?;
        let comparison = analysis.comparison;
        let (Some(package), Some(identity), true) = (
            analysis.package,
            comparison.package.clone(),
            comparison.valid,
        ) else {
            return Err(invalid_package(comparison.issues));
        };
        if comparison.fingerprint != command.fingerprint {
            return Err(AppError::conflict_code(
                PLAN_OUTDATED,
                "the organization or the installed version changed since the comparison; compare again",
            ));
        }
        validate_resolutions(&comparison.items, &command.resolutions)?;
        let resolution = |id: &str| {
            command
                .resolutions
                .get(id)
                .copied()
                .unwrap_or(Resolution::KeepLocal)
        };

        let kept: Vec<&VersionItemComparison> = comparison
            .items
            .iter()
            .filter(|item| item.conflict && resolution(&item.id) == Resolution::KeepLocal)
            .collect();
        let new_fragments = fragments(&package.sections);
        let mut merged = package.clone();
        if !kept.is_empty() {
            let local = self.local_fragments(scope, &kept).await?;
            for item in &kept {
                if let Some(fragment) = new_fragments.get(&item.id) {
                    replace_fragment(
                        &mut merged.sections,
                        fragment,
                        local.get(&item.id).map(|local| local.value.clone()),
                    );
                }
            }
            let merged_value = serde_json::to_value(&merged)
                .map_err(|error| AppError::Internal(error.to_string()))?;
            let (_, issues) = self.inspect(scope, &merged_value).await?;
            if !issues.is_empty() {
                return Err(invalid_package(issues));
            }
        }

        let plan = self.plan_items(scope, &merged).await?;
        let decisions = Decisions::new(
            plan.iter()
                .filter(|item| item.action == PlanAction::SkipConflict)
                .map(|item| {
                    let decision = match resolution(&item.id) {
                        Resolution::AcceptPackage => ConflictDecision::Apply,
                        Resolution::KeepLocal => ConflictDecision::Skip,
                    };
                    (item.id.clone(), decision)
                })
                .collect(),
        );
        let kept_ids: BTreeSet<String> = kept.iter().map(|item| item.id.clone()).collect();
        let mut version_items: Vec<VersionItem> = plan
            .iter()
            .map(|item| VersionItem {
                item_id: item.id.clone(),
                section: item.section.clone(),
                kind: item.kind.clone(),
                key: item.key.clone(),
                package_hash: fragment_hash(
                    &new_fragments,
                    &package.sections,
                    &item.id,
                    &item.section,
                )
                .unwrap_or_default(),
                resolution: if kept_ids.contains(&item.id) {
                    ItemResolution::KeptLocal
                } else if decisions.executes(item) {
                    ItemResolution::Applied
                } else if item.action == PlanAction::Unchanged {
                    ItemResolution::Unchanged
                } else {
                    ItemResolution::Skipped
                },
            })
            .collect();
        for item in &kept {
            if version_items.iter().any(|stored| stored.item_id == item.id) {
                continue;
            }
            version_items.push(VersionItem {
                item_id: item.id.clone(),
                section: item.section.clone(),
                kind: item.kind.clone(),
                key: item.key.clone(),
                package_hash: new_fragments
                    .get(&item.id)
                    .map(PackageFragment::hash)
                    .unwrap_or_default(),
                resolution: ItemResolution::KeptLocal,
            });
        }

        let packages = self.ports.packages();
        let now = packages.clock().now();
        let mut context = ApplyContext::new(now);
        let mut tx = packages.transactions().begin().await?;
        for section in packages.sections().sections() {
            let Some(content) = merged.section(section.key()) else {
                continue;
            };
            section
                .apply(&mut tx, scope, &merged, content, &decisions, &mut context)
                .await?;
        }
        let applied: Vec<&PlanItem> = plan
            .iter()
            .filter(|item| decisions.executes(item))
            .collect();
        let created = applied
            .iter()
            .filter(|item| item.action == PlanAction::Create)
            .count() as u64;
        let summary = PlanSummary {
            create: created,
            update: applied.len() as u64 - created,
            skip_conflict: plan
                .iter()
                .filter(|item| item.action == PlanAction::SkipConflict && !decisions.executes(item))
                .count() as u64,
            unchanged: plan
                .iter()
                .filter(|item| item.action == PlanAction::Unchanged)
                .count() as u64,
        };
        let sections: Vec<String> = packages
            .sections()
            .keys()
            .into_iter()
            .filter(|key| merged.section(key).is_some())
            .map(str::to_owned)
            .collect();
        let previous_version = comparison
            .installed
            .as_ref()
            .map(|installed| installed.version.clone());
        let install = ConfigPackageInstall {
            id: Uuid::now_v7(),
            name: package.name.clone(),
            format_version: package.format_version,
            template_key: None,
            package: command.package.clone(),
            summary: json!({
                "create": summary.create,
                "update": summary.update,
                "skip_conflict": summary.skip_conflict,
                "unchanged": summary.unchanged,
                "sections": sections,
                "package_id": identity.id,
                "version": identity.version,
                "kept_local": kept_ids.len(),
            }),
            installed_by: Some(scope.user_id().clone()),
            installed_by_name: None,
            installed_at: now,
        };
        packages.installs().insert(&mut tx, &install).await?;
        let version = InstalledVersion {
            id: Uuid::now_v7(),
            package_id: identity.id.clone(),
            version: identity.version.clone(),
            name: package.name.clone(),
            install_id: install.id,
            origin: command.origin.clone(),
            package_sha256: command
                .checksum
                .clone()
                .unwrap_or_else(|| canonical_hash(&command.package)),
            applied_package: serde_json::to_value(&merged)
                .map_err(|error| AppError::Internal(error.to_string()))?,
            installed_by: Some(scope.user_id().clone()),
            installed_by_name: None,
            installed_at: now,
        };
        self.ports
            .versions()
            .insert(&mut tx, &version, &version_items)
            .await?;
        let record = RecordRef::new(CONFIG_PACKAGE_VERSION_RECORD_TYPE, version.id);
        let actor = Actor::from_scope(scope);
        let kept_local: Vec<String> = kept_ids.into_iter().collect();
        packages
            .events()
            .publish(
                &mut tx,
                DomainEvent::new(actor.clone(), CONFIG_PACKAGE_VERSION_APPLIED, now)
                    .with_record(record.clone())
                    .with_payload(json!({
                        "version_id": version.id.to_string(),
                        "install_id": install.id.to_string(),
                        "package_id": version.package_id,
                        "version": version.version,
                        "previous_version": previous_version,
                        "origin": version.origin,
                        "summary": install.summary,
                        "kept_local": kept_local,
                    })),
            )
            .await?;
        let mut changes = vec![FieldChange::new(
            "version",
            previous_version.clone().map_or(Value::Null, Value::from),
            Value::from(version.version.clone()),
        )];
        changes.extend(
            version_items
                .iter()
                .filter(|item| {
                    matches!(
                        item.resolution,
                        ItemResolution::Applied | ItemResolution::KeptLocal
                    )
                })
                .map(|item| {
                    FieldChange::new(
                        item.item_id.clone(),
                        Value::Null,
                        Value::from(item.resolution.as_str()),
                    )
                }),
        );
        packages
            .audit()
            .record(
                &mut tx,
                AuditEntry::new(actor, CONFIG_PACKAGE_VERSION_APPLIED, now)
                    .with_record(record)
                    .with_changes(changes)
                    .with_ip(command.ip),
            )
            .await?;
        packages.transactions().commit(tx).await?;
        Ok(VersionApplyResult {
            version,
            previous_version,
            kept_local,
            summary,
        })
    }

    async fn analyze(&self, scope: &AccessScope, value: &Value) -> Result<Analysis, AppError> {
        let (package, issues) = self.inspect(scope, value).await?;
        let identity = package.as_ref().and_then(PackageIdentity::of);
        let Some(package) = package.filter(|_| issues.is_empty()) else {
            return Ok(Analysis {
                comparison: VersionComparison {
                    package: identity,
                    installed: None,
                    relation: None,
                    valid: false,
                    issues,
                    sections: Vec::new(),
                    items: Vec::new(),
                    summary: ComparisonSummary::default(),
                    fingerprint: String::new(),
                },
                package: None,
            });
        };
        let identity = identity.ok_or_else(|| {
            AppError::Internal("a valid versioned package must have an identity".to_owned())
        })?;
        let installed = self.ports.versions().latest(&identity.id).await?;
        let new_fragments = fragments(&package.sections);
        let plan = self.plan_items(scope, &package).await?;

        let mut previous_sections = BTreeMap::new();
        let mut stored: HashMap<String, VersionItem> = HashMap::new();
        let mut local = LocalState {
            changed: HashMap::new(),
            unknown_sections: BTreeSet::new(),
        };
        if let Some(installed) = &installed {
            stored = self
                .ports
                .versions()
                .items(installed.id)
                .await?
                .into_iter()
                .map(|item| (item.item_id.clone(), item))
                .collect();
            let original = self
                .ports
                .packages()
                .installs()
                .find(installed.install_id)
                .await?
                .map(|install| install.package)
                .unwrap_or_else(|| installed.applied_package.clone());
            previous_sections = ConfigPackage::parse(&original)
                .or_else(|_| ConfigPackage::parse(&installed.applied_package))
                .map(|package| package.sections)
                .unwrap_or_default();
            local = self.local_state(scope, &installed.applied_package).await;
        }
        let previous_fragments = fragments(&previous_sections);

        let mut items: Vec<VersionItemComparison> = Vec::new();
        let mut seen = BTreeSet::new();
        for plan_item in plan {
            seen.insert(plan_item.id.clone());
            let new_hash = fragment_hash(
                &new_fragments,
                &package.sections,
                &plan_item.id,
                &plan_item.section,
            );
            let previous_hash = installed.as_ref().and_then(|_| {
                fragment_hash(
                    &previous_fragments,
                    &previous_sections,
                    &plan_item.id,
                    &plan_item.section,
                )
            });
            let status = match previous_hash {
                None => PackageStatus::Added,
                Some(hash) if Some(&hash) == new_hash.as_ref() => PackageStatus::Unchanged,
                Some(_) => PackageStatus::Changed,
            };
            let changes = if status == PackageStatus::Changed {
                package_changes(
                    previous_fragments
                        .get(&plan_item.id)
                        .map(|fragment| &fragment.value),
                    new_fragments
                        .get(&plan_item.id)
                        .map(|fragment| &fragment.value),
                )
            } else {
                Vec::new()
            };
            let locally_modified = status != PackageStatus::Added
                && (stored
                    .get(&plan_item.id)
                    .is_some_and(|item| item.resolution == ItemResolution::KeptLocal)
                    || local.unknown_sections.contains(&plan_item.section)
                    || local.changed.get(&plan_item.id).copied().unwrap_or(false));
            items.push(VersionItemComparison {
                id: plan_item.id.clone(),
                section: plan_item.section.clone(),
                kind: plan_item.kind.clone(),
                key: plan_item.key.clone(),
                label: plan_item.label.clone(),
                package_status: status,
                package_changes: changes,
                locally_modified,
                conflict: status == PackageStatus::Changed && locally_modified,
                plan: Some(plan_item),
            });
        }
        for (id, fragment) in &previous_fragments {
            if new_fragments.contains_key(id) || seen.contains(id) {
                continue;
            }
            items.push(VersionItemComparison {
                id: id.clone(),
                section: fragment.section.clone(),
                kind: fragment.kind.clone(),
                key: fragment.key.clone(),
                label: fragment
                    .value
                    .get("name")
                    .or_else(|| fragment.value.get("label"))
                    .and_then(Value::as_str)
                    .unwrap_or(&fragment.key)
                    .to_owned(),
                package_status: PackageStatus::Removed,
                package_changes: package_changes(Some(&fragment.value), None),
                locally_modified: false,
                conflict: false,
                plan: None,
            });
        }

        let relation = Some(match &installed {
            None => VersionRelation::Install,
            Some(installed) => match compare_versions(&identity.version, &installed.version) {
                Some(Ordering::Greater) => VersionRelation::Upgrade,
                Some(Ordering::Less) => VersionRelation::Downgrade,
                _ => VersionRelation::Reinstall,
            },
        });
        let fingerprint = sha256_hex(
            &serde_json::to_vec(&json!({
                "installed": installed.as_ref().map(|installed| installed.id.to_string()),
                "items": items,
            }))
            .map_err(|error| AppError::Internal(error.to_string()))?,
        );
        let sections = self
            .ports
            .packages()
            .sections()
            .keys()
            .into_iter()
            .filter(|key| package.section(key).is_some())
            .map(str::to_owned)
            .collect();
        Ok(Analysis {
            comparison: VersionComparison {
                package: Some(identity),
                installed,
                relation,
                valid: true,
                issues: Vec::new(),
                sections,
                summary: ComparisonSummary::of(&items),
                items,
                fingerprint,
            },
            package: Some(package),
        })
    }

    async fn local_state(&self, scope: &AccessScope, applied: &Value) -> LocalState {
        let mut state = LocalState {
            changed: HashMap::new(),
            unknown_sections: BTreeSet::new(),
        };
        let Ok(package) = ConfigPackage::parse(applied) else {
            state.unknown_sections = self
                .ports
                .packages()
                .sections()
                .keys()
                .into_iter()
                .map(str::to_owned)
                .collect();
            return state;
        };
        for section in self.ports.packages().sections().sections() {
            let Some(content) = package.section(section.key()) else {
                continue;
            };
            match section.plan(scope, &package, content).await {
                Ok(items) => {
                    for item in items {
                        state
                            .changed
                            .insert(item.id, item.action != PlanAction::Unchanged);
                    }
                }
                Err(_) => {
                    state.unknown_sections.insert(section.key().to_owned());
                }
            }
        }
        state
    }

    async fn local_fragments(
        &self,
        scope: &AccessScope,
        items: &[&VersionItemComparison],
    ) -> Result<BTreeMap<String, PackageFragment>, AppError> {
        let wanted: BTreeSet<&str> = items.iter().map(|item| item.section.as_str()).collect();
        let mut exported = BTreeMap::new();
        for section in self.ports.packages().sections().sections() {
            if !wanted.contains(section.key()) {
                continue;
            }
            if let Some(content) = section.export(scope).await? {
                exported.insert(section.key().to_owned(), content);
            }
        }
        Ok(fragments(&exported))
    }

    async fn inspect(
        &self,
        scope: &AccessScope,
        value: &Value,
    ) -> Result<(Option<ConfigPackage>, Vec<PackageIssue>), AppError> {
        let package = match ConfigPackage::parse(value) {
            Ok(package) => package,
            Err(issues) => return Ok((None, issues)),
        };
        let mut issues = package
            .header_issues(&aphel_kernel::application::organization_profile::SUPPORTED_LOCALES);
        issues.extend(identity_issues(&package));
        let registry = self.ports.packages().sections();
        let known: BTreeSet<&str> = registry.keys().into_iter().collect();
        for key in package.sections.keys() {
            if !known.contains(key.as_str()) {
                issues.push(PackageIssue::new(
                    format!("sections.{key}"),
                    "unknown_section",
                    "is not a section known by this Aphel",
                ));
            }
        }
        for section in registry.sections() {
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
        Ok((Some(package), issues))
    }

    async fn plan_items(
        &self,
        scope: &AccessScope,
        package: &ConfigPackage,
    ) -> Result<Vec<PlanItem>, AppError> {
        let mut items = Vec::new();
        for section in self.ports.packages().sections().sections() {
            let Some(content) = package.section(section.key()) else {
                continue;
            };
            items.extend(section.plan(scope, package, content).await?);
        }
        Ok(items)
    }
}

fn validate_resolutions(
    items: &[VersionItemComparison],
    resolutions: &HashMap<String, Resolution>,
) -> Result<(), AppError> {
    let mut violations: Vec<FieldViolation> = resolutions
        .keys()
        .filter_map(|id| {
            let path = format!("resolutions.{id}");
            match items.iter().find(|item| &item.id == id) {
                None => Some(FieldViolation::new(
                    path,
                    "is not an item of the comparison",
                )),
                Some(item) if !item.needs_resolution() => Some(FieldViolation::new(
                    path,
                    "is not a conflict that can be resolved",
                )),
                Some(_) => None,
            }
        })
        .collect();
    if violations.is_empty() {
        return Ok(());
    }
    violations.sort_by(|left, right| left.field.cmp(&right.field));
    Err(AppError::invalid_fields_code(
        INVALID_RESOLUTION,
        "invalid conflict resolutions",
        violations,
    ))
}

fn invalid_package(issues: Vec<PackageIssue>) -> AppError {
    AppError::invalid_fields_code(
        INVALID_PACKAGE,
        "the package is not valid",
        issues
            .into_iter()
            .map(|issue| FieldViolation::new(issue.path, issue.message))
            .collect(),
    )
}
