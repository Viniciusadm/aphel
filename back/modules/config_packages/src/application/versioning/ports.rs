use async_trait::async_trait;
use serde::Serialize;
use uuid::Uuid;

use crate::application::{ConfigPackagePorts, ConfigPackagesTx};
use aphel_kernel::errors::AppError;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ItemResolution {
    Applied,
    Unchanged,
    KeptLocal,
    Skipped,
}

impl ItemResolution {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Applied => "applied",
            Self::Unchanged => "unchanged",
            Self::KeptLocal => "kept_local",
            Self::Skipped => "skipped",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "applied" => Some(Self::Applied),
            "unchanged" => Some(Self::Unchanged),
            "kept_local" => Some(Self::KeptLocal),
            "skipped" => Some(Self::Skipped),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VersionItem {
    pub item_id: String,
    pub section: String,
    pub kind: String,
    pub key: String,
    pub package_hash: String,
    pub resolution: ItemResolution,
}

pub use aphel_sdk::configuration::versioning::InstalledVersion;

#[async_trait]
pub trait ConfigVersionRepository: Send + Sync {
    type Tx: Send;

    async fn insert(
        &self,
        tx: &mut Self::Tx,
        version: &InstalledVersion,
        items: &[VersionItem],
    ) -> Result<(), AppError>;
    async fn latest(&self, package_id: &str) -> Result<Option<InstalledVersion>, AppError>;
    async fn items(&self, version_id: Uuid) -> Result<Vec<VersionItem>, AppError>;
    async fn list(&self) -> Result<Vec<InstalledVersion>, AppError>;
}

pub trait ConfigVersionPorts: Send + Sync {
    type Packages: ConfigPackagePorts;
    type Versions: ConfigVersionRepository<Tx = ConfigPackagesTx<Self::Packages>>;

    fn packages(&self) -> &Self::Packages;
    fn versions(&self) -> &Self::Versions;
}
