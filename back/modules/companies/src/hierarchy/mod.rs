use aphel_contract_crm::{CrmPermissions, CrmRecordTypes};
use std::collections::HashSet;

use chrono::{DateTime, Utc};
use serde_json::{Value, json};
use uuid::Uuid;

use crate::domain::COMPANY_UPDATED;
use crate::store::CompanyStore;
use aphel_kernel::application::access::Authorizer;
use aphel_kernel::application::shared::{
    AuditLog, EventPublisher, FieldChange, RecordRef, RecordType,
};
use aphel_kernel::database::transaction::MySqlTx;
use aphel_kernel::domain::access::{AccessScope, Permission};
use aphel_kernel::domain::fields::FieldValueError;
use aphel_kernel::errors::{AppError, FieldViolation};
use aphel_kernel::records::{RecordMutation, changes_payload, publish_mutation};

pub(crate) mod http;
pub(crate) mod restore;
pub(crate) mod service;
pub(crate) mod store;

pub use store::HierarchyStore;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct HierarchyEntry {
    pub(crate) id: Uuid,
    pub(crate) parent_id: Option<Uuid>,
    pub(crate) depth: u32,
}

pub(crate) const MAX_HIERARCHY_DEPTH: usize = 10;
pub(crate) const COMPANY_HIERARCHY_CYCLE: &str = "company_hierarchy_cycle";
pub(crate) const COMPANY_HIERARCHY_TOO_DEEP: &str = "company_hierarchy_too_deep";
pub(crate) const PARENT_FIELD: &str = "parent_company_id";

fn cycle() -> AppError {
    AppError::invalid_fields_code(
        COMPANY_HIERARCHY_CYCLE,
        "invalid company hierarchy",
        vec![FieldViolation::new(
            PARENT_FIELD,
            "cannot be the company itself or one of its subsidiaries",
        )],
    )
}

fn too_deep() -> AppError {
    AppError::invalid_fields_code(
        COMPANY_HIERARCHY_TOO_DEEP,
        "invalid company hierarchy",
        vec![FieldViolation::new(
            PARENT_FIELD,
            format!("a company group can have at most {MAX_HIERARCHY_DEPTH} levels"),
        )],
    )
}

fn parent_not_found() -> AppError {
    AppError::invalid_fields(
        "invalid company",
        vec![FieldViolation::new(
            PARENT_FIELD,
            FieldValueError::ReferenceNotFound.message(),
        )],
    )
}

pub(crate) async fn check_parent(
    hierarchy: &HierarchyStore,
    companies: &CompanyStore,
    tx: &mut MySqlTx,
    scope: &AccessScope,
    company_id: Uuid,
    parent_id: Option<Uuid>,
) -> Result<(), AppError> {
    let Some(parent_id) = parent_id else {
        return Ok(());
    };
    if parent_id == company_id {
        return Err(cycle());
    }
    let visibility = scope.record_filter(&Permission::COMPANIES_READ)?;
    if companies
        .find(scope, &visibility, parent_id)
        .await?
        .is_none()
    {
        return Err(parent_not_found());
    }
    hierarchy.lock_company(tx, company_id).await?;
    let lineage = hierarchy.locked_lineage(tx, parent_id).await?;
    if lineage.is_empty() {
        return Err(parent_not_found());
    }
    check_placement(hierarchy, tx, company_id, &lineage).await
}

async fn check_placement(
    hierarchy: &HierarchyStore,
    tx: &mut MySqlTx,
    company_id: Uuid,
    parent_lineage: &[Uuid],
) -> Result<(), AppError> {
    if parent_lineage.contains(&company_id) {
        return Err(cycle());
    }
    let height = hierarchy.locked_height(tx, company_id).await?;
    if parent_lineage.len() + 1 + height > MAX_HIERARCHY_DEPTH {
        return Err(too_deep());
    }
    Ok(())
}

pub(crate) async fn reattach_children<E, A>(
    hierarchy: &HierarchyStore,
    events: &E,
    audit: &A,
    tx: &mut MySqlTx,
    scope: &AccessScope,
    company_id: Uuid,
    now: DateTime<Utc>,
) -> Result<Vec<Uuid>, AppError>
where
    E: EventPublisher<MySqlTx> + ?Sized,
    A: AuditLog<MySqlTx> + ?Sized,
{
    let Some(current_parent) = hierarchy.parent_of(tx, company_id).await? else {
        return Ok(Vec::new());
    };
    let candidates = hierarchy
        .detached_children(tx, company_id, current_parent)
        .await?;
    let mut reattached = Vec::new();
    if !candidates.is_empty() {
        let lineage = hierarchy.locked_lineage(tx, company_id).await?;
        for child in candidates {
            if check_placement(hierarchy, tx, child, &lineage)
                .await
                .is_ok()
            {
                reattached.push(child);
            }
        }
    }
    hierarchy.clear_detached(tx, company_id).await?;
    if reattached.is_empty() {
        return Ok(reattached);
    }
    hierarchy
        .set_parent(tx, &reattached, Some(company_id), now)
        .await?;
    let old_value = current_parent.map_or(Value::Null, |id| json!(id.to_string()));
    for child in &reattached {
        let changes = vec![FieldChange::new(
            PARENT_FIELD,
            old_value.clone(),
            json!(company_id.to_string()),
        )];
        publish_mutation(
            events,
            audit,
            tx,
            scope,
            now,
            RecordMutation {
                record: RecordRef::new(RecordType::COMPANY, *child),
                event_type: COMPANY_UPDATED,
                payload: changes_payload(&changes),
                changes,
                ip: None,
            },
        )
        .await?;
    }
    Ok(reattached)
}

pub async fn blocked_parents(
    hierarchy: &HierarchyStore,
    tx: &mut MySqlTx,
    roots: &[Uuid],
) -> Result<HashSet<Uuid>, AppError> {
    let mut blocked = HashSet::new();
    for root in roots {
        blocked.insert(*root);
        for entry in hierarchy.subtree_in(tx, *root).await? {
            blocked.insert(entry.id);
        }
    }
    Ok(blocked)
}

pub struct Reparent {
    pub from_parent: Uuid,
    pub to_parent: Option<Uuid>,
    pub except: Option<Uuid>,
    pub now: DateTime<Utc>,
    pub ip: Option<String>,
}

pub async fn reparent_children<E, A>(
    hierarchy: &HierarchyStore,
    events: &E,
    audit: &A,
    tx: &mut MySqlTx,
    scope: &AccessScope,
    reparent: Reparent,
) -> Result<Vec<Uuid>, AppError>
where
    E: EventPublisher<MySqlTx> + ?Sized,
    A: AuditLog<MySqlTx> + ?Sized,
{
    let children: Vec<Uuid> = hierarchy
        .children(tx, reparent.from_parent)
        .await?
        .into_iter()
        .filter(|id| Some(*id) != reparent.except && Some(*id) != reparent.to_parent)
        .collect();
    if children.is_empty() {
        return Ok(children);
    }
    if let Some(to_parent) = reparent.to_parent {
        let lineage = hierarchy.locked_lineage(tx, to_parent).await?;
        for child in &children {
            check_placement(hierarchy, tx, *child, &lineage).await?;
        }
    }
    hierarchy
        .set_parent(tx, &children, reparent.to_parent, reparent.now)
        .await?;
    let new_value = reparent
        .to_parent
        .map_or(Value::Null, |id| json!(id.to_string()));
    for child in &children {
        let changes = vec![FieldChange::new(
            PARENT_FIELD,
            json!(reparent.from_parent.to_string()),
            new_value.clone(),
        )];
        publish_mutation(
            events,
            audit,
            tx,
            scope,
            reparent.now,
            RecordMutation {
                record: RecordRef::new(RecordType::COMPANY, *child),
                event_type: COMPANY_UPDATED,
                payload: changes_payload(&changes),
                changes,
                ip: reparent.ip.clone(),
            },
        )
        .await?;
    }
    Ok(children)
}
