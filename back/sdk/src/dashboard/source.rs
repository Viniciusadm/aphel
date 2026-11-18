use super::{WidgetContext, WidgetPorts};
use crate::{AppError, access::AccessScope};
use aphel_kernel::database::Db;
use async_trait::async_trait;
use serde_json::Value;

#[async_trait]
pub trait DashboardSource: Send + Sync {
    fn key(&self) -> &str;
    fn collection(&self) -> &str;
    fn available(&self, scope: &AccessScope) -> bool;
    async fn data(
        &self,
        context: &WidgetContext<'_, WidgetPorts>,
        settings: &Value,
    ) -> Result<Value, AppError>;
}

pub trait RecentEventExclusion: Send + Sync {
    fn exclude(&self, builder: &mut aphel_kernel::database::SqlBuilder);
}

#[async_trait]
pub trait DashboardEventScope: Send + Sync {
    fn record_type(&self) -> crate::shared::RecordType;
    async fn restrict(
        &self,
        pool: &Db,
        scope: &AccessScope,
        records: &[aphel_kernel::application::query::QueryPlan],
        builder: &mut aphel_kernel::database::SqlBuilder,
    ) -> Result<(), AppError>;
    async fn targets(
        &self,
        pool: &Db,
        ids: &[uuid::Uuid],
    ) -> Result<std::collections::HashMap<uuid::Uuid, crate::shared::RecordRef>, AppError>;
}

#[derive(Debug, Clone)]
pub struct WidgetReferenceSetting {
    pub field: &'static str,
    pub package_field: Option<&'static str>,
    pub kind: crate::ReferenceKind,
}

#[async_trait]
pub trait DashboardReferenceCatalog: Send + Sync {
    fn kind(&self) -> crate::ReferenceKind;
    fn package_keys(&self, sections: &std::collections::BTreeMap<String, Value>) -> Vec<String>;
    async fn keys(
        &self,
        pool: &Db,
    ) -> Result<std::collections::HashMap<uuid::Uuid, String>, AppError>;
}

#[async_trait]
pub trait ActivityScope: Send + Sync {
    async fn followed(
        &self,
        pool: &Db,
        user: &aphel_kernel::domain::UserId,
        records: &[crate::shared::RecordRef],
    ) -> Result<std::collections::HashSet<crate::shared::RecordRef>, AppError>;
}
