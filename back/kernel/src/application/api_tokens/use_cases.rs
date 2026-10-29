use chrono::{DateTime, Duration, Utc};
use serde_json::{Value, json};
use uuid::Uuid;

use crate::application::access::AccessRepository;
use crate::application::access::{Authorizer, build_access_scope};
use crate::application::api_tokens::{ApiToken, ApiTokenPorts, ApiTokenRepository};
use crate::application::auth::UserRepository;
use crate::application::security::{Clock, SecureTokens};
use crate::application::security::{
    NewSecurityEvent, NoopSecurityEventRecorder, SecurityEventKind, SharedSecurityEventRecorder,
    record_quietly,
};
use crate::application::shared::{
    Actor, AuditEntry, AuditLog, DomainEvent, EventPublisher, FieldChange, RecordRef, RecordType,
    TransactionManager,
};
use crate::domain::Secret;
use crate::domain::access::{AccessScope, Permission, permission_catalog};
use crate::domain::api_tokens::{ApiTokenScope, ApiTokenScopes};
use crate::domain::custom_objects::is_custom_record_type;
use crate::domain::users::User;
use crate::errors::{AppError, FieldViolation};

pub const API_TOKEN_NOT_FOUND: &str = "api_token_not_found";
pub const INVALID_API_TOKEN: &str = "invalid_api_token";
pub const TOO_MANY_API_TOKENS: &str = "too_many_api_tokens";

pub const API_TOKEN_CREATED: &str = "api_token.created";
pub const API_TOKEN_REVOKED: &str = "api_token.revoked";

pub const API_TOKEN_RECORD_TYPE: RecordType = RecordType::from_static("api_token");
pub const TOKEN_PREFIX: &str = "aphel_";
pub const TOKEN_ID_LENGTH: usize = 8;
pub const MAX_NAME_LENGTH: usize = 100;
pub const MAX_USABLE_TOKENS_PER_USER: u64 = 50;
const USE_TOUCH_INTERVAL_SECONDS: i64 = 60;

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct CreateApiTokenCommand {
    pub name: String,
    pub scopes: Vec<String>,
    pub expires_at: Option<DateTime<Utc>>,
    pub ip: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CreatedApiToken {
    pub token: ApiToken,
    pub secret: Secret,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApiTokenAuthentication {
    pub token_id: Uuid,
    pub user: User,
    pub scope: AccessScope,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ApiTokenOwnerFilter {
    Mine,
    All,
}

pub fn invalid_token() -> AppError {
    AppError::unauthorized_code(
        INVALID_API_TOKEN,
        "the API token is invalid, expired or revoked",
    )
}

fn api_token_not_found() -> AppError {
    AppError::not_found_code(API_TOKEN_NOT_FOUND, "API token was not found")
}

fn parse_token_prefix(raw: &str) -> Option<&str> {
    let rest = raw.strip_prefix(TOKEN_PREFIX)?;
    let (prefix, secret) = rest.split_once('_')?;
    let valid = prefix.len() == TOKEN_ID_LENGTH
        && secret.len() >= 32
        && prefix
            .chars()
            .all(|character| character.is_ascii_hexdigit())
        && secret
            .chars()
            .all(|character| character.is_ascii_hexdigit());
    valid.then_some(prefix)
}

fn token_snapshot(token: &ApiToken) -> Vec<FieldChange> {
    vec![
        FieldChange::new("name", Value::Null, json!(token.name)),
        FieldChange::new("prefix", Value::Null, json!(token.token_prefix)),
        FieldChange::new("scopes", Value::Null, json!(token.scopes.to_strings())),
        FieldChange::new(
            "expires_at",
            Value::Null,
            json!(token.expires_at.map(|at| at.to_rfc3339())),
        ),
    ]
}

fn creator_scopes(scope: &AccessScope) -> Vec<ApiTokenScope> {
    let mut scopes: Vec<ApiTokenScope> = permission_catalog()
        .into_iter()
        .map(|definition| definition.permission)
        .chain(scope.permissions().catalog_extension().iter().cloned())
        .chain(
            scope
                .permissions()
                .grants()
                .map(|grant| grant.permission)
                .filter(|permission| {
                    is_custom_record_type(permission.as_str().split('.').next().unwrap_or_default())
                }),
        )
        .filter(|permission| scope.scope_for(permission).is_some())
        .filter_map(|permission| ApiTokenScope::for_permission(&permission))
        .collect();
    scopes.sort();
    scopes.dedup();
    scopes
}

pub struct ApiTokenUseCases<P> {
    ports: P,
    security_events: SharedSecurityEventRecorder,
}

impl<P> ApiTokenUseCases<P>
where
    P: ApiTokenPorts,
{
    pub fn new(ports: P) -> Self {
        Self {
            ports,
            security_events: std::sync::Arc::new(NoopSecurityEventRecorder),
        }
    }

    pub fn with_security_events(mut self, recorder: SharedSecurityEventRecorder) -> Self {
        self.security_events = recorder;
        self
    }

    async fn record_security_event(
        &self,
        kind: SecurityEventKind,
        scope: &AccessScope,
        token: &ApiToken,
        ip: Option<&str>,
        now: DateTime<Utc>,
    ) {
        record_quietly(
            &self.security_events,
            NewSecurityEvent::new(kind, now)
                .user(&token.created_by)
                .client(ip, None)
                .metadata(json!({
                    "token_id": token.id.to_string(),
                    "token_name": token.name,
                    "token_prefix": token.token_prefix,
                    "actor_user_id": scope.user_id().as_str(),
                })),
        )
        .await;
    }

    pub fn available_scopes(&self, scope: &AccessScope) -> Vec<ApiTokenScope> {
        let mut scopes = creator_scopes(scope);
        scopes.sort();
        scopes.dedup();
        scopes
    }

    pub async fn list(
        &self,
        scope: &AccessScope,
        owner: ApiTokenOwnerFilter,
    ) -> Result<Vec<ApiToken>, AppError> {
        let created_by = match owner {
            ApiTokenOwnerFilter::Mine => Some(scope.user_id()),
            ApiTokenOwnerFilter::All => {
                scope.require(&Permission::USERS_MANAGE)?;
                None
            }
        };
        self.ports.tokens().list(created_by).await
    }

    pub async fn create(
        &self,
        scope: &AccessScope,
        command: CreateApiTokenCommand,
    ) -> Result<CreatedApiToken, AppError> {
        let now = self.ports.clock().now();
        let mut violations = Vec::new();
        let name = command.name.trim().to_owned();
        if name.is_empty() || name.chars().count() > MAX_NAME_LENGTH {
            violations.push(FieldViolation::new(
                "name",
                format!("must have between 1 and {MAX_NAME_LENGTH} characters"),
            ));
        }
        let scopes = match ApiTokenScopes::parse(command.scopes.iter().map(String::as_str)) {
            Ok(scopes) => {
                let allowed = creator_scopes(scope);
                if let Some(extra) = scopes.iter().find(|scope| !allowed.contains(scope)) {
                    violations.push(FieldViolation::new(
                        "scopes",
                        format!("you do not have the permissions behind {extra}"),
                    ));
                }
                Some(scopes)
            }
            Err(_) => {
                violations.push(FieldViolation::new(
                    "scopes",
                    "must be a non-empty list of known scopes such as contacts:read",
                ));
                None
            }
        };
        if command
            .expires_at
            .is_some_and(|expires_at| expires_at <= now)
        {
            violations.push(FieldViolation::new("expires_at", "must be in the future"));
        }
        let Some(scopes) = scopes.filter(|_| violations.is_empty()) else {
            return Err(AppError::invalid_fields("invalid API token", violations));
        };
        if self
            .ports
            .tokens()
            .count_usable(scope.user_id(), now)
            .await?
            >= MAX_USABLE_TOKENS_PER_USER
        {
            return Err(AppError::validation_code(
                TOO_MANY_API_TOKENS,
                format!("a user can have at most {MAX_USABLE_TOKENS_PER_USER} active API tokens"),
            ));
        }

        let prefix: String = self
            .ports
            .secrets()
            .issue()?
            .token
            .expose()
            .chars()
            .take(TOKEN_ID_LENGTH)
            .collect();
        let secret = self.ports.secrets().issue()?.token;
        let raw = Secret::new(format!("{TOKEN_PREFIX}{prefix}_{}", secret.expose()));
        let token_hash = self.ports.secrets().digest(raw.expose());
        let token = ApiToken {
            id: Uuid::now_v7(),
            created_by: scope.user_id().clone(),
            created_by_name: None,
            name,
            token_prefix: format!("{TOKEN_PREFIX}{prefix}"),
            scopes,
            expires_at: command.expires_at,
            last_used_at: None,
            last_used_ip: None,
            revoked_at: None,
            created_at: now,
        };

        let mut tx = self.ports.transactions().begin().await?;
        self.ports
            .tokens()
            .insert(&mut tx, &token, &token_hash)
            .await?;
        self.publish(
            &mut tx,
            scope,
            &token,
            API_TOKEN_CREATED,
            token_snapshot(&token),
            command.ip.clone(),
            now,
        )
        .await?;
        self.ports.transactions().commit(tx).await?;
        self.record_security_event(
            SecurityEventKind::ApiTokenCreated,
            scope,
            &token,
            command.ip.as_deref(),
            now,
        )
        .await;
        Ok(CreatedApiToken { token, secret: raw })
    }

    pub async fn revoke(
        &self,
        scope: &AccessScope,
        token_id: &str,
        ip: Option<String>,
    ) -> Result<ApiToken, AppError> {
        let id = Uuid::parse_str(token_id.trim()).map_err(|_| api_token_not_found())?;
        let mut token = self
            .ports
            .tokens()
            .find(id)
            .await?
            .ok_or_else(api_token_not_found)?;
        if &token.created_by != scope.user_id() {
            scope
                .require(&Permission::USERS_MANAGE)
                .map_err(|_| api_token_not_found())?;
        }
        if token.revoked_at.is_some() {
            return Ok(token);
        }
        let now = self.ports.clock().now();
        let mut tx = self.ports.transactions().begin().await?;
        self.ports.tokens().revoke(&mut tx, id, now).await?;
        self.publish(
            &mut tx,
            scope,
            &token,
            API_TOKEN_REVOKED,
            vec![FieldChange::new(
                "revoked_at",
                Value::Null,
                json!(now.to_rfc3339()),
            )],
            ip.clone(),
            now,
        )
        .await?;
        self.ports.transactions().commit(tx).await?;
        self.record_security_event(
            SecurityEventKind::ApiTokenRevoked,
            scope,
            &token,
            ip.as_deref(),
            now,
        )
        .await;
        token.revoked_at = Some(now);
        Ok(token)
    }

    pub async fn authenticate(
        &self,
        raw: &str,
        ip: Option<&str>,
    ) -> Result<ApiTokenAuthentication, AppError> {
        parse_token_prefix(raw).ok_or_else(invalid_token)?;
        let now = self.ports.clock().now();
        let token = self
            .ports
            .tokens()
            .find_by_hash(&self.ports.secrets().digest(raw))
            .await?
            .filter(|token| token.is_usable_at(now))
            .ok_or_else(invalid_token)?;
        let user = self
            .ports
            .users()
            .find_by_id(&token.created_by)
            .await?
            .filter(User::is_active)
            .ok_or_else(invalid_token)?;
        let access = self
            .ports
            .access()
            .load_access(&token.created_by)
            .await?
            .ok_or_else(invalid_token)?;
        let scope = build_access_scope(token.created_by.clone(), access)
            .through_api_token(token.id.to_string(), token.scopes.clone());

        let stale = token.last_used_at.is_none_or(|last_used_at| {
            now - last_used_at >= Duration::seconds(USE_TOUCH_INTERVAL_SECONDS)
        });
        if stale || token.last_used_ip.as_deref() != ip {
            self.ports.tokens().record_use(token.id, now, ip).await?;
        }
        Ok(ApiTokenAuthentication {
            token_id: token.id,
            user,
            scope,
        })
    }

    #[allow(clippy::too_many_arguments)]
    async fn publish(
        &self,
        tx: &mut <P::Transactions as TransactionManager>::Tx,
        scope: &AccessScope,
        token: &ApiToken,
        event_type: &'static str,
        changes: Vec<FieldChange>,
        ip: Option<String>,
        now: DateTime<Utc>,
    ) -> Result<(), AppError> {
        let actor = Actor::from_scope(scope);
        let record = RecordRef::new(API_TOKEN_RECORD_TYPE, token.id);
        self.ports
            .events()
            .publish(
                tx,
                DomainEvent::new(actor.clone(), event_type, now)
                    .with_record(record.clone())
                    .with_payload(json!({
                        "name": token.name,
                        "prefix": token.token_prefix,
                        "scopes": token.scopes.to_strings(),
                        "created_by": token.created_by.as_str(),
                    })),
            )
            .await?;
        self.ports
            .audit()
            .record(
                tx,
                AuditEntry::new(actor, event_type, now)
                    .with_record(record)
                    .with_changes(changes)
                    .with_ip(ip),
            )
            .await
    }
}
