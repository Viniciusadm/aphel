use crate::{AppError, Ctx};
use aphel_kernel::domain::access::RecordScope;
use serde_json::Value;
use std::sync::Arc;

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct CreateObjectCommand {
    pub key: String,
    pub label_singular: String,
    pub label_plural: String,
    pub description: Option<String>,
    pub icon: Option<String>,
    pub color: Option<String>,
    pub owner_enabled: Option<bool>,
    pub ip: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ManagedObject {
    pub entity_definition: Option<Value>,
    pub module_key: String,
    pub permission_scope: Option<RecordScope>,
    pub internal: bool,
}

#[async_trait::async_trait]
pub trait ManagedObjectWriter: Send + Sync {
    async fn create(
        &self,
        ctx: &Ctx<'_>,
        command: CreateObjectCommand,
        managed: ManagedObject,
    ) -> Result<(), AppError>;
}

pub fn managed_object_writer(ctx: &Ctx<'_>) -> Result<Arc<dyn ManagedObjectWriter>, AppError> {
    ctx.services()
        .get::<dyn ManagedObjectWriter>()
        .ok_or_else(|| {
            AppError::unavailable_code(
                "managed_object_writer_unavailable",
                "managed object administration is not enabled",
            )
        })
}
