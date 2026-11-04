use std::time::Duration;

use async_trait::async_trait;

pub const DEFAULT_CATALOG_TIMEOUT: Duration = Duration::from_secs(10);
pub const MAX_CATALOG_INDEX_BYTES: usize = 1024 * 1024;
pub const BUILTIN_CATALOG_SCHEME: &str = "builtin:";
pub const BUILTIN_CATALOG_SOURCE: &str = "builtin:index.json";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CatalogSettings {
    pub index_url: Option<String>,
    pub allow_http: bool,
    pub timeout: Duration,
}

impl Default for CatalogSettings {
    fn default() -> Self {
        Self {
            index_url: None,
            allow_http: false,
            timeout: DEFAULT_CATALOG_TIMEOUT,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CatalogFetchError {
    Disallowed(String),
    TooLarge(usize),
    Unavailable(String),
}

impl std::fmt::Display for CatalogFetchError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Disallowed(message) | Self::Unavailable(message) => f.write_str(message),
            Self::TooLarge(limit) => write!(f, "the response is larger than {limit} bytes"),
        }
    }
}

#[async_trait]
pub trait CatalogFetcher: Send + Sync {
    async fn fetch(
        &self,
        url: &str,
        settings: &CatalogSettings,
        max_bytes: usize,
    ) -> Result<Vec<u8>, CatalogFetchError>;
}

#[async_trait]
pub trait CatalogPorts: Send + Sync {
    type Fetcher: CatalogFetcher;

    fn settings(&self) -> &CatalogSettings;
    fn fetcher(&self) -> &Self::Fetcher;
    async fn installed_versions(
        &self,
        scope: &aphel_sdk::access::AccessScope,
    ) -> Result<Vec<(String, String)>, aphel_sdk::AppError>;
}
