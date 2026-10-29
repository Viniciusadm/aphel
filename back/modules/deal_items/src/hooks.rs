use async_trait::async_trait;
use rust_decimal::Decimal;
use uuid::Uuid;

use crate::domain::{DEAL_CURRENCY_HAS_ITEMS, item_command};
use crate::recurring::{apply_revenue, won_on};
use crate::service;
use crate::store::{DealItemStore, sync_deal_rate, sync_open_currency_rate};
use aphel_kernel::Ctx;
use aphel_kernel::application::fields::WriteMode;
use aphel_kernel::application::query::{RecordQueryExecutor, RecordQueryPorts};
use aphel_kernel::application::shared::RecordType;
use aphel_kernel::database::transaction::MySqlTx;
use aphel_kernel::domain::access::AccessScope;
use aphel_kernel::errors::{AppError, FieldViolation};
use aphel_kernel::modules::HookCtx;
use aphel_kernel::records::ChangedRecord;
use deals::domain::Deal;
use deals::hooks::AmountCalculator;
use products::domain::DealItemData;
use quotes::DealItemsWriter;
use quotes::domain::DealItemsOutcome;

pub(crate) struct ItemsAmountCalculator;

#[async_trait]
impl AmountCalculator for ItemsAmountCalculator {
    async fn amount_locked(&self, ctx: &Ctx<'_>, deal_id: Uuid) -> Result<bool, AppError> {
        DealItemStore::of(ctx).amount_managed(deal_id).await
    }

    async fn refresh_revenue(&self, ctx: &Ctx<'_>, deal: &mut Deal) -> Result<(), AppError> {
        let items = DealItemStore::of(ctx).list(deal.id).await?;
        let timezone = ctx.query().executor().timezone().await?;
        let won_on = won_on(deal, timezone);
        apply_revenue(&mut deal.revenue, &items, won_on);
        Ok(())
    }

    async fn sync_rate(
        &self,
        tx: &mut MySqlTx,
        deal_id: Uuid,
        rate: Option<Decimal>,
    ) -> Result<(), AppError> {
        sync_deal_rate(tx, deal_id, rate).await
    }

    async fn sync_currency_rate(
        &self,
        tx: &mut MySqlTx,
        currency: &str,
        rate: Option<Decimal>,
    ) -> Result<(), AppError> {
        sync_open_currency_rate(tx, currency, rate).await
    }
}

pub(crate) struct QuoteDealItemsWriter;

#[async_trait]
impl DealItemsWriter for QuoteDealItemsWriter {
    async fn replace(
        &self,
        ctx: &Ctx<'_>,
        scope: &AccessScope,
        deal_id: Uuid,
        items: Vec<DealItemData>,
    ) -> Result<DealItemsOutcome, AppError> {
        let ctx = ctx.with_scope(scope);
        let deal = deal_id.to_string();
        let currency = service::summary(&ctx, &deal).await?.currency;
        let mut inputs = Vec::with_capacity(items.len());
        for item in &items {
            let with_product = match item.product_id {
                Some(id) => products::store::ProductStore::of(&ctx)
                    .find(id)
                    .await?
                    .is_some_and(|product| {
                        product.data.is_active && product.data.currency == currency
                    }),
                None => false,
            };
            inputs.push(item_command(item, with_product).input);
        }
        let summary = service::replace(&ctx, &deal, inputs, None).await?;
        Ok(DealItemsOutcome {
            amount_managed: summary.amount_managed,
        })
    }
}

pub(crate) const CURRENCY_LOCK_PRIORITY: i32 = 4;

pub(crate) async fn block_currency_change_with_items(
    ctx: &mut HookCtx<'_>,
    args: &ChangedRecord,
) -> Result<(), AppError> {
    if args.record_type != RecordType::DEAL || args.mode != WriteMode::Update {
        return Ok(());
    }
    if args.write().changed("currency").is_none() {
        return Ok(());
    }
    let has_items: bool =
        sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM deal_items WHERE deal_id = ?)")
            .bind(args.record.record_id.to_string())
            .fetch_one(&mut **ctx.tx())
            .await?;
    if !has_items {
        return Ok(());
    }
    Err(AppError::invalid_fields_code(
        DEAL_CURRENCY_HAS_ITEMS,
        "the currency of a deal with items cannot be changed",
        vec![FieldViolation::new(
            "currency",
            "remove the items of the deal before changing its currency",
        )],
    ))
}
