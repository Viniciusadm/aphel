use std::sync::{Arc, Mutex, OnceLock};

use sqlx::MySqlPool;

use crate::api::rate_limit::{RateLimits, SlidingWindowLimiter};
use crate::application::auth::{AuthSettings, AuthUseCases};
use crate::application::custom_objects::{CustomSchema, RecordTypeCatalog};
use crate::application::fields::RecordTypeRegistry;
use crate::application::mail::SharedMailer;
use crate::application::organization_profile::{
    ApiTokenLimits, OrganizationConfig, OrganizationConfigCatalog, PublicBaseUrl,
};
use crate::application::security::{NoopSecurityEventRecorder, SharedSecurityEventRecorder};
use crate::application::shared::SharedFileStorage;
use crate::config::OutboundHttpConfig;
use crate::errors::AppError;
use crate::infrastructure::audit::MySqlAuditLog;
use crate::infrastructure::custom_objects::MySqlCustomSchemaSource;
use crate::infrastructure::events::MySqlEventPublisher;
use crate::infrastructure::identity::MySqlAuthPorts;
use crate::infrastructure::jobs::MySqlJobQueue;
use crate::infrastructure::mail::{MemoryMailer, SmtpMailerProvider};
use crate::infrastructure::noop::{
    NoopAuditLog, NoopEventPublisher, NoopJobQueue, UnavailableFileStorage, UnavailableSecretVault,
};
use crate::infrastructure::organization_profile::organization_config_catalog;
use crate::infrastructure::outbound_http::OutboundHttpNetwork;
pub use crate::infrastructure::platform::{
    SharedAuditLog, SharedEventPublisher, SharedJobQueue, SharedSecretVault,
};
use crate::infrastructure::secrets::EncryptionKeyring;
use crate::infrastructure::security::MySqlSecurityEventRecorder;
use crate::infrastructure::storage::DynamicFileStorage;
use crate::infrastructure::transactions::MySqlTransactionManager;
use crate::infrastructure::two_factor::{MySqlTwoFactorPorts, MySqlTwoFactorUseCases};

type CachedOutboundNetwork = (OutboundHttpConfig, Arc<OutboundHttpNetwork>);
type CachedApiTokenLimiter = (ApiTokenLimits, Arc<SlidingWindowLimiter>);

#[derive(Clone)]
pub struct AppState {
    pub db: Option<MySqlPool>,
    pub app: Arc<crate::modules::App>,
    pub auth_settings: Arc<AuthSettings>,
    pub organization_config: Arc<OrganizationConfigCatalog>,
    pub rate_limits: Arc<RateLimits>,
    pub events: SharedEventPublisher,
    pub audit: SharedAuditLog,
    pub jobs: SharedJobQueue,
    pub storage: SharedFileStorage,
    pub secrets: SharedSecretVault,
    pub security_events: SharedSecurityEventRecorder,
    pub encryption: Arc<EncryptionKeyring>,
    pub record_types: Arc<RecordTypeCatalog>,
    pub outbound_http: OutboundHttpConfig,
    mailer: Option<SharedMailer>,
    outbound_http_network: Arc<Mutex<Option<CachedOutboundNetwork>>>,
    api_token_limiter: Arc<Mutex<Option<CachedApiTokenLimiter>>>,
}

impl AppState {
    pub fn new(db: Option<MySqlPool>) -> Self {
        let record_types = match &db {
            Some(pool) => RecordTypeCatalog::new(
                RecordTypeRegistry::builtin(),
                Arc::new(MySqlCustomSchemaSource::new(pool.clone())),
            ),
            None => RecordTypeCatalog::builtin_only(RecordTypeRegistry::builtin()),
        };
        let organization_config = match &db {
            Some(pool) => organization_config_catalog(pool),
            None => Arc::new(OrganizationConfigCatalog::defaults_only()),
        };
        Self {
            db,
            app: crate::modules::app().clone(),
            auth_settings: Arc::new(AuthSettings {
                app_base_url: PublicBaseUrl::live(organization_config.clone()),
                ..AuthSettings::default()
            }),
            organization_config,
            rate_limits: Arc::new(RateLimits::default()),
            events: Arc::new(NoopEventPublisher),
            audit: Arc::new(NoopAuditLog),
            jobs: Arc::new(NoopJobQueue),
            storage: Arc::new(UnavailableFileStorage),
            secrets: Arc::new(UnavailableSecretVault),
            security_events: Arc::new(NoopSecurityEventRecorder),
            encryption: Arc::new(EncryptionKeyring::default()),
            record_types: Arc::new(record_types),
            outbound_http: OutboundHttpConfig::from_env(),
            mailer: None,
            outbound_http_network: Arc::new(Mutex::new(None)),
            api_token_limiter: Arc::new(Mutex::new(None)),
        }
    }

    pub fn with_mysql_platform(self) -> Self {
        let Some(pool) = self.db.clone() else {
            return self;
        };
        self.with_events(Arc::new(MySqlEventPublisher::new()))
            .with_audit(Arc::new(MySqlAuditLog::new()))
            .with_jobs(Arc::new(MySqlJobQueue::new(pool.clone())))
            .with_security_events(Arc::new(MySqlSecurityEventRecorder::new(pool)))
    }

    pub fn with_security_events(mut self, recorder: SharedSecurityEventRecorder) -> Self {
        self.security_events = recorder;
        self
    }

    pub fn with_encryption(mut self, keyring: EncryptionKeyring) -> Self {
        self.encryption = Arc::new(keyring);
        self
    }

    pub fn with_events(mut self, events: SharedEventPublisher) -> Self {
        self.events = events;
        self
    }

    pub fn with_audit(mut self, audit: SharedAuditLog) -> Self {
        self.audit = audit;
        self
    }

    pub fn with_jobs(mut self, jobs: SharedJobQueue) -> Self {
        self.jobs = jobs;
        self
    }

    pub fn with_storage(mut self, storage: SharedFileStorage) -> Self {
        self.storage = storage;
        self
    }

    pub fn with_organization_storage(self) -> Self {
        let storage =
            DynamicFileStorage::shared(self.organization_config.clone(), self.secrets.clone());
        self.with_storage(storage)
    }

    pub fn with_outbound_http(mut self, outbound_http: OutboundHttpConfig) -> Self {
        self.outbound_http = outbound_http;
        self.outbound_http_network = Arc::new(Mutex::new(None));
        self
    }

    pub fn configuration(&self) -> Arc<OrganizationConfig> {
        self.organization_config.snapshot()
    }

    pub fn public_base_url(&self) -> PublicBaseUrl {
        PublicBaseUrl::live(self.organization_config.clone())
    }

    pub fn secure_cookies(&self) -> bool {
        self.configuration().secure_cookies()
    }

    pub fn max_upload_bytes(&self) -> u64 {
        self.configuration().max_upload_bytes()
    }

    pub fn outbound_http_network(&self) -> Result<Arc<OutboundHttpNetwork>, AppError> {
        let config = self
            .configuration()
            .outbound_http_config(&self.outbound_http);
        let mut cached = self
            .outbound_http_network
            .lock()
            .map_err(|_| AppError::Internal("outbound HTTP network lock poisoned".to_owned()))?;
        if let Some((cached_config, network)) = cached.as_ref()
            && *cached_config == config
        {
            return Ok(network.clone());
        }
        let network = Arc::new(OutboundHttpNetwork::new(&config)?);
        *cached = Some((config, network.clone()));
        Ok(network)
    }

    pub fn api_token_limiter(&self) -> Result<Arc<SlidingWindowLimiter>, AppError> {
        let limits = self.configuration().api_tokens;
        let mut cached = self
            .api_token_limiter
            .lock()
            .map_err(|_| AppError::Internal("rate limiter lock poisoned".to_owned()))?;
        if let Some((cached_limits, limiter)) = cached.as_ref()
            && *cached_limits == limits
        {
            return Ok(limiter.clone());
        }
        let limiter = Arc::new(SlidingWindowLimiter::new(limits.limit, limits.window()));
        *cached = Some((limits, limiter.clone()));
        Ok(limiter)
    }

    pub fn with_secrets(mut self, secrets: SharedSecretVault) -> Self {
        self.secrets = secrets;
        self
    }

    pub fn with_record_catalog(mut self, record_types: Arc<RecordTypeCatalog>) -> Self {
        self.record_types = record_types;
        self
    }

    pub fn with_record_types(mut self, record_types: RecordTypeCatalog) -> Self {
        self.record_types = Arc::new(record_types);
        self
    }

    pub async fn custom_schema(&self) -> Result<Arc<CustomSchema>, AppError> {
        self.record_types.schema().await
    }

    pub async fn registry(&self) -> Result<Arc<RecordTypeRegistry>, AppError> {
        self.record_types.registry().await
    }

    pub fn with_mailer(mut self, mailer: SharedMailer) -> Self {
        self.mailer = Some(mailer);
        self
    }

    pub fn mailer(&self) -> SharedMailer {
        if let Some(mailer) = &self.mailer {
            return mailer.clone();
        }
        match &self.db {
            Some(pool) => SmtpMailerProvider::shared(pool.clone(), self.secrets.clone()),
            None => Arc::new(MemoryMailer::new()),
        }
    }

    pub fn with_auth_settings(mut self, settings: AuthSettings) -> Self {
        self.auth_settings = Arc::new(settings);
        self
    }

    pub fn db(&self) -> Result<MySqlPool, AppError> {
        self.db
            .clone()
            .ok_or_else(|| AppError::Configuration("DATABASE_URL is not configured".to_owned()))
    }

    pub fn transactions(&self) -> Result<MySqlTransactionManager, AppError> {
        Ok(MySqlTransactionManager::new(self.db()?))
    }

    pub fn auth_use_cases(&self) -> Result<AuthUseCases<MySqlAuthPorts>, AppError> {
        Ok(AuthUseCases::new(
            MySqlAuthPorts::new(
                self.db()?,
                self.mailer(),
                &self.auth_settings,
                self.two_factor_use_cases()?,
            ),
            self.auth_settings.as_ref().clone(),
        ))
    }

    pub fn two_factor_use_cases(&self) -> Result<MySqlTwoFactorUseCases, AppError> {
        Ok(MySqlTwoFactorUseCases::new(MySqlTwoFactorPorts::new(
            self.db()?,
            self.encryption.clone(),
            self.security_events.clone(),
            self.events.clone(),
            self.audit.clone(),
        )))
    }
}

static RUNTIME_STATE: OnceLock<AppState> = OnceLock::new();

pub fn install_runtime_state(state: AppState) {
    if crate::tenancy::is_multi() {
        tracing::error!(
            "a process-wide runtime state cannot be installed when serving many tenants"
        );
        return;
    }
    if RUNTIME_STATE.set(state).is_err() {
        tracing::debug!("runtime state already installed");
    }
}

pub fn runtime_pool() -> Result<MySqlPool, AppError> {
    runtime_state()?.db()
}

pub fn runtime_state() -> Result<AppState, AppError> {
    if let Some(state) = crate::tenancy::current_state() {
        return Ok(state);
    }
    if crate::tenancy::is_multi() {
        return Err(crate::tenancy::outside_scope());
    }
    RUNTIME_STATE
        .get()
        .cloned()
        .ok_or_else(|| AppError::Configuration("the runtime state is not installed".to_owned()))
}

#[derive(Clone)]
pub struct Runtime {
    state: AppState,
}

impl AsRef<AppState> for AppState {
    fn as_ref(&self) -> &AppState {
        self
    }
}

impl AsRef<AppState> for Runtime {
    fn as_ref(&self) -> &AppState {
        &self.state
    }
}

impl From<AppState> for Runtime {
    fn from(state: AppState) -> Self {
        Self { state }
    }
}

impl Runtime {
    pub async fn ctx<'s>(
        &self,
        scope: &'s crate::domain::access::AccessScope,
    ) -> Result<crate::Ctx<'s>, AppError> {
        crate::Ctx::from_state(&self.state, scope).await
    }

    pub fn db(&self) -> Result<MySqlPool, AppError> {
        self.state.db()
    }

    pub async fn registry(&self) -> Result<Arc<RecordTypeRegistry>, AppError> {
        self.state.registry().await
    }

    pub fn record_types(&self) -> &Arc<RecordTypeCatalog> {
        &self.state.record_types
    }

    pub fn app(&self) -> &Arc<crate::modules::App> {
        &self.state.app
    }

    pub fn events(&self) -> &SharedEventPublisher {
        &self.state.events
    }

    pub fn audit(&self) -> &SharedAuditLog {
        &self.state.audit
    }

    pub fn jobs(&self) -> &SharedJobQueue {
        &self.state.jobs
    }

    pub fn storage(&self) -> &SharedFileStorage {
        &self.state.storage
    }

    pub fn secrets(&self) -> &SharedSecretVault {
        &self.state.secrets
    }

    pub fn configuration(&self) -> Arc<OrganizationConfig> {
        self.state.configuration()
    }

    pub fn outbound_http_network(&self) -> Result<Arc<OutboundHttpNetwork>, AppError> {
        self.state.outbound_http_network()
    }

    pub fn public_base_url(&self) -> PublicBaseUrl {
        self.state.public_base_url()
    }

    pub fn max_upload_bytes(&self) -> u64 {
        self.state.max_upload_bytes()
    }

    pub fn mailer(&self) -> SharedMailer {
        self.state.mailer()
    }

    pub fn outbound_http_settings(&self) -> &OutboundHttpConfig {
        &self.state.outbound_http
    }

    pub fn secure_cookies(&self) -> bool {
        self.state.secure_cookies()
    }

    pub fn auth_use_cases(&self) -> Result<AuthUseCases<MySqlAuthPorts>, AppError> {
        self.state.auth_use_cases()
    }

    pub fn with_events(self, events: SharedEventPublisher) -> Self {
        Self {
            state: self.state.with_events(events),
        }
    }
}

pub fn runtime() -> Result<Runtime, AppError> {
    runtime_state().map(Runtime::from)
}
