use super::{WidgetContext, WidgetPorts};
use crate::{AppError, access::AccessScope};
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
    fn exclude(&self, builder: &mut sqlx::QueryBuilder<sqlx::MySql>);
}

#[async_trait]
pub trait DashboardEventScope: Send + Sync {
    fn record_type(&self) -> crate::shared::RecordType;
    async fn restrict(
        &self,
        pool: &sqlx::MySqlPool,
        scope: &AccessScope,
        records: &[aphel_kernel::application::query::QueryPlan],
        builder: &mut sqlx::QueryBuilder<sqlx::MySql>,
    ) -> Result<(), AppError>;
    async fn targets(
        &self,
        pool: &sqlx::MySqlPool,
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
        pool: &sqlx::MySqlPool,
    ) -> Result<std::collections::HashMap<uuid::Uuid, String>, AppError>;
}
