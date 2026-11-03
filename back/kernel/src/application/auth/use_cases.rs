use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::application::access::{AccessRepository, build_access_scope};
use crate::application::auth::two_factor::{
    TWO_FACTOR_ATTEMPTS_EXCEEDED, TwoFactorGate, invalid_code,
};
use crate::application::auth::{
    AcceptInvitationCommand, AccountOverview, AuthPorts, AuthSettings, AuthenticatedRequest,
    ClientMetadata, InvitationPreview, IssuedSession, LoginCommand, LoginOutcome, NewSession,
    NewUserToken, PendingChallenge, PendingTwoFactor, ResetPasswordCommand, SessionRepository,
    SessionStage, UserRepository, UserTokenPurpose, UserTokenRecord, UserTokenRepository,
};
use crate::application::mail::{EmailMessage, MailLocale, Mailer, templates};
use crate::application::organization_profile::OrganizationRepository;
use crate::application::security::{
    Clock, LoginAttempt, LoginFailure, LoginMonitor, PasswordHasher, SecureTokens,
    password_violation,
};
use crate::domain::access::AccessScope;
use crate::domain::users::User;
use crate::domain::users::UserStatus;
use crate::domain::{NonEmptyString, Secret, SessionId, UserId};
use crate::errors::{AppError, FieldViolation};

pub const EMAIL_TAKEN: &str = "email_taken";
pub const INVALID_TOKEN: &str = "invalid_token";
const INVALID_CREDENTIALS: &str = "invalid credentials";
const INVALID_SESSION: &str = "invalid or expired session";
const MAX_NAME_LENGTH: usize = 140;

pub struct AuthUseCases<P> {
    ports: P,
    settings: AuthSettings,
}

impl<P> AuthUseCases<P>
where
    P: AuthPorts,
{
    pub fn new(ports: P, settings: AuthSettings) -> Self {
        Self { ports, settings }
    }

    pub async fn login(&self, command: LoginCommand) -> Result<LoginOutcome, AppError> {
        let attempt = LoginAttempt::new(&command.email, &command.client);
        self.ports.monitor().ensure_allowed(&attempt).await?;
        let Some(user) = self.ports.users().find_by_email(&attempt.email).await? else {
            return self
                .reject_login(&attempt, None, LoginFailure::UnknownAccount)
                .await;
        };
        let password_matches = match user.password_hash.as_ref() {
            Some(password_hash) => self
                .ports
                .hasher()
                .verify_password(command.password.expose(), password_hash.as_str())?,
            None => false,
        };
        if !password_matches {
            return self
                .reject_login(&attempt, Some(&user), LoginFailure::WrongPassword)
                .await;
        }
        if !user.is_active() {
            return self
                .reject_login(&attempt, Some(&user), LoginFailure::InactiveAccount)
                .await;
        }

        self.sign_in(user, command.client).await
    }

    pub async fn sign_in_external(
        &self,
        user_id: &UserId,
        client: ClientMetadata,
    ) -> Result<LoginOutcome, AppError> {
        let user = self
            .ports
            .users()
            .find_by_id(user_id)
            .await?
            .filter(User::is_active)
            .ok_or_else(|| AppError::Unauthorized(INVALID_CREDENTIALS.to_owned()))?;
        self.sign_in(user, client).await
    }

    async fn sign_in(&self, user: User, client: ClientMetadata) -> Result<LoginOutcome, AppError> {
        if self.ports.two_factor().is_enabled(&user.id).await? {
            return self
                .start_pending_session(&user, client)
                .await
                .map(LoginOutcome::TwoFactorRequired);
        }
        self.complete_sign_in(user, client)
            .await
            .map(|session| LoginOutcome::Session(Box::new(session)))
    }

    async fn complete_sign_in(
        &self,
        user: User,
        client: ClientMetadata,
    ) -> Result<IssuedSession, AppError> {
        let session = self.start_session(user, client.clone()).await?;
        if let Err(error) = self
            .ports
            .monitor()
            .succeeded(&session.account.user, &session.session_id, &client)
            .await
        {
            tracing::warn!(%error, "failed to record successful login");
        }
        Ok(session)
    }

    async fn start_pending_session(
        &self,
        user: &User,
        client: ClientMetadata,
    ) -> Result<PendingTwoFactor, AppError> {
        let now = self.now();
        let issued = self.ports.secrets().issue()?;
        let expires_at = now + self.settings.two_factor_challenge_ttl;
        self.ports
            .sessions()
            .create(NewSession {
                id: SessionId::new(new_id())?,
                user_id: user.id.clone(),
                token_hash: issued.hash,
                stage: SessionStage::TwoFactorPending,
                ip: client.ip,
                user_agent: client.user_agent,
                created_at: now,
                expires_at,
            })
            .await?;
        Ok(PendingTwoFactor {
            token: issued.token,
            expires_at,
        })
    }

    pub async fn pending_challenge(&self, token: &str) -> Result<PendingChallenge, AppError> {
        let now = self.now();
        let session = self
            .ports
            .sessions()
            .find_by_token_hash(&self.ports.secrets().digest(token))
            .await?
            .filter(|session| {
                session.is_valid_at(now) && session.stage == SessionStage::TwoFactorPending
            })
            .ok_or_else(invalid_session)?;
        let user = self
            .ports
            .users()
            .find_by_id(&session.user_id)
            .await?
            .filter(User::is_active)
            .ok_or_else(invalid_session)?;
        Ok(PendingChallenge { session, user })
    }

    pub async fn complete_two_factor(
        &self,
        pending: PendingChallenge,
        code: &Secret,
        client: ClientMetadata,
    ) -> Result<IssuedSession, AppError> {
        let PendingChallenge { session, user } = pending;
        let attempt = LoginAttempt::new(user.email.as_str(), &client);
        self.ports.monitor().ensure_allowed(&attempt).await?;
        let verified = self
            .ports
            .two_factor()
            .verify_challenge(&user, code.expose(), &client)
            .await?;
        if !verified {
            let attempts = self
                .ports
                .sessions()
                .record_two_factor_failure(&session.id)
                .await?;
            if let Err(error) = self
                .ports
                .monitor()
                .failed(&attempt, Some(&user), LoginFailure::InvalidTwoFactorCode)
                .await
            {
                tracing::warn!(%error, "failed to record failed two-factor challenge");
            }
            if attempts >= self.settings.two_factor_max_attempts {
                self.ports
                    .sessions()
                    .revoke(&session.id, self.now())
                    .await?;
                return Err(AppError::unauthorized_code(
                    TWO_FACTOR_ATTEMPTS_EXCEEDED,
                    "too many invalid codes, sign in again",
                ));
            }
            return Err(invalid_code());
        }

        self.ports
            .sessions()
            .revoke(&session.id, self.now())
            .await?;
        self.complete_sign_in(user, client).await
    }

    async fn reject_login<T>(
        &self,
        attempt: &LoginAttempt,
        user: Option<&User>,
        reason: LoginFailure,
    ) -> Result<T, AppError> {
        if let Err(error) = self.ports.monitor().failed(attempt, user, reason).await {
            tracing::warn!(%error, "failed to record failed login");
        }
        Err(invalid_credentials())
    }

    pub async fn authenticate(&self, token: &str) -> Result<AuthenticatedRequest, AppError> {
        let now = self.now();
        let session = self
            .ports
            .sessions()
            .find_by_token_hash(&self.ports.secrets().digest(token))
            .await?
            .filter(|session| session.is_valid_at(now) && session.stage == SessionStage::Full)
            .ok_or_else(invalid_session)?;
        let user = self
            .ports
            .users()
            .find_by_id(&session.user_id)
            .await?
            .filter(User::is_active)
            .ok_or_else(invalid_session)?;

        if now - session.last_seen_at >= self.settings.session_touch_interval {
            self.ports.sessions().touch(&session.id, now).await?;
        }

        let scope = self
            .load_scope(&user.id)
            .await?
            .ok_or_else(invalid_session)?;
        let two_factor = self.ports.two_factor().status(&user.id, &scope).await?;
        Ok(AuthenticatedRequest {
            session_id: session.id,
            user,
            scope,
            two_factor,
        })
    }

    pub async fn account(
        &self,
        request: AuthenticatedRequest,
    ) -> Result<AccountOverview, AppError> {
        self.overview(request.user, request.scope).await
    }

    pub async fn logout(&self, session_id: &SessionId) -> Result<(), AppError> {
        self.ports.sessions().revoke(session_id, self.now()).await
    }

    pub async fn logout_token(&self, token: &str) -> Result<(), AppError> {
        let now = self.now();
        let session = self
            .ports
            .sessions()
            .find_by_token_hash(&self.ports.secrets().digest(token))
            .await?
            .filter(|session| session.is_valid_at(now))
            .ok_or_else(invalid_session)?;
        self.ports.sessions().revoke(&session.id, now).await
    }

    pub async fn logout_all(&self, user_id: &UserId) -> Result<(), AppError> {
        self.ports
            .sessions()
            .revoke_all_for_user(user_id, self.now())
            .await
            .map(|_| ())
    }

    pub async fn verify_email(&self, token: &Secret) -> Result<(), AppError> {
        let record = self
            .consume_token(UserTokenPurpose::EmailVerification, token)
            .await?;
        self.ports
            .users()
            .mark_email_verified(&record.user_id, self.now())
            .await
    }

    pub async fn resend_email_verification(&self, user: &User) -> Result<(), AppError> {
        if user.is_email_verified() {
            return Ok(());
        }
        self.send_email_verification(user).await
    }

    pub async fn request_password_reset(&self, email: &str) -> Result<(), AppError> {
        let Some(user) = self
            .ports
            .users()
            .find_by_email(&email.trim().to_ascii_lowercase())
            .await?
            .filter(|user| user.is_active() && user.has_password())
        else {
            return Ok(());
        };

        let now = self.now();
        self.ports
            .tokens()
            .invalidate_for_user(&user.id, UserTokenPurpose::PasswordReset, now)
            .await?;
        let token = self
            .issue_user_token(
                &user.id,
                UserTokenPurpose::PasswordReset,
                None,
                now + self.settings.password_reset_ttl,
            )
            .await?;
        let link = self.settings.link("/reset-password", &token);
        self.deliver(templates::password_reset(
            user.email.as_str(),
            user.display_name.as_str(),
            &link,
            MailLocale::resolve(user.locale.as_deref(), None),
        ))
        .await;
        Ok(())
    }

    pub async fn reset_password(&self, command: ResetPasswordCommand) -> Result<(), AppError> {
        if let Some(message) = password_violation(command.password.expose()) {
            return Err(AppError::invalid_fields(
                "invalid password",
                vec![FieldViolation::new("password", message)],
            ));
        }
        let record = self
            .consume_token(UserTokenPurpose::PasswordReset, &command.token)
            .await?;
        let now = self.now();
        let password_hash = self
            .ports
            .hasher()
            .hash_password(command.password.expose())?;
        self.ports
            .users()
            .set_password(&record.user_id, &password_hash)
            .await?;
        self.ports
            .users()
            .mark_email_verified(&record.user_id, now)
            .await?;
        self.ports
            .sessions()
            .revoke_all_for_user(&record.user_id, now)
            .await?;
        let user = self.require_user(&record.user_id).await?;
        if let Err(error) = self
            .ports
            .monitor()
            .password_changed(&user, &command.client)
            .await
        {
            tracing::warn!(%error, "failed to record password change");
        }
        Ok(())
    }

    pub async fn invitation_preview(&self, token: &Secret) -> Result<InvitationPreview, AppError> {
        let (_, user) = self.pending_invitation(token).await?;
        let organization = self
            .ports
            .organization_profile()
            .find()
            .await?
            .ok_or_else(invalid_token)?;
        Ok(InvitationPreview {
            organization_name: organization.name.as_str().to_owned(),
            email: user.email.as_str().to_owned(),
            requires_account: !user.has_password(),
        })
    }

    pub async fn accept_invitation(
        &self,
        command: AcceptInvitationCommand,
    ) -> Result<LoginOutcome, AppError> {
        let (record, user) = self.pending_invitation(&command.token).await?;
        let now = self.now();

        match user.password_hash.as_ref() {
            None => {
                let mut violations = Vec::new();
                let display_name = bounded_name(
                    command.display_name.as_deref().unwrap_or_default(),
                    "display_name",
                    &mut violations,
                );
                if let Some(message) = password_violation(command.password.expose()) {
                    violations.push(FieldViolation::new("password", message));
                }
                let (Some(display_name), true) = (display_name, violations.is_empty()) else {
                    return Err(AppError::invalid_fields("invalid account data", violations));
                };
                let password_hash = self
                    .ports
                    .hasher()
                    .hash_password(command.password.expose())?;
                self.ports
                    .users()
                    .complete_account(&user.id, &display_name, &password_hash, now)
                    .await?;
            }
            Some(password_hash) => {
                if !self
                    .ports
                    .hasher()
                    .verify_password(command.password.expose(), password_hash.as_str())?
                {
                    return Err(invalid_credentials());
                }
                if !user.is_email_verified() {
                    self.ports
                        .users()
                        .mark_email_verified(&user.id, now)
                        .await?;
                }
            }
        }

        if !self.ports.tokens().mark_used(&record.id, now).await? {
            return Err(invalid_token());
        }
        self.ports.users().activate(&user.id, now).await?;

        let user = self.require_user(&user.id).await?;
        self.sign_in(user, command.client).await
    }

    async fn pending_invitation(
        &self,
        token: &Secret,
    ) -> Result<(UserTokenRecord, User), AppError> {
        let record = self
            .usable_token(UserTokenPurpose::Invitation, token)
            .await?;
        let user = self
            .ports
            .users()
            .find_by_id(&record.user_id)
            .await?
            .filter(|user| user.status == UserStatus::Invited)
            .ok_or_else(invalid_token)?;
        Ok((record, user))
    }

    async fn start_session(
        &self,
        user: User,
        client: ClientMetadata,
    ) -> Result<IssuedSession, AppError> {
        let now = self.now();
        let issued = self.ports.secrets().issue()?;
        let session_id = SessionId::new(new_id())?;
        let expires_at = now + self.settings.session_ttl;

        self.ports
            .sessions()
            .create(NewSession {
                id: session_id.clone(),
                user_id: user.id.clone(),
                token_hash: issued.hash,
                stage: SessionStage::Full,
                ip: client.ip,
                user_agent: client.user_agent,
                created_at: now,
                expires_at,
            })
            .await?;
        self.ports.users().record_login(&user.id, now).await?;

        let scope = self
            .load_scope(&user.id)
            .await?
            .ok_or_else(invalid_session)?;
        Ok(IssuedSession {
            token: issued.token,
            session_id,
            expires_at,
            account: self.overview(user, scope).await?,
        })
    }

    async fn overview(&self, user: User, scope: AccessScope) -> Result<AccountOverview, AppError> {
        let organization = self.ports.organization_profile().find().await?;
        let two_factor = self.ports.two_factor().status(&user.id, &scope).await?;
        Ok(AccountOverview {
            user,
            organization,
            scope,
            two_factor,
        })
    }

    async fn load_scope(&self, user_id: &UserId) -> Result<Option<AccessScope>, AppError> {
        Ok(self
            .ports
            .access()
            .load_access(user_id)
            .await?
            .map(|access| build_access_scope(user_id.clone(), access)))
    }

    async fn send_email_verification(&self, user: &User) -> Result<(), AppError> {
        let now = self.now();
        self.ports
            .tokens()
            .invalidate_for_user(&user.id, UserTokenPurpose::EmailVerification, now)
            .await?;
        let token = self
            .issue_user_token(
                &user.id,
                UserTokenPurpose::EmailVerification,
                None,
                now + self.settings.email_verification_ttl,
            )
            .await?;
        let link = self.settings.link("/verify-email", &token);
        self.deliver(templates::email_verification(
            user.email.as_str(),
            user.display_name.as_str(),
            &link,
            MailLocale::resolve(user.locale.as_deref(), None),
        ))
        .await;
        Ok(())
    }

    async fn issue_user_token(
        &self,
        user_id: &UserId,
        purpose: UserTokenPurpose,
        payload: Option<serde_json::Value>,
        expires_at: DateTime<Utc>,
    ) -> Result<Secret, AppError> {
        let issued = self.ports.secrets().issue()?;
        self.ports
            .tokens()
            .create(NewUserToken {
                id: new_id(),
                user_id: user_id.clone(),
                purpose,
                token_hash: issued.hash,
                payload,
                expires_at,
            })
            .await?;
        Ok(issued.token)
    }

    async fn usable_token(
        &self,
        purpose: UserTokenPurpose,
        token: &Secret,
    ) -> Result<UserTokenRecord, AppError> {
        let now = self.now();
        self.ports
            .tokens()
            .find_by_hash(purpose, &self.ports.secrets().digest(token.expose()))
            .await?
            .filter(|record| record.is_usable_at(now))
            .ok_or_else(invalid_token)
    }

    async fn consume_token(
        &self,
        purpose: UserTokenPurpose,
        token: &Secret,
    ) -> Result<UserTokenRecord, AppError> {
        let record = self.usable_token(purpose, token).await?;
        if !self
            .ports
            .tokens()
            .mark_used(&record.id, self.now())
            .await?
        {
            return Err(invalid_token());
        }
        Ok(record)
    }

    async fn require_user(&self, user_id: &UserId) -> Result<User, AppError> {
        self.ports
            .users()
            .find_by_id(user_id)
            .await?
            .ok_or_else(|| AppError::Internal("user disappeared after being written".to_owned()))
    }

    async fn deliver(&self, message: EmailMessage) {
        let subject = message.subject.clone();
        if let Err(error) = self.ports.mailer().send(message).await {
            tracing::warn!(%error, %subject, "failed to send email");
        }
    }

    fn now(&self) -> DateTime<Utc> {
        self.ports.clock().now()
    }
}

fn bounded_name(
    value: &str,
    field: &'static str,
    violations: &mut Vec<FieldViolation>,
) -> Option<NonEmptyString> {
    let trimmed = value.trim();
    if trimmed.is_empty() || trimmed.chars().count() > MAX_NAME_LENGTH {
        violations.push(FieldViolation::new(
            field,
            format!("must have between 1 and {MAX_NAME_LENGTH} characters"),
        ));
        return None;
    }
    NonEmptyString::new(trimmed, field).ok()
}

fn new_id() -> String {
    Uuid::now_v7().to_string()
}

fn invalid_credentials() -> AppError {
    AppError::Unauthorized(INVALID_CREDENTIALS.to_owned())
}

fn invalid_session() -> AppError {
    AppError::Unauthorized(INVALID_SESSION.to_owned())
}

fn invalid_token() -> AppError {
    AppError::validation_code(INVALID_TOKEN, "the link is invalid or has expired")
}
