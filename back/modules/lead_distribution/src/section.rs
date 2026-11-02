use aphel_kernel::application::config_packages::ConfigReferenceKind;
use std::collections::{BTreeSet, HashMap};
use std::sync::Arc;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use uuid::Uuid;

use crate::domain::{
    DistributionRule, DistributionStrategy, MAX_MEMBERS, MAX_ROUTE_USERS, MAX_ROUTE_VALUE_LENGTH,
    MAX_ROUTE_VALUES, MAX_ROUTES, MAX_WEIGHT, MIN_WEIGHT, Route, Routing, RoutingFallback,
    RuleMember, RuleStats,
};
use crate::service::{MAX_DESCRIPTION_LENGTH, MAX_NAME_LENGTH};
use crate::store::{DirectoryTeam, DirectoryUser, DistributionStore};
use aphel_kernel::application::config_packages::sections::common::{check_key, diff};
use aphel_kernel::application::config_packages::{
    ApplyContext, ConfigPackage, ConfigSection, Decisions, PackageIssue, PlanAction, PlanChange,
    PlanItem, parse_section,
};
use aphel_kernel::application::fields::RecordTypeRegistry;
use aphel_kernel::database::transaction::MySqlTx;
use aphel_kernel::domain::access::AccessScope;
use aphel_kernel::domain::custom_objects::is_custom_record_type;
use aphel_kernel::domain::{TeamId, UserId};
use aphel_kernel::errors::AppError;

pub(crate) const LEAD_DISTRIBUTION_SECTION: &str = "lead_distribution_rules";
const RULE_KIND: &str = "lead_distribution_rule";
pub(crate) const MAX_PACKAGE_RULES: usize = 100;
pub(crate) const MEMBERS_MISSING: &str = "lead_distribution_members_missing";
pub(crate) const RECORD_TYPE_UNAVAILABLE: &str = "lead_distribution_record_type_unavailable";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct PackageMember {
    pub(crate) email: String,
    #[serde(default = "default_weight")]
    pub(crate) weight: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct PackageRoute {
    pub(crate) values: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) team: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub(crate) users: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct PackageRouting {
    pub(crate) field: String,
    pub(crate) routes: Vec<PackageRoute>,
    #[serde(default = "default_fallback")]
    pub(crate) fallback: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct PackageRule {
    pub(crate) key: String,
    pub(crate) name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) description: Option<String>,
    pub(crate) record_type: String,
    pub(crate) strategy: String,
    #[serde(default = "default_active")]
    pub(crate) is_active: bool,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub(crate) members: Vec<PackageMember>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) routing: Option<PackageRouting>,
}

fn default_weight() -> u32 {
    MIN_WEIGHT
}

fn default_fallback() -> String {
    RoutingFallback::Members.as_str().to_owned()
}

fn default_active() -> bool {
    true
}

fn is_distributable(record_type: &str) -> bool {
    matches!(
        record_type,
        "contacts.contact" | "companies.company" | "deals.deal"
    ) || is_custom_record_type(record_type)
}

struct Directory {
    users_by_email: HashMap<String, UserId>,
    emails_by_user: HashMap<UserId, String>,
    teams_by_name: HashMap<String, TeamId>,
    names_by_team: HashMap<TeamId, String>,
}

impl Directory {
    fn new(users: Vec<DirectoryUser>, teams: Vec<DirectoryTeam>) -> Self {
        let mut directory = Self {
            users_by_email: HashMap::new(),
            emails_by_user: HashMap::new(),
            teams_by_name: HashMap::new(),
            names_by_team: HashMap::new(),
        };
        for user in users {
            directory
                .users_by_email
                .insert(user.email.trim().to_lowercase(), user.user_id.clone());
            directory.emails_by_user.insert(user.user_id, user.email);
        }
        for team in teams {
            directory
                .teams_by_name
                .entry(team.name.trim().to_lowercase())
                .or_insert_with(|| team.team_id.clone());
            directory.names_by_team.insert(team.team_id, team.name);
        }
        directory
    }

    fn user(&self, email: &str, missing: &mut Vec<String>) -> Option<UserId> {
        let found = self
            .users_by_email
            .get(&email.trim().to_lowercase())
            .cloned();
        if found.is_none() && !missing.contains(&email.to_owned()) {
            missing.push(email.to_owned());
        }
        found
    }

    fn team(&self, name: &str, missing: &mut Vec<String>) -> Option<TeamId> {
        let found = self.teams_by_name.get(&name.trim().to_lowercase()).cloned();
        let label = format!("team {name}");
        if found.is_none() && !missing.contains(&label) {
            missing.push(label);
        }
        found
    }
}

struct RulePlan {
    item: PlanItem,
    rule: DistributionRule,
    is_new: bool,
}

pub(crate) struct LeadDistributionSection {
    registry: Arc<RecordTypeRegistry>,
    store: DistributionStore,
}

impl LeadDistributionSection {
    pub(crate) fn new(registry: Arc<RecordTypeRegistry>, store: DistributionStore) -> Self {
        Self { registry, store }
    }

    async fn directory(&self) -> Result<Directory, AppError> {
        Ok(Directory::new(
            self.store.users_directory().await?,
            self.store.teams_directory().await?,
        ))
    }

    fn record_type_problem(&self, record_type: &str) -> Option<String> {
        match self.registry.require(record_type) {
            Ok(definition) if definition.owner_column.is_none() => {
                Some(format!("the record type {record_type} has no owner"))
            }
            Ok(_) => None,
            Err(_) if is_custom_record_type(record_type) => None,
            Err(_) => Some(format!("the record type {record_type} is not available")),
        }
    }

    async fn reconcile(
        &self,
        scope: &AccessScope,
        package: &[PackageRule],
        at: DateTime<Utc>,
    ) -> Result<Vec<RulePlan>, AppError> {
        let directory = self.directory().await?;
        let existing = self.store.all().await?;
        let mut plans = Vec::new();
        for entry in package {
            let current = existing.iter().find(|rule| rule.key == entry.key);
            let mut missing = Vec::new();
            let mut rule = build_rule(entry, current, &directory, &mut missing, scope, at);
            let needs_members = rule
                .routing
                .as_ref()
                .is_none_or(|routing| routing.fallback == RoutingFallback::Members);
            let disabled = needs_members && rule.members.is_empty() && rule.is_active;
            if disabled {
                rule.is_active = false;
            }
            let item = PlanItem::new(
                LEAD_DISTRIBUTION_SECTION,
                RULE_KIND,
                entry.key.clone(),
                rule.name.clone(),
            );
            let proposed = snapshot(&rule, &directory);
            let changes = match current {
                Some(current) => diff(&snapshot(current, &directory), &proposed),
                None => proposed
                    .into_iter()
                    .map(|(property, value)| PlanChange::new(property, Value::Null, value))
                    .collect(),
            };
            let item = if let Some(problem) = self.record_type_problem(&rule.record_type) {
                item.conflicting(changes, RECORD_TYPE_UNAVAILABLE, problem, false)
            } else if !missing.is_empty() || disabled {
                let mut message = if missing.is_empty() {
                    "the rule has no members in this organization".to_owned()
                } else {
                    format!(
                        "not found in this organization and left out of the rule: {}",
                        missing.join(", ")
                    )
                };
                if disabled {
                    message.push_str("; the rule is installed inactive until members are added");
                }
                item.conflicting(changes, MEMBERS_MISSING, message, true)
            } else if current.is_some() {
                item.compared(changes)
            } else {
                item.created(changes)
            };
            plans.push(RulePlan {
                item,
                rule,
                is_new: current.is_none(),
            });
        }
        Ok(plans)
    }
}

fn build_rule(
    entry: &PackageRule,
    current: Option<&DistributionRule>,
    directory: &Directory,
    missing: &mut Vec<String>,
    scope: &AccessScope,
    at: DateTime<Utc>,
) -> DistributionRule {
    let previous: HashMap<&UserId, &RuleMember> = current
        .map(|rule| {
            rule.members
                .iter()
                .map(|member| (&member.user_id, member))
                .collect()
        })
        .unwrap_or_default();
    let mut members: Vec<RuleMember> = Vec::new();
    for member in &entry.members {
        let Some(user_id) = directory.user(&member.email, missing) else {
            continue;
        };
        if members.iter().any(|existing| existing.user_id == user_id) {
            continue;
        }
        let mut rule_member = previous
            .get(&user_id)
            .map(|existing| (*existing).clone())
            .unwrap_or_else(|| RuleMember::new(user_id));
        rule_member.weight = member.weight.clamp(MIN_WEIGHT, MAX_WEIGHT);
        members.push(rule_member);
    }
    let routing = entry.routing.as_ref().map(|routing| Routing {
        field: routing.field.trim().to_owned(),
        fallback: RoutingFallback::parse(routing.fallback.trim())
            .unwrap_or(RoutingFallback::Members),
        routes: routing
            .routes
            .iter()
            .filter_map(|route| {
                let team_id = route
                    .team
                    .as_deref()
                    .and_then(|name| directory.team(name, missing));
                let mut user_ids: Vec<UserId> = Vec::new();
                for email in &route.users {
                    if let Some(user_id) = directory.user(email, missing)
                        && !user_ids.contains(&user_id)
                    {
                        user_ids.push(user_id);
                    }
                }
                (team_id.is_some() || !user_ids.is_empty()).then(|| Route {
                    values: route
                        .values
                        .iter()
                        .map(|value| value.trim().to_owned())
                        .filter(|value| !value.is_empty())
                        .collect(),
                    team_id,
                    user_ids,
                })
            })
            .collect(),
    });
    let base = current.cloned().unwrap_or_else(|| DistributionRule {
        id: Uuid::now_v7(),
        key: entry.key.clone(),
        name: String::new(),
        description: None,
        record_type: String::new(),
        strategy: DistributionStrategy::RoundRobin,
        is_active: true,
        members: Vec::new(),
        routing: None,
        created_by: Some(scope.user_id().clone()),
        created_at: at,
        updated_at: at,
        stats: RuleStats::default(),
    });
    DistributionRule {
        name: entry.name.trim().to_owned(),
        description: entry
            .description
            .as_deref()
            .map(str::trim)
            .filter(|text| !text.is_empty())
            .map(str::to_owned),
        record_type: entry.record_type.trim().to_owned(),
        strategy: DistributionStrategy::parse(entry.strategy.trim()).unwrap_or(base.strategy),
        is_active: entry.is_active,
        members,
        routing,
        updated_at: at,
        ..base
    }
}

fn snapshot(rule: &DistributionRule, directory: &Directory) -> Vec<(String, Value)> {
    let email = |user_id: &UserId| {
        directory
            .emails_by_user
            .get(user_id)
            .cloned()
            .unwrap_or_else(|| user_id.as_str().to_owned())
    };
    vec![
        ("name".to_owned(), json!(rule.name)),
        ("description".to_owned(), json!(rule.description)),
        ("record_type".to_owned(), json!(rule.record_type)),
        ("strategy".to_owned(), json!(rule.strategy.as_str())),
        ("is_active".to_owned(), json!(rule.is_active)),
        (
            "members".to_owned(),
            json!(
                rule.members
                    .iter()
                    .map(
                        |member| json!({ "email": email(&member.user_id), "weight": member.weight })
                    )
                    .collect::<Vec<_>>()
            ),
        ),
        (
            "routing".to_owned(),
            json!(rule.routing.as_ref().map(|routing| json!({
                "field": routing.field,
                "fallback": routing.fallback.as_str(),
                "routes": routing.routes.iter().map(|route| json!({
                    "values": route.values,
                    "team": route.team_id.as_ref().map(|team_id| directory
                        .names_by_team
                        .get(team_id)
                        .cloned()
                        .unwrap_or_else(|| team_id.as_str().to_owned())),
                    "users": route.user_ids.iter().map(email).collect::<Vec<_>>(),
                })).collect::<Vec<_>>(),
            }))),
        ),
    ]
}

fn export_rule(rule: DistributionRule, directory: &Directory) -> PackageRule {
    let email = |user_id: &UserId| directory.emails_by_user.get(user_id).cloned();
    PackageRule {
        key: rule.key,
        name: rule.name,
        description: rule.description,
        record_type: rule.record_type,
        strategy: rule.strategy.as_str().to_owned(),
        is_active: rule.is_active,
        members: rule
            .members
            .iter()
            .filter_map(|member| {
                email(&member.user_id).map(|email| PackageMember {
                    email,
                    weight: member.weight,
                })
            })
            .collect(),
        routing: rule.routing.map(|routing| PackageRouting {
            field: routing.field,
            fallback: routing.fallback.as_str().to_owned(),
            routes: routing
                .routes
                .iter()
                .map(|route| PackageRoute {
                    values: route.values.clone(),
                    team: route
                        .team_id
                        .as_ref()
                        .and_then(|team_id| directory.names_by_team.get(team_id).cloned()),
                    users: route.user_ids.iter().filter_map(email).collect(),
                })
                .collect(),
        }),
    }
}

fn parse(content: &Value) -> Result<Vec<PackageRule>, AppError> {
    parse_section(content).map_err(|issue| AppError::Validation(issue.message))
}

fn validate_rule(index: usize, rule: &PackageRule, issues: &mut Vec<PackageIssue>) {
    let path = |property: &str| format!("{index}.{property}");
    let name = rule.name.trim();
    if name.is_empty() || name.chars().count() > MAX_NAME_LENGTH {
        issues.push(PackageIssue::new(
            path("name"),
            "invalid_name",
            format!("must have between 1 and {MAX_NAME_LENGTH} characters"),
        ));
    }
    if rule
        .description
        .as_ref()
        .is_some_and(|text| text.chars().count() > MAX_DESCRIPTION_LENGTH)
    {
        issues.push(PackageIssue::new(
            path("description"),
            "invalid_description",
            format!("must have at most {MAX_DESCRIPTION_LENGTH} characters"),
        ));
    }
    if !is_distributable(rule.record_type.trim()) {
        issues.push(PackageIssue::new(
            path("record_type"),
            "invalid_record_type",
            "must be contact, company, deal or a custom object (custom.<key>)",
        ));
    }
    if DistributionStrategy::parse(rule.strategy.trim()).is_none() {
        issues.push(PackageIssue::new(
            path("strategy"),
            "invalid_strategy",
            "must be round_robin, weighted or least_loaded",
        ));
    }
    if rule.members.len() > MAX_MEMBERS {
        issues.push(PackageIssue::new(
            path("members"),
            "too_many_members",
            format!("must have at most {MAX_MEMBERS} members"),
        ));
    }
    let mut emails = BTreeSet::new();
    for (member_index, member) in rule.members.iter().enumerate() {
        if member.email.trim().is_empty() || !member.email.contains('@') {
            issues.push(PackageIssue::new(
                path(&format!("members.{member_index}.email")),
                "invalid_email",
                "must be the e-mail of a user",
            ));
        } else if !emails.insert(member.email.trim().to_lowercase()) {
            issues.push(PackageIssue::new(
                path(&format!("members.{member_index}.email")),
                "duplicated_member",
                "is declared more than once",
            ));
        }
        if !(MIN_WEIGHT..=MAX_WEIGHT).contains(&member.weight) {
            issues.push(PackageIssue::new(
                path(&format!("members.{member_index}.weight")),
                "invalid_weight",
                format!("must be between {MIN_WEIGHT} and {MAX_WEIGHT}"),
            ));
        }
    }
    let Some(routing) = &rule.routing else {
        if rule.members.is_empty() {
            issues.push(PackageIssue::new(
                path("members"),
                "members_required",
                "must have at least one member (or routes with fallback none)",
            ));
        }
        return;
    };
    if routing.field.trim().is_empty() {
        issues.push(PackageIssue::new(
            path("routing.field"),
            "invalid_field",
            "is required",
        ));
    }
    match RoutingFallback::parse(routing.fallback.trim()) {
        None => issues.push(PackageIssue::new(
            path("routing.fallback"),
            "invalid_fallback",
            "must be members or none",
        )),
        Some(RoutingFallback::Members) if rule.members.is_empty() => {
            issues.push(PackageIssue::new(
                path("members"),
                "members_required",
                "must have at least one member (or routes with fallback none)",
            ))
        }
        Some(_) => {}
    }
    if routing.routes.len() > MAX_ROUTES {
        issues.push(PackageIssue::new(
            path("routing.routes"),
            "too_many_routes",
            format!("must have at most {MAX_ROUTES} routes"),
        ));
    }
    for (route_index, route) in routing.routes.iter().enumerate() {
        let route_path = |property: &str| path(&format!("routing.routes.{route_index}.{property}"));
        if route.values.is_empty()
            || route.values.len() > MAX_ROUTE_VALUES
            || route.values.iter().any(|value| {
                value.trim().is_empty() || value.chars().count() > MAX_ROUTE_VALUE_LENGTH
            })
        {
            issues.push(PackageIssue::new(
                route_path("values"),
                "invalid_values",
                format!(
                    "must have between 1 and {MAX_ROUTE_VALUES} non-empty values of at most {MAX_ROUTE_VALUE_LENGTH} characters"
                ),
            ));
        }
        if route.team.is_none() && route.users.is_empty() {
            issues.push(PackageIssue::new(
                route_path("team"),
                "route_target_required",
                "a route needs a team or users",
            ));
        }
        if route.users.len() > MAX_ROUTE_USERS {
            issues.push(PackageIssue::new(
                route_path("users"),
                "too_many_users",
                format!("must have at most {MAX_ROUTE_USERS} users"),
            ));
        }
    }
}

#[async_trait]
impl ConfigSection<MySqlTx> for LeadDistributionSection {
    fn key(&self) -> &'static str {
        LEAD_DISTRIBUTION_SECTION
    }

    async fn export(&self, _scope: &AccessScope) -> Result<Option<Value>, AppError> {
        let rules = self.store.all().await?;
        if rules.is_empty() {
            return Ok(None);
        }
        let directory = self.directory().await?;
        let rules: Vec<PackageRule> = rules
            .into_iter()
            .map(|rule| export_rule(rule, &directory))
            .collect();
        Ok(Some(json!(rules)))
    }

    async fn validate(
        &self,
        _scope: &AccessScope,
        _package: &ConfigPackage,
        content: &Value,
    ) -> Result<Vec<PackageIssue>, AppError> {
        let rules: Vec<PackageRule> = match parse_section(content) {
            Ok(rules) => rules,
            Err(issue) => return Ok(vec![issue]),
        };
        let mut issues = Vec::new();
        if rules.len() > MAX_PACKAGE_RULES {
            issues.push(PackageIssue::new(
                "",
                "too_many_rules",
                format!("a package can have at most {MAX_PACKAGE_RULES} lead distribution rules"),
            ));
        }
        let mut keys = BTreeSet::new();
        for (index, rule) in rules.iter().enumerate() {
            check_key(&rule.key, &format!("{index}.key"), &mut keys, &mut issues);
            validate_rule(index, rule, &mut issues);
        }
        Ok(issues)
    }

    async fn plan(
        &self,
        scope: &AccessScope,
        _package: &ConfigPackage,
        content: &Value,
    ) -> Result<Vec<PlanItem>, AppError> {
        Ok(self
            .reconcile(scope, &parse(content)?, DateTime::<Utc>::UNIX_EPOCH)
            .await?
            .into_iter()
            .map(|plan| plan.item)
            .collect())
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
        for plan in self.reconcile(scope, &parse(content)?, context.at).await? {
            if !decisions.executes(&plan.item) {
                if plan.item.action == PlanAction::Unchanged {
                    context.resolve(
                        ConfigReferenceKind::new("assignment.rule"),
                        &plan.rule.key,
                        plan.rule.id,
                    );
                }
                continue;
            }
            if plan.is_new {
                self.store.insert(tx, &plan.rule).await?;
            } else {
                self.store.update(tx, &plan.rule).await?;
            }
            context.resolve(
                ConfigReferenceKind::new("assignment.rule"),
                &plan.rule.key,
                plan.rule.id,
            );
        }
        Ok(())
    }
}
