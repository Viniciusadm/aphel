use aphel_kernel::domain::UserId;
use aphel_sdk::assignment::AssignmentSelector;
use aphel_sdk::{AppError, Ctx};
use async_trait::async_trait;
use serde_json::{Map, Value};

pub(crate) struct NativeAssignmentSelector;
#[async_trait]
impl AssignmentSelector for NativeAssignmentSelector {
    async fn options(
        &self,
        ctx: &Ctx<'_>,
        record_type: &str,
    ) -> Result<Vec<aphel_sdk::assignment::AssignmentRuleOption>, AppError> {
        let definition = ctx.registry().require(record_type)?;
        if !aphel_sdk::records::enabled(ctx, &definition.record_type) {
            return Ok(Vec::new());
        }
        let listing = crate::DistributionStore::of(ctx)
            .list(&crate::RuleListFilter {
                record_type: Some(record_type.to_owned()),
                active: Some(true),
                window: aphel_sdk::platform::application::organization_admin::ListWindow {
                    offset: 0,
                    limit: 100,
                },
            })
            .await?;
        Ok(listing
            .items
            .into_iter()
            .map(|rule| aphel_sdk::assignment::AssignmentRuleOption {
                id: rule.id,
                name: rule.name,
            })
            .collect())
    }

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

    async fn create_assigned(
        &self,
        ctx: &Ctx<'_>,
        request: aphel_sdk::assignment::AssignedCreate<'_>,
    ) -> Result<Option<aphel_sdk::platform::application::shared::RecordRef>, AppError> {
        let source = crate::domain::AssignmentSource::parse(request.source)
            .ok_or_else(|| AppError::Validation("invalid assignment source".into()))?;
        let writer = crate::service::CreateOwner::new(
            ctx,
            request.record_type,
            request.owner_field,
            request.body,
        );
        let result = crate::service::assign(
            ctx,
            crate::service::AssignRequest {
                rule_id: request.rule,
                record_type: request.record_type,
                record: None,
                values: request.values,
                source,
                idempotency_key: Some(request.idempotency_key),
            },
            &writer,
        )
        .await;
        match result {
            Ok(_) => Ok(writer.created()),
            Err(AppError::Coded { code, .. }) if code == crate::service::RULE_NOT_FOUND => Ok(None),
            Err(error) => Err(error),
        }
    }
}
