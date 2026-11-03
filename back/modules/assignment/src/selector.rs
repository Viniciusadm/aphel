use aphel_kernel::domain::UserId;
use aphel_sdk::assignment::AssignmentSelector;
use aphel_sdk::{AppError, Ctx};
use async_trait::async_trait;
use serde_json::{Map, Value};

pub(crate) struct NativeAssignmentSelector;
#[async_trait]
impl AssignmentSelector for NativeAssignmentSelector {
    async fn usable(&self, ctx: &Ctx<'_>, rule: &str, record_type: &str) -> Result<bool, AppError> {
        Ok(crate::service::check(ctx, rule)
            .await?
            .is_some_and(|rule| rule.is_active && rule.record_type == record_type))
    }
    async fn select(
        &self,
        ctx: &Ctx<'_>,
        rule: &str,
        record_type: &str,
        values: &Map<String, Value>,
        source: &str,
        idempotency_key: String,
    ) -> Result<Option<UserId>, AppError> {
        let source = crate::domain::AssignmentSource::parse(source)
            .ok_or_else(|| AppError::Validation("invalid assignment source".into()))?;
        let request = crate::service::AssignRequest {
            rule_id: rule,
            record_type,
            record: None,
            values,
            source,
            idempotency_key: Some(idempotency_key),
        };
        match crate::service::assign(ctx, request, &crate::service::PickOnly).await {
            Ok(outcome) => Ok(outcome.user_id().cloned()),
            Err(AppError::Coded { code, .. }) if code == crate::service::RULE_NOT_FOUND => Ok(None),
            Err(error) => Err(error),
        }
    }
}
