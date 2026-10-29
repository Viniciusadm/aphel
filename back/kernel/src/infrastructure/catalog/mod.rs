use async_trait::async_trait;
use reqwest::Client;
use reqwest::redirect::Policy;
use rust_embed::Embed;
use sqlx::MySqlPool;

use crate::application::catalog::{
    BUILTIN_CATALOG_SCHEME, BUILTIN_CATALOG_SOURCE, CatalogFetchError, CatalogFetcher,
    CatalogPorts, CatalogSettings, is_allowed_url,
};
use crate::application::config_packages::APHEL_VERSION;
use crate::infrastructure::config_packages::versioning::MySqlConfigVersionRepository;

const MAX_REDIRECTS: usize = 5;
const BUILTIN_INDEX: &[u8] = include_bytes!("../../../../../catalog/index.json");
const BUILTIN_PACKAGES_DIR: &str = "packages/";

#[derive(Embed)]
#[folder = "../../catalog/packages/"]
struct BuiltinPackages;

fn builtin_file(path: &str, max_bytes: usize) -> Result<Vec<u8>, CatalogFetchError> {
    let content = if path == BUILTIN_CATALOG_SOURCE.trim_start_matches(BUILTIN_CATALOG_SCHEME) {
        Some(BUILTIN_INDEX.to_vec())
    } else {
        path.strip_prefix(BUILTIN_PACKAGES_DIR)
            .filter(|name| !name.contains('/'))
            .and_then(BuiltinPackages::get)
            .map(|file| file.data.into_owned())
    };
    let content = content.ok_or_else(|| {
        CatalogFetchError::Unavailable(format!("the built-in catalog has no file {path}"))
    })?;
    if content.len() > max_bytes {
        return Err(CatalogFetchError::TooLarge(max_bytes));
    }
    Ok(content)
}

#[derive(Clone, Default)]
pub(crate) struct HttpCatalogFetcher;

#[async_trait]
impl CatalogFetcher for HttpCatalogFetcher {
    async fn fetch(
        &self,
        url: &str,
        settings: &CatalogSettings,
        max_bytes: usize,
    ) -> Result<Vec<u8>, CatalogFetchError> {
        if let Some(path) = url.strip_prefix(BUILTIN_CATALOG_SCHEME) {
            return builtin_file(path, max_bytes);
        }
        if !is_allowed_url(url, settings.allow_http) {
            return Err(CatalogFetchError::Disallowed(format!(
                "{url} is not allowed: only https URLs are accepted (allow http in the catalog settings for development)"
            )));
        }
        let allow_http = settings.allow_http;
        let client = Client::builder()
            .timeout(settings.timeout)
            .user_agent(format!("Aphel/{APHEL_VERSION}"))
            .redirect(Policy::custom(move |attempt| {
                let next = attempt.url().as_str().to_owned();
                if attempt.previous().len() >= MAX_REDIRECTS {
                    attempt.error("too many redirects")
                } else if !is_allowed_url(&next, allow_http) {
                    attempt.error("redirect to a URL that is not https")
                } else {
                    attempt.follow()
                }
            }))
            .build()
            .map_err(|error| CatalogFetchError::Unavailable(error.to_string()))?;
        let mut response = client
            .get(url)
            .send()
            .await
            .map_err(|error| CatalogFetchError::Unavailable(error.to_string()))?;
        if !response.status().is_success() {
            return Err(CatalogFetchError::Unavailable(format!(
                "{url} answered HTTP {}",
                response.status().as_u16()
            )));
        }
        if response
            .content_length()
            .is_some_and(|length| length > max_bytes as u64)
        {
            return Err(CatalogFetchError::TooLarge(max_bytes));
        }
        let mut body = Vec::new();
        while let Some(chunk) = response
            .chunk()
            .await
            .map_err(|error| CatalogFetchError::Unavailable(error.to_string()))?
        {
            if body.len() + chunk.len() > max_bytes {
                return Err(CatalogFetchError::TooLarge(max_bytes));
            }
            body.extend_from_slice(&chunk);
        }
        Ok(body)
    }
}

pub(crate) struct MySqlCatalogPorts {
    settings: CatalogSettings,
    fetcher: HttpCatalogFetcher,
    versions: MySqlConfigVersionRepository,
}

impl MySqlCatalogPorts {
    pub(crate) fn new(pool: MySqlPool, settings: CatalogSettings) -> Self {
        Self {
            settings,
            fetcher: HttpCatalogFetcher,
            versions: MySqlConfigVersionRepository::new(pool),
        }
    }
}

impl CatalogPorts for MySqlCatalogPorts {
    type Fetcher = HttpCatalogFetcher;
    type Versions = MySqlConfigVersionRepository;

    fn settings(&self) -> &CatalogSettings {
        &self.settings
    }

    fn fetcher(&self) -> &Self::Fetcher {
        &self.fetcher
    }

    fn versions(&self) -> &Self::Versions {
        &self.versions
    }
}
