use async_trait::async_trait;
use chrono::{DateTime, Utc};

use crate::application::auth::NewUserToken;
use crate::application::mail::Mailer;
use crate::application::organization_profile::NewRole;
use crate::application::security::{Clock, SecureTokens};
use crate::application::shared::{AuditLog, EventPublisher, TransactionManager};
use crate::domain::access::{PermissionGrant, RoleKey};
use crate::domain::users::UserStatus;
use crate::domain::{Email, NonEmptyString, RoleId, TeamId, UserId};
use crate::errors::AppError;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ListWindow {
    pub offset: u64,
    pub limit: u64,
}

impl ListWindow {
    pub fn all() -> Self {
        Self {
            offset: 0,
            limit: u64::from(u32::MAX),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Listing<T> {
    pub items: Vec<T>,
    pub total: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RoleSummary {
    pub id: RoleId,
    pub key: RoleKey,
    pub name: String,
    pub is_system: bool,
}

impl RoleSummary {
    pub fn is_system_admin(&self) -> bool {
        self.is_system && self.key.is_admin()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TeamReference {
    pub id: TeamId,
    pub name: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MemberView {
    pub user_id: UserId,
    pub email: String,
    pub display_name: String,
    pub role: RoleSummary,
    pub status: UserStatus,
    pub invited_by: Option<UserId>,
    pub joined_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub teams: Vec<TeamReference>,
}

impl MemberView {
    pub fn is_active_admin(&self) -> bool {
        self.status == UserStatus::Active && self.role.is_system_admin()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InvitedUser {
    pub id: UserId,
    pub email: Email,
    pub display_name: NonEmptyString,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewInvitation {
    pub user: InvitedUser,
    pub role_id: RoleId,
    pub invited_by: UserId,
    pub token: NewUserToken,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InvitationContext {
    pub organization_name: String,
    pub inviter_display_name: String,
    pub organization_locale: String,
    pub invitee_locale: Option<String>,
}

#[async_trait]
pub trait MemberRepository: Send + Sync {
    type Tx: Send;

    async fn list(&self, window: ListWindow) -> Result<Listing<MemberView>, AppError>;
    async fn find(&self, user_id: &UserId) -> Result<Option<MemberView>, AppError>;
    async fn email_exists(&self, email: &Email) -> Result<bool, AppError>;
    async fn invitation_context(
        &self,
        inviter_id: &UserId,
        invitee_email: &str,
    ) -> Result<InvitationContext, AppError>;
    async fn create_invitation(
        &self,
        tx: &mut Self::Tx,
        invitation: NewInvitation,
    ) -> Result<(), AppError>;
    async fn replace_invitation_token(
        &self,
        tx: &mut Self::Tx,
        user_id: &UserId,
        token: NewUserToken,
        at: DateTime<Utc>,
    ) -> Result<(), AppError>;
    async fn cancel_invitation(
        &self,
        tx: &mut Self::Tx,
        user_id: &UserId,
        at: DateTime<Utc>,
    ) -> Result<bool, AppError>;
    async fn change_role(
        &self,
        tx: &mut Self::Tx,
        user_id: &UserId,
        role_id: &RoleId,
    ) -> Result<(), AppError>;
    async fn deactivate(
        &self,
        tx: &mut Self::Tx,
        user_id: &UserId,
        at: DateTime<Utc>,
    ) -> Result<(), AppError>;
    async fn reactivate(&self, tx: &mut Self::Tx, user_id: &UserId) -> Result<(), AppError>;
    async fn remove(
        &self,
        tx: &mut Self::Tx,
        user_id: &UserId,
        at: DateTime<Utc>,
    ) -> Result<(), AppError>;
    async fn count_active_admins(&self) -> Result<u64, AppError>;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RoleView {
    pub role: RoleSummary,
    pub grants: Vec<PermissionGrant>,
    pub member_count: u64,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RoleChanges {
    pub name: String,
    pub key: RoleKey,
    pub grants: Option<Vec<PermissionGrant>>,
}

#[async_trait]
pub trait RoleRepository: Send + Sync {
    type Tx: Send;

    async fn list(&self, window: ListWindow) -> Result<Listing<RoleView>, AppError>;
    async fn find(&self, role_id: &RoleId) -> Result<Option<RoleView>, AppError>;
    async fn key_exists(&self, key: &RoleKey) -> Result<bool, AppError>;
    async fn create(&self, tx: &mut Self::Tx, role: NewRole) -> Result<(), AppError>;
    async fn update(
        &self,
        tx: &mut Self::Tx,
        role_id: &RoleId,
        changes: RoleChanges,
    ) -> Result<(), AppError>;
    async fn delete(&self, tx: &mut Self::Tx, role_id: &RoleId) -> Result<(), AppError>;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TeamMemberView {
    pub user_id: UserId,
    pub email: String,
    pub display_name: String,
    pub is_manager: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TeamView {
    pub id: TeamId,
    pub name: String,
    pub parent_team_id: Option<TeamId>,
    pub members: Vec<TeamMemberView>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TeamMemberAssignment {
    pub user_id: UserId,
    pub is_manager: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TeamDraft {
    pub name: NonEmptyString,
    pub parent_team_id: Option<TeamId>,
    pub members: Vec<TeamMemberAssignment>,
}

#[async_trait]
pub trait TeamRepository: Send + Sync {
    type Tx: Send;

    async fn list(&self, window: ListWindow) -> Result<Listing<TeamView>, AppError>;
    async fn find(&self, team_id: &TeamId) -> Result<Option<TeamView>, AppError>;
    async fn ancestors(&self, team_id: &TeamId) -> Result<Vec<TeamId>, AppError>;
    async fn members_of_organization(&self, user_ids: &[UserId]) -> Result<Vec<UserId>, AppError>;
    async fn create(
        &self,
        tx: &mut Self::Tx,
        team_id: &TeamId,
        draft: TeamDraft,
    ) -> Result<(), AppError>;
    async fn update(
        &self,
        tx: &mut Self::Tx,
        team_id: &TeamId,
        draft: TeamDraft,
    ) -> Result<(), AppError>;
    async fn delete(&self, tx: &mut Self::Tx, team_id: &TeamId) -> Result<(), AppError>;
}

pub type AdminTx<P> = <<P as OrganizationAdminPorts>::Transactions as TransactionManager>::Tx;

pub trait OrganizationAdminPorts: Send + Sync {
    type Transactions: TransactionManager;
    type Members: MemberRepository<Tx = <Self::Transactions as TransactionManager>::Tx>;
    type Roles: RoleRepository<Tx = <Self::Transactions as TransactionManager>::Tx>;
    type Teams: TeamRepository<Tx = <Self::Transactions as TransactionManager>::Tx>;
    type Secrets: SecureTokens;
    type Mailer: Mailer;
    type Clock: Clock;
    type Events: EventPublisher<<Self::Transactions as TransactionManager>::Tx>;
    type Audit: AuditLog<<Self::Transactions as TransactionManager>::Tx>;

    fn transactions(&self) -> &Self::Transactions;
    fn members(&self) -> &Self::Members;
    fn roles(&self) -> &Self::Roles;
    fn teams(&self) -> &Self::Teams;
    fn secrets(&self) -> &Self::Secrets;
    fn mailer(&self) -> &Self::Mailer;
    fn clock(&self) -> &Self::Clock;
    fn events(&self) -> &Self::Events;
    fn audit(&self) -> &Self::Audit;
}
