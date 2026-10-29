use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::api::dto::common::double_option;
use crate::api::dto::members::timestamp;
use crate::application::organization_admin::{
    TeamCommand, TeamMemberInput, TeamMemberView, TeamView, UpdateTeamCommand,
};

#[derive(Debug, Serialize, ToSchema)]
pub struct TeamMemberResponse {
    pub user_id: String,
    pub email: String,
    pub display_name: String,
    pub is_manager: bool,
}

impl From<TeamMemberView> for TeamMemberResponse {
    fn from(member: TeamMemberView) -> Self {
        Self {
            user_id: member.user_id.into_inner(),
            email: member.email,
            display_name: member.display_name,
            is_manager: member.is_manager,
        }
    }
}

#[derive(Debug, Serialize, ToSchema)]
pub struct TeamResponse {
    pub id: String,
    pub name: String,
    pub parent_team_id: Option<String>,
    pub members: Vec<TeamMemberResponse>,
    #[schema(example = "2026-10-20T13:45:00Z")]
    pub updated_at: String,
}

impl From<TeamView> for TeamResponse {
    fn from(team: TeamView) -> Self {
        Self {
            id: team.id.into_inner(),
            name: team.name,
            parent_team_id: team.parent_team_id.map(|id| id.into_inner()),
            members: team.members.into_iter().map(Into::into).collect(),
            updated_at: timestamp(team.updated_at),
        }
    }
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct TeamMemberRequest {
    pub user_id: String,
    #[serde(default)]
    pub is_manager: bool,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct TeamRequest {
    #[schema(example = "Sales")]
    pub name: String,
    pub parent_team_id: Option<String>,
    #[serde(default)]
    pub members: Vec<TeamMemberRequest>,
}

impl From<TeamRequest> for TeamCommand {
    fn from(request: TeamRequest) -> Self {
        Self {
            name: request.name,
            parent_team_id: request.parent_team_id,
            members: request
                .members
                .into_iter()
                .map(|member| TeamMemberInput {
                    user_id: member.user_id,
                    is_manager: member.is_manager,
                })
                .collect(),
        }
    }
}

#[derive(Debug, Default, Deserialize, ToSchema)]
pub struct UpdateTeamRequest {
    #[schema(example = "Sales")]
    pub name: Option<String>,
    #[serde(default, deserialize_with = "double_option")]
    #[schema(value_type = Option<String>)]
    pub parent_team_id: Option<Option<String>>,
    pub members: Option<Vec<TeamMemberRequest>>,
}

impl UpdateTeamRequest {
    pub fn into_command(self, expected_updated_at: Option<DateTime<Utc>>) -> UpdateTeamCommand {
        UpdateTeamCommand {
            name: self.name,
            parent_team_id: self.parent_team_id,
            members: self.members.map(|members| {
                members
                    .into_iter()
                    .map(|member| TeamMemberInput {
                        user_id: member.user_id,
                        is_manager: member.is_manager,
                    })
                    .collect()
            }),
            expected_updated_at,
        }
    }
}
