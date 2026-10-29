use sqlx::MySqlPool;

use crate::application::mail::SharedMailer;
use crate::application::organization_admin::OrganizationAdminPorts;
use crate::infrastructure::auth::{Sha256SecureTokens, SystemClock};
use crate::infrastructure::platform::{SharedAuditLog, SharedEventPublisher};
use crate::infrastructure::repositories::mysql_members::MySqlMemberRepository;
use crate::infrastructure::repositories::mysql_roles::MySqlRoleRepository;
use crate::infrastructure::repositories::mysql_teams::MySqlTeamRepository;
use crate::infrastructure::transactions::MySqlTransactionManager;

pub struct MySqlOrganizationAdminPorts {
    transactions: MySqlTransactionManager,
    members: MySqlMemberRepository,
    roles: MySqlRoleRepository,
    teams: MySqlTeamRepository,
    mailer: SharedMailer,
    events: SharedEventPublisher,
    audit: SharedAuditLog,
}

impl MySqlOrganizationAdminPorts {
    pub(crate) fn new(
        pool: MySqlPool,
        mailer: SharedMailer,
        events: SharedEventPublisher,
        audit: SharedAuditLog,
    ) -> Self {
        Self {
            transactions: MySqlTransactionManager::new(pool.clone()),
            members: MySqlMemberRepository::new(pool.clone()),
            roles: MySqlRoleRepository::new(pool.clone()),
            teams: MySqlTeamRepository::new(pool),
            mailer,
            events,
            audit,
        }
    }
}

impl OrganizationAdminPorts for MySqlOrganizationAdminPorts {
    type Transactions = MySqlTransactionManager;
    type Members = MySqlMemberRepository;
    type Roles = MySqlRoleRepository;
    type Teams = MySqlTeamRepository;
    type Secrets = Sha256SecureTokens;
    type Mailer = SharedMailer;
    type Clock = SystemClock;
    type Events = SharedEventPublisher;
    type Audit = SharedAuditLog;

    fn transactions(&self) -> &Self::Transactions {
        &self.transactions
    }

    fn members(&self) -> &Self::Members {
        &self.members
    }

    fn roles(&self) -> &Self::Roles {
        &self.roles
    }

    fn teams(&self) -> &Self::Teams {
        &self.teams
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

    fn events(&self) -> &Self::Events {
        &self.events
    }

    fn audit(&self) -> &Self::Audit {
        &self.audit
    }
}
