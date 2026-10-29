use std::collections::BTreeSet;

use crate::application::access::Authorizer;
use crate::application::organization_admin::use_cases::{
    TEAM_CYCLE, TEAM_NOT_FOUND, bounded_name, name_message, new_id,
};
use crate::application::organization_admin::{
    AdminChange, TEAM_CREATED, TEAM_DELETED, TEAM_UPDATED,
};
use crate::application::organization_admin::{
    ListWindow, Listing, OrganizationAdminPorts, OrganizationAdminUseCases, TeamCommand, TeamDraft,
    TeamMemberAssignment, TeamMemberInput, TeamRepository, TeamView, UpdateTeamCommand,
};
use crate::application::shared::ensure_version;
use crate::application::shared::{RecordType, TransactionManager};
use crate::domain::access::{AccessScope, Permission};
use crate::domain::{NonEmptyString, TeamId, UserId};
use crate::errors::{AppError, FieldViolation};

impl<P> OrganizationAdminUseCases<P>
where
    P: OrganizationAdminPorts,
{
    pub async fn list_teams(
        &self,
        scope: &AccessScope,
        window: ListWindow,
    ) -> Result<Listing<TeamView>, AppError> {
        scope.require(&Permission::SETTINGS_MANAGE)?;
        self.ports.teams().list(window).await
    }

    pub async fn get_team(&self, scope: &AccessScope, team_id: &str) -> Result<TeamView, AppError> {
        scope.require(&Permission::SETTINGS_MANAGE)?;
        self.require_team(team_id).await
    }

    pub async fn create_team(
        &self,
        scope: &AccessScope,
        command: TeamCommand,
    ) -> Result<TeamView, AppError> {
        scope.require(&Permission::SETTINGS_MANAGE)?;
        let team_id = TeamId::new(new_id())?;
        let draft = self.team_draft(None, command).await?;
        let transactions = self.ports.transactions();
        let mut tx = transactions.begin().await?;
        self.ports.teams().create(&mut tx, &team_id, draft).await?;
        self.record_admin_change(
            &mut tx,
            scope,
            AdminChange::new(
                TEAM_CREATED,
                RecordType::TEAM,
                team_id.as_str(),
                serde_json::json!({}),
            )?,
        )
        .await?;
        transactions.commit(tx).await?;
        self.require_team(team_id.as_str()).await
    }

    pub async fn update_team(
        &self,
        scope: &AccessScope,
        team_id: &str,
        command: UpdateTeamCommand,
    ) -> Result<TeamView, AppError> {
        scope.require(&Permission::SETTINGS_MANAGE)?;
        let current = self.require_team(team_id).await?;
        ensure_version(command.expected_updated_at, current.updated_at)?;
        let merged = TeamCommand {
            name: command.name.unwrap_or_else(|| current.name.clone()),
            parent_team_id: match command.parent_team_id {
                Some(parent_team_id) => parent_team_id,
                None => current
                    .parent_team_id
                    .as_ref()
                    .map(|id| id.as_str().to_owned()),
            },
            members: command.members.unwrap_or_else(|| {
                current
                    .members
                    .iter()
                    .map(|member| TeamMemberInput {
                        user_id: member.user_id.as_str().to_owned(),
                        is_manager: member.is_manager,
                    })
                    .collect()
            }),
        };
        let draft = self.team_draft(Some(&current.id), merged).await?;
        let transactions = self.ports.transactions();
        let mut tx = transactions.begin().await?;
        self.ports
            .teams()
            .update(&mut tx, &current.id, draft)
            .await?;
        self.record_admin_change(
            &mut tx,
            scope,
            AdminChange::new(
                TEAM_UPDATED,
                RecordType::TEAM,
                current.id.as_str(),
                serde_json::json!({ "name": current.name }),
            )?,
        )
        .await?;
        transactions.commit(tx).await?;
        self.require_team(team_id).await
    }

    pub async fn delete_team(&self, scope: &AccessScope, team_id: &str) -> Result<(), AppError> {
        scope.require(&Permission::SETTINGS_MANAGE)?;
        let current = self.require_team(team_id).await?;
        let transactions = self.ports.transactions();
        let mut tx = transactions.begin().await?;
        self.ports.teams().delete(&mut tx, &current.id).await?;
        self.record_admin_change(
            &mut tx,
            scope,
            AdminChange::new(
                TEAM_DELETED,
                RecordType::TEAM,
                current.id.as_str(),
                serde_json::json!({ "name": current.name }),
            )?,
        )
        .await?;
        transactions.commit(tx).await?;
        Ok(())
    }

    async fn require_team(&self, team_id: &str) -> Result<TeamView, AppError> {
        let Ok(team_id) = TeamId::new(team_id) else {
            return Err(team_not_found());
        };
        self.ports
            .teams()
            .find(&team_id)
            .await?
            .ok_or_else(team_not_found)
    }

    async fn team_draft(
        &self,
        team_id: Option<&TeamId>,
        command: TeamCommand,
    ) -> Result<TeamDraft, AppError> {
        let mut violations = Vec::new();
        let name = bounded_name(&command.name);
        if name.is_none() {
            violations.push(FieldViolation::new("name", name_message()));
        }

        let parent_team_id = match command
            .parent_team_id
            .as_deref()
            .map(str::trim)
            .filter(|id| !id.is_empty())
        {
            None => None,
            Some(parent) => {
                let parent = TeamId::new(parent)?;
                if self.ports.teams().find(&parent).await?.is_none() {
                    violations.push(FieldViolation::new("parent_team_id", "team was not found"));
                    None
                } else {
                    Some(parent)
                }
            }
        };

        let mut seen = BTreeSet::new();
        let mut members = Vec::new();
        for (index, input) in command.members.into_iter().enumerate() {
            let field = format!("members[{index}].user_id");
            let Ok(user_id) = UserId::new(input.user_id.trim()) else {
                violations.push(FieldViolation::new(field, "is required"));
                continue;
            };
            if !seen.insert(user_id.clone()) {
                violations.push(FieldViolation::new(field, "is duplicated"));
                continue;
            }
            members.push((
                index,
                TeamMemberAssignment {
                    user_id,
                    is_manager: input.is_manager,
                },
            ));
        }
        let user_ids: Vec<UserId> = members
            .iter()
            .map(|(_, member)| member.user_id.clone())
            .collect();
        let known: BTreeSet<UserId> = if user_ids.is_empty() {
            BTreeSet::new()
        } else {
            self.ports
                .teams()
                .members_of_organization(&user_ids)
                .await?
                .into_iter()
                .collect()
        };
        for (index, member) in &members {
            if !known.contains(&member.user_id) {
                violations.push(FieldViolation::new(
                    format!("members[{index}].user_id"),
                    "is not a member of the organization",
                ));
            }
        }

        let Some(name) = name.filter(|_| violations.is_empty()) else {
            return Err(AppError::invalid_fields("invalid team", violations));
        };

        if let (Some(team_id), Some(parent)) = (team_id, parent_team_id.as_ref())
            && (parent == team_id
                || self
                    .ports
                    .teams()
                    .ancestors(parent)
                    .await?
                    .contains(team_id))
        {
            return Err(AppError::validation_code(
                TEAM_CYCLE,
                "a team cannot be placed under itself or one of its sub-teams",
            ));
        }

        Ok(TeamDraft {
            name: NonEmptyString::new(name, "name")?,
            parent_team_id,
            members: members.into_iter().map(|(_, member)| member).collect(),
        })
    }
}

fn team_not_found() -> AppError {
    AppError::not_found_code(TEAM_NOT_FOUND, "team was not found")
}
