use aphel_sdk::configuration::ConfigReferenceKind;
use std::collections::BTreeSet;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sqlx::MySqlPool;
use uuid::Uuid;

use crate::team::domain::{
    DealMemberRole, MAX_DEAL_MEMBER_ROLE_NAME_LENGTH, MAX_DEAL_MEMBER_ROLES,
};
use crate::team::store::DealMemberRoleStore;
use aphel_kernel::errors::AppError;
use aphel_sdk::configuration::sections::common::{check_key, diff};
use aphel_sdk::configuration::{
    ApplyContext, ConfigPackage, ConfigSection, Decisions, PackageIssue, PlanChange, PlanItem,
    parse_section,
};
use aphel_sdk::platform::database::transaction::MySqlTx;
use aphel_sdk::platform::domain::access::AccessScope;

pub(crate) const DEAL_MEMBER_ROLES_SECTION: &str = "deal_member_roles";
pub(crate) const DEAL_MEMBER_ROLE_ITEM: &str = "deal_member_role";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct PackageDealMemberRole {
    pub(crate) key: String,
    pub(crate) name: String,
}

struct RolePlan {
    item: PlanItem,
    role: DealMemberRole,
    is_new: bool,
}

pub(crate) struct DealMemberRolesSection {
    roles: DealMemberRoleStore,
}

impl DealMemberRolesSection {
    pub(crate) fn new(pool: MySqlPool) -> Self {
        Self {
            roles: DealMemberRoleStore::new(pool),
        }
    }

    async fn reconcile(
        &self,
        package: &[PackageDealMemberRole],
        at: DateTime<Utc>,
    ) -> Result<Vec<RolePlan>, AppError> {
        let existing = self.roles.list(true).await?;
        let mut position = existing
            .iter()
            .map(|role| role.position)
            .max()
            .unwrap_or(-1);
        let mut plans = Vec::new();
        for entry in package {
            let name = entry.name.trim().to_owned();
            let item = PlanItem::new(
                DEAL_MEMBER_ROLES_SECTION,
                DEAL_MEMBER_ROLE_ITEM,
                entry.key.clone(),
                name.clone(),
            );
            match existing.iter().find(|current| current.key == entry.key) {
                None => {
                    position += 1;
                    let created = DealMemberRole {
                        id: Uuid::now_v7(),
                        key: entry.key.clone(),
                        name,
                        is_active: true,
                        position,
                        created_at: at,
                        updated_at: at,
                    };
                    let changes = snapshot(&created)
                        .into_iter()
                        .map(|(property, value)| PlanChange::new(property, Value::Null, value))
                        .collect();
                    plans.push(RolePlan {
                        item: item.created(changes),
                        role: created,
                        is_new: true,
                    });
                }
                Some(current) => {
                    let updated = DealMemberRole {
                        name,
                        is_active: true,
                        updated_at: at,
                        ..current.clone()
                    };
                    let changes = diff(&snapshot(current), &snapshot(&updated));
                    plans.push(RolePlan {
                        item: item.compared(changes),
                        role: updated,
                        is_new: false,
                    });
                }
            }
        }
        Ok(plans)
    }
}

fn snapshot(role: &DealMemberRole) -> Vec<(String, Value)> {
    vec![
        ("name".to_owned(), json!(role.name)),
        ("active".to_owned(), json!(role.is_active)),
    ]
}

fn parse(content: &Value) -> Result<Vec<PackageDealMemberRole>, AppError> {
    parse_section(content).map_err(|issue| AppError::Validation(issue.message))
}

#[async_trait]
impl ConfigSection<MySqlTx> for DealMemberRolesSection {
    fn key(&self) -> &'static str {
        DEAL_MEMBER_ROLES_SECTION
    }

    async fn export(&self, _scope: &AccessScope) -> Result<Option<Value>, AppError> {
        let roles: Vec<PackageDealMemberRole> = self
            .roles
            .list(false)
            .await?
            .into_iter()
            .map(|role| PackageDealMemberRole {
                key: role.key,
                name: role.name,
            })
            .collect();
        Ok(Some(json!(roles)))
    }

    async fn validate(
        &self,
        _scope: &AccessScope,
        _package: &ConfigPackage,
        content: &Value,
    ) -> Result<Vec<PackageIssue>, AppError> {
        let roles: Vec<PackageDealMemberRole> = match parse_section(content) {
            Ok(roles) => roles,
            Err(issue) => return Ok(vec![issue]),
        };
        let mut issues = Vec::new();
        if roles.len() > MAX_DEAL_MEMBER_ROLES {
            issues.push(PackageIssue::new(
                "",
                "too_many_deal_member_roles",
                format!("a package can have at most {MAX_DEAL_MEMBER_ROLES} deal member roles"),
            ));
        }
        let mut keys = BTreeSet::new();
        for (index, role) in roles.iter().enumerate() {
            check_key(&role.key, &format!("{index}.key"), &mut keys, &mut issues);
            let length = role.name.trim().chars().count();
            if length == 0 || length > MAX_DEAL_MEMBER_ROLE_NAME_LENGTH {
                issues.push(PackageIssue::new(
                    format!("{index}.name"),
                    "invalid_name",
                    format!(
                        "must have between 1 and {MAX_DEAL_MEMBER_ROLE_NAME_LENGTH} characters"
                    ),
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
                self.roles.insert(tx, &plan.role).await?;
            } else if decisions.executes(&plan.item) {
                self.roles.update(tx, &plan.role).await?;
            }
            context.resolve(
                ConfigReferenceKind::new("deals.member_role"),
                &plan.role.key,
                plan.role.id,
            );
        }
        Ok(())
    }
}
