use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::{AppError, Ctx};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProvisionMode {
    #[default]
    Seed,
    Managed,
}

impl ProvisionMode {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Seed => "seed",
            Self::Managed => "managed",
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PathConflict {
    #[default]
    Fail,
    Suffix,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ExistingResource {
    pub id: String,
    pub applied: Value,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ProvisionRequest {
    pub pack: String,
    pub key: String,
    pub target: Option<String>,
    pub mode: ProvisionMode,
    pub data: Value,
    pub unique: Option<String>,
    pub conflict: PathConflict,
    pub existing: Option<ExistingResource>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProvisionAction {
    Created,
    Updated,
    Kept,
    UserChanged,
    Removed,
}

impl ProvisionAction {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Created => "created",
            Self::Updated => "updated",
            Self::Kept => "kept",
            Self::UserChanged => "user_changed",
            Self::Removed => "removed_by_user",
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ProvisionOutcome {
    pub action: ProvisionAction,
    pub id: String,
    pub applied: Value,
}

#[async_trait]
pub trait Provisioner: Send + Sync {
    fn kind(&self) -> &'static str;

    async fn apply(
        &self,
        ctx: &Ctx<'_>,
        request: &ProvisionRequest,
    ) -> Result<ProvisionOutcome, AppError>;
}

pub fn provisioner(ctx: &Ctx<'_>, kind: &str) -> Option<std::sync::Arc<dyn Provisioner>> {
    ctx.app()
        .contributions::<dyn Provisioner>()
        .into_iter()
        .find(|provisioner| provisioner.kind() == kind)
}

pub fn suffixed(value: &str, attempt: u32) -> String {
    format!("{value}-{attempt}")
}

pub const MAX_SUFFIX_ATTEMPTS: u32 = 9;
