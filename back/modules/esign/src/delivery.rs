use async_trait::async_trait;
use chrono::Utc;
use serde_json::{Value, json};
use uuid::Uuid;

use aphel_kernel::Ctx;
use aphel_kernel::application::access::Authorizer;
use aphel_kernel::application::shared::{FieldChange, JobContext, JobHandler};
use aphel_kernel::domain::access::{AccessScope, Permission};
use aphel_kernel::errors::AppError;
use aphel_kernel::infrastructure::deferred_authority::DeferredAuthority;
use aphel_kernel::state::AppState;

use crate::provider::{EnvelopeRequest, EnvelopeSigner, SignatureProvider};
use crate::{service, store};

pub(crate) struct Deliver {
    pub(crate) state: AppState,
}

#[async_trait]
impl JobHandler for Deliver {
    fn kind(&self) -> &str {
        "esign.deliver"
    }

    async fn handle(&self, context: &JobContext, payload: Value) -> Result<(), AppError> {
        let authority = payload
            .get("authority")
            .map(|value| {
                serde_json::from_value::<
                    aphel_kernel::infrastructure::deferred_authority::DeferredAuthority,
                >(value.clone())
            })
            .transpose()
            .map_err(|error| AppError::Validation(error.to_string()))?;
        let delivery = self.deliver(context, payload);
        match authority {
            Some(authority) => authority.run(delivery).await,
            None => delivery.await,
        }
    }
}

impl Deliver {
    async fn deliver(&self, _context: &JobContext, payload: Value) -> Result<(), AppError> {
        let id = payload
            .get("request_id")
            .and_then(Value::as_str)
            .and_then(|value| Uuid::parse_str(value).ok())
            .ok_or_else(|| {
                AppError::Validation("esign.deliver requires a request id".to_owned())
            })?;
        let system = AccessScope::system();
        let ctx = Ctx::from_state(&self.state, &system).await?;
        let cancel = payload
            .get("cancel")
            .and_then(Value::as_bool)
            .unwrap_or(false);
        let Some(request) = store::find_request(ctx.pool(), id).await? else {
            return Ok(());
        };
        let expected = if cancel { "cancel_pending" } else { "queued" };
        if request.status != expected {
            if matches!(
                request.status.as_str(),
                "sending" | "cancelling" | "delivery_unknown"
            ) {
                return Err(AppError::conflict_code(
                    "esign_delivery_unknown",
                    "the previous provider call has an unknown outcome; reconcile it with the provider before retrying",
                ));
            }
            return Ok(());
        }
        let authority: DeferredAuthority = serde_json::from_value(payload["authority"].clone())
            .map_err(|error| AppError::Validation(error.to_string()))?;
        let result = async {
            let scope = authority.resolve(ctx.pool()).await?;
            if !scope.module_enabled(crate::KEY) {
                return Err(aphel_kernel::modules::module_disabled());
            }
            scope.require(&crate::SEND)?;
            scope.require(&Permission::DEALS_READ)?;
            let ctx = ctx.with_scope(&scope);
            if cancel {
                reserve(&ctx, id, expected, "delivery_unknown").await?;
                service::deliver_cancel(&ctx, &scope, id).await.map(drop)
            } else {
                send(&ctx, id, &request, &payload).await
            }
        }
        .await;
        if let Err(error) = &result {
            let mut tx = ctx.begin().await?;
            if let Some(current) = store::lock_request(&mut tx, id).await?
                && matches!(
                    current.status.as_str(),
                    "queued" | "sending" | "cancel_pending" | "cancelling" | "delivery_unknown"
                )
            {
                let status = if matches!(
                    current.status.as_str(),
                    "sending" | "cancelling" | "delivery_unknown"
                ) {
                    "delivery_unknown"
                } else {
                    "failed"
                };
                store::set_request_status(&mut tx, id, status, Utc::now()).await?;
                let signers = store::signers_in_tx(&mut tx, id).await?;
                service::record_event(
                    &ctx,
                    &mut tx,
                    &system,
                    crate::FAILED,
                    current.deal_id,
                    service::event_payload(
                        &current,
                        &signers,
                        json!({ "status": status, "requested_by": current.created_by }),
                    ),
                    vec![FieldChange::new(
                        "status",
                        json!(current.status),
                        json!(status),
                    )],
                )
                .await?;
            }
            tx.commit().await?;
            tracing::warn!(%error, request_id = %id, "signature delivery failed");
        }
        result
    }
}

async fn reserve(ctx: &Ctx<'_>, id: Uuid, expected: &str, next: &str) -> Result<(), AppError> {
    let mut tx = ctx.begin().await?;
    let current = store::lock_request(&mut tx, id)
        .await?
        .ok_or_else(service::request_not_found)?;
    if current.status != expected {
        return Err(AppError::conflict_code(
            "esign_delivery_claimed",
            "the signature request is no longer pending",
        ));
    }
    store::set_request_status(&mut tx, id, next, Utc::now()).await?;
    tx.commit().await?;
    Ok(())
}

async fn send(
    ctx: &Ctx<'_>,
    id: Uuid,
    request: &store::RequestRow,
    payload: &Value,
) -> Result<(), AppError> {
    let deal = service::load_deal(ctx, ctx.scope, request.deal_id).await?;
    service::ensure_attached(ctx, ctx.scope, request.deal_id, request.file_id).await?;
    if let Some(quote_id) = payload.get("quote_id").and_then(Value::as_str) {
        let quote_id = Uuid::parse_str(quote_id)
            .map_err(|_| AppError::Validation("invalid quote id".to_owned()))?;
        service::ensure_quote(ctx, ctx.scope, quote_id, request.deal_id).await?;
    }
    let file = service::read_file(ctx, ctx.scope, request.file_id).await?;
    let provider = service::provider(ctx, &request.environment).await?;
    let mut signers = store::signers_of(ctx.pool(), &[id]).await?;
    let name = format!(
        "{} - {}",
        deal.get("title")
            .and_then(Value::as_str)
            .unwrap_or_default(),
        file.file_name
    );
    reserve(ctx, id, "queued", "delivery_unknown").await?;
    let sent = provider
        .send(&EnvelopeRequest {
            name: name.chars().take(200).collect(),
            file_name: file.file_name,
            content: file.content,
            signers: signers
                .iter()
                .map(|signer| EnvelopeSigner {
                    name: signer.name.clone(),
                    email: signer.email.clone(),
                })
                .collect(),
            message: request.message.clone(),
        })
        .await?;
    let mut tx = ctx.begin().await?;
    let mut current = store::lock_request(&mut tx, id)
        .await?
        .ok_or_else(service::request_not_found)?;
    sqlx::query(
        "UPDATE esign_requests SET provider_envelope_id = ?, provider_document_id = ? WHERE id = ?",
    )
    .bind(&sent.envelope_id)
    .bind(&sent.document_id)
    .bind(id.to_string())
    .execute(&mut *tx)
    .await?;
    for (index, signer) in signers.iter_mut().enumerate() {
        signer.provider_signer_id = sent.signer_ids.get(index).cloned();
        sqlx::query("UPDATE esign_signers SET provider_signer_id = ? WHERE id = ?")
            .bind(&signer.provider_signer_id)
            .bind(signer.id.to_string())
            .execute(&mut *tx)
            .await?;
    }
    store::set_request_status(&mut tx, id, store::STATUS_SENT, Utc::now()).await?;
    current.status = store::STATUS_SENT.to_owned();
    current.provider_envelope_id = sent.envelope_id;
    current.provider_document_id = Some(sent.document_id);
    service::record_event(
        ctx,
        &mut tx,
        ctx.scope,
        crate::SENT,
        current.deal_id,
        service::event_payload(
            &current,
            &signers,
            json!({ "quote_id": payload.get("quote_id"), "requested_by": current.created_by }),
        ),
        vec![FieldChange::new(
            "status",
            json!("queued"),
            json!(store::STATUS_SENT),
        )],
    )
    .await?;
    tx.commit().await?;
    Ok(())
}
