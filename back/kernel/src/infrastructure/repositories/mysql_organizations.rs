use async_trait::async_trait;
use sqlx::{MySqlConnection, MySqlPool, QueryBuilder, Row};

use crate::application::fields::RecordTypeRegistry;
use crate::application::organizations::{
    OrganizationChanges, OrganizationPorts, OrganizationRegistration, OrganizationRepository,
    OrganizationSettingsRepository,
};
use crate::application::shared::{FileStorage, SharedFileStorage};
use crate::database::transaction::MySqlTx;
use crate::domain::organizations::Organization;
use crate::domain::users::UserStatus;
use crate::errors::AppError;
use crate::infrastructure::auth::SystemClock;
use crate::infrastructure::crm::seed_crm_defaults;
use crate::infrastructure::fields::{seed_managed_fields, seed_system_fields};
use crate::infrastructure::platform::{SharedAuditLog, SharedEventPublisher};
use crate::infrastructure::repositories::mapper;
use crate::infrastructure::storage::MySqlFileRepository;
use crate::infrastructure::transactions::MySqlTransactionManager;

#[derive(Clone)]
pub struct MySqlOrganizationRepository {
    pool: MySqlPool,
}

impl MySqlOrganizationRepository {
    pub fn new(pool: MySqlPool) -> Self {
        Self { pool }
    }
}

fn map_owner_conflict(error: sqlx::Error) -> AppError {
    if let sqlx::Error::Database(database_error) = &error
        && database_error.is_unique_violation()
    {
        return AppError::Conflict("user or organization already exists".to_owned());
    }
    AppError::from(error)
}

#[async_trait]
impl OrganizationRepository for MySqlOrganizationRepository {
    async fn find(&self) -> Result<Option<Organization>, AppError> {
        find_organization(&self.pool).await
    }
}

#[async_trait]
impl OrganizationSettingsRepository for MySqlOrganizationRepository {
    type Tx = MySqlTx;

    async fn find(&self) -> Result<Option<Organization>, AppError> {
        find_organization(&self.pool).await
    }

    async fn update(
        &self,
        tx: &mut MySqlTx,
        changes: &OrganizationChanges,
    ) -> Result<(), AppError> {
        sqlx::query(
            "UPDATE organization_settings SET name = ?, primary_color = ?, locale = ?, timezone = ?, currency = ?, logo_file_id = ? WHERE id = 1",
        )
        .bind(changes.name.as_str())
        .bind(changes.primary_color.as_deref())
        .bind(&changes.locale)
        .bind(&changes.timezone)
        .bind(&changes.currency)
        .bind(changes.logo_file_id.as_deref())
        .execute(&mut **tx)
        .await?;
        Ok(())
    }
}

pub async fn find_organization(pool: &MySqlPool) -> Result<Option<Organization>, AppError> {
    let row = sqlx::query(
        "SELECT name, logo_file_id, primary_color, locale, timezone, currency, installed_at, created_at, updated_at \
         FROM organization_settings WHERE id = 1",
    )
    .fetch_optional(pool)
    .await?;

    row.map(|row| {
        Ok(Organization {
            name: mapper::non_empty(row.try_get("name")?, "organization.name")?,
            logo_file_id: row.try_get("logo_file_id")?,
            primary_color: row.try_get("primary_color")?,
            locale: row.try_get("locale")?,
            timezone: row.try_get("timezone")?,
            currency: row.try_get("currency")?,
            installed_at: row
                .try_get::<Option<chrono::NaiveDateTime>, _>("installed_at")?
                .map(mapper::datetime)
                .transpose()?,
            created_at: mapper::datetime(row.try_get("created_at")?)?,
            updated_at: mapper::datetime(row.try_get("updated_at")?)?,
        })
    })
    .transpose()
}

pub async fn insert_registration(
    connection: &mut MySqlConnection,
    registration: &OrganizationRegistration,
    record_types: &RecordTypeRegistry,
) -> Result<(), AppError> {
    sqlx::query(
        "INSERT INTO organization_settings (id, name, locale, timezone, currency, installed_at) \
         VALUES (1, ?, ?, ?, ?, CURRENT_TIMESTAMP(6))",
    )
    .bind(registration.organization.name.as_str())
    .bind(&registration.organization.locale)
    .bind(&registration.organization.timezone)
    .bind(&registration.organization.currency)
    .execute(&mut *connection)
    .await
    .map_err(map_owner_conflict)?;

    for role in &registration.roles {
        sqlx::query("INSERT INTO roles (id, `key`, name, is_system) VALUES (?, ?, ?, ?)")
            .bind(role.id.as_str())
            .bind(role.key.as_str())
            .bind(&role.name)
            .bind(role.is_system)
            .execute(&mut *connection)
            .await?;

        if !role.grants.is_empty() {
            let mut builder =
                QueryBuilder::new("INSERT INTO role_permissions (role_id, permission, scope) ");
            builder.push_values(&role.grants, |mut values, grant| {
                values
                    .push_bind(role.id.as_str())
                    .push_bind(grant.permission.as_str())
                    .push_bind(grant.scope.as_str());
            });
            builder.build().execute(&mut *connection).await?;
        }
    }
    crate::modules::migrations::apply_enabled_grants(&mut *connection, crate::modules::app())
        .await?;

    sqlx::query(
        "INSERT INTO users (id, email, password_hash, display_name, status, role_id, joined_at) VALUES (?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(registration.owner.id.as_str())
    .bind(registration.owner.email.as_str())
    .bind(&registration.owner.password_hash)
    .bind(registration.owner.display_name.as_str())
    .bind(UserStatus::Active.as_str())
    .bind(registration.owner_role_id.as_str())
    .bind(mapper::naive(registration.joined_at))
    .execute(&mut *connection)
    .await
    .map_err(map_owner_conflict)?;

    seed_system_fields(&mut *connection, record_types).await?;
    seed_managed_fields(&mut *connection, record_types).await?;
    seed_crm_defaults(&mut *connection).await?;

    Ok(())
}

pub(crate) struct MySqlOrganizationPorts {
    transactions: MySqlTransactionManager,
    organizations: MySqlOrganizationRepository,
    files: MySqlFileRepository,
    storage: SharedFileStorage,
    events: SharedEventPublisher,
    audit: SharedAuditLog,
    clock: SystemClock,
}

impl MySqlOrganizationPorts {
    pub(crate) fn new(
        pool: MySqlPool,
        storage: SharedFileStorage,
        events: SharedEventPublisher,
        audit: SharedAuditLog,
    ) -> Self {
        Self {
            transactions: MySqlTransactionManager::new(pool.clone()),
            organizations: MySqlOrganizationRepository::new(pool.clone()),
            files: MySqlFileRepository::new(pool),
            storage,
            events,
            audit,
            clock: SystemClock,
        }
    }
}

impl OrganizationPorts for MySqlOrganizationPorts {
    type Transactions = MySqlTransactionManager;
    type Organizations = MySqlOrganizationRepository;
    type Files = MySqlFileRepository;
    type Storage = dyn FileStorage;
    type Events = SharedEventPublisher;
    type Audit = SharedAuditLog;
    type Clock = SystemClock;

    fn transactions(&self) -> &Self::Transactions {
        &self.transactions
    }

    fn organizations(&self) -> &Self::Organizations {
        &self.organizations
    }

    fn files(&self) -> &Self::Files {
        &self.files
    }

    fn storage(&self) -> &Self::Storage {
        self.storage.as_ref()
    }

    fn events(&self) -> &Self::Events {
        &self.events
    }

    fn audit(&self) -> &Self::Audit {
        &self.audit
    }

    fn clock(&self) -> &Self::Clock {
        &self.clock
    }
}
