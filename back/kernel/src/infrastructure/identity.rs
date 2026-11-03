use sqlx::MySqlPool;

use crate::application::auth::{AuthPorts, AuthSettings};
use crate::application::mail::SharedMailer;
use crate::infrastructure::auth::{Argon2PasswordHasher, Sha256SecureTokens, SystemClock};
use crate::infrastructure::repositories::mysql_access::MySqlAccessRepository;
use crate::infrastructure::repositories::mysql_organization_profile::MySqlOrganizationRepository;
use crate::infrastructure::repositories::mysql_sessions::MySqlSessionRepository;
use crate::infrastructure::repositories::mysql_user_tokens::MySqlUserTokenRepository;
use crate::infrastructure::repositories::mysql_users::MySqlUserRepository;
use crate::infrastructure::security::{MySqlLoginMonitor, mysql_login_monitor};
use crate::infrastructure::two_factor::MySqlTwoFactorUseCases;

pub struct MySqlAuthPorts {
    users: MySqlUserRepository,
    sessions: MySqlSessionRepository,
    tokens: MySqlUserTokenRepository,
    organization_profile: MySqlOrganizationRepository,
    access: MySqlAccessRepository,
    mailer: SharedMailer,
    monitor: MySqlLoginMonitor,
    two_factor: MySqlTwoFactorUseCases,
}

impl MySqlAuthPorts {
    pub(crate) fn new(
        pool: MySqlPool,
        mailer: SharedMailer,
        settings: &AuthSettings,
        two_factor: MySqlTwoFactorUseCases,
    ) -> Self {
        Self {
            two_factor,
            monitor: mysql_login_monitor(pool.clone(), mailer.clone(), settings),
            users: MySqlUserRepository::new(pool.clone()),
            sessions: MySqlSessionRepository::new(pool.clone()),
            tokens: MySqlUserTokenRepository::new(pool.clone()),
            organization_profile: MySqlOrganizationRepository::new(pool.clone()),
            access: MySqlAccessRepository::new(pool),
            mailer,
        }
    }
}

impl AuthPorts for MySqlAuthPorts {
    type Users = MySqlUserRepository;
    type Sessions = MySqlSessionRepository;
    type Tokens = MySqlUserTokenRepository;
    type OrganizationProfile = MySqlOrganizationRepository;
    type Access = MySqlAccessRepository;
    type Hasher = Argon2PasswordHasher;
    type Secrets = Sha256SecureTokens;
    type Mailer = SharedMailer;
    type Clock = SystemClock;
    type Monitor = MySqlLoginMonitor;
    type TwoFactor = MySqlTwoFactorUseCases;

    fn users(&self) -> &Self::Users {
        &self.users
    }

    fn sessions(&self) -> &Self::Sessions {
        &self.sessions
    }

    fn tokens(&self) -> &Self::Tokens {
        &self.tokens
    }

    fn organization_profile(&self) -> &Self::OrganizationProfile {
        &self.organization_profile
    }

    fn access(&self) -> &Self::Access {
        &self.access
    }

    fn hasher(&self) -> &Self::Hasher {
        &Argon2PasswordHasher
    }

    fn secrets(&self) -> &Self::Secrets {
        &Sha256SecureTokens
    }

    fn mailer(&self) -> &Self::Mailer {
        &self.mailer
    }

    fn clock(&self) -> &Self::Clock {
        &SystemClock
    }

    fn monitor(&self) -> &Self::Monitor {
        &self.monitor
    }

    fn two_factor(&self) -> &Self::TwoFactor {
        &self.two_factor
    }
}
