use std::collections::BTreeSet;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use uuid::Uuid;

use crate::domain::CallOutcome;
use crate::store::CallOutcomeStore;
use aphel_kernel::errors::AppError;
use aphel_sdk::configuration::ConfigReferenceKind;
use aphel_sdk::configuration::sections::common::{check_key, diff};
use aphel_sdk::configuration::{
    ApplyContext, ConfigPackage, ConfigSection, Decisions, PackageIssue, PlanChange, PlanItem,
    parse_section,
};
use aphel_sdk::platform::database::transaction::MySqlTx;
use aphel_sdk::platform::domain::access::AccessScope;

pub(crate) const CALL_OUTCOMES_SECTION: &str = "call_outcomes";
const CALL_OUTCOME_KIND: &str = "call_outcome";
pub(crate) const MAX_PACKAGE_CALL_OUTCOMES: usize = 50;
const MAX_NAME_LENGTH: usize = 140;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct PackageCallOutcome {
    pub(crate) key: String,
    pub(crate) name: String,
    #[serde(default)]
    pub(crate) connected: bool,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub(crate) system: bool,
}

struct OutcomePlan {
    item: PlanItem,
    outcome: CallOutcome,
    is_new: bool,
}

pub(crate) struct CallOutcomesSection {
    outcomes: CallOutcomeStore,
}

pub(crate) fn call_outcomes_section(pool: sqlx::MySqlPool) -> CallOutcomesSection {
    CallOutcomesSection {
        outcomes: CallOutcomeStore::new(pool),
    }
}

fn snapshot(outcome: &CallOutcome) -> Vec<(String, Value)> {
    vec![
        ("name".to_owned(), json!(outcome.name)),
        ("connected".to_owned(), json!(outcome.is_connected)),
        ("active".to_owned(), json!(outcome.is_active)),
    ]
}

fn parse(content: &Value) -> Result<Vec<PackageCallOutcome>, AppError> {
    parse_section(content).map_err(|issue| AppError::Validation(issue.message))
}

impl CallOutcomesSection {
    async fn reconcile(
        &self,
        package: &[PackageCallOutcome],
        at: DateTime<Utc>,
    ) -> Result<Vec<OutcomePlan>, AppError> {
        let existing = self.outcomes.list(true).await?;
        let mut position = existing
            .iter()
            .map(|outcome| outcome.position)
            .max()
            .unwrap_or(-1);
        let mut plans = Vec::new();
        for entry in package {
            let name = entry.name.trim().to_owned();
            let item = PlanItem::new(
                CALL_OUTCOMES_SECTION,
                CALL_OUTCOME_KIND,
                entry.key.clone(),
                name.clone(),
            );
            match existing.iter().find(|current| current.key == entry.key) {
                None => {
                    position += 1;
                    let created = CallOutcome {
                        id: Uuid::now_v7(),
                        key: entry.key.clone(),
                        name,
                        is_connected: entry.connected,
                        is_system: entry.system,
                        is_active: true,
                        position,
                        created_at: at,
                        updated_at: at,
                    };
                    let changes = snapshot(&created)
                        .into_iter()
                        .map(|(property, value)| PlanChange::new(property, Value::Null, value))
                        .collect();
                    plans.push(OutcomePlan {
                        item: item.created(changes),
                        outcome: created,
                        is_new: true,
                    });
                }
                Some(current) => {
                    let updated = CallOutcome {
                        name,
                        is_connected: entry.connected,
                        is_active: true,
                        updated_at: at,
                        ..current.clone()
                    };
                    let changes = diff(&snapshot(current), &snapshot(&updated));
                    plans.push(OutcomePlan {
                        item: item.compared(changes),
                        outcome: updated,
                        is_new: false,
                    });
                }
            }
        }
        Ok(plans)
    }
}

#[async_trait]
impl ConfigSection<MySqlTx> for CallOutcomesSection {
    fn key(&self) -> &'static str {
        CALL_OUTCOMES_SECTION
    }

    async fn export(&self, _scope: &AccessScope) -> Result<Option<Value>, AppError> {
        let mut outcomes = self.outcomes.list(false).await?;
        outcomes.sort_by_key(|outcome| (outcome.position, outcome.id));
        let outcomes: Vec<PackageCallOutcome> = outcomes
            .into_iter()
            .map(|outcome| PackageCallOutcome {
                key: outcome.key,
                name: outcome.name,
                connected: outcome.is_connected,
                system: outcome.is_system,
            })
            .collect();
        Ok(Some(json!(outcomes)))
    }

    async fn validate(
        &self,
        _scope: &AccessScope,
        _package: &ConfigPackage,
        content: &Value,
    ) -> Result<Vec<PackageIssue>, AppError> {
        let outcomes: Vec<PackageCallOutcome> = match parse_section(content) {
            Ok(outcomes) => outcomes,
            Err(issue) => return Ok(vec![issue]),
        };
        let mut issues = Vec::new();
        if outcomes.len() > MAX_PACKAGE_CALL_OUTCOMES {
            issues.push(PackageIssue::new(
                "",
                "too_many_call_outcomes",
                format!("a package can have at most {MAX_PACKAGE_CALL_OUTCOMES} call outcomes"),
            ));
        }
        let mut keys = BTreeSet::new();
        for (index, outcome) in outcomes.iter().enumerate() {
            check_key(
                &outcome.key,
                &format!("{index}.key"),
                &mut keys,
                &mut issues,
            );
            let name = outcome.name.trim();
            if name.is_empty() || name.chars().count() > MAX_NAME_LENGTH {
                issues.push(PackageIssue::new(
                    format!("{index}.name"),
                    "invalid_name",
                    format!("must have 1 to {MAX_NAME_LENGTH} characters"),
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
                self.outcomes.insert(tx, &plan.outcome).await?;
            } else if decisions.executes(&plan.item) {
                self.outcomes.update(tx, &plan.outcome).await?;
            }
            context.resolve(
                ConfigReferenceKind::new("telephony.call_outcome"),
                &plan.outcome.key,
                plan.outcome.id,
            );
        }
        Ok(())
    }
}
