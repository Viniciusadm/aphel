use std::sync::{Arc, RwLock};
use std::time::Duration;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::application::access::Authorizer;
use crate::application::audit::{DiffOptions, diff_changes};

use crate::application::secrets::{SecretKey, SecretVault};
use crate::application::security::Clock;
use crate::application::shared::{
    Actor, AuditEntry, AuditLog, DomainEvent, EventPublisher, FieldChange, RecordRef,
    TransactionManager,
};
use crate::config::OutboundHttpConfig;
use crate::domain::Secret;
use crate::domain::access::{AccessScope, Permission};
use crate::errors::{AppError, FieldViolation};

pub const DEFAULT_PUBLIC_BASE_URL: &str = "http://localhost:5173";
pub const DEFAULT_UPLOAD_LIMIT_MB: u64 = 25;
pub const DEFAULT_LOCAL_FILES_PATH: &str = "./storage";
pub const ORGANIZATION_SETTINGS_UPDATED: &str = "organization.settings_updated";
pub const OBJECT_STORE_SECRET_NAME: &str = "storage_secret_access_key";
pub const REDACTED: &str = "[redacted]";
pub const ROOT_PATH: &str = "/";
pub const RESERVED_PREFIX: &str = "/_aphel";
pub const RECOVERY_APP_PATH: &str = "/_aphel/app";
const RESERVED_MOUNT_SEGMENTS: [&str; 3] = ["api", "_aphel", ".well-known"];
const MOUNT_SEGMENT_MAX_LEN: usize = 40;
const UPLOAD_LIMIT_MB_CEILING: u64 = 4096;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct OrganizationConfig {
    pub public_base_url: Option<String>,
    pub max_upload_mb: u64,
    pub storage: StorageSettings,
    pub mail: MailSettings,
    pub api_tokens: ApiTokenLimits,
    pub mounts: MountSettings,
    #[serde(flatten)]
    pub extensions: std::collections::BTreeMap<String, Value>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct MountSettings {
    pub app: AppMount,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct AppMount {
    pub base_path: String,
}

impl Default for AppMount {
    fn default() -> Self {
        Self {
            base_path: ROOT_PATH.to_owned(),
        }
    }
}

impl AppMount {
    pub fn prefix(&self) -> &str {
        self.base_path.trim_end_matches('/')
    }

    pub fn is_root(&self) -> bool {
        self.prefix().is_empty()
    }
}

pub fn normalize_app_base_path(raw: &str) -> Option<String> {
    let trimmed = raw.trim().trim_matches('/');
    if trimmed.is_empty() {
        return Some(ROOT_PATH.to_owned());
    }
    let segment = trimmed.to_ascii_lowercase();
    let valid = segment.len() <= MOUNT_SEGMENT_MAX_LEN
        && !RESERVED_MOUNT_SEGMENTS.contains(&segment.as_str())
        && segment
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || character == '-')
        && !segment.starts_with('-')
        && !segment.ends_with('-');
    valid.then(|| format!("/{segment}"))
}

impl Default for OrganizationConfig {
    fn default() -> Self {
        Self {
            public_base_url: None,
            max_upload_mb: DEFAULT_UPLOAD_LIMIT_MB,
            storage: StorageSettings::default(),
            mail: MailSettings::default(),
            api_tokens: ApiTokenLimits::default(),
            mounts: MountSettings::default(),
            extensions: configuration_defaults(),
        }
    }
}

impl OrganizationConfig {
    pub fn public_base_url(&self) -> &str {
        self.public_base_url
            .as_deref()
            .unwrap_or(DEFAULT_PUBLIC_BASE_URL)
    }

    pub fn app_prefix(&self) -> &str {
        self.mounts.app.prefix()
    }

    pub fn app_base_url(&self) -> String {
        format!(
            "{}{}",
            self.public_base_url().trim_end_matches('/'),
            self.app_prefix()
        )
    }

    pub fn secure_cookies(&self) -> bool {
        self.public_base_url().starts_with("https://")
    }

    pub fn max_upload_bytes(&self) -> u64 {
        self.max_upload_mb.saturating_mul(1024 * 1024)
    }

    pub fn outbound_http_config(&self, base: &OutboundHttpConfig) -> OutboundHttpConfig {
        let mut config = base.clone();
        if let Some(app) = crate::modules::try_app() {
            for policy in
                app.contributions::<dyn crate::application::outbound_http::OutboundHttpPolicy>()
            {
                config = policy.configure(self, &config);
            }
        }
        config
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(default)]
pub struct MailSettings {
    pub host: Option<String>,
    pub port: u16,
    pub from: Option<String>,
    pub username: Option<String>,
    pub security: String,
}
impl Default for MailSettings {
    fn default() -> Self {
        Self {
            host: None,
            port: 587,
            from: None,
            username: None,
            security: "starttls".to_owned(),
        }
    }
}
pub fn mail_secret_key() -> SecretKey {
    SecretKey::organization("smtp_password")
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "driver", rename_all = "snake_case")]
pub enum StorageSettings {
    Local { path: String },
    S3(ObjectStoreSettings),
}

impl Default for StorageSettings {
    fn default() -> Self {
        Self::Local {
            path: DEFAULT_LOCAL_FILES_PATH.to_owned(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct ObjectStoreSettings {
    pub bucket: String,
    pub region: String,
    pub endpoint: Option<String>,
    pub access_key_id: Option<String>,
    pub allow_http: bool,
    pub force_path_style: bool,
}

impl Default for ObjectStoreSettings {
    fn default() -> Self {
        Self {
            bucket: "aphel".to_owned(),
            region: "us-east-1".to_owned(),
            endpoint: None,
            access_key_id: None,
            allow_http: false,
            force_path_style: true,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct ApiTokenLimits {
    pub limit: u32,
    pub window_secs: u64,
}

impl Default for ApiTokenLimits {
    fn default() -> Self {
        Self {
            limit: 600,
            window_secs: 60,
        }
    }
}

impl ApiTokenLimits {
    pub fn window(&self) -> Duration {
        Duration::from_secs(self.window_secs)
    }
}

pub fn normalize_public_base_url(raw: &str) -> Option<String> {
    let value = raw.trim().trim_end_matches('/');
    let rest = value
        .strip_prefix("https://")
        .or_else(|| value.strip_prefix("http://"))?;
    let host = rest.split('/').next().unwrap_or_default();
    let valid = !host.is_empty()
        && !rest.contains(char::is_whitespace)
        && !rest.contains(['?', '#', '@', '\\']);
    valid.then(|| value.to_owned())
}

fn check_range<T: PartialOrd + std::fmt::Display + Copy>(
    value: T,
    min: T,
    max: T,
    field: &str,
    violations: &mut Vec<FieldViolation>,
) {
    if value < min || value > max {
        violations.push(FieldViolation::new(
            field,
            format!("must be between {min} and {max}"),
        ));
    }
}

fn is_http_url(value: &str, allow_http: bool) -> bool {
    let rest = value
        .strip_prefix("https://")
        .or_else(|| value.strip_prefix("http://").filter(|_| allow_http));
    rest.is_some_and(|rest| !rest.is_empty() && !rest.contains(char::is_whitespace))
}

pub fn validate_config(config: &OrganizationConfig) -> Vec<FieldViolation> {
    let mut violations = Vec::new();
    if let Some(url) = &config.public_base_url
        && normalize_public_base_url(url).as_deref() != Some(url.as_str())
    {
        violations.push(FieldViolation::new(
            "public_base_url",
            "must be an http(s) address without path suffixes such as ? or #",
        ));
    }
    if normalize_app_base_path(&config.mounts.app.base_path).as_deref()
        != Some(config.mounts.app.base_path.as_str())
    {
        violations.push(FieldViolation::new(
            "mounts.app.base_path",
            "must be / or a single lowercase segment such as /app (letters, digits and hyphens), other than /api and /_aphel",
        ));
    }
    check_range(
        config.max_upload_mb,
        1,
        UPLOAD_LIMIT_MB_CEILING,
        "max_upload_mb",
        &mut violations,
    );
    match &config.storage {
        StorageSettings::Local { path } => {
            if path.trim().is_empty() {
                violations.push(FieldViolation::new("storage.path", "is required"));
            }
        }
        StorageSettings::S3(settings) => {
            if settings.bucket.trim().is_empty() {
                violations.push(FieldViolation::new("storage.bucket", "is required"));
            }
            if settings.region.trim().is_empty() {
                violations.push(FieldViolation::new("storage.region", "is required"));
            }
            if let Some(endpoint) = &settings.endpoint
                && !is_http_url(endpoint, settings.allow_http)
            {
                violations.push(FieldViolation::new(
                    "storage.endpoint",
                    "must be an https URL (http only when allowed)",
                ));
            }
        }
    }
    check_range(
        config.api_tokens.limit,
        1,
        100_000,
        "api_tokens.limit",
        &mut violations,
    );
    check_range(
        config.api_tokens.window_secs,
        1,
        3600,
        "api_tokens.window_secs",
        &mut violations,
    );
    if config
        .extensions
        .get("mail")
        .and_then(|mail| mail.get("password"))
        .is_some()
    {
        violations.push(FieldViolation::new(
            "mail.password",
            "SMTP passwords must be supplied through SMTP_PASSWORD",
        ));
    }
    if let Some(app) = crate::modules::try_app() {
        for extension in app.contributions::<dyn OrganizationConfigExtension>() {
            violations.extend(extension.validate(&config.extensions));
        }
    }
    if config.mail.port == 0 {
        violations.push(FieldViolation::new(
            "mail.port",
            "must be between 1 and 65535",
        ));
    }
    if !["none", "tls", "starttls"].contains(&config.mail.security.as_str()) {
        violations.push(FieldViolation::new(
            "mail.security",
            "must be none, tls or starttls",
        ));
    }
    if let Some(host) = &config.mail.host
        && (host.trim().is_empty()
            || host.chars().any(char::is_whitespace)
            || host.contains(['/', '@']))
    {
        violations.push(FieldViolation::new(
            "mail.host",
            "must be an SMTP hostname or IP address",
        ));
    }
    if let Some(from) = &config.mail.from
        && crate::domain::Email::new(from.clone()).is_err()
    {
        violations.push(FieldViolation::new(
            "mail.from",
            "must be a valid email address",
        ));
    }
    if config.mail.host.is_some() != config.mail.from.is_some() {
        violations.push(FieldViolation::new(
            "mail.from",
            "host and sender address must be configured together",
        ));
    }
    violations
}

#[async_trait]
pub trait OrganizationConfigSource: Send + Sync {
    async fn version(&self) -> Result<Option<i64>, AppError>;
    async fn load(&self) -> Result<Option<(i64, OrganizationConfig)>, AppError>;
}

type CachedConfig = Option<(i64, Arc<OrganizationConfig>)>;

pub struct OrganizationConfigCatalog {
    source: Option<Arc<dyn OrganizationConfigSource>>,
    cached: RwLock<CachedConfig>,
}

impl OrganizationConfigCatalog {
    pub fn new(source: Arc<dyn OrganizationConfigSource>) -> Self {
        Self {
            source: Some(source),
            cached: RwLock::new(None),
        }
    }

    pub fn defaults_only() -> Self {
        Self {
            source: None,
            cached: RwLock::new(None),
        }
    }

    fn cached(&self) -> CachedConfig {
        self.cached.read().ok().and_then(|cached| cached.clone())
    }

    fn remember(&self, version: i64, config: Arc<OrganizationConfig>) {
        if let Ok(mut cached) = self.cached.write() {
            *cached = Some((version, config));
        }
    }

    pub fn snapshot(&self) -> Arc<OrganizationConfig> {
        self.cached().map(|(_, config)| config).unwrap_or_default()
    }

    pub fn invalidate(&self) {
        if let Ok(mut cached) = self.cached.write()
            && let Some((version, _)) = cached.as_mut()
        {
            *version = i64::MIN;
        }
    }

    pub async fn current(&self) -> Result<Arc<OrganizationConfig>, AppError> {
        let Some(source) = &self.source else {
            return Ok(Arc::default());
        };
        let cached = self.cached();
        let Some(version) = source.version().await? else {
            return Ok(Arc::default());
        };
        if let Some((cached_version, config)) = cached
            && cached_version == version
        {
            return Ok(config);
        }
        let Some((version, config)) = source.load().await? else {
            return Ok(Arc::default());
        };
        let config = Arc::new(config);
        self.remember(version, config.clone());
        Ok(config)
    }

    pub async fn refresh(&self) {
        if let Err(error) = self.current().await {
            tracing::warn!(%error, "failed to refresh the organization settings");
        }
    }
}

#[derive(Clone)]
enum BaseUrlSource {
    Fixed(String),
    Live(Arc<OrganizationConfigCatalog>),
}

#[derive(Clone)]
pub struct PublicBaseUrl(BaseUrlSource);

impl PublicBaseUrl {
    pub fn live(catalog: Arc<OrganizationConfigCatalog>) -> Self {
        Self(BaseUrlSource::Live(catalog))
    }

    pub fn get(&self) -> String {
        match &self.0 {
            BaseUrlSource::Fixed(url) => url.clone(),
            BaseUrlSource::Live(catalog) => catalog.snapshot().public_base_url().to_owned(),
        }
    }

    pub fn app_root(&self) -> String {
        match &self.0 {
            BaseUrlSource::Fixed(url) => url.trim_end_matches('/').to_owned(),
            BaseUrlSource::Live(catalog) => catalog.snapshot().app_base_url(),
        }
    }

    pub fn app_link(&self, path: &str) -> String {
        format!("{}{path}", self.app_root())
    }
}

impl Default for PublicBaseUrl {
    fn default() -> Self {
        Self(BaseUrlSource::Fixed(DEFAULT_PUBLIC_BASE_URL.to_owned()))
    }
}

impl From<String> for PublicBaseUrl {
    fn from(url: String) -> Self {
        Self(BaseUrlSource::Fixed(url))
    }
}

impl From<&str> for PublicBaseUrl {
    fn from(url: &str) -> Self {
        Self(BaseUrlSource::Fixed(url.to_owned()))
    }
}

impl std::fmt::Debug for PublicBaseUrl {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("PublicBaseUrl").field(&self.get()).finish()
    }
}

impl std::fmt::Display for PublicBaseUrl {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.get())
    }
}

impl PartialEq for PublicBaseUrl {
    fn eq(&self, other: &Self) -> bool {
        self.get() == other.get()
    }
}

impl Eq for PublicBaseUrl {}

#[async_trait]
pub trait OrganizationConfigRepository: Send + Sync {
    type Tx: Send;

    async fn find(&self) -> Result<Option<OrganizationConfig>, AppError>;
    async fn find_for_update(
        &self,
        tx: &mut Self::Tx,
    ) -> Result<Option<OrganizationConfig>, AppError>;
    async fn save(&self, tx: &mut Self::Tx, config: &OrganizationConfig) -> Result<(), AppError>;
}

#[async_trait]
pub trait StorageProbe: Send + Sync {
    async fn probe(
        &self,
        settings: &StorageSettings,
        secret: Option<&Secret>,
    ) -> Result<(), String>;
}

#[async_trait]
pub trait MailStatus: Send + Sync {
    async fn configured(&self) -> Result<bool, AppError>;
}

pub type OrganizationConfigTx<P> =
    <<P as OrganizationConfigPorts>::Transactions as TransactionManager>::Tx;

pub trait OrganizationConfigPorts: Send + Sync {
    type Transactions: TransactionManager;
    type Settings: OrganizationConfigRepository<Tx = OrganizationConfigTx<Self>>;
    type Secrets: SecretVault<OrganizationConfigTx<Self>>;
    type Events: EventPublisher<OrganizationConfigTx<Self>>;
    type Audit: AuditLog<OrganizationConfigTx<Self>>;
    type Storage: StorageProbe;
    type Mail: MailStatus;
    type Clock: Clock;

    fn transactions(&self) -> &Self::Transactions;
    fn settings(&self) -> &Self::Settings;
    fn secrets(&self) -> &Self::Secrets;
    fn events(&self) -> &Self::Events;
    fn audit(&self) -> &Self::Audit;
    fn storage(&self) -> &Self::Storage;
    fn mail(&self) -> &Self::Mail;
    fn clock(&self) -> &Self::Clock;
    fn catalog(&self) -> &OrganizationConfigCatalog;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OrganizationConfigView {
    pub config: OrganizationConfig,
    pub storage_secret_configured: bool,
    pub mail_configured: bool,
    pub mail_secret_configured: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SecretChange {
    Keep,
    Set(Secret),
    Clear,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UpdateOrganizationConfig {
    pub public_base_url: Option<String>,
    pub app_base_path: Option<String>,
    pub max_upload_mb: Option<u64>,
    pub storage: Option<StorageSettings>,
    pub storage_secret: SecretChange,
    pub mail: Option<MailSettings>,
    pub mail_secret: SecretChange,
    pub api_tokens: Option<ApiTokenLimits>,
    pub extensions: std::collections::BTreeMap<String, Value>,
    pub ip: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StorageTestOutcome {
    pub ok: bool,
    pub message: Option<String>,
}

pub fn storage_secret_key() -> SecretKey {
    SecretKey::organization(OBJECT_STORE_SECRET_NAME)
}

fn uses_object_store(settings: &StorageSettings) -> bool {
    matches!(settings, StorageSettings::S3(_))
}

fn not_installed() -> AppError {
    AppError::NotFound("the organization is not installed".to_owned())
}

pub struct OrganizationConfigUseCases<P> {
    ports: P,
}

impl<P> OrganizationConfigUseCases<P>
where
    P: OrganizationConfigPorts,
{
    pub fn new(ports: P) -> Self {
        Self { ports }
    }

    async fn view(
        &self,
        scope: &AccessScope,
        mut config: OrganizationConfig,
    ) -> Result<OrganizationConfigView, AppError> {
        for (key, value) in configuration_defaults() {
            config.extensions.entry(key).or_insert(value);
        }
        let storage_secret_configured = self
            .ports
            .secrets()
            .reveal(scope, &storage_secret_key())
            .await
            .ok()
            .flatten()
            .is_some();
        Ok(OrganizationConfigView {
            config,
            storage_secret_configured,
            mail_configured: self.ports.mail().configured().await?,
            mail_secret_configured: self
                .ports
                .secrets()
                .reveal(scope, &mail_secret_key())
                .await?
                .is_some(),
        })
    }

    pub async fn get(&self, scope: &AccessScope) -> Result<OrganizationConfigView, AppError> {
        scope.require(&Permission::SETTINGS_MANAGE)?;
        let config = self
            .ports
            .settings()
            .find()
            .await?
            .ok_or_else(not_installed)?;
        self.view(scope, config).await
    }

    pub async fn update(
        &self,
        scope: &AccessScope,
        command: UpdateOrganizationConfig,
    ) -> Result<OrganizationConfigView, AppError> {
        scope.require(&Permission::SETTINGS_MANAGE)?;
        let now = self.ports.clock().now();
        let transactions = self.ports.transactions();
        let mut tx = transactions.begin().await?;
        let current = self
            .ports
            .settings()
            .find_for_update(&mut tx)
            .await?
            .ok_or_else(not_installed)?;
        let mut next = current.clone();
        if let Some(url) = command.public_base_url {
            next.public_base_url =
                Some(normalize_public_base_url(&url).unwrap_or_else(|| url.trim().to_owned()));
        }
        if let Some(base_path) = command.app_base_path {
            next.mounts.app.base_path =
                normalize_app_base_path(&base_path).unwrap_or_else(|| base_path.trim().to_owned());
            if next.mounts.app != current.mounts.app {
                scope.require(&Permission::SYSTEM_MANAGE)?;
            }
        }
        if let Some(max_upload_mb) = command.max_upload_mb {
            next.max_upload_mb = max_upload_mb;
        }
        if let Some(storage) = command.storage {
            next.storage = storage;
        }
        if let Some(api_tokens) = command.api_tokens {
            next.api_tokens = api_tokens;
        }
        if let Some(mail) = command.mail {
            next.mail = mail;
        }
        next.extensions.extend(command.extensions);
        let violations = validate_config(&next);
        if !violations.is_empty() {
            return Err(AppError::invalid_fields(
                "invalid organization settings",
                violations,
            ));
        }

        let mut changes = diff_changes(&current, &next, &DiffOptions::default())?;
        let secret_key = storage_secret_key();
        match &command.storage_secret {
            SecretChange::Keep => {}
            SecretChange::Set(value) => {
                self.ports
                    .secrets()
                    .store(&mut tx, scope, &secret_key, value)
                    .await?;
                changes.push(redacted_change());
            }
            SecretChange::Clear => {
                self.ports
                    .secrets()
                    .remove(&mut tx, scope, &secret_key)
                    .await?;
                changes.push(redacted_change());
            }
        }
        match &command.mail_secret {
            SecretChange::Keep => {}
            SecretChange::Set(value) => {
                self.ports
                    .secrets()
                    .store(&mut tx, scope, &mail_secret_key(), value)
                    .await?;
                changes.push(FieldChange::new(
                    "mail.password",
                    json!(REDACTED),
                    json!(REDACTED),
                ));
            }
            SecretChange::Clear => {
                self.ports
                    .secrets()
                    .remove(&mut tx, scope, &mail_secret_key())
                    .await?;
                changes.push(FieldChange::new(
                    "mail.password",
                    json!(REDACTED),
                    json!(REDACTED),
                ));
            }
        }
        if changes.is_empty() {
            drop(tx);
            return self.view(scope, current).await;
        }

        self.ports.settings().save(&mut tx, &next).await?;
        let changed: Vec<Value> = changes.iter().map(|change| json!(change.field)).collect();
        let actor = Actor::from_scope(scope);
        let record = RecordRef::organization();
        self.ports
            .events()
            .publish(
                &mut tx,
                DomainEvent::new(actor.clone(), ORGANIZATION_SETTINGS_UPDATED, now)
                    .with_record(record.clone())
                    .with_payload(json!({ "changed": changed })),
            )
            .await?;
        self.ports
            .audit()
            .record(
                &mut tx,
                AuditEntry::new(actor, ORGANIZATION_SETTINGS_UPDATED, now)
                    .with_record(record)
                    .with_changes(changes)
                    .with_ip(command.ip),
            )
            .await?;
        transactions.commit(tx).await?;
        self.ports.catalog().invalidate();
        self.ports.catalog().refresh().await;
        self.view(scope, next).await
    }

    pub async fn test_storage(
        &self,
        scope: &AccessScope,
        settings: StorageSettings,
        secret: Option<Secret>,
    ) -> Result<StorageTestOutcome, AppError> {
        scope.require(&Permission::SETTINGS_MANAGE)?;
        let probe_config = OrganizationConfig {
            storage: settings,
            ..OrganizationConfig::default()
        };
        let violations: Vec<FieldViolation> = validate_config(&probe_config)
            .into_iter()
            .filter(|violation| violation.field.starts_with("storage."))
            .collect();
        if !violations.is_empty() {
            return Err(AppError::invalid_fields(
                "invalid storage settings",
                violations,
            ));
        }
        let secret = match secret {
            Some(secret) => Some(secret),
            None if uses_object_store(&probe_config.storage) => {
                self.ports
                    .secrets()
                    .reveal(scope, &storage_secret_key())
                    .await?
            }
            None => None,
        };
        let outcome = self
            .ports
            .storage()
            .probe(&probe_config.storage, secret.as_ref())
            .await;
        Ok(StorageTestOutcome {
            ok: outcome.is_ok(),
            message: outcome.err(),
        })
    }
}

fn redacted_change() -> FieldChange {
    FieldChange::new(
        "storage.secret_access_key",
        json!(REDACTED),
        json!(REDACTED),
    )
}

pub trait OrganizationConfigExtension: Send + Sync {
    fn defaults(&self) -> (String, Value);
    fn validate(&self, values: &std::collections::BTreeMap<String, Value>) -> Vec<FieldViolation>;
}

fn configuration_defaults() -> std::collections::BTreeMap<String, Value> {
    crate::modules::try_app()
        .map(|app| {
            app.contributions::<dyn OrganizationConfigExtension>()
                .into_iter()
                .map(|source| source.defaults())
                .collect()
        })
        .unwrap_or_default()
}
