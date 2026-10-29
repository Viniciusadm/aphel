use chrono::{DateTime, SecondsFormat, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::application::organization_admin::{
    ChangeMemberRoleCommand, InviteMemberCommand, MemberView, RoleSummary, TeamReference,
};
use crate::domain::users::UserStatus;

#[derive(Debug, Clone, Copy, Serialize, ToSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum MemberStatusDto {
    Invited,
    Active,
    Deactivated,
}

impl From<UserStatus> for MemberStatusDto {
    fn from(value: UserStatus) -> Self {
        match value {
            UserStatus::Invited => Self::Invited,
            UserStatus::Active => Self::Active,
            UserStatus::Deactivated => Self::Deactivated,
        }
    }
}

#[derive(Debug, Serialize, ToSchema)]
pub struct RoleReferenceResponse {
    pub id: String,
    #[schema(example = "manager")]
    pub key: String,
    pub name: String,
    pub is_system: bool,
}

impl From<RoleSummary> for RoleReferenceResponse {
    fn from(role: RoleSummary) -> Self {
        Self {
            id: role.id.into_inner(),
            key: role.key.as_str().to_owned(),
            name: role.name,
            is_system: role.is_system,
        }
    }
}

#[derive(Debug, Serialize, ToSchema)]
pub struct TeamReferenceResponse {
    pub id: String,
    pub name: String,
}

impl From<TeamReference> for TeamReferenceResponse {
    fn from(team: TeamReference) -> Self {
        Self {
            id: team.id.into_inner(),
            name: team.name,
        }
    }
}

#[derive(Debug, Serialize, ToSchema)]
pub struct MemberResponse {
    pub id: String,
    pub user_id: String,
    pub email: String,
    pub display_name: String,
    pub role: RoleReferenceResponse,
    pub status: MemberStatusDto,
    pub invited_by: Option<String>,
    #[schema(format = DateTime)]
    pub joined_at: Option<String>,
    #[schema(format = DateTime)]
    pub created_at: String,
    #[schema(format = DateTime, example = "2026-10-20T13:45:00Z")]
    pub updated_at: String,
    pub teams: Vec<TeamReferenceResponse>,
}

pub fn timestamp(value: DateTime<Utc>) -> String {
    value.to_rfc3339_opts(SecondsFormat::Secs, true)
}

impl From<MemberView> for MemberResponse {
    fn from(member: MemberView) -> Self {
        Self {
            id: member.user_id.as_str().to_owned(),
            user_id: member.user_id.into_inner(),
            email: member.email,
            display_name: member.display_name,
            role: member.role.into(),
            status: member.status.into(),
            invited_by: member.invited_by.map(|id| id.into_inner()),
            joined_at: member.joined_at.map(timestamp),
            created_at: timestamp(member.created_at),
            updated_at: timestamp(member.updated_at),
            teams: member.teams.into_iter().map(Into::into).collect(),
        }
    }
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct InviteMemberRequest {
    #[schema(example = "bia@example.com")]
    pub email: String,
    pub role_id: String,
}

impl From<InviteMemberRequest> for InviteMemberCommand {
    fn from(request: InviteMemberRequest) -> Self {
        Self {
            email: request.email,
            role_id: request.role_id,
        }
    }
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct ChangeMemberRoleRequest {
    pub role_id: String,
}

impl ChangeMemberRoleRequest {
    pub fn into_command(
        self,
        expected_updated_at: Option<DateTime<Utc>>,
    ) -> ChangeMemberRoleCommand {
        ChangeMemberRoleCommand {
            role_id: self.role_id,
            expected_updated_at,
            client: Default::default(),
        }
    }
}
