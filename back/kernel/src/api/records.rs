use std::sync::Arc;

use async_trait::async_trait;
use serde_json::Value;
use uuid::Uuid;

use crate::api::dto::custom_objects::{CustomRecordRequest, CustomRecordResponse};
use crate::api::routes::v1::custom_objects::custom_record_use_cases;
use crate::api::routes::v1::field_permissions::redacted;
use crate::application::shared::RecordType;
use crate::database::transaction::MySqlTx;
use crate::errors::AppError;
use crate::records::{
    PendingCommit, RecordCtx, RecordTypeHandler, optional_record, values_request,
};

pub struct CustomRecordHandler {
    record_type: RecordType,
}

impl CustomRecordHandler {
    async fn object_key(&self, ctx: &RecordCtx<'_>) -> Result<String, AppError> {
        Ok(custom_record_use_cases(ctx.state)
            .await?
            .object_by_type(self.record_type.as_str())?
            .key
            .clone())
    }

    async fn response(
        ctx: &RecordCtx<'_>,
        response: CustomRecordResponse,
    ) -> Result<Value, AppError> {
        let record_type = RecordType::parse(response.record_type.as_str())?;
        redacted(ctx.state, ctx.scope, &record_type, response).await
    }
}

#[async_trait]
impl RecordTypeHandler for CustomRecordHandler {
    fn durable_mutations(&self) -> bool {
        true
    }

    fn record_type(&self) -> &str {
        self.record_type.as_str()
    }

    async fn create(&self, ctx: &RecordCtx<'_>, values: Value) -> Result<Value, AppError> {
        let key = self.object_key(ctx).await?;
        let command = values_request::<CustomRecordRequest>(values)?.into_command(None, None);
        let details = custom_record_use_cases(ctx.state)
            .await?
            .create(ctx.scope, &key, command)
            .await?;
        Self::response(ctx, details.into()).await
    }

    async fn update(
        &self,
        ctx: &RecordCtx<'_>,
        id: Uuid,
        values: Value,
    ) -> Result<Value, AppError> {
        let key = self.object_key(ctx).await?;
        let command = values_request::<CustomRecordRequest>(values)?.into_command(None, None);
        let details = custom_record_use_cases(ctx.state)
            .await?
            .update(ctx.scope, &key, &id.to_string(), command)
            .await?;
        Self::response(ctx, details.into()).await
    }

    async fn create_in_tx(
        &self,
        ctx: &RecordCtx<'_>,
        tx: &mut MySqlTx,
        values: Value,
    ) -> Result<PendingCommit, AppError> {
        let key = self.object_key(ctx).await?;
        let command = values_request::<CustomRecordRequest>(values)?.into_command(None, None);
        let (_, pending) = custom_record_use_cases(ctx.state)
            .await?
            .create_in_tx(tx, ctx.scope, &key, command)
            .await?;
        Ok(pending)
    }

    async fn update_in_tx(
        &self,
        ctx: &RecordCtx<'_>,
        tx: &mut MySqlTx,
        id: Uuid,
        values: Value,
    ) -> Result<Option<PendingCommit>, AppError> {
        let key = self.object_key(ctx).await?;
        let command = values_request::<CustomRecordRequest>(values)?.into_command(None, None);
        let (_, pending) = custom_record_use_cases(ctx.state)
            .await?
            .update_in_tx(tx, ctx.scope, &key, &id.to_string(), command)
            .await?;
        Ok(pending)
    }

    async fn delete(&self, ctx: &RecordCtx<'_>, id: Uuid) -> Result<(), AppError> {
        let key = self.object_key(ctx).await?;
        custom_record_use_cases(ctx.state)
            .await?
            .delete(ctx.scope, &key, &id.to_string(), None)
            .await
    }

    async fn load(&self, ctx: &RecordCtx<'_>, id: Uuid) -> Result<Option<Value>, AppError> {
        let use_cases = custom_record_use_cases(ctx.state).await?;
        let Some(key) = optional_record(
            use_cases
                .object_by_type(self.record_type.as_str())
                .map(|object| object.key.clone()),
        )?
        else {
            return Ok(None);
        };
        let Some(details) = optional_record(use_cases.get(ctx.scope, &key, &id.to_string()).await)?
        else {
            return Ok(None);
        };
        Ok(Some(Self::response(ctx, details.into()).await?))
    }

    async fn load_listed(&self, ctx: &RecordCtx<'_>, ids: &[Uuid]) -> Result<Vec<Value>, AppError> {
        let key = self.object_key(ctx).await?;
        let listed = custom_record_use_cases(ctx.state)
            .await?
            .listed(ctx.scope, &key, ids)
            .await?;
        let mut records = Vec::with_capacity(listed.len());
        for details in listed {
            records.push(Self::response(ctx, details.into()).await?);
        }
        Ok(records)
    }

    async fn archive(&self, ctx: &RecordCtx<'_>, id: Uuid) -> Result<(), AppError> {
        let key = self.object_key(ctx).await?;
        custom_record_use_cases(ctx.state)
            .await?
            .set_archived(ctx.scope, &key, &id.to_string(), true, None)
            .await
            .map(|_| ())
    }

    async fn restore(&self, ctx: &RecordCtx<'_>, id: Uuid) -> Result<(), AppError> {
        let key = self.object_key(ctx).await?;
        custom_record_use_cases(ctx.state)
            .await?
            .set_archived(ctx.scope, &key, &id.to_string(), false, None)
            .await
            .map(|_| ())
    }
}

pub fn custom_record_handler(record_type: &RecordType) -> Arc<dyn RecordTypeHandler> {
    Arc::new(CustomRecordHandler {
        record_type: record_type.clone(),
    })
}
