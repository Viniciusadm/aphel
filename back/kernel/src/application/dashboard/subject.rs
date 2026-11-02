use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::application::query::{Filter, FilterOperator, OWNER_TEAM_FIELD};
use crate::domain::access::{AccessScope, Permission, RecordScope};
use crate::domain::{TeamId, UserId};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SubjectKind {
    User,
    Team,
}

impl SubjectKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::User => "user",
            Self::Team => "team",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "user" => Some(Self::User),
            "team" => Some(Self::Team),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LayoutSubject {
    #[serde(rename = "type")]
    pub kind: SubjectKind,
    pub id: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SubjectUser {
    pub id: UserId,
    pub display_name: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SubjectTeam {
    pub id: TeamId,
    pub name: String,
    pub member_ids: Vec<UserId>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct SubjectDirectory {
    pub users: Vec<SubjectUser>,
    pub teams: Vec<SubjectTeam>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SubjectOptions {
    pub scope: RecordScope,
    pub users: Vec<SubjectUser>,
    pub teams: Vec<SubjectTeam>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WidgetSubject {
    pub kind: SubjectKind,
    pub id: String,
    pub name: String,
    pub member_ids: Vec<UserId>,
}

impl WidgetSubject {
    pub fn owner_filter(&self) -> Filter {
        match self.kind {
            SubjectKind::User => Filter::condition("owner_id", FilterOperator::Eq, json!(self.id)),
            SubjectKind::Team => {
                Filter::condition(OWNER_TEAM_FIELD, FilterOperator::In, json!([self.id]))
            }
        }
    }

    pub fn layout(&self) -> LayoutSubject {
        LayoutSubject {
            kind: self.kind,
            id: self.id.clone(),
        }
    }
}

pub fn subject_scope(scope: &AccessScope, permissions: &[Permission]) -> RecordScope {
    permissions
        .iter()
        .filter_map(|permission| scope.scope_for(permission))
        .max()
        .unwrap_or(RecordScope::Own)
}

fn own_user(scope: &AccessScope, directory: &SubjectDirectory) -> SubjectUser {
    directory
        .users
        .iter()
        .find(|user| &user.id == scope.user_id())
        .cloned()
        .unwrap_or_else(|| SubjectUser {
            id: scope.user_id().clone(),
            display_name: String::new(),
        })
}

pub fn visible_subjects(
    scope: &AccessScope,
    permissions: &[Permission],
    directory: SubjectDirectory,
) -> SubjectOptions {
    let record_scope = subject_scope(scope, permissions);
    match record_scope {
        RecordScope::Organization => SubjectOptions {
            scope: record_scope,
            users: directory.users,
            teams: directory.teams,
        },
        RecordScope::Team => {
            let teams: Vec<SubjectTeam> = directory
                .teams
                .iter()
                .filter(|team| scope.team_ids().contains(&team.id))
                .cloned()
                .collect();
            let mut users: Vec<SubjectUser> = directory
                .users
                .iter()
                .filter(|user| {
                    &user.id != scope.user_id()
                        && teams.iter().any(|team| team.member_ids.contains(&user.id))
                })
                .cloned()
                .collect();
            users.insert(0, own_user(scope, &directory));
            SubjectOptions {
                scope: record_scope,
                users,
                teams,
            }
        }
        RecordScope::Own => SubjectOptions {
            scope: record_scope,
            users: vec![own_user(scope, &directory)],
            teams: Vec::new(),
        },
    }
}

impl SubjectOptions {
    pub fn resolve(&self, kind: SubjectKind, id: &str) -> Option<WidgetSubject> {
        match kind {
            SubjectKind::User => {
                self.users
                    .iter()
                    .find(|user| user.id.as_str() == id)
                    .map(|user| WidgetSubject {
                        kind,
                        id: user.id.as_str().to_owned(),
                        name: user.display_name.clone(),
                        member_ids: vec![user.id.clone()],
                    })
            }
            SubjectKind::Team => {
                self.teams
                    .iter()
                    .find(|team| team.id.as_str() == id)
                    .map(|team| WidgetSubject {
                        kind,
                        id: team.id.as_str().to_owned(),
                        name: team.name.clone(),
                        member_ids: team.member_ids.clone(),
                    })
            }
        }
    }
}
