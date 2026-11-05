use crate::domain::{DEAL_AMOUNT_CHANGED, DEAL_UPDATED, revenue_changes};
use aphel_contract_crm::CrmRecordTypes;
use aphel_kernel::{
    Ctx,
    application::shared::{FieldChange, RecordRef, RecordType},
    database::transaction::MySqlTx,
    errors::AppError,
    records::{RecordMutation, changes_payload, publish_mutation},
};
use aphel_sdk::{
    deals::{DealFinancials, RevenueCalculator},
    line_items::{AmountMode, LineItemProjection, TotalsChanged, items_total},
};
use serde_json::json;

pub(crate) struct Projection;
#[async_trait::async_trait]
impl LineItemProjection for Projection {
    fn record_type(&self) -> RecordType {
        RecordType::DEAL
    }
    async fn apply(
        &self,
        ctx: &Ctx<'_>,
        tx: &mut MySqlTx,
        totals: &TotalsChanged,
    ) -> Result<(), AppError> {
        let owner = ctx.services().require::<dyn DealFinancials>()?;
        let id = totals.record.record_id;
        if totals.mode == AmountMode::Items {
            let current = owner
                .lock_amount(tx, id)
                .await?
                .ok_or_else(crate::domain::deal_not_found)?;
            let total = items_total(&totals.items);
            if current.amount != Some(total) {
                owner.set_amount(tx, id, total, totals.at).await?;
                let old = current
                    .amount
                    .map(aphel_sdk::platform::application::fields::decimal_json)
                    .unwrap_or(serde_json::Value::Null);
                let new = aphel_sdk::platform::application::fields::decimal_json(total);
                publish_mutation(ctx.events(), ctx.audit(), tx, ctx.scope, totals.at, RecordMutation {
                    record: RecordRef::new(RecordType::DEAL, id), event_type: DEAL_AMOUNT_CHANGED,
                    payload: json!({"old": old, "new": new, "currency": current.currency, "source": "items"}),
                    changes: vec![FieldChange::new("amount", old, new)], ip: totals.ip.clone(),
                }).await?;
            }
        }
        if let Some(calculator) = ctx.services().get::<dyn RevenueCalculator>() {
            let current = owner
                .lock_revenue(tx, id)
                .await?
                .ok_or_else(crate::domain::deal_not_found)?;
            let mut revenue = current.revenue.clone();
            calculator.project(&mut revenue, &totals.items, current.won_on);
            let changes = revenue_changes(&current.revenue, &revenue);
            if !changes.is_empty() {
                owner.save_revenue(tx, id, &revenue, totals.at).await?;
                let mut payload = changes_payload(&changes);
                payload["source"] = json!("items");
                publish_mutation(
                    ctx.events(),
                    ctx.audit(),
                    tx,
                    ctx.scope,
                    totals.at,
                    RecordMutation {
                        record: totals.record.clone(),
                        event_type: DEAL_UPDATED,
                        payload,
                        changes,
                        ip: totals.ip.clone(),
                    },
                )
                .await?;
            }
        }
        Ok(())
    }
}
