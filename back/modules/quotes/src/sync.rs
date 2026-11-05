use serde_json::{Value, json};
use uuid::Uuid;

use crate::domain::{QUOTE_DEAL_SYNC_FAILED, QUOTE_DEAL_SYNCED, Quote, QuoteStatus};
use crate::gateway::QuoteGateway;
use crate::pricing::deal_item_from_quote;
use crate::service::{SystemEvent, any, now, record_system_event};
use crate::store::QuoteStore;
use aphel_kernel::Ctx;
use aphel_kernel::errors::{AppError, CodedKind};
use aphel_sdk::platform::domain::access::AccessScope;

const MAX_SYNC_ERROR_LENGTH: usize = 500;

struct SyncFailure {
    reason: &'static str,
    code: Option<String>,
    message: String,
}

impl SyncFailure {
    fn new(reason: &'static str, message: impl Into<String>) -> Self {
        Self {
            reason,
            code: None,
            message: message.into(),
        }
    }

    fn from_error(reason: &'static str, error: &AppError) -> Self {
        let code = match error {
            AppError::Coded { code, .. }
            | AppError::CodedDetails { code, .. }
            | AppError::CodedFields { code, .. } => Some((*code).to_owned()),
            AppError::InvalidFields { .. } | AppError::Validation(_) => {
                Some("validation_error".to_owned())
            }
            _ => None,
        };
        Self {
            reason,
            code,
            message: error.client_message().to_owned(),
        }
    }

    fn text(&self) -> String {
        let text = match &self.code {
            Some(code) => format!("{}: {} ({code})", self.reason, self.message),
            None => format!("{}: {}", self.reason, self.message),
        };
        text.chars().take(MAX_SYNC_ERROR_LENGTH).collect()
    }
}

fn is_business(error: &AppError) -> bool {
    match error {
        AppError::Conflict(_)
        | AppError::Forbidden(_)
        | AppError::NotFound(_)
        | AppError::Validation(_)
        | AppError::InvalidFields { .. }
        | AppError::CodedFields { .. } => true,
        AppError::Coded { kind, .. } | AppError::CodedDetails { kind, .. } => {
            !matches!(kind, CodedKind::TooManyRequests | CodedKind::Unauthorized)
        }
        _ => false,
    }
}

pub(crate) async fn sync_deal(ctx: &Ctx<'_>, quote_id: Uuid) -> Result<(), AppError> {
    let system = AccessScope::system();
    let Ok(quote) = any(&ctx.with_scope(&system), quote_id).await else {
        return Ok(());
    };
    if quote.status != QuoteStatus::Accepted || quote.deal_synced_at.is_some() {
        return Ok(());
    }
    let acting = match quote.owner_id.as_ref().or(quote.created_by.as_ref()) {
        Some(user_id) => QuoteGateway::of(ctx).member_scope(user_id).await?,
        None => None,
    };
    let Some(acting) = acting else {
        let failure = SyncFailure::new(
            "owner_not_member",
            "the quote owner is no longer an active user",
        );
        return finish_sync(&ctx.with_scope(&system), &quote, false, Some(failure)).await;
    };
    match apply_to_deal(&ctx.with_scope(&acting), &quote).await {
        Ok(None) => finish_sync(&ctx.with_scope(&system), &quote, true, None).await,
        Ok(Some(failure)) => {
            finish_sync(&ctx.with_scope(&system), &quote, true, Some(failure)).await
        }
        Err(error) if is_business(&error) => {
            let failure = SyncFailure::from_error("items_failed", &error);
            finish_sync(&ctx.with_scope(&system), &quote, false, Some(failure)).await
        }
        Err(error) => Err(error),
    }
}

async fn apply_to_deal(ctx: &Ctx<'_>, quote: &Quote) -> Result<Option<SyncFailure>, AppError> {
    let scope = ctx.scope;
    let gateway = QuoteGateway::of(ctx);
    let deal = gateway.deal(scope, quote.deal_id).await?;
    if deal.currency != quote.currency {
        return Err(AppError::validation_code(
            "currency_mismatch",
            format!(
                "the deal uses {} but the quote uses {}",
                deal.currency, quote.currency
            ),
        ));
    }
    let items = QuoteStore::of(ctx).items(quote.id).await?;
    let outcome = gateway
        .replace_line_items(
            scope,
            deal.id,
            items
                .iter()
                .map(|item| deal_item_from_quote(item, true))
                .collect(),
        )
        .await?;
    if !outcome.amount_managed {
        gateway.set_deal_amount(scope, deal.id, quote.total).await?;
    }
    let settings = QuoteStore::of(ctx).settings().await?;
    let Some(stage_id) = settings.accept_stages.get(&deal.pipeline_id).copied() else {
        return Ok(None);
    };
    if !deal.is_open || deal.stage_id == stage_id {
        return Ok(None);
    }
    match gateway.move_deal(scope, deal.id, stage_id).await {
        Ok(()) => Ok(None),
        Err(error) if is_business(&error) => {
            Ok(Some(SyncFailure::from_error("stage_move_failed", &error)))
        }
        Err(error) => Err(error),
    }
}

async fn finish_sync(
    ctx: &Ctx<'_>,
    quote: &Quote,
    synced: bool,
    failure: Option<SyncFailure>,
) -> Result<(), AppError> {
    let mut tx = ctx.begin().await?;
    let Some(mut locked) = QuoteStore::of(ctx).lock(&mut tx, quote.id).await? else {
        return Ok(());
    };
    if locked.deal_synced_at.is_some() {
        return Ok(());
    }
    let now = now(ctx);
    if synced {
        locked.deal_synced_at = Some(now);
    }
    locked.deal_sync_error = failure.as_ref().map(SyncFailure::text);
    QuoteStore::of(ctx).update(&mut tx, &locked).await?;
    let requested_by = locked
        .owner_id
        .as_ref()
        .map(|owner| Value::String(owner.as_str().to_owned()))
        .unwrap_or(Value::Null);
    match &failure {
        None => {
            record_system_event(
                ctx,
                &mut tx,
                &locked,
                SystemEvent {
                    event_type: QUOTE_DEAL_SYNCED,
                    extra: json!({ "requested_by": requested_by }),
                    changes: Vec::new(),
                    ip: None,
                },
            )
            .await?
        }
        Some(failure) => {
            record_system_event(
                ctx,
                &mut tx,
                &locked,
                SystemEvent {
                    event_type: QUOTE_DEAL_SYNC_FAILED,
                    extra: json!({
                        "requested_by": requested_by,
                        "reason": failure.reason,
                        "code": failure.code,
                        "message": failure.message,
                        "items_synced": synced,
                    }),
                    changes: Vec::new(),
                    ip: None,
                },
            )
            .await?
        }
    }
    Ok(tx.commit().await?)
}
