use chrono::{DateTime, Utc};

use crate::application::auth::ClientMetadata;
use crate::domain::access::RecordScope;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InviteMemberCommand {
    pub email: String,
    pub role_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChangeMemberRoleCommand {
    pub role_id: String,
    pub expected_updated_at: Option<DateTime<Utc>>,
    pub client: ClientMetadata,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GrantInput {
    pub permission: String,
    pub scope: RecordScope,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CreateRoleCommand {
    pub name: String,
    pub key: Option<String>,
    pub permissions: Vec<GrantInput>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct UpdateRoleCommand {
    pub name: Option<String>,
    pub key: Option<String>,
    pub permissions: Option<Vec<GrantInput>>,
    pub expected_updated_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TeamMemberInput {
    pub user_id: String,
    pub is_manager: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TeamCommand {
    pub name: String,
    pub parent_team_id: Option<String>,
    pub members: Vec<TeamMemberInput>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct UpdateTeamCommand {
    pub name: Option<String>,
    pub parent_team_id: Option<Option<String>>,
    pub members: Option<Vec<TeamMemberInput>>,
    pub expected_updated_at: Option<DateTime<Utc>>,
}
