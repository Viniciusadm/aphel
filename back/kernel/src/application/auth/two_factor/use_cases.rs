use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde_json::{Value, json};

use crate::application::access::Authorizer;
use crate::application::auth::ClientMetadata;
use crate::application::auth::two_factor::{
    ConfirmTwoFactorCommand, DisableTwoFactorCommand, OneTimePasswords,
    RegenerateRecoveryCodesCommand, TwoFactorGate, TwoFactorOverview, TwoFactorPolicy,
    TwoFactorPolicyRepository, TwoFactorPolicyView, TwoFactorPorts, TwoFactorRecord,
    TwoFactorRepository, TwoFactorSetup, TwoFactorStatus, UpdateTwoFactorPolicyCommand,
    VerifiedFactor,
};
use crate::application::security::{
    Clock, NewSecurityEvent, PasswordHasher, SecureTokens, SecurityEventKind, SecurityEventRecorder,
};
use crate::application::shared::{
    Actor, AuditEntry, AuditLog, DomainEvent, EventPublisher, FieldChange, RecordRef,
    TransactionManager,
};
use crate::domain::access::{AccessScope, Permission};
use crate::domain::users::User;
use crate::domain::{Secret, UserId};
use crate::errors::{AppError, FieldViolation};

pub const TWO_FACTOR_ALREADY_ENABLED: &str = "two_factor_already_enabled";
pub const TWO_FACTOR_NOT_ENABLED: &str = "two_factor_not_enabled";
pub const TWO_FACTOR_SETUP_NOT_STARTED: &str = "two_factor_setup_not_started";
pub const INVALID_TWO_FACTOR_CODE: &str = "invalid_two_factor_code";
pub const INVALID_PASSWORD: &str = "invalid_current_password";
pub const TWO_FACTOR_SETUP_REQUIRED: &str = "two_factor_setup_required";
pub const TWO_FACTOR_ATTEMPTS_EXCEEDED: &str = "two_factor_attempts_exceeded";
pub const TWO_FACTOR_POLICY_CHANGED: &str = "organization.two_factor_policy_changed";
pub const RECOVERY_CODE_COUNT: usize = 10;
pub const TOTP_DIGITS: usize = 6;
pub const RECOVERY_CODE_LENGTH: usize = 10;
const ISSUER: &str = "Aphel";

pub struct TwoFactorUseCases<P> {
    ports: P,
}

impl<P> TwoFactorUseCases<P>
where
    P: TwoFactorPorts,
{
    pub fn new(ports: P) -> Self {
        Self { ports }
    }

    pub async fn overview(&self, user: &User) -> Result<TwoFactorOverview, AppError> {
        let record = self.ports.store().find(&user.id).await?;
        let enabled = record.as_ref().is_some_and(TwoFactorRecord::is_enabled);
        let recovery_codes_remaining = if enabled {
            self.ports
                .store()
                .remaining_recovery_codes(&user.id)
                .await?
        } else {
            0
        };
        Ok(TwoFactorOverview {
            enabled,
            pending_setup: record.as_ref().is_some_and(|record| !record.is_enabled()),
            enabled_at: record.and_then(|record| record.enabled_at),
            recovery_codes_remaining,
        })
    }

    pub async fn begin_setup(&self, user: &User) -> Result<TwoFactorSetup, AppError> {
        if self.enabled_record(&user.id).await?.is_some() {
            return Err(already_enabled());
        }
        let secret = self.ports.passwords().generate_secret()?;
        if !self
            .ports
            .store()
            .save_pending(&user.id, &secret, self.now())
            .await?
        {
            return Err(already_enabled());
        }
        let otpauth_uri =
            self.ports
                .passwords()
                .provisioning_uri(&secret, user.email.as_str(), ISSUER)?;
        let qr_code_svg = self.ports.passwords().qr_code_svg(&otpauth_uri)?;
        Ok(TwoFactorSetup {
            secret,
            otpauth_uri,
            qr_code_svg,
        })
    }

    pub async fn confirm_setup(
        &self,
        user: &User,
        command: ConfirmTwoFactorCommand,
    ) -> Result<Vec<Secret>, AppError> {
        let record = self
            .ports
            .store()
            .find(&user.id)
            .await?
            .ok_or_else(setup_not_started)?;
        if record.is_enabled() {
            return Err(already_enabled());
        }
        let now = self.now();
        let step = match totp_candidate(command.code.expose()) {
            Some(code) => self
                .ports
                .passwords()
                .matching_step(&record.secret, &code, now)?,
            None => None,
        }
        .ok_or_else(invalid_code)?;

        let codes = self.new_recovery_codes()?;
        let hashes = self.hashes(&codes);
        if !self
            .ports
            .store()
            .enable(&user.id, step, &hashes, now)
            .await?
        {
            return Err(already_enabled());
        }
        self.record(
            SecurityEventKind::TwoFactorEnabled,
            &user.id,
            &command.client,
            json!({}),
        )
        .await;
        Ok(codes)
    }

    pub async fn disable(
        &self,
        user: &User,
        command: DisableTwoFactorCommand,
    ) -> Result<(), AppError> {
        let record = self
            .enabled_record(&user.id)
            .await?
            .ok_or_else(not_enabled)?;
        self.ensure_password(user, &command.password)?;
        let factor = self
            .verify_factor(&record, command.code.expose(), &command.client)
            .await?
            .ok_or_else(invalid_code)?;
        self.ports.store().disable(&user.id).await?;
        self.record(
            SecurityEventKind::TwoFactorDisabled,
            &user.id,
            &command.client,
            json!({ "factor": factor.as_str() }),
        )
        .await;
        Ok(())
    }

    pub async fn regenerate_recovery_codes(
        &self,
        user: &User,
        command: RegenerateRecoveryCodesCommand,
    ) -> Result<Vec<Secret>, AppError> {
        let record = self
            .enabled_record(&user.id)
            .await?
            .ok_or_else(not_enabled)?;
        let password = command
            .password
            .as_ref()
            .filter(|password| !password.expose().is_empty());
        let code = command
            .code
            .as_ref()
            .filter(|code| !code.expose().trim().is_empty());
        match (password, code) {
            (Some(password), _) => self.ensure_password(user, password)?,
            (None, Some(code)) => {
                self.verify_factor(&record, code.expose(), &command.client)
                    .await?
                    .ok_or_else(invalid_code)?;
            }
            (None, None) => {
                return Err(AppError::invalid_fields(
                    "confirm with your password or a two-factor code",
                    vec![
                        FieldViolation::new("password", "password or code is required"),
                        FieldViolation::new("code", "password or code is required"),
                    ],
                ));
            }
        }

        let codes = self.new_recovery_codes()?;
        self.ports
            .store()
            .replace_recovery_codes(&user.id, &self.hashes(&codes), self.now())
            .await?;
        self.record(
            SecurityEventKind::RecoveryCodesRegenerated,
            &user.id,
            &command.client,
            json!({}),
        )
        .await;
        Ok(codes)
    }

    pub async fn policy(&self, scope: &AccessScope) -> Result<TwoFactorPolicyView, AppError> {
        scope.require(&Permission::SETTINGS_MANAGE)?;
        self.policy_view().await
    }

    pub async fn update_policy(
        &self,
        scope: &AccessScope,
        command: UpdateTwoFactorPolicyCommand,
    ) -> Result<TwoFactorPolicyView, AppError> {
        scope.require(&Permission::SETTINGS_MANAGE)?;
        let policy = TwoFactorPolicy::parse(command.policy.trim()).ok_or_else(|| {
            AppError::invalid_fields(
                "invalid two-factor policy",
                vec![FieldViolation::new(
                    "policy",
                    "must be one of none, admins, everyone",
                )],
            )
        })?;
        let current = self.ports.policies().find().await?;
        if current == policy {
            return self.policy_view().await;
        }
        if scope.api_token_id().is_none() && policy.applies_to(scope) {
            let enabled = self.enabled_record(scope.user_id()).await?.is_some();
            if !enabled {
                return Err(AppError::conflict_code(
                    TWO_FACTOR_NOT_ENABLED,
                    "enable two-factor authentication on your account before requiring it",
                ));
            }
        }

        let record = RecordRef::organization();
        let actor = Actor::from_scope(scope);
        let now = self.now();
        let event = DomainEvent::new(actor.clone(), TWO_FACTOR_POLICY_CHANGED, now)
            .with_record(record.clone())
            .with_payload(json!({
                "old_policy": current.as_str(),
                "new_policy": policy.as_str(),
            }));
        let entry = AuditEntry::new(actor, TWO_FACTOR_POLICY_CHANGED, now)
            .with_record(record)
            .with_changes(vec![FieldChange::new(
                "two_factor_policy",
                json!(current.as_str()),
                json!(policy.as_str()),
            )])
            .with_ip(command.client.ip);

        let transactions = self.ports.transactions();
        let mut tx = transactions.begin().await?;
        self.ports
            .policies()
            .save(&mut tx, scope.user_id(), policy, now)
            .await?;
        self.ports.events().publish(&mut tx, event).await?;
        self.ports.audit().record(&mut tx, entry).await?;
        transactions.commit(tx).await?;

        self.policy_view().await
    }

    async fn policy_view(&self) -> Result<TwoFactorPolicyView, AppError> {
        let policy = self.ports.policies().find().await?;
        let members_without_two_factor = self
            .ports
            .policies()
            .members_without_two_factor(policy)
            .await?;
        Ok(TwoFactorPolicyView {
            policy,
            members_without_two_factor,
        })
    }

    async fn enabled_record(&self, user_id: &UserId) -> Result<Option<TwoFactorRecord>, AppError> {
        Ok(self
            .ports
            .store()
            .find(user_id)
            .await?
            .filter(TwoFactorRecord::is_enabled))
    }

    async fn verify_factor(
        &self,
        record: &TwoFactorRecord,
        code: &str,
        client: &ClientMetadata,
    ) -> Result<Option<VerifiedFactor>, AppError> {
        if let Some(code) = totp_candidate(code) {
            let Some(step) =
                self.ports
                    .passwords()
                    .matching_step(&record.secret, &code, self.now())?
            else {
                return Ok(None);
            };
            if record.last_used_step.is_some_and(|last| last >= step) {
                return Ok(None);
            }
            let claimed = self.ports.store().claim_step(&record.user_id, step).await?;
            return Ok(claimed.then_some(VerifiedFactor::Totp));
        }

        let Some(code) = recovery_candidate(code) else {
            return Ok(None);
        };
        let consumed = self
            .ports
            .store()
            .consume_recovery_code(
                &record.user_id,
                &self.ports.digests().digest(&code),
                self.now(),
            )
            .await?;
        if !consumed {
            return Ok(None);
        }
        let remaining = self
            .ports
            .store()
            .remaining_recovery_codes(&record.user_id)
            .await?;
        self.record(
            SecurityEventKind::RecoveryCodeUsed,
            &record.user_id,
            client,
            json!({ "remaining": remaining }),
        )
        .await;
        Ok(Some(VerifiedFactor::RecoveryCode))
    }

    fn ensure_password(&self, user: &User, password: &Secret) -> Result<(), AppError> {
        let matches = match user.password_hash.as_ref() {
            Some(hash) => self
                .ports
                .hasher()
                .verify_password(password.expose(), hash.as_str())?,
            None => false,
        };
        if matches {
            Ok(())
        } else {
            Err(AppError::validation_code(
                INVALID_PASSWORD,
                "the password is incorrect",
            ))
        }
    }

    fn new_recovery_codes(&self) -> Result<Vec<Secret>, AppError> {
        (0..RECOVERY_CODE_COUNT)
            .map(|_| self.ports.passwords().recovery_code())
            .collect()
    }

    fn hashes(&self, codes: &[Secret]) -> Vec<String> {
        codes
            .iter()
            .filter_map(|code| recovery_candidate(code.expose()))
            .map(|code| self.ports.digests().digest(&code))
            .collect()
    }

    async fn record(
        &self,
        kind: SecurityEventKind,
        user_id: &UserId,
        client: &ClientMetadata,
        metadata: Value,
    ) {
        let event = NewSecurityEvent::new(kind, self.now())
            .user(user_id)
            .client(client.ip.as_deref(), client.user_agent.as_deref())
            .metadata(metadata);
        if let Err(error) = self.ports.recorder().record(event).await {
            tracing::warn!(%error, kind = kind.as_str(), "failed to record security event");
        }
    }

    fn now(&self) -> DateTime<Utc> {
        self.ports.clock().now()
    }
}

#[async_trait]
impl<P> TwoFactorGate for TwoFactorUseCases<P>
where
    P: TwoFactorPorts,
{
    async fn status(
        &self,
        user_id: &UserId,
        scope: &AccessScope,
    ) -> Result<TwoFactorStatus, AppError> {
        let requirement = self.ports.store().requirement(user_id).await?;
        Ok(TwoFactorStatus {
            enabled: requirement.enabled,
            required: requirement.policy.applies_to(scope),
        })
    }

    async fn is_enabled(&self, user_id: &UserId) -> Result<bool, AppError> {
        Ok(self.ports.store().requirement(user_id).await?.enabled)
    }

    async fn verify_challenge(
        &self,
        user: &User,
        code: &str,
        client: &ClientMetadata,
    ) -> Result<bool, AppError> {
        let Some(record) = self.enabled_record(&user.id).await? else {
            return Ok(false);
        };
        let verified = self.verify_factor(&record, code, client).await?;
        if verified.is_none() {
            self.record(
                SecurityEventKind::TwoFactorChallengeFailed,
                &user.id,
                client,
                json!({}),
            )
            .await;
        }
        Ok(verified.is_some())
    }
}

fn compact(code: &str) -> String {
    code.chars()
        .filter(|character| !character.is_whitespace() && *character != '-')
        .collect()
}

fn totp_candidate(code: &str) -> Option<String> {
    let code = compact(code);
    (code.len() == TOTP_DIGITS && code.chars().all(|character| character.is_ascii_digit()))
        .then_some(code)
}

fn recovery_candidate(code: &str) -> Option<String> {
    let code = compact(code).to_ascii_lowercase();
    (code.len() == RECOVERY_CODE_LENGTH
        && code
            .chars()
            .all(|character| character.is_ascii_alphanumeric()))
    .then_some(code)
}

fn already_enabled() -> AppError {
    AppError::conflict_code(
        TWO_FACTOR_ALREADY_ENABLED,
        "two-factor authentication is already enabled",
    )
}

fn not_enabled() -> AppError {
    AppError::conflict_code(
        TWO_FACTOR_NOT_ENABLED,
        "two-factor authentication is not enabled",
    )
}

fn setup_not_started() -> AppError {
    AppError::conflict_code(
        TWO_FACTOR_SETUP_NOT_STARTED,
        "start the two-factor setup before confirming it",
    )
}

pub fn invalid_code() -> AppError {
    AppError::validation_code(
        INVALID_TWO_FACTOR_CODE,
        "the two-factor code is invalid or was already used",
    )
}

pub fn two_factor_setup_required() -> AppError {
    AppError::forbidden_code(
        TWO_FACTOR_SETUP_REQUIRED,
        "this organization requires two-factor authentication; enable it in your security settings",
    )
}
