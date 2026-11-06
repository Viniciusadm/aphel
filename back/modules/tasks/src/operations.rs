use aphel_contract_crm::CrmPermissions;
use aphel_sdk::platform::application::access::Authorizer;
use aphel_sdk::platform::database::receipt::{self, MutationReceipt};
use aphel_sdk::platform::domain::access::Permission;
use async_trait::async_trait;
use serde_json::Map;
use sha2::{Digest, Sha256};
use uuid::Uuid;

use crate::domain::CreateActivityCommand;
use crate::{service, types};
use aphel_contract_automation::application::{TaskOperations, TaskSpec};
use aphel_kernel::Ctx;
use aphel_kernel::errors::AppError;
use aphel_sdk::platform::application::organization_admin::ListWindow;

pub(crate) const DEFAULT_TASK_TYPE: &str = "task";

pub(crate) struct AutomationTasks;

#[async_trait]
impl TaskOperations for AutomationTasks {
    async fn create_task(&self, ctx: &Ctx<'_>, task: TaskSpec) -> Result<Uuid, AppError> {
        ctx.scope.require(&Permission::TASKS_CREATE)?;
        let digest = Sha256::digest(format!(
            "automation:create_task:{}:{}",
            task.run_id, task.step_id
        ));
        let mut bytes = [0u8; 16];
        bytes[..6].copy_from_slice(&task.run_id.as_bytes()[..6]);
        bytes[6..].copy_from_slice(&digest[..10]);
        bytes[6] = (bytes[6] & 0x0f) | 0x70;
        bytes[8] = (bytes[8] & 0x3f) | 0x80;
        let id = Uuid::from_bytes(bytes);
        if receipt::applied(ctx.pool(), id).await? {
            return Ok(id);
        }
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
        let mut prepared = service::prepare_create(ctx, command, Default::default()).await?;
        prepared.id = id;
        let result = MutationReceipt::scope(id, async {
            let mut tx = ctx.begin().await?;
            service::insert_prepared(ctx, &mut tx, &prepared).await?;
            tx.commit().await?;
            Ok::<_, AppError>(id)
        })
        .await;
        match result {
            Ok(id) => Ok(id),
            Err(_) if receipt::applied(ctx.pool(), id).await? => Ok(id),
            Err(error) => Err(error),
        }
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
