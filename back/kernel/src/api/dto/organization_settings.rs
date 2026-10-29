use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::api::dto::common::double_option;
use crate::application::organizations::{
    ApiTokenLimits, CatalogMode, CatalogSource, ObjectStoreSettings, OrganizationConfigView,
    SecretChange, StorageSettings, StorageTestOutcome, UpdateOrganizationConfig,
    WebhookDeliverySettings,
};
use crate::domain::Secret;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum StorageDriverDto {
    Local,
    S3,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct StorageSettingsDto {
    pub driver: StorageDriverDto,
    #[schema(example = "./storage")]
    pub path: Option<String>,
    pub bucket: Option<String>,
    pub region: Option<String>,
    pub endpoint: Option<String>,
    pub access_key_id: Option<String>,
    #[serde(default)]
    pub allow_http: bool,
    #[serde(default = "default_true")]
    pub force_path_style: bool,
}

fn default_true() -> bool {
    true
}

fn non_empty(value: Option<String>) -> Option<String> {
    value
        .map(|value| value.trim().to_owned())
        .filter(|value| !value.is_empty())
}

impl From<StorageSettings> for StorageSettingsDto {
    fn from(settings: StorageSettings) -> Self {
        match settings {
            StorageSettings::Local { path } => Self {
                driver: StorageDriverDto::Local,
                path: Some(path),
                bucket: None,
                region: None,
                endpoint: None,
                access_key_id: None,
                allow_http: false,
                force_path_style: true,
            },
            StorageSettings::S3(s3) => Self {
                driver: StorageDriverDto::S3,
                path: None,
                bucket: Some(s3.bucket),
                region: Some(s3.region),
                endpoint: s3.endpoint,
                access_key_id: s3.access_key_id,
                allow_http: s3.allow_http,
                force_path_style: s3.force_path_style,
            },
        }
    }
}

impl From<StorageSettingsDto> for StorageSettings {
    fn from(dto: StorageSettingsDto) -> Self {
        match dto.driver {
            StorageDriverDto::Local => Self::Local {
                path: non_empty(dto.path).unwrap_or_default(),
            },
            StorageDriverDto::S3 => Self::S3(ObjectStoreSettings {
                bucket: non_empty(dto.bucket).unwrap_or_default(),
                region: non_empty(dto.region).unwrap_or_default(),
                endpoint: non_empty(dto.endpoint),
                access_key_id: non_empty(dto.access_key_id),
                allow_http: dto.allow_http,
                force_path_style: dto.force_path_style,
            }),
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, ToSchema)]
pub struct WebhookDeliveryDto {
    pub require_https: bool,
    #[schema(minimum = 1, maximum = 60)]
    pub timeout_secs: u64,
    #[schema(minimum = 1, maximum = 1000)]
    pub disable_after_failures: u32,
    #[schema(minimum = 1, maximum = 365)]
    pub retention_days: u32,
}

impl From<WebhookDeliverySettings> for WebhookDeliveryDto {
    fn from(settings: WebhookDeliverySettings) -> Self {
        Self {
            require_https: settings.require_https,
            timeout_secs: settings.timeout_secs,
            disable_after_failures: settings.disable_after_failures,
            retention_days: settings.retention_days,
        }
    }
}

impl From<WebhookDeliveryDto> for WebhookDeliverySettings {
    fn from(dto: WebhookDeliveryDto) -> Self {
        Self {
            require_https: dto.require_https,
            timeout_secs: dto.timeout_secs,
            disable_after_failures: dto.disable_after_failures,
            retention_days: dto.retention_days,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, ToSchema)]
pub struct ApiTokenLimitsDto {
    #[schema(minimum = 1, maximum = 100000)]
    pub limit: u32,
    #[schema(minimum = 1, maximum = 3600)]
    pub window_secs: u64,
}

impl From<ApiTokenLimits> for ApiTokenLimitsDto {
    fn from(limits: ApiTokenLimits) -> Self {
        Self {
            limit: limits.limit,
            window_secs: limits.window_secs,
        }
    }
}

impl From<ApiTokenLimitsDto> for ApiTokenLimits {
    fn from(dto: ApiTokenLimitsDto) -> Self {
        Self {
            limit: dto.limit,
            window_secs: dto.window_secs,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum CatalogModeDto {
    Builtin,
    Remote,
    Off,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CatalogSourceDto {
    pub mode: CatalogModeDto,
    pub index_url: Option<String>,
    #[serde(default)]
    pub allow_http: bool,
    #[schema(minimum = 1, maximum = 120)]
    pub timeout_secs: u64,
}

impl From<CatalogSource> for CatalogSourceDto {
    fn from(source: CatalogSource) -> Self {
        Self {
            mode: match source.mode {
                CatalogMode::Builtin => CatalogModeDto::Builtin,
                CatalogMode::Remote => CatalogModeDto::Remote,
                CatalogMode::Off => CatalogModeDto::Off,
            },
            index_url: source.index_url,
            allow_http: source.allow_http,
            timeout_secs: source.timeout_secs,
        }
    }
}

impl From<CatalogSourceDto> for CatalogSource {
    fn from(dto: CatalogSourceDto) -> Self {
        Self {
            mode: match dto.mode {
                CatalogModeDto::Builtin => CatalogMode::Builtin,
                CatalogModeDto::Remote => CatalogMode::Remote,
                CatalogModeDto::Off => CatalogMode::Off,
            },
            index_url: non_empty(dto.index_url),
            allow_http: dto.allow_http,
            timeout_secs: dto.timeout_secs,
        }
    }
}

#[derive(Debug, Serialize, ToSchema)]
pub struct OrganizationSettingsResponse {
    #[schema(example = "https://crm.example.com")]
    pub public_base_url: Option<String>,
    pub effective_public_base_url: String,
    pub secure_cookies: bool,
    pub max_upload_mb: u64,
    pub storage: StorageSettingsDto,
    pub storage_secret_configured: bool,
    pub webhooks: WebhookDeliveryDto,
    pub api_tokens: ApiTokenLimitsDto,
    pub catalog: CatalogSourceDto,
    pub mail_configured: bool,
}

impl From<OrganizationConfigView> for OrganizationSettingsResponse {
    fn from(view: OrganizationConfigView) -> Self {
        let config = view.config;
        Self {
            effective_public_base_url: config.public_base_url().to_owned(),
            secure_cookies: config.secure_cookies(),
            public_base_url: config.public_base_url,
            max_upload_mb: config.max_upload_mb,
            storage: config.storage.into(),
            storage_secret_configured: view.storage_secret_configured,
            webhooks: config.webhooks.into(),
            api_tokens: config.api_tokens.into(),
            catalog: config.catalog.into(),
            mail_configured: view.mail_configured,
        }
    }
}

#[derive(Deserialize, ToSchema)]
pub struct UpdateOrganizationSettingsRequest {
    pub public_base_url: Option<String>,
    #[schema(minimum = 1, maximum = 4096)]
    pub max_upload_mb: Option<u64>,
    pub storage: Option<StorageSettingsDto>,
    #[serde(default, deserialize_with = "double_option")]
    #[schema(value_type = Option<String>, nullable = true)]
    pub storage_secret_access_key: Option<Option<String>>,
    pub webhooks: Option<WebhookDeliveryDto>,
    pub api_tokens: Option<ApiTokenLimitsDto>,
    pub catalog: Option<CatalogSourceDto>,
}

impl UpdateOrganizationSettingsRequest {
    pub fn into_command(self, ip: Option<String>) -> UpdateOrganizationConfig {
        let storage_secret = match self.storage_secret_access_key {
            None => SecretChange::Keep,
            Some(None) => SecretChange::Clear,
            Some(Some(value)) if value.is_empty() => SecretChange::Clear,
            Some(Some(value)) => SecretChange::Set(Secret::new(value)),
        };
        UpdateOrganizationConfig {
            public_base_url: self.public_base_url,
            max_upload_mb: self.max_upload_mb,
            storage: self.storage.map(Into::into),
            storage_secret,
            webhooks: self.webhooks.map(Into::into),
            api_tokens: self.api_tokens.map(Into::into),
            catalog: self.catalog.map(Into::into),
            ip,
        }
    }
}

#[derive(Deserialize, ToSchema)]
pub struct StorageTestRequest {
    pub storage: StorageSettingsDto,
    pub secret_access_key: Option<String>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct StorageTestResponse {
    pub ok: bool,
    pub message: Option<String>,
}

impl From<StorageTestOutcome> for StorageTestResponse {
    fn from(outcome: StorageTestOutcome) -> Self {
        Self {
            ok: outcome.ok,
            message: outcome.message,
        }
    }
}
