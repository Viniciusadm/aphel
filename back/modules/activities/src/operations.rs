use async_trait::async_trait;
use serde_json::Map;
use uuid::Uuid;

use crate::domain::CreateActivityCommand;
use crate::{service, types};
use aphel_kernel::Ctx;
use aphel_kernel::application::automations::{TaskOperations, TaskSpec};
use aphel_kernel::application::organization_admin::ListWindow;
use aphel_kernel::errors::AppError;

pub(crate) const DEFAULT_TASK_TYPE: &str = "task";

pub(crate) struct AutomationTasks;

#[async_trait]
impl TaskOperations for AutomationTasks {
    async fn create_task(&self, ctx: &Ctx<'_>, task: TaskSpec) -> Result<Uuid, AppError> {
        let activity_type_id = match task.activity_type_id {
            Some(id) => id,
            None => default_task_type(ctx).await?,
        };
        let command = CreateActivityCommand {
            title: task.title,
            activity_type_id,
            status: None,
            priority: task.priority,
            due_at: task.due_at,
            completed_at: None,
            owner_id: task.owner_id,
            record_type: task
                .record
                .as_ref()
                .map(|record| record.record_type.as_str().to_owned()),
            record_id: task
                .record
                .as_ref()
                .map(|record| record.record_id.to_string()),
            links: Vec::new(),
            duration_minutes: None,
            description: task.description,
            location: None,
            custom_fields: Map::new(),
            ip: None,
        };
        Ok(service::create(ctx, command).await?.activity.id)
    }
}

async fn default_task_type(ctx: &Ctx<'_>) -> Result<String, AppError> {
    let types = types::list(
        ctx,
        false,
        ListWindow {
            offset: 0,
            limit: 200,
        },
    )
    .await?
    .items;
    types
        .iter()
        .find(|activity_type| activity_type.key == DEFAULT_TASK_TYPE)
        .or_else(|| types.first())
        .map(|activity_type| activity_type.id.to_string())
        .ok_or_else(|| AppError::Validation("there is no active activity type".to_owned()))
}

pub(crate) struct ActivityTypeRefs;

#[async_trait]
impl aphel_sdk::points::ReferenceProvider for ActivityTypeRefs {
    fn kind(&self) -> aphel_sdk::ReferenceKind {
        aphel_contract_crm::refs::ACTIVITY_TYPE
    }

    async fn exists(&self, ctx: &Ctx<'_>, id: &str) -> Result<bool, AppError> {
        aphel_sdk::points::lookup_exists(types::get(ctx, id).await)
    }
}
