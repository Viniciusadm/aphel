use aphel_kernel::{
    Ctx, application::shared::RecordRef, database::transaction::Tx, errors::AppError,
};
use aphel_sdk::line_items::{LineData, LineItem, LineItemSummary, LineItemsPort};
use rust_decimal::Decimal;
pub(crate) struct Port;
#[async_trait::async_trait]
impl LineItemsPort for Port {
    async fn summary(
        &self,
        ctx: &Ctx<'_>,
        record: &RecordRef,
    ) -> Result<LineItemSummary, AppError> {
        let summary = super::service::summary(ctx, record).await?;
        Ok(LineItemSummary {
            currency: summary.currency,
            items: summary.items.into_iter().map(|line| line.item).collect(),
            subtotal: summary.subtotal,
            discount_total: summary.discount_total,
            total: summary.total,
        })
    }
    async fn rows(&self, ctx: &Ctx<'_>, record: &RecordRef) -> Result<Vec<LineItem>, AppError> {
        super::service::summary(ctx, record)
            .await
            .map(|summary| summary.items.into_iter().map(|line| line.item).collect())
    }
    async fn managed(&self, ctx: &Ctx<'_>, record: &RecordRef) -> Result<bool, AppError> {
        use aphel_sdk::line_items::LineItemProjection;
        if !ctx
            .app()
            .contributions::<dyn LineItemProjection>()
            .iter()
            .any(|source| source.record_type() == record.record_type)
        {
            return Ok(false);
        }
        super::store::LineItemStore::of(ctx)
            .amount_managed(record)
            .await
    }
    async fn sync_rate(
        &self,
        tx: &mut Tx,
        record: &RecordRef,
        rate: Option<Decimal>,
    ) -> Result<(), AppError> {
        super::store::sync_record_rate(tx, record, rate).await
    }
    async fn rebase_amounts(
        &self,
        tx: &mut Tx,
        record: &RecordRef,
        factor: Decimal,
    ) -> Result<(), AppError> {
        super::store::rebase_record_amounts(tx, record, factor).await
    }
    async fn replace(
        &self,
        ctx: &Ctx<'_>,
        record: &RecordRef,
        items: Vec<LineData>,
    ) -> Result<bool, AppError> {
        let currency = super::service::host_for(ctx, record, true).await?.currency;
        let mut inputs = Vec::with_capacity(items.len());
        for item in items {
            let keep_reference = match item.product_id {
                Some(id) => aphel_sdk::line_items::catalog_item(ctx, id)
                    .await?
                    .is_some_and(|product| product.active && product.currency == currency),
                None => false,
            };
            inputs.push(super::domain::item_command(&item, keep_reference).input);
        }
        Ok(super::service::replace(ctx, record, inputs, None)
            .await?
            .amount_managed)
    }
}
