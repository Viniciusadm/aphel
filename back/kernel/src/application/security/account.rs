use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde_json::{Value, json};

use crate::application::access::Authorizer;
use crate::application::auth::{ClientMetadata, UserRepository};
use crate::application::organization_admin::{ListWindow, Listing};
use crate::application::security::{
    Clock, NewSecurityEvent, PasswordHasher, SecurityEventKind, SecurityEventRecorder,
    describe_device, password_violation, record_quietly,
};
use crate::domain::access::{AccessScope, Permission};
use crate::domain::users::User;
use crate::domain::{Secret, SessionId, UserId};
use crate::errors::{AppError, FieldViolation};

pub const SESSION_NOT_FOUND: &str = "session_not_found";
pub const CURRENT_SESSION_NOT_REVOCABLE: &str = "current_session_not_revocable";
pub const INVALID_CURRENT_PASSWORD: &str = "invalid_current_password";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActiveSession {
    pub id: SessionId,
    pub ip: Option<String>,
    pub user_agent: Option<String>,
    pub created_at: DateTime<Utc>,
    pub last_seen_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionView {
    pub session: ActiveSession,
    pub current: bool,
}

#[async_trait]
pub trait AccountSessionRepository: Send + Sync {
    async fn list_active(
        &self,
        user_id: &UserId,
        now: DateTime<Utc>,
    ) -> Result<Vec<ActiveSession>, AppError>;
    async fn find_active(
        &self,
        user_id: &UserId,
        session_id: &str,
        now: DateTime<Utc>,
    ) -> Result<Option<ActiveSession>, AppError>;
    async fn revoke(
        &self,
        user_id: &UserId,
        session_id: &SessionId,
        at: DateTime<Utc>,
    ) -> Result<bool, AppError>;
    async fn revoke_others(
        &self,
        user_id: &UserId,
        keep: &SessionId,
        at: DateTime<Utc>,
    ) -> Result<u64, AppError>;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SecurityEventOwner {
    User(UserId),
    All,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SecurityEventFilter {
    pub owner: SecurityEventOwner,
    pub kinds: Vec<SecurityEventKind>,
    pub from: Option<DateTime<Utc>>,
    pub to: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SecurityEventUser {
    pub id: UserId,
    pub name: String,
    pub email: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SecurityEventView {
    pub id: String,
    pub kind: SecurityEventKind,
    pub user: Option<SecurityEventUser>,
    pub ip: Option<String>,
    pub user_agent: Option<String>,
    pub metadata: Value,
    pub occurred_at: DateTime<Utc>,
}

#[async_trait]
pub trait SecurityEventRepository: Send + Sync {
    async fn list(
        &self,
        filter: &SecurityEventFilter,
        window: ListWindow,
    ) -> Result<Listing<SecurityEventView>, AppError>;
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SecurityEventQuery {
    pub kind: Option<String>,
    pub from: Option<String>,
    pub to: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChangePasswordCommand {
    pub current_password: Secret,
    pub new_password: Secret,
    pub client: ClientMetadata,
}

pub trait AccountSecurityPorts: Send + Sync {
    type Sessions: AccountSessionRepository;
    type Users: UserRepository;
    type Hasher: PasswordHasher;
    type Events: SecurityEventRepository;
    type Recorder: SecurityEventRecorder;
    type Clock: Clock;

    fn sessions(&self) -> &Self::Sessions;
    fn users(&self) -> &Self::Users;
    fn hasher(&self) -> &Self::Hasher;
    fn events(&self) -> &Self::Events;
    fn recorder(&self) -> &Self::Recorder;
    fn clock(&self) -> &Self::Clock;
}

pub struct AccountSecurityUseCases<P> {
    ports: P,
}

fn session_not_found() -> AppError {
    AppError::not_found_code(SESSION_NOT_FOUND, "session not found")
}

fn session_metadata(session: &ActiveSession, scope: &str) -> Value {
    let device = describe_device(session.user_agent.as_deref());
    json!({
        "scope": scope,
        "session_id": session.id.as_str(),
        "session_ip": session.ip,
        "session_device": device.label(),
    })
}

impl<P> AccountSecurityUseCases<P>
where
    P: AccountSecurityPorts,
{
    pub fn new(ports: P) -> Self {
        Self { ports }
    }

    pub async fn list_sessions(
        &self,
        user: &User,
        current: &SessionId,
    ) -> Result<Vec<SessionView>, AppError> {
        let now = self.ports.clock().now();
        let mut sessions: Vec<SessionView> = self
            .ports
            .sessions()
            .list_active(&user.id, now)
            .await?
            .into_iter()
            .map(|session| SessionView {
                current: &session.id == current,
                session,
            })
            .collect();
        sessions.sort_by(|left, right| {
            right
                .current
                .cmp(&left.current)
                .then(right.session.last_seen_at.cmp(&left.session.last_seen_at))
        });
        Ok(sessions)
    }

    pub async fn revoke_session(
        &self,
        user: &User,
        current: &SessionId,
        session_id: &str,
        client: &ClientMetadata,
    ) -> Result<(), AppError> {
        let now = self.ports.clock().now();
        let session = self
            .ports
            .sessions()
            .find_active(&user.id, session_id.trim(), now)
            .await?
            .ok_or_else(session_not_found)?;
        if &session.id == current {
            return Err(AppError::validation_code(
                CURRENT_SESSION_NOT_REVOCABLE,
                "the current session is ended by signing out",
            ));
        }
        if !self
            .ports
            .sessions()
            .revoke(&user.id, &session.id, now)
            .await?
        {
            return Err(session_not_found());
        }
        record_quietly(
            self.ports.recorder(),
            NewSecurityEvent::new(SecurityEventKind::SessionRevoked, now)
                .user(&user.id)
                .client(client.ip.as_deref(), client.user_agent.as_deref())
                .metadata(session_metadata(&session, "single")),
        )
        .await;
        Ok(())
    }

    pub async fn revoke_other_sessions(
        &self,
        user: &User,
        current: &SessionId,
        client: &ClientMetadata,
    ) -> Result<u64, AppError> {
        let now = self.ports.clock().now();
        let revoked = self
            .ports
            .sessions()
            .revoke_others(&user.id, current, now)
            .await?;
        if revoked > 0 {
            record_quietly(
                self.ports.recorder(),
                NewSecurityEvent::new(SecurityEventKind::SessionRevoked, now)
                    .user(&user.id)
                    .client(client.ip.as_deref(), client.user_agent.as_deref())
                    .metadata(json!({ "scope": "others", "count": revoked })),
            )
            .await;
        }
        Ok(revoked)
    }

    pub async fn change_password(
        &self,
        user: &User,
        current: &SessionId,
        command: ChangePasswordCommand,
    ) -> Result<u64, AppError> {
        let mut violations = Vec::new();
        if command.current_password.expose().is_empty() {
            violations.push(FieldViolation::new("current_password", "is required"));
        }
        if let Some(message) = password_violation(command.new_password.expose()) {
            violations.push(FieldViolation::new("new_password", message));
        } else if command.new_password.expose() == command.current_password.expose() {
            violations.push(FieldViolation::new(
                "new_password",
                "must be different from the current password",
            ));
        }
        if !violations.is_empty() {
            return Err(AppError::invalid_fields("invalid password", violations));
        }
        let stored = self
            .ports
            .users()
            .find_by_id(&user.id)
            .await?
            .ok_or_else(|| AppError::Unauthorized("missing session".to_owned()))?;
        let matches = match stored.password_hash.as_ref() {
            Some(hash) => self
                .ports
                .hasher()
                .verify_password(command.current_password.expose(), hash.as_str())?,
            None => false,
        };
        if !matches {
            return Err(AppError::invalid_fields_code(
                INVALID_CURRENT_PASSWORD,
                "the current password is not correct",
                vec![FieldViolation::new(
                    "current_password",
                    "the current password is not correct",
                )],
            ));
        }
        let now = self.ports.clock().now();
        let password_hash = self
            .ports
            .hasher()
            .hash_password(command.new_password.expose())?;
        self.ports
            .users()
            .set_password(&user.id, &password_hash)
            .await?;
        let revoked = self
            .ports
            .sessions()
            .revoke_others(&user.id, current, now)
            .await?;
        record_quietly(
            self.ports.recorder(),
            NewSecurityEvent::new(SecurityEventKind::PasswordChanged, now)
                .user(&user.id)
                .client(
                    command.client.ip.as_deref(),
                    command.client.user_agent.as_deref(),
                )
                .metadata(json!({ "source": "account", "revoked_sessions": revoked })),
        )
        .await;
        Ok(revoked)
    }

    pub async fn list_own_events(
        &self,
        user: &User,
        query: SecurityEventQuery,
        window: ListWindow,
    ) -> Result<Listing<SecurityEventView>, AppError> {
        let filter = parse_filter(SecurityEventOwner::User(user.id.clone()), query)?;
        self.ports.events().list(&filter, window).await
    }

    pub async fn list_organization_events(
        &self,
        scope: &AccessScope,
        query: SecurityEventQuery,
        window: ListWindow,
    ) -> Result<Listing<SecurityEventView>, AppError> {
        scope.require(&Permission::USERS_MANAGE)?;
        let filter = parse_filter(SecurityEventOwner::All, query)?;
        self.ports.events().list(&filter, window).await
    }
}

fn present(value: Option<String>) -> Option<String> {
    value
        .map(|value| value.trim().to_owned())
        .filter(|value| !value.is_empty())
}

fn parse_filter(
    owner: SecurityEventOwner,
    query: SecurityEventQuery,
) -> Result<SecurityEventFilter, AppError> {
    let mut violations = Vec::new();
    let mut kinds = Vec::new();
    if let Some(raw) = present(query.kind) {
        for value in raw
            .split(',')
            .map(str::trim)
            .filter(|value| !value.is_empty())
        {
            match SecurityEventKind::parse(value) {
                Some(kind) if !kinds.contains(&kind) => kinds.push(kind),
                Some(_) => {}
                None => violations.push(FieldViolation::new(
                    "kind",
                    format!("unknown security event kind: {value}"),
                )),
            }
        }
    }
    let from = parse_instant(query.from, "from", &mut violations);
    let to = parse_instant(query.to, "to", &mut violations);
    if let (Some(from), Some(to)) = (from, to)
        && from > to
    {
        violations.push(FieldViolation::new("to", "must not be before from"));
    }
    if !violations.is_empty() {
        return Err(AppError::invalid_fields(
            "invalid security event filters",
            violations,
        ));
    }
    Ok(SecurityEventFilter {
        owner,
        kinds,
        from,
        to,
    })
}

fn parse_instant(
    value: Option<String>,
    field: &str,
    violations: &mut Vec<FieldViolation>,
) -> Option<DateTime<Utc>> {
    let value = present(value)?;
    match DateTime::parse_from_rfc3339(&value) {
        Ok(instant) => Some(instant.with_timezone(&Utc)),
        Err(_) => {
            violations.push(FieldViolation::new(
                field,
                "must be an ISO-8601 date-time with time zone",
            ));
            None
        }
    }
}
