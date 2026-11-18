use aphel_contract_crm::{CrmPermissions, CrmRecordTypes};
use std::collections::{BTreeMap, BTreeSet};

use chrono::{Datelike, Days, Months, NaiveDate};
use rust_decimal::Decimal;
use serde::Serialize;
use serde_json::{Value, json};
use uuid::Uuid;

use crate::domain::{DealRecord, deal_not_found};
use crate::team::domain::{
    AddDealMemberCommand, CreateDealMemberRoleCommand, CreditTotals, CreditedUsers,
    DEAL_MEMBER_ADDED, DEAL_MEMBER_EXISTS, DEAL_MEMBER_REMOVED, DEAL_MEMBER_ROLE_CREATED,
    DEAL_MEMBER_ROLE_DELETED, DEAL_MEMBER_ROLE_KEY_TAKEN, DEAL_MEMBER_ROLE_RECORD_TYPE,
    DEAL_MEMBER_ROLE_UPDATED, DEAL_MEMBER_ROLES_REORDERED, DEAL_MEMBER_UPDATED,
    DEAL_MEMBERS_LIMIT_REACHED, DealMember, DealMemberRole, DealMemberRoleRef, DealTeam,
    FULL_CREDIT, MAX_DEAL_MEMBER_ROLES, MAX_DEAL_MEMBERS, NewDealMember, UpdateDealMemberCommand,
    UpdateDealMemberRoleCommand, allocated, credit_exceeded, deal_member_role_not_found,
    member_not_found, parse_credit_percent, parse_id, percent_text, role_name,
};
use crate::team::store::{DealCreditReader, DealMemberRoleStore, DealMemberStore};
use aphel_kernel::Ctx;
use aphel_kernel::domain::UserId;
use aphel_kernel::errors::{AppError, FieldViolation};
use aphel_kernel::records::{RecordMutation, publish_mutation};
use aphel_sdk::configuration::{is_config_key, key_from_name};
use aphel_sdk::platform::application::access::Authorizer;
use aphel_sdk::platform::application::audit::{DiffOptions, diff_changes};
use aphel_sdk::platform::application::fields::FieldSet;
use aphel_sdk::platform::application::fields::permissions::{
    NOT_ACCESSIBLE_MESSAGE, not_accessible,
};
use aphel_sdk::platform::application::files::UserReference;
use aphel_sdk::platform::application::organization_admin::ListWindow;
use aphel_sdk::platform::application::query::{
    ArchivedFilter, Filter, FilterOperator, QueryPlan, RecordQuery, RecordQueryExecutor,
    RecordQueryPorts, ResolveContext, resolve_query,
};
use aphel_sdk::platform::application::shared::{
    Actor, AuditEntry, AuditLog, DomainEvent, EventPublisher, FieldChange, RecordRef, RecordType,
    ensure_version,
};
use aphel_sdk::platform::database::transaction::Tx;
use aphel_sdk::platform::domain::access::Permission;

const INVALID_DEAL_MEMBER: &str = "invalid deal member";
const STATUS: &str = "status";
const CLOSED_AT: &str = "closed_at";
const DAY_FORMAT: &str = "%Y-%m-%d";

#[derive(Debug, Serialize)]
struct RoleSnapshot<'a> {
    key: &'a str,
    name: &'a str,
    is_active: bool,
}

impl<'a> From<&'a DealMemberRole> for RoleSnapshot<'a> {
    fn from(role: &'a DealMemberRole) -> Self {
        Self {
            key: &role.key,
            name: &role.name,
            is_active: role.is_active,
        }
    }
}

fn invalid_role(violations: Vec<FieldViolation>) -> AppError {
    AppError::invalid_fields("invalid deal member role", violations)
}

fn member_payload(member: &DealMember) -> Value {
    json!({
        "member_id": member.id.to_string(),
        "user_id": member.user.id.as_str(),
        "user_name": member.user.display_name,
        "role_id": member.role.as_ref().map(|role| role.id.to_string()),
        "role_key": member.role.as_ref().map(|role| role.key.clone()),
        "role_name": member.role.as_ref().map(|role| role.name.clone()),
        "credit_percent": percent_text(member.credit_percent),
    })
}

fn role_value(role: Option<&DealMemberRoleRef>) -> Value {
    role.map_or(Value::Null, |role| Value::String(role.key.clone()))
}

fn member_changes(old: Option<&DealMember>, new: Option<&DealMember>) -> Vec<FieldChange> {
    let user = |member: Option<&DealMember>| {
        member.map_or(Value::Null, |member| {
            Value::String(member.user.id.as_str().to_owned())
        })
    };
    let role = |member: Option<&DealMember>| role_value(member.and_then(|m| m.role.as_ref()));
    let credit = |member: Option<&DealMember>| {
        member.map_or(Value::Null, |member| {
            Value::String(percent_text(member.credit_percent))
        })
    };
    [
        ("member.user_id", user(old), user(new)),
        ("member.role", role(old), role(new)),
        ("member.credit_percent", credit(old), credit(new)),
    ]
    .into_iter()
    .filter(|(_, old, new)| old != new)
    .map(|(field, old, new)| FieldChange::new(field, old, new))
    .collect()
}

fn credit(value: Option<&str>, violations: &mut Vec<FieldViolation>) -> Option<Decimal> {
    let text = value.map(str::trim).filter(|text| !text.is_empty())?;
    let parsed = parse_credit_percent(text);
    if parsed.is_none() {
        violations.push(FieldViolation::new(
            "credit_percent",
            "must be a number between 0 and 100 with at most 2 decimal places",
        ));
    }
    parsed
}

async fn deal_for(
    ctx: &Ctx<'_>,
    deal_id: &str,
    permission: &Permission,
) -> Result<DealRecord, AppError> {
    let visibility = ctx.scope.record_filter(permission)?;
    crate::service::require(ctx, &visibility, deal_id).await
}

async fn resolve_role(
    ctx: &Ctx<'_>,
    role_id: Option<&str>,
    violations: &mut Vec<FieldViolation>,
) -> Result<Option<DealMemberRole>, AppError> {
    let Some(role_id) = role_id.map(str::trim).filter(|id| !id.is_empty()) else {
        return Ok(None);
    };
    let role = match parse_id(role_id) {
        Some(id) => DealMemberRoleStore::of(ctx).find(id).await?,
        None => None,
    };
    match role {
        Some(role) if role.is_active => Ok(Some(role)),
        _ => {
            violations.push(FieldViolation::new(
                "role_id",
                "must be an active deal member role",
            ));
            Ok(None)
        }
    }
}

async fn lock(
    members: &DealMemberStore,
    tx: &mut Tx,
    deal_id: Uuid,
) -> Result<Vec<DealMember>, AppError> {
    if !members.lock_deal(tx, deal_id).await? {
        return Err(deal_not_found());
    }
    members.list_locked(tx, deal_id).await
}

async fn record_member(
    ctx: &Ctx<'_>,
    tx: &mut Tx,
    member: &DealMember,
    event_type: &'static str,
    changes: Vec<FieldChange>,
    ip: Option<String>,
) -> Result<(), AppError> {
    let mut payload = member_payload(member);
    payload["changes"] = json!(
        changes
            .iter()
            .map(|change| json!({
                "field": change.field,
                "old": change.old,
                "new": change.new,
            }))
            .collect::<Vec<_>>()
    );
    publish_mutation(
        ctx.events(),
        ctx.audit(),
        tx,
        ctx.scope,
        ctx.now(),
        RecordMutation {
            record: RecordRef::new(RecordType::DEAL, member.deal_id),
            event_type,
            payload,
            changes,
            ip,
        },
    )
    .await
}

pub(crate) async fn team(ctx: &Ctx<'_>, deal_id: &str) -> Result<DealTeam, AppError> {
    let deal = deal_for(ctx, deal_id, &Permission::DEALS_READ).await?;
    let members = DealMemberStore::of(ctx).list(deal.deal.id).await?;
    Ok(DealTeam::new(deal.deal.id, deal.owner, members))
}

pub async fn add_member(
    ctx: &Ctx<'_>,
    deal_id: &str,
    command: AddDealMemberCommand,
) -> Result<DealMember, AppError> {
    let scope = ctx.scope;
    let deal = deal_for(ctx, deal_id, &Permission::DEALS_UPDATE).await?;
    let store = DealMemberStore::of(ctx);
    let mut violations = Vec::new();
    let user_id = parse_id(&command.user_id).and_then(|id| UserId::new(id.to_string()).ok());
    if user_id.is_none() {
        violations.push(FieldViolation::new("user_id", "must be a valid user id"));
    }
    let user_name = match &user_id {
        Some(user_id) => {
            let name = store.active_user_name(user_id).await?;
            if name.is_none() {
                violations.push(FieldViolation::new(
                    "user_id",
                    "must be an active member of the organization",
                ));
            }
            name
        }
        None => None,
    };
    let role = resolve_role(ctx, command.role_id.as_deref(), &mut violations).await?;
    let credit_percent = credit(command.credit_percent.as_deref(), &mut violations);
    let (Some(user_id), Some(user_name), true) = (user_id, user_name, violations.is_empty()) else {
        return Err(AppError::invalid_fields(INVALID_DEAL_MEMBER, violations));
    };
    let credit_percent = credit_percent.unwrap_or(Decimal::ZERO);
    let now = ctx.now();
    let mut tx = ctx.begin().await?;
    let members = lock(&store, &mut tx, deal.deal.id).await?;
    if members.iter().any(|member| member.user.id == user_id) {
        return Err(AppError::conflict_code(
            DEAL_MEMBER_EXISTS,
            "the user is already a member of this deal",
        ));
    }
    if members.len() >= MAX_DEAL_MEMBERS {
        return Err(AppError::validation_code(
            DEAL_MEMBERS_LIMIT_REACHED,
            format!("a deal can have at most {MAX_DEAL_MEMBERS} members"),
        ));
    }
    let others = allocated(&members);
    if others + credit_percent > FULL_CREDIT {
        return Err(credit_exceeded(others));
    }
    let member = DealMember {
        id: Uuid::now_v7(),
        deal_id: deal.deal.id,
        user: UserReference {
            id: user_id.clone(),
            display_name: user_name,
        },
        user_active: true,
        role: role.as_ref().map(DealMemberRoleRef::from),
        credit_percent,
        added_by: Some(scope.user_id().clone()),
        created_at: now,
        updated_at: now,
    };
    store
        .insert(
            &mut tx,
            &NewDealMember {
                id: member.id,
                deal_id: member.deal_id,
                user_id,
                role_id: role.as_ref().map(|role| role.id),
                credit_percent,
                added_by: member.added_by.clone(),
                at: now,
            },
        )
        .await?;
    record_member(
        ctx,
        &mut tx,
        &member,
        DEAL_MEMBER_ADDED,
        member_changes(None, Some(&member)),
        command.ip,
    )
    .await?;
    tx.commit().await?;
    Ok(member)
}

pub(crate) async fn update_member(
    ctx: &Ctx<'_>,
    deal_id: &str,
    member_id: &str,
    command: UpdateDealMemberCommand,
) -> Result<DealMember, AppError> {
    let deal = deal_for(ctx, deal_id, &Permission::DEALS_UPDATE).await?;
    let member_id = parse_id(member_id).ok_or_else(member_not_found)?;
    let store = DealMemberStore::of(ctx);
    let mut violations = Vec::new();
    let role = match &command.role_id {
        Some(role_id) => Some(resolve_role(ctx, role_id.as_deref(), &mut violations).await?),
        None => None,
    };
    let credit_percent = credit(command.credit_percent.as_deref(), &mut violations);
    if !violations.is_empty() {
        return Err(AppError::invalid_fields(INVALID_DEAL_MEMBER, violations));
    }
    let now = ctx.now();
    let mut tx = ctx.begin().await?;
    let members = lock(&store, &mut tx, deal.deal.id).await?;
    let current = members
        .iter()
        .find(|member| member.id == member_id)
        .cloned()
        .ok_or_else(member_not_found)?;
    ensure_version(command.expected_updated_at, current.updated_at)?;
    let mut next = current.clone();
    if let Some(role) = role {
        next.role = role.as_ref().map(DealMemberRoleRef::from);
    }
    if let Some(credit_percent) = credit_percent {
        next.credit_percent = credit_percent;
    }
    let changes = member_changes(Some(&current), Some(&next));
    if changes.is_empty() {
        return Ok(current);
    }
    let others: Decimal = members
        .iter()
        .filter(|member| member.id != member_id)
        .map(|member| member.credit_percent)
        .sum();
    if next.credit_percent > current.credit_percent && others + next.credit_percent > FULL_CREDIT {
        return Err(credit_exceeded(others));
    }
    next.updated_at = now;
    store
        .update(
            &mut tx,
            next.id,
            next.role.as_ref().map(|role| role.id),
            next.credit_percent,
            now,
        )
        .await?;
    record_member(
        ctx,
        &mut tx,
        &next,
        DEAL_MEMBER_UPDATED,
        changes,
        command.ip,
    )
    .await?;
    tx.commit().await?;
    Ok(next)
}

pub(crate) async fn remove_member(
    ctx: &Ctx<'_>,
    deal_id: &str,
    member_id: &str,
    ip: Option<String>,
) -> Result<(), AppError> {
    let deal = deal_for(ctx, deal_id, &Permission::DEALS_UPDATE).await?;
    let member_id = parse_id(member_id).ok_or_else(member_not_found)?;
    let store = DealMemberStore::of(ctx);
    let mut tx = ctx.begin().await?;
    let members = lock(&store, &mut tx, deal.deal.id).await?;
    let current = members
        .iter()
        .find(|member| member.id == member_id)
        .cloned()
        .ok_or_else(member_not_found)?;
    store.delete(&mut tx, deal.deal.id, current.id).await?;
    record_member(
        ctx,
        &mut tx,
        &current,
        DEAL_MEMBER_REMOVED,
        member_changes(Some(&current), None),
        ip,
    )
    .await?;
    tx.commit().await?;
    Ok(())
}

pub async fn list_roles(
    ctx: &Ctx<'_>,
    include_inactive: bool,
) -> Result<Vec<DealMemberRole>, AppError> {
    ctx.scope.require(&Permission::DEALS_READ)?;
    DealMemberRoleStore::of(ctx).list(include_inactive).await
}

pub(crate) async fn create_role(
    ctx: &Ctx<'_>,
    command: CreateDealMemberRoleCommand,
) -> Result<DealMemberRole, AppError> {
    ctx.scope.require(&Permission::SETTINGS_MANAGE)?;
    let store = DealMemberRoleStore::of(ctx);
    let mut violations = Vec::new();
    let name = role_name(&command.name, &mut violations);
    let explicit_key = command
        .key
        .as_deref()
        .map(str::trim)
        .filter(|key| !key.is_empty());
    if let Some(key) = explicit_key
        && !is_config_key(key)
    {
        violations.push(FieldViolation::new(
            "key",
            "must start with a lowercase letter and use only lowercase letters, digits and underscores",
        ));
    }
    let existing = store.list(true).await?;
    if existing.len() >= MAX_DEAL_MEMBER_ROLES {
        violations.push(FieldViolation::new(
            "name",
            format!("an organization can have at most {MAX_DEAL_MEMBER_ROLES} deal member roles"),
        ));
    }
    let (Some(name), true) = (name, violations.is_empty()) else {
        return Err(invalid_role(violations));
    };
    let taken = |key: &str| existing.iter().any(|role| role.key == key);
    let key = match explicit_key {
        Some(key) if taken(key) => {
            return Err(AppError::conflict_code(
                DEAL_MEMBER_ROLE_KEY_TAKEN,
                "another deal member role already uses this key",
            ));
        }
        Some(key) => key.to_owned(),
        None => {
            let base = key_from_name(&name);
            let mut candidate = base.clone();
            let mut suffix = 2;
            while taken(&candidate) {
                candidate = format!("{base}_{suffix}");
                suffix += 1;
            }
            candidate
        }
    };
    let now = ctx.now();
    let role = DealMemberRole {
        id: Uuid::now_v7(),
        key,
        name,
        is_active: true,
        position: existing
            .iter()
            .map(|role| role.position)
            .max()
            .map_or(0, |position| position + 1),
        created_at: now,
        updated_at: now,
    };
    let changes = diff_changes(
        &Value::Null,
        &RoleSnapshot::from(&role),
        &DiffOptions::default(),
    )?;
    let mut tx = ctx.begin().await?;
    store.insert(&mut tx, &role).await?;
    publish_role(
        ctx,
        &mut tx,
        &role,
        DEAL_MEMBER_ROLE_CREATED,
        changes,
        command.ip,
    )
    .await?;
    tx.commit().await?;
    Ok(role)
}

pub(crate) async fn update_role(
    ctx: &Ctx<'_>,
    id: &str,
    command: UpdateDealMemberRoleCommand,
) -> Result<DealMemberRole, AppError> {
    ctx.scope.require(&Permission::SETTINGS_MANAGE)?;
    let store = DealMemberRoleStore::of(ctx);
    let current = find_role(&store, id).await?;
    ensure_version(command.expected_updated_at, current.updated_at)?;
    let mut violations = Vec::new();
    let mut next = current.clone();
    if let Some(name) = &command.name
        && let Some(name) = role_name(name, &mut violations)
    {
        next.name = name;
    }
    if let Some(is_active) = command.is_active {
        next.is_active = is_active;
    }
    if !violations.is_empty() {
        return Err(invalid_role(violations));
    }
    let changes = diff_changes(
        &RoleSnapshot::from(&current),
        &RoleSnapshot::from(&next),
        &DiffOptions::default(),
    )?;
    if changes.is_empty() {
        return Ok(current);
    }
    next.updated_at = ctx.now();
    let mut tx = ctx.begin().await?;
    store.update(&mut tx, &next).await?;
    publish_role(
        ctx,
        &mut tx,
        &next,
        DEAL_MEMBER_ROLE_UPDATED,
        changes,
        command.ip,
    )
    .await?;
    tx.commit().await?;
    Ok(next)
}

pub(crate) async fn delete_role(
    ctx: &Ctx<'_>,
    id: &str,
    ip: Option<String>,
) -> Result<(), AppError> {
    ctx.scope.require(&Permission::SETTINGS_MANAGE)?;
    let store = DealMemberRoleStore::of(ctx);
    let current = find_role(&store, id).await?;
    let changes = diff_changes(
        &RoleSnapshot::from(&current),
        &Value::Null,
        &DiffOptions::default(),
    )?;
    let mut tx = ctx.begin().await?;
    store.delete(&mut tx, current.id).await?;
    publish_role(
        ctx,
        &mut tx,
        &current,
        DEAL_MEMBER_ROLE_DELETED,
        changes,
        ip,
    )
    .await?;
    tx.commit().await?;
    Ok(())
}

pub(crate) async fn reorder_roles(
    ctx: &Ctx<'_>,
    ids: &[String],
    ip: Option<String>,
) -> Result<Vec<DealMemberRole>, AppError> {
    ctx.scope.require(&Permission::SETTINGS_MANAGE)?;
    let store = DealMemberRoleStore::of(ctx);
    let existing = store.list(true).await?;
    let invalid_order = || {
        AppError::invalid_fields(
            "invalid order",
            vec![FieldViolation::new(
                "ids",
                "must list every deal member role of the organization exactly once",
            )],
        )
    };
    let ordered = ids
        .iter()
        .map(|id| Uuid::parse_str(id.trim()).map_err(|_| invalid_order()))
        .collect::<Result<Vec<_>, _>>()?;
    let requested: BTreeSet<Uuid> = ordered.iter().copied().collect();
    let known: BTreeSet<Uuid> = existing.iter().map(|role| role.id).collect();
    if requested.len() != ordered.len() || requested != known {
        return Err(invalid_order());
    }
    let previous: Vec<String> = existing.iter().map(|role| role.key.clone()).collect();
    let keys: Vec<String> = ordered
        .iter()
        .filter_map(|id| existing.iter().find(|role| role.id == *id))
        .map(|role| role.key.clone())
        .collect();
    if previous == keys {
        return Ok(existing);
    }
    let now = ctx.now();
    let actor = Actor::from_scope(ctx.scope);
    let mut tx = ctx.begin().await?;
    store.reorder(&mut tx, &ordered, now).await?;
    ctx.events()
        .publish(
            &mut tx,
            DomainEvent::new(actor.clone(), DEAL_MEMBER_ROLES_REORDERED, now)
                .with_payload(json!({ "keys": keys })),
        )
        .await?;
    ctx.audit()
        .record(
            &mut tx,
            AuditEntry::new(actor, DEAL_MEMBER_ROLES_REORDERED, now)
                .with_changes(vec![FieldChange::new(
                    "order",
                    json!(previous),
                    json!(keys),
                )])
                .with_ip(ip),
        )
        .await?;
    tx.commit().await?;
    store.list(true).await
}

async fn find_role(store: &DealMemberRoleStore, id: &str) -> Result<DealMemberRole, AppError> {
    let id = Uuid::parse_str(id.trim()).map_err(|_| deal_member_role_not_found())?;
    store.find(id).await?.ok_or_else(deal_member_role_not_found)
}

async fn publish_role(
    ctx: &Ctx<'_>,
    tx: &mut Tx,
    role: &DealMemberRole,
    event_type: &'static str,
    changes: Vec<FieldChange>,
    ip: Option<String>,
) -> Result<(), AppError> {
    let now = ctx.now();
    let actor = Actor::from_scope(ctx.scope);
    let record = RecordRef::new(DEAL_MEMBER_ROLE_RECORD_TYPE, role.id);
    ctx.events()
        .publish(
            tx,
            DomainEvent::new(actor.clone(), event_type, now)
                .with_record(record.clone())
                .with_payload(json!({
                    "key": role.key,
                    "name": role.name,
                    "is_active": role.is_active,
                })),
        )
        .await?;
    ctx.audit()
        .record(
            tx,
            AuditEntry::new(actor, event_type, now)
                .with_record(record)
                .with_changes(changes)
                .with_ip(ip),
        )
        .await
}

pub(crate) struct CreditPlan {
    pub(crate) plan: QueryPlan,
    pub(crate) amount_column: String,
}

pub(crate) async fn credit_plan(
    ctx: &Ctx<'_>,
    amount_key: &str,
    conditions: Vec<Filter>,
) -> Result<CreditPlan, AppError> {
    let scope = ctx.scope;
    let query = ctx.query();
    let definition = ctx.registry().require(RecordType::DEAL.as_str())?;
    let visibility = scope.record_filter(&definition.read_permission)?;
    let fields = FieldSet::for_scope(query.definitions(), definition, scope).await?;
    if fields.is_hidden(amount_key) {
        return Err(not_accessible(vec![FieldViolation::new(
            amount_key,
            NOT_ACCESSIBLE_MESSAGE,
        )]));
    }
    let amount_column = definition
        .system_field_by_key(amount_key)
        .map(|field| field.column.clone())
        .ok_or_else(|| AppError::Internal(format!("deal field {amount_key} is not a column")))?;
    let context = ResolveContext {
        now: ctx.now(),
        timezone: query.executor().timezone().await?,
        user_id: scope.user_id().clone(),
    };
    let plan = resolve_query(
        &fields,
        &context,
        visibility,
        RecordQuery {
            filter: (!conditions.is_empty()).then(|| Filter::and(conditions)),
            sort: Vec::new(),
            search: None,
            archived: ArchivedFilter::Exclude,
            window: ListWindow {
                offset: 0,
                limit: 1,
            },
        },
    )?;
    Ok(CreditPlan {
        plan,
        amount_column,
    })
}

pub(crate) fn won_between(from: NaiveDate, to: NaiveDate) -> Vec<Filter> {
    let day = |date: NaiveDate| Value::String(date.format(DAY_FORMAT).to_string());
    vec![
        Filter::condition(STATUS, FilterOperator::Eq, json!("won")),
        Filter::condition(
            CLOSED_AT,
            FilterOperator::Between,
            Value::Array(vec![day(from), day(to)]),
        ),
    ]
}

pub async fn credited_won_total(
    ctx: &Ctx<'_>,
    credited: &CreditedUsers,
    amount_key: &str,
    from: NaiveDate,
    to: NaiveDate,
) -> Result<CreditTotals, AppError> {
    let CreditPlan {
        plan,
        amount_column,
    } = credit_plan(ctx, amount_key, won_between(from, to)).await?;
    DealCreditReader::of(ctx)
        .credited_total(ctx.scope, &plan, &amount_column, credited)
        .await
}

pub use aphel_sdk::deals::credited_from_owner_filter;

pub async fn credited_won_months(
    ctx: &Ctx<'_>,
    credited: &CreditedUsers,
    amount_key: &str,
    from: NaiveDate,
    to: NaiveDate,
) -> Result<BTreeMap<NaiveDate, CreditTotals>, AppError> {
    let mut months = BTreeMap::new();
    let mut cursor = from.with_day(1).unwrap_or(from);
    while cursor <= to {
        let Some(next) = cursor.checked_add_months(Months::new(1)) else {
            break;
        };
        let end = next
            .checked_sub_days(Days::new(1))
            .unwrap_or(cursor)
            .min(to);
        let totals = credited_won_total(ctx, credited, amount_key, cursor, end).await?;
        months.insert(cursor, totals);
        cursor = next;
    }
    Ok(months)
}
