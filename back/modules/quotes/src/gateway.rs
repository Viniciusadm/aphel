use rust_decimal::Decimal;
use serde_json::{Value, json};
use uuid::Uuid;

use crate::domain::{DealItemsOutcome, QuoteDeal, QuoteEmail};
use crate::hooks::DealItemsWriter;
use crate::store::QuoteStore;
use aphel_kernel::Ctx;
use aphel_kernel::application::access::{AccessRepository, build_access_scope};
use aphel_kernel::application::shared::{RecordRef, RecordType};
use aphel_kernel::database::transaction::MySqlTx;
use aphel_kernel::domain::UserId;
use aphel_kernel::domain::access::AccessScope;
use aphel_kernel::errors::{AppError, CodedKind};
use aphel_kernel::infrastructure::repositories::mysql_access::MySqlAccessRepository;
use aphel_kernel::records::RecordCtx;
use approvals::approval_gate;
use approvals::domain::{
    ApprovalCheckpoint, ApprovalRequestSummary, ApprovalSubmission, SubmissionOutcome,
};
use email::EmailAddress;
use email::compose::{RecordInput, SendEmailCommand};
use price_lists::domain::{
    BundleExpansion, PriceListSelection, PriceListSummary, PriceRequest, ResolvedPrice,
};
use price_lists::{bundle_expander, discount_limits, price_list_resolver};
use products::domain::DealItemData;

use aphel_kernel::application::shared::money_text;

pub(crate) const DEAL_NOT_FOUND: &str = "deal_not_found";

pub(crate) fn deal_not_found() -> AppError {
    AppError::not_found_code(DEAL_NOT_FOUND, "deal was not found")
}

pub(crate) fn quote_base_url(ctx: &Ctx<'_>) -> String {
    ctx.app_base_url()
}

fn is_hidden(error: &AppError) -> bool {
    matches!(
        error,
        AppError::NotFound(_)
            | AppError::Forbidden(_)
            | AppError::Coded {
                kind: CodedKind::NotFound | CodedKind::Forbidden,
                ..
            }
    )
}

fn text(record: &Value, key: &str) -> Option<String> {
    record
        .get(key)
        .and_then(Value::as_str)
        .map(str::to_owned)
        .filter(|value| !value.is_empty())
}

fn id(record: &Value, key: &str) -> Option<Uuid> {
    text(record, key).and_then(|value| Uuid::parse_str(&value).ok())
}

pub(crate) struct QuoteGateway<'a, 'b> {
    ctx: &'a Ctx<'b>,
}

impl<'a, 'b> QuoteGateway<'a, 'b> {
    pub(crate) fn of(ctx: &'a Ctx<'b>) -> Self {
        Self { ctx }
    }

    async fn load(
        &self,
        scope: &AccessScope,
        record_type: &RecordType,
        id: Uuid,
    ) -> Result<Option<Value>, AppError> {
        let state = self.ctx.state();
        let handler = self.ctx.registry().handler(record_type.as_str())?;
        handler.load(&RecordCtx::new(scope, &state), id).await
    }

    async fn update_deal(
        &self,
        scope: &AccessScope,
        deal_id: Uuid,
        values: Value,
    ) -> Result<(), AppError> {
        let state = self.ctx.state();
        let handler = self.ctx.registry().handler(RecordType::DEAL.as_str())?;
        handler
            .update(&RecordCtx::new(scope, &state), deal_id, values)
            .await
            .map(|_| ())
    }

    pub(crate) async fn member_scope(
        &self,
        user_id: &UserId,
    ) -> Result<Option<AccessScope>, AppError> {
        Ok(MySqlAccessRepository::new(self.ctx.pool().clone())
            .load_access(user_id)
            .await?
            .map(|access| build_access_scope(user_id.clone(), access)))
    }

    pub(crate) async fn deal(
        &self,
        scope: &AccessScope,
        deal_id: Uuid,
    ) -> Result<QuoteDeal, AppError> {
        let deal = self
            .load(scope, &RecordType::DEAL, deal_id)
            .await?
            .ok_or_else(deal_not_found)?;
        let pipeline_id = id(&deal, "pipeline_id").ok_or_else(deal_not_found)?;
        let stage_id = id(&deal, "stage_id").ok_or_else(deal_not_found)?;
        Ok(QuoteDeal {
            id: deal_id,
            title: text(&deal, "title").unwrap_or_default(),
            currency: text(&deal, "currency").unwrap_or_default(),
            owner_id: text(&deal, "owner_id").and_then(|id| UserId::new(id).ok()),
            contact_id: id(&deal, "contact_id"),
            company_id: id(&deal, "company_id"),
            pipeline_id,
            stage_id,
            is_open: text(&deal, "status").as_deref() == Some("open"),
        })
    }

    pub(crate) async fn deal_items(&self, deal_id: Uuid) -> Result<Vec<DealItemData>, AppError> {
        QuoteStore::of(self.ctx).deal_items(deal_id).await
    }

    pub(crate) async fn contact_email(
        &self,
        scope: &AccessScope,
        contact_id: Uuid,
    ) -> Result<Option<EmailAddress>, AppError> {
        let contact = match self.load(scope, &RecordType::CONTACT, contact_id).await {
            Ok(Some(contact)) => contact,
            Ok(None) => return Ok(None),
            Err(error) if is_hidden(&error) => return Ok(None),
            Err(error) => return Err(error),
        };
        let name = text(&contact, "name").or_else(|| {
            let full = [text(&contact, "first_name"), text(&contact, "last_name")]
                .into_iter()
                .flatten()
                .collect::<Vec<_>>()
                .join(" ");
            Some(full)
        });
        Ok(text(&contact, "email").map(|address| EmailAddress {
            address,
            name: name.filter(|name| !name.trim().is_empty()),
        }))
    }

    pub(crate) async fn send_email(
        &self,
        tx: &mut MySqlTx,
        scope: &AccessScope,
        email: QuoteEmail,
    ) -> Result<(), AppError> {
        let ctx = self.ctx.with_scope(scope);
        email::service::compose::enqueue_in_tx(
            &ctx,
            tx,
            SendEmailCommand {
                mailbox_id: email.mailbox_id,
                to: email.to,
                cc: email.cc,
                subject: Some(email.subject),
                body_html: Some(email.body_html),
                record: Some(RecordInput {
                    record_type: RecordType::DEAL.as_str().to_owned(),
                    record_id: email.deal_id.to_string(),
                }),
                ip: email.ip,
                ..SendEmailCommand::default()
            },
        )
        .await
        .map(|_| ())
    }

    pub(crate) async fn replace_deal_items(
        &self,
        scope: &AccessScope,
        deal_id: Uuid,
        items: Vec<DealItemData>,
    ) -> Result<DealItemsOutcome, AppError> {
        match self.ctx.services().get::<dyn DealItemsWriter>() {
            Some(writer) => writer.replace(self.ctx, scope, deal_id, items).await,
            None => Ok(DealItemsOutcome {
                amount_managed: false,
            }),
        }
    }

    pub(crate) async fn set_deal_amount(
        &self,
        scope: &AccessScope,
        deal_id: Uuid,
        amount: Decimal,
    ) -> Result<(), AppError> {
        self.update_deal(scope, deal_id, json!({ "amount": money_text(amount) }))
            .await
    }

    pub(crate) async fn move_deal(
        &self,
        scope: &AccessScope,
        deal_id: Uuid,
        stage_id: Uuid,
    ) -> Result<(), AppError> {
        self.update_deal(scope, deal_id, json!({ "stage_id": stage_id.to_string() }))
            .await
    }
}

pub(crate) struct QuoteCatalog<'a, 'b> {
    ctx: &'a Ctx<'b>,
}

impl<'a, 'b> QuoteCatalog<'a, 'b> {
    pub(crate) fn of(ctx: &'a Ctx<'b>) -> Self {
        Self { ctx }
    }

    pub(crate) async fn find(
        &self,
        scope: &AccessScope,
        price_list_id: Uuid,
    ) -> Result<Option<PriceListSummary>, AppError> {
        match price_list_resolver(self.ctx) {
            Some(resolver) => {
                resolver
                    .find(&self.ctx.with_scope(scope), price_list_id)
                    .await
            }
            None => Ok(None),
        }
    }

    pub(crate) async fn default_for(
        &self,
        scope: &AccessScope,
        selection: &PriceListSelection,
    ) -> Result<Option<PriceListSummary>, AppError> {
        match price_list_resolver(self.ctx) {
            Some(resolver) => {
                resolver
                    .default_for(&self.ctx.with_scope(scope), selection)
                    .await
            }
            None => Ok(None),
        }
    }

    pub(crate) async fn price(
        &self,
        scope: &AccessScope,
        request: &PriceRequest,
    ) -> Result<Option<ResolvedPrice>, AppError> {
        match price_list_resolver(self.ctx) {
            Some(resolver) => resolver.price(&self.ctx.with_scope(scope), request).await,
            None => Ok(None),
        }
    }

    pub(crate) async fn max_discount_percent(
        &self,
        scope: &AccessScope,
        user_id: &UserId,
    ) -> Result<Option<Decimal>, AppError> {
        match discount_limits(self.ctx) {
            Some(limits) => {
                limits
                    .max_discount_percent(&self.ctx.with_scope(scope), user_id)
                    .await
            }
            None => Ok(None),
        }
    }

    pub(crate) async fn expand(
        &self,
        scope: &AccessScope,
        product_id: Uuid,
        price_list_id: Option<Uuid>,
    ) -> Result<Option<BundleExpansion>, AppError> {
        match bundle_expander(self.ctx) {
            Some(expander) => {
                expander
                    .expand(&self.ctx.with_scope(scope), product_id, price_list_id)
                    .await
            }
            None => Ok(None),
        }
    }
}

pub(crate) struct QuoteApprovals<'a, 'b> {
    ctx: &'a Ctx<'b>,
}

impl<'a, 'b> QuoteApprovals<'a, 'b> {
    pub(crate) fn of(ctx: &'a Ctx<'b>) -> Self {
        Self { ctx }
    }

    pub(crate) async fn checkpoint(
        &self,
        tx: &mut MySqlTx,
        scope: &AccessScope,
        checkpoint: &ApprovalCheckpoint,
    ) -> Result<(), AppError> {
        match approval_gate(self.ctx) {
            Some(gate) => {
                gate.checkpoint(&self.ctx.with_scope(scope), tx, checkpoint)
                    .await
            }
            None => Ok(()),
        }
    }

    pub(crate) async fn submit(
        &self,
        tx: &mut MySqlTx,
        scope: &AccessScope,
        submission: &ApprovalSubmission,
    ) -> Result<SubmissionOutcome, AppError> {
        match approval_gate(self.ctx) {
            Some(gate) => {
                gate.submit(&self.ctx.with_scope(scope), tx, submission)
                    .await
            }
            None => Ok(SubmissionOutcome::NotRequired),
        }
    }

    pub(crate) async fn cancel_open(
        &self,
        tx: &mut MySqlTx,
        scope: &AccessScope,
        record: &RecordRef,
        reason: &str,
    ) -> Result<u32, AppError> {
        match approval_gate(self.ctx) {
            Some(gate) => {
                gate.cancel_open(&self.ctx.with_scope(scope), tx, record, reason)
                    .await
            }
            None => Ok(0),
        }
    }

    pub(crate) async fn requests_for(
        &self,
        scope: &AccessScope,
        record: &RecordRef,
    ) -> Result<Vec<ApprovalRequestSummary>, AppError> {
        match approval_gate(self.ctx) {
            Some(gate) => gate.requests_for(&self.ctx.with_scope(scope), record).await,
            None => Ok(Vec::new()),
        }
    }
}
