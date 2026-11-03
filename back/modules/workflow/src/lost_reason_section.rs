use ::config_packages::application::ConfigReferenceKind;
use std::collections::BTreeSet;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use uuid::Uuid;

use crate::domain::{LostReason, normalize_name};
use crate::store::PipelineStore;
use ::config_packages::application::sections::common::{check_key, diff};
use ::config_packages::application::{
    ApplyContext, ConfigKeyRepository, ConfigPackage, ConfigSection, Decisions, PackageIssue,
    PlanChange, PlanItem, assign_keys, parse_section,
};
use aphel_kernel::database::transaction::MySqlTx;
use aphel_kernel::domain::access::AccessScope;
use aphel_kernel::errors::AppError;

pub(crate) const LOST_REASONS_SECTION: &str = "lost_reasons";
const LOST_REASON_KIND: &str = "lost_reason";
pub(crate) const MAX_PACKAGE_LOST_REASONS: usize = 100;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct PackageLostReason {
    pub(crate) key: String,
    pub(crate) name: String,
}

struct LostReasonPlan {
    item: PlanItem,
    reason: LostReason,
    is_new: bool,
    key: String,
}

pub(crate) struct LostReasonsSection<K> {
    workflow: PipelineStore,
    keys: K,
}

impl<K> LostReasonsSection<K>
where
    K: ConfigKeyRepository,
{
    pub(crate) fn new(workflow: PipelineStore, keys: K) -> Self {
        Self { workflow, keys }
    }

    async fn keyed(
        &self,
    ) -> Result<Vec<(LostReason, ::config_packages::application::KeyedItem)>, AppError> {
        let reasons = self.workflow.list_lost_reasons().await?;
        let stored = self
            .keys
            .keys(ConfigReferenceKind::new("workflow.lost_reason"))
            .await?;
        let keyed = assign_keys(
            reasons
                .iter()
                .map(|reason| (reason.id, reason.name.as_str())),
            &stored,
        );
        Ok(reasons.into_iter().zip(keyed).collect())
    }

    async fn reconcile(
        &self,
        package: &[PackageLostReason],
        at: DateTime<Utc>,
    ) -> Result<Vec<LostReasonPlan>, AppError> {
        let existing = self.keyed().await?;
        let mut position = existing
            .iter()
            .map(|(reason, _)| reason.position)
            .max()
            .unwrap_or(-1);
        let mut plans = Vec::new();
        for entry in package {
            let name = normalize_name(&entry.name).unwrap_or_else(|| entry.name.clone());
            let item = PlanItem::new(
                LOST_REASONS_SECTION,
                LOST_REASON_KIND,
                entry.key.clone(),
                name.clone(),
            );
            match existing.iter().find(|(_, keyed)| keyed.key == entry.key) {
                None => {
                    position += 1;
                    plans.push(LostReasonPlan {
                        item: item.created(vec![PlanChange::new("name", Value::Null, json!(name))]),
                        reason: LostReason {
                            id: Uuid::now_v7(),
                            name,
                            position,
                            is_active: true,
                            created_at: at,
                            updated_at: at,
                        },
                        is_new: true,
                        key: entry.key.clone(),
                    });
                }
                Some((current, _)) => {
                    let updated = LostReason {
                        name,
                        is_active: true,
                        updated_at: at,
                        ..current.clone()
                    };
                    let changes = diff(&snapshot(current), &snapshot(&updated));
                    plans.push(LostReasonPlan {
                        item: item.compared(changes),
                        reason: updated,
                        is_new: false,
                        key: entry.key.clone(),
                    });
                }
            }
        }
        Ok(plans)
    }
}

fn snapshot(reason: &LostReason) -> Vec<(String, Value)> {
    vec![
        ("name".to_owned(), json!(reason.name)),
        ("active".to_owned(), json!(reason.is_active)),
    ]
}

fn parse(content: &Value) -> Result<Vec<PackageLostReason>, AppError> {
    parse_section(content).map_err(|issue| AppError::Validation(issue.message))
}

#[async_trait]
impl<K> ConfigSection<MySqlTx> for LostReasonsSection<K>
where
    K: ConfigKeyRepository<Tx = MySqlTx>,
{
    fn key(&self) -> &'static str {
        LOST_REASONS_SECTION
    }

    async fn export(&self, _scope: &AccessScope) -> Result<Option<Value>, AppError> {
        let reasons: Vec<PackageLostReason> = self
            .keyed()
            .await?
            .into_iter()
            .filter(|(reason, _)| reason.is_active)
            .map(|(reason, keyed)| PackageLostReason {
                key: keyed.key,
                name: reason.name,
            })
            .collect();
        Ok(Some(json!(reasons)))
    }

    async fn validate(
        &self,
        _scope: &AccessScope,
        _package: &ConfigPackage,
        content: &Value,
    ) -> Result<Vec<PackageIssue>, AppError> {
        let reasons: Vec<PackageLostReason> = match parse_section(content) {
            Ok(reasons) => reasons,
            Err(issue) => return Ok(vec![issue]),
        };
        let mut issues = Vec::new();
        if reasons.len() > MAX_PACKAGE_LOST_REASONS {
            issues.push(PackageIssue::new(
                "",
                "too_many_lost_reasons",
                format!("a package can have at most {MAX_PACKAGE_LOST_REASONS} lost reasons"),
            ));
        }
        let mut keys = BTreeSet::new();
        for (index, reason) in reasons.iter().enumerate() {
            check_key(&reason.key, &format!("{index}.key"), &mut keys, &mut issues);
            if normalize_name(&reason.name).is_none() {
                issues.push(PackageIssue::new(
                    format!("{index}.name"),
                    "invalid_name",
                    "must have between 1 and 140 characters",
                ));
            }
        }
        Ok(issues)
    }

    async fn plan(
        &self,
        _scope: &AccessScope,
        _package: &ConfigPackage,
        content: &Value,
    ) -> Result<Vec<PlanItem>, AppError> {
        Ok(self
            .reconcile(&parse(content)?, DateTime::<Utc>::UNIX_EPOCH)
            .await?
            .into_iter()
            .map(|plan| plan.item)
            .collect())
    }

    async fn apply(
        &self,
        tx: &mut MySqlTx,
        _scope: &AccessScope,
        _package: &ConfigPackage,
        content: &Value,
        decisions: &Decisions,
        context: &mut ApplyContext,
    ) -> Result<(), AppError> {
        for plan in self.reconcile(&parse(content)?, context.at).await? {
            if plan.is_new {
                self.workflow.insert_lost_reason(tx, &plan.reason).await?;
            } else if decisions.executes(&plan.item) {
                self.workflow.update_lost_reason(tx, &plan.reason).await?;
            }
            self.keys
                .assign(
                    tx,
                    ConfigReferenceKind::new("workflow.lost_reason"),
                    plan.reason.id,
                    &plan.key,
                )
                .await?;
            context.resolve(
                ConfigReferenceKind::new("workflow.lost_reason"),
                &plan.key,
                plan.reason.id,
            );
        }
        Ok(())
    }
}
