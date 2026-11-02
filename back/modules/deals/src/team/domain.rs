use std::str::FromStr;

use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde_json::json;
use uuid::Uuid;

use crate::domain::OwnerRef;
use aphel_kernel::application::files::UserReference;
use aphel_kernel::application::shared::RecordType;
use aphel_kernel::domain::{TeamId, UserId};
use aphel_kernel::errors::{AppError, CodedKind, FieldViolation};

pub use aphel_kernel::application::shared::parse_uuid as parse_id;

pub(crate) const MAX_DEAL_MEMBERS: usize = 50;
pub(crate) const MAX_DEAL_MEMBER_ROLES: usize = 50;
pub(crate) const MAX_DEAL_MEMBER_ROLE_NAME_LENGTH: usize = 140;
pub(crate) const CREDIT_DECIMALS: u32 = 2;
pub(crate) const FULL_CREDIT: Decimal = Decimal::ONE_HUNDRED;

pub(crate) const DEAL_MEMBER_NOT_FOUND: &str = "deal_member_not_found";
pub(crate) const DEAL_MEMBER_EXISTS: &str = "deal_member_exists";
pub(crate) const DEAL_MEMBERS_LIMIT_REACHED: &str = "deal_members_limit_reached";
pub(crate) const DEAL_CREDIT_EXCEEDS_TOTAL: &str = "deal_credit_exceeds_total";
pub(crate) const DEAL_MEMBER_ROLE_NOT_FOUND: &str = "deal_member_role_not_found";
pub(crate) const DEAL_MEMBER_ROLE_KEY_TAKEN: &str = "deal_member_role_key_taken";

pub(crate) const DEAL_MEMBER_ADDED: &str = "deal.member_added";
pub(crate) const DEAL_MEMBER_UPDATED: &str = "deal.member_updated";
pub(crate) const DEAL_MEMBER_REMOVED: &str = "deal.member_removed";

pub(crate) const DEAL_MEMBER_ROLE_RECORD_TYPE: RecordType =
    RecordType::from_static("deals.deal_member_role");
pub(crate) const DEAL_MEMBER_ROLE_CREATED: &str = "deal_member_role.created";
pub(crate) const DEAL_MEMBER_ROLE_UPDATED: &str = "deal_member_role.updated";
pub(crate) const DEAL_MEMBER_ROLE_DELETED: &str = "deal_member_role.deleted";
pub(crate) const DEAL_MEMBER_ROLES_REORDERED: &str = "deal_member_role.reordered";

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct DefaultDealMemberRole {
    pub(crate) key: &'static str,
    pub(crate) name: &'static str,
}

pub(crate) const DEFAULT_DEAL_MEMBER_ROLES: [DefaultDealMemberRole; 4] = [
    DefaultDealMemberRole {
        key: "sales",
        name: "Vendedor",
    },
    DefaultDealMemberRole {
        key: "pre_sales",
        name: "Pré-vendas",
    },
    DefaultDealMemberRole {
        key: "specialist",
        name: "Especialista",
    },
    DefaultDealMemberRole {
        key: "manager",
        name: "Gestor",
    },
];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DealMemberRole {
    pub id: Uuid,
    pub(crate) key: String,
    pub(crate) name: String,
    pub(crate) is_active: bool,
    pub(crate) position: i32,
    pub(crate) created_at: DateTime<Utc>,
    pub(crate) updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct DealMemberRoleRef {
    pub(crate) id: Uuid,
    pub(crate) key: String,
    pub(crate) name: String,
}

impl From<&DealMemberRole> for DealMemberRoleRef {
    fn from(role: &DealMemberRole) -> Self {
        Self {
            id: role.id,
            key: role.key.clone(),
            name: role.name.clone(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DealMember {
    pub(crate) id: Uuid,
    pub(crate) deal_id: Uuid,
    pub(crate) user: UserReference,
    pub(crate) user_active: bool,
    pub(crate) role: Option<DealMemberRoleRef>,
    pub(crate) credit_percent: Decimal,
    pub(crate) added_by: Option<UserId>,
    pub(crate) created_at: DateTime<Utc>,
    pub(crate) updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct NewDealMember {
    pub(crate) id: Uuid,
    pub(crate) deal_id: Uuid,
    pub(crate) user_id: UserId,
    pub(crate) role_id: Option<Uuid>,
    pub(crate) credit_percent: Decimal,
    pub(crate) added_by: Option<UserId>,
    pub(crate) at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct DealTeam {
    pub(crate) deal_id: Uuid,
    pub(crate) owner: Option<OwnerRef>,
    pub(crate) owner_percent: Option<Decimal>,
    pub(crate) allocated_percent: Decimal,
    pub(crate) remaining_percent: Decimal,
    pub(crate) members: Vec<DealMember>,
}

impl DealTeam {
    pub(crate) fn new(deal_id: Uuid, owner: Option<OwnerRef>, members: Vec<DealMember>) -> Self {
        let allocated_percent = allocated(&members);
        let remaining_percent = (FULL_CREDIT - allocated_percent).max(Decimal::ZERO);
        let owner_percent = owner.as_ref().map(|owner| {
            remaining_percent
                + members
                    .iter()
                    .filter(|member| member.user.id == owner.id)
                    .map(|member| member.credit_percent)
                    .sum::<Decimal>()
        });
        Self {
            deal_id,
            owner,
            owner_percent,
            allocated_percent,
            remaining_percent,
            members,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct AddDealMemberCommand {
    pub user_id: String,
    pub role_id: Option<String>,
    pub credit_percent: Option<String>,
    pub ip: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) struct UpdateDealMemberCommand {
    pub(crate) role_id: Option<Option<String>>,
    pub(crate) credit_percent: Option<String>,
    pub(crate) expected_updated_at: Option<DateTime<Utc>>,
    pub(crate) ip: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) struct CreateDealMemberRoleCommand {
    pub(crate) key: Option<String>,
    pub(crate) name: String,
    pub(crate) ip: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) struct UpdateDealMemberRoleCommand {
    pub(crate) name: Option<String>,
    pub(crate) is_active: Option<bool>,
    pub(crate) expected_updated_at: Option<DateTime<Utc>>,
    pub(crate) ip: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CreditedUsers {
    User(UserId),
    Team(TeamId),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct CreditTotals {
    pub count: u64,
    pub amount: Decimal,
}

pub(crate) fn allocated(members: &[DealMember]) -> Decimal {
    members.iter().map(|member| member.credit_percent).sum()
}

pub(crate) fn parse_credit_percent(text: &str) -> Option<Decimal> {
    let text = text.trim();
    let value = Decimal::from_str(text).ok()?;
    if value.is_sign_negative()
        || value > FULL_CREDIT
        || value.normalize().scale() > CREDIT_DECIMALS
    {
        return None;
    }
    Some(value.round_dp(CREDIT_DECIMALS))
}

pub(crate) fn percent_text(value: Decimal) -> String {
    format!("{:.2}", value.round_dp(2))
}

pub(crate) fn member_not_found() -> AppError {
    AppError::not_found_code(DEAL_MEMBER_NOT_FOUND, "deal member was not found")
}

pub(crate) fn deal_member_role_not_found() -> AppError {
    AppError::not_found_code(DEAL_MEMBER_ROLE_NOT_FOUND, "deal member role was not found")
}

pub(crate) fn credit_exceeded(others: Decimal) -> AppError {
    let available = (FULL_CREDIT - others).max(Decimal::ZERO);
    AppError::with_details(
        CodedKind::Validation,
        DEAL_CREDIT_EXCEEDS_TOTAL,
        format!(
            "the credit of the members cannot exceed 100%; at most {} is available",
            percent_text(available)
        ),
        json!({
            "allocated_percent": percent_text(others),
            "available_percent": percent_text(available),
        }),
    )
}

pub(crate) fn role_name(value: &str, violations: &mut Vec<FieldViolation>) -> Option<String> {
    let name = value.trim();
    if name.is_empty() {
        violations.push(FieldViolation::new("name", "is required"));
        return None;
    }
    if name.chars().count() > MAX_DEAL_MEMBER_ROLE_NAME_LENGTH {
        violations.push(FieldViolation::new(
            "name",
            format!("must have at most {MAX_DEAL_MEMBER_ROLE_NAME_LENGTH} characters"),
        ));
        return None;
    }
    Some(name.to_owned())
}
