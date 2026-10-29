use std::collections::BTreeSet;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use uuid::Uuid;

use crate::domain::{
    DisqualificationReason, LifecycleSettings, MAX_DISQUALIFICATION_REASON_NAME_LENGTH,
    MAX_DISQUALIFICATION_REASONS, MAX_STAGE_LENGTH, TransitionRule, TransitionTrigger,
};
use crate::store::{ReasonStore, SettingsStore};
use aphel_kernel::application::config_packages::sections::common::{check_key, diff};
use aphel_kernel::application::config_packages::{
    ApplyContext, ConfigPackage, ConfigSection, Decisions, PackageIssue, PlanChange, PlanItem,
    parse_section,
};
use aphel_kernel::database::transaction::MySqlTx;
use aphel_kernel::domain::access::AccessScope;
use aphel_kernel::errors::AppError;

pub(crate) const LIFECYCLE_SECTION: &str = "lifecycle";
pub(crate) const DISQUALIFICATION_REASON_ITEM: &str = "lead_disqualification_reason";
pub(crate) const LIFECYCLE_SETTINGS_ITEM: &str = "lifecycle_settings";
const SETTINGS_KEY: &str = "settings";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct PackageDisqualificationReason {
    pub(crate) key: String,
    pub(crate) name: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct PackageTransition {
    pub(crate) trigger: String,
    pub(crate) enabled: bool,
    pub(crate) target_stage: String,
    #[serde(default)]
    pub(crate) from_stages: Vec<String>,
    #[serde(default)]
    pub(crate) from_empty: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(deny_unknown_fields)]
pub(crate) struct PackageLifecycle {
    #[serde(default)]
    pub(crate) disqualification_reasons: Vec<PackageDisqualificationReason>,
    #[serde(default)]
    pub(crate) transitions: Vec<PackageTransition>,
    #[serde(default)]
    pub(crate) disqualified_stage: Option<String>,
}

struct ReasonPlan {
    item: PlanItem,
    reason: DisqualificationReason,
    is_new: bool,
}

struct SettingsPlan {
    item: PlanItem,
    settings: LifecycleSettings,
}

pub(crate) struct LifecycleSection {
    settings: SettingsStore,
    reasons: ReasonStore,
}

pub(crate) fn lifecycle_section(pool: &sqlx::MySqlPool) -> LifecycleSection {
    LifecycleSection::new(
        SettingsStore::new(pool.clone()),
        ReasonStore::new(pool.clone()),
    )
}

fn reason_snapshot(reason: &DisqualificationReason) -> Vec<(String, Value)> {
    vec![
        ("name".to_owned(), json!(reason.name)),
        ("active".to_owned(), json!(reason.is_active)),
    ]
}

fn settings_snapshot(settings: &LifecycleSettings) -> Vec<(String, Value)> {
    let mut snapshot: Vec<(String, Value)> = settings
        .rules
        .iter()
        .map(|rule| {
            (
                format!("transitions.{}", rule.trigger.as_str()),
                json!({
                    "enabled": rule.enabled,
                    "target_stage": rule.target_stage,
                    "from_stages": rule.from_stages,
                    "from_empty": rule.from_empty,
                }),
            )
        })
        .collect();
    snapshot.push((
        "disqualified_stage".to_owned(),
        json!(settings.disqualified_stage),
    ));
    snapshot
}

fn parse(content: &Value) -> Result<PackageLifecycle, AppError> {
    parse_section(content).map_err(|issue| AppError::Validation(issue.message))
}

fn is_stage(value: &str) -> bool {
    let length = value.trim().chars().count();
    length > 0 && length <= MAX_STAGE_LENGTH
}

impl LifecycleSection {
    pub(crate) fn new(settings: SettingsStore, reasons: ReasonStore) -> Self {
        Self { settings, reasons }
    }

    async fn reconcile_reasons(
        &self,
        package: &[PackageDisqualificationReason],
        at: DateTime<Utc>,
    ) -> Result<Vec<ReasonPlan>, AppError> {
        let existing = self.reasons.list(true).await?;
        let mut position = existing
            .iter()
            .map(|reason| reason.position)
            .max()
            .unwrap_or(-1);
        let mut plans = Vec::new();
        for entry in package {
            let name = entry.name.trim().to_owned();
            let item = PlanItem::new(
                LIFECYCLE_SECTION,
                DISQUALIFICATION_REASON_ITEM,
                entry.key.clone(),
                name.clone(),
            );
            match existing.iter().find(|current| current.key == entry.key) {
                None => {
                    position += 1;
                    let created = DisqualificationReason {
                        id: Uuid::now_v7(),
                        key: entry.key.clone(),
                        name,
                        is_active: true,
                        position,
                        created_at: at,
                        updated_at: at,
                    };
                    let changes = reason_snapshot(&created)
                        .into_iter()
                        .map(|(property, value)| PlanChange::new(property, Value::Null, value))
                        .collect();
                    plans.push(ReasonPlan {
                        item: item.created(changes),
                        reason: created,
                        is_new: true,
                    });
                }
                Some(current) => {
                    let updated = DisqualificationReason {
                        name,
                        is_active: true,
                        updated_at: at,
                        ..current.clone()
                    };
                    let changes = diff(&reason_snapshot(current), &reason_snapshot(&updated));
                    plans.push(ReasonPlan {
                        item: item.compared(changes),
                        reason: updated,
                        is_new: false,
                    });
                }
            }
        }
        Ok(plans)
    }

    async fn reconcile_settings(
        &self,
        package: &PackageLifecycle,
    ) -> Result<Option<SettingsPlan>, AppError> {
        if package.transitions.is_empty() && package.disqualified_stage.is_none() {
            return Ok(None);
        }
        let current = self.settings.get().await?.normalized();
        let mut next = current.clone();
        for transition in &package.transitions {
            let Some(trigger) = TransitionTrigger::parse(&transition.trigger) else {
                continue;
            };
            next.rules.retain(|rule| rule.trigger != trigger);
            next.rules.push(TransitionRule {
                trigger,
                enabled: transition.enabled,
                target_stage: transition.target_stage.trim().to_owned(),
                from_stages: transition
                    .from_stages
                    .iter()
                    .map(|stage| stage.trim().to_owned())
                    .collect(),
                from_empty: transition.from_empty,
            });
        }
        if let Some(stage) = &package.disqualified_stage {
            let stage = stage.trim();
            next.disqualified_stage = (!stage.is_empty()).then(|| stage.to_owned());
        }
        let next = next.normalized();
        let item = PlanItem::new(
            LIFECYCLE_SECTION,
            LIFECYCLE_SETTINGS_ITEM,
            SETTINGS_KEY,
            "Ciclo de vida",
        );
        let changes = diff(&settings_snapshot(&current), &settings_snapshot(&next));
        let item = if current.updated_at.is_none() && !changes.is_empty() {
            item.created(changes)
        } else {
            item.compared(changes)
        };
        Ok(Some(SettingsPlan {
            item,
            settings: next,
        }))
    }
}

#[async_trait]
impl ConfigSection<MySqlTx> for LifecycleSection {
    fn key(&self) -> &'static str {
        LIFECYCLE_SECTION
    }

    async fn export(&self, _scope: &AccessScope) -> Result<Option<Value>, AppError> {
        let reasons = self
            .reasons
            .list(false)
            .await?
            .into_iter()
            .map(|reason| PackageDisqualificationReason {
                key: reason.key,
                name: reason.name,
            })
            .collect();
        let settings = self.settings.get().await?.normalized();
        let package = PackageLifecycle {
            disqualification_reasons: reasons,
            transitions: settings
                .rules
                .iter()
                .map(|rule| PackageTransition {
                    trigger: rule.trigger.as_str().to_owned(),
                    enabled: rule.enabled,
                    target_stage: rule.target_stage.clone(),
                    from_stages: rule.from_stages.clone(),
                    from_empty: rule.from_empty,
                })
                .collect(),
            disqualified_stage: settings.disqualified_stage,
        };
        Ok(Some(json!(package)))
    }

    async fn validate(
        &self,
        _scope: &AccessScope,
        _package: &ConfigPackage,
        content: &Value,
    ) -> Result<Vec<PackageIssue>, AppError> {
        let package: PackageLifecycle = match parse_section(content) {
            Ok(package) => package,
            Err(issue) => return Ok(vec![issue]),
        };
        let mut issues = Vec::new();
        if package.disqualification_reasons.len() > MAX_DISQUALIFICATION_REASONS {
            issues.push(PackageIssue::new(
                "disqualification_reasons",
                "too_many_disqualification_reasons",
                format!(
                    "a package can have at most {MAX_DISQUALIFICATION_REASONS} disqualification reasons"
                ),
            ));
        }
        let mut keys = BTreeSet::new();
        for (index, reason) in package.disqualification_reasons.iter().enumerate() {
            check_key(
                &reason.key,
                &format!("disqualification_reasons.{index}.key"),
                &mut keys,
                &mut issues,
            );
            let length = reason.name.trim().chars().count();
            if length == 0 || length > MAX_DISQUALIFICATION_REASON_NAME_LENGTH {
                issues.push(PackageIssue::new(
                    format!("disqualification_reasons.{index}.name"),
                    "invalid_name",
                    format!(
                        "must have between 1 and {MAX_DISQUALIFICATION_REASON_NAME_LENGTH} characters"
                    ),
                ));
            }
        }
        let mut triggers = BTreeSet::new();
        for (index, transition) in package.transitions.iter().enumerate() {
            match TransitionTrigger::parse(&transition.trigger) {
                None => issues.push(PackageIssue::new(
                    format!("transitions.{index}.trigger"),
                    "invalid_trigger",
                    "must be deal_created, deal_won or all_deals_lost",
                )),
                Some(trigger) if !triggers.insert(trigger) => issues.push(PackageIssue::new(
                    format!("transitions.{index}.trigger"),
                    "duplicated_trigger",
                    "is declared more than once",
                )),
                Some(_) => {}
            }
            if !is_stage(&transition.target_stage)
                || !transition.from_stages.iter().all(|stage| is_stage(stage))
            {
                issues.push(PackageIssue::new(
                    format!("transitions.{index}"),
                    "invalid_stage",
                    format!("stages must have between 1 and {MAX_STAGE_LENGTH} characters"),
                ));
            }
        }
        if let Some(stage) = &package.disqualified_stage
            && stage.trim().chars().count() > MAX_STAGE_LENGTH
        {
            issues.push(PackageIssue::new(
                "disqualified_stage",
                "invalid_stage",
                format!("must have at most {MAX_STAGE_LENGTH} characters"),
            ));
        }
        Ok(issues)
    }

    async fn plan(
        &self,
        _scope: &AccessScope,
        _package: &ConfigPackage,
        content: &Value,
    ) -> Result<Vec<PlanItem>, AppError> {
        let package = parse(content)?;
        let mut items: Vec<PlanItem> = self
            .reconcile_reasons(
                &package.disqualification_reasons,
                DateTime::<Utc>::UNIX_EPOCH,
            )
            .await?
            .into_iter()
            .map(|plan| plan.item)
            .collect();
        if let Some(plan) = self.reconcile_settings(&package).await? {
            items.push(plan.item);
        }
        Ok(items)
    }

    async fn apply(
        &self,
        tx: &mut MySqlTx,
        scope: &AccessScope,
        _package: &ConfigPackage,
        content: &Value,
        decisions: &Decisions,
        context: &mut ApplyContext,
    ) -> Result<(), AppError> {
        let package = parse(content)?;
        for plan in self
            .reconcile_reasons(&package.disqualification_reasons, context.at)
            .await?
        {
            if plan.is_new {
                self.reasons.insert(tx, &plan.reason).await?;
            } else if decisions.executes(&plan.item) {
                self.reasons.update(tx, &plan.reason).await?;
            }
            context.resolve(
                DISQUALIFICATION_REASON_ITEM,
                &plan.reason.key,
                plan.reason.id,
            );
        }
        if let Some(plan) = self.reconcile_settings(&package).await?
            && decisions.executes(&plan.item)
        {
            let mut settings = plan.settings;
            settings.updated_at = Some(context.at);
            self.settings
                .save(tx, &settings, Some(scope.user_id()), context.at)
                .await?;
        }
        Ok(())
    }
}
