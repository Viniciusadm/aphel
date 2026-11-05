use aphel_sdk::maintenance::{DatasetSeed, DatasetSession, push_record_tuples};
use aphel_sdk::platform::database::transaction::MySqlTx;
use aphel_sdk::shared::{RecordRef, RecordType};
use aphel_sdk::{AppError, Ctx};
use async_trait::async_trait;
use sqlx::QueryBuilder;

const PRODUCT_RECORD_TYPE_VALUE: &str = "catalog_items.record_type";
const DEAL_RECORD_TYPE_VALUE: &str = "deals.record_type";
const DEALS_WITH_ITEMS_VALUE: &str = "line_items.deals";
const DEALS_WITH_ITEMS: usize = 6;

pub(crate) struct LineItemsSeed;

#[async_trait]
impl DatasetSeed for LineItemsSeed {
    fn order(&self) -> i32 {
        500
    }

    fn entities(&self) -> Vec<RecordType> {
        Vec::new()
    }

    async fn seed(&self, ctx: &Ctx<'_>, session: &mut DatasetSession) -> Result<(), AppError> {
        let product_type = session
            .value(PRODUCT_RECORD_TYPE_VALUE)
            .unwrap_or_default()
            .to_owned();
        let deal_type = session
            .value(DEAL_RECORD_TYPE_VALUE)
            .unwrap_or_default()
            .to_owned();
        let products: Vec<(String, uuid::Uuid)> = session
            .entries(&product_type)
            .iter()
            .map(|entry| {
                (
                    entry.attributes.get("kind").cloned().unwrap_or_default(),
                    entry.record.record_id,
                )
            })
            .collect();
        if products.is_empty() {
            return Ok(());
        }
        let open_deals: Vec<uuid::Uuid> = session
            .entries(&deal_type)
            .iter()
            .filter(|entry| {
                entry
                    .attributes
                    .get("open")
                    .is_some_and(|open| open == "true")
            })
            .map(|entry| entry.record.record_id)
            .take(DEALS_WITH_ITEMS)
            .collect();
        let port = ctx
            .services()
            .require::<dyn aphel_sdk::line_items::LineItemsPort>()?;
        for (index, deal_id) in open_deals.iter().enumerate() {
            let seats = 5 + (index as i64 * 3);
            let support_or_training = if index % 2 == 0 {
                "support"
            } else {
                "training"
            };
            let lines = [
                ("license", seats.to_string()),
                ("setup", "1".to_owned()),
                (support_or_training, "1".to_owned()),
            ];
            let mut items = Vec::new();
            for (kind, quantity) in lines {
                let Some((_, product_id)) = products
                    .iter()
                    .find(|(product_kind, _)| product_kind == kind)
                else {
                    continue;
                };
                let Some(product) = aphel_sdk::line_items::catalog_item(ctx, *product_id).await?
                else {
                    continue;
                };
                items.push(aphel_sdk::line_items::LineData {
                    product_id: Some(product.id),
                    name: product.name,
                    unit: product.unit,
                    quantity: quantity
                        .parse()
                        .map_err(|_| AppError::Internal("invalid seed quantity".to_owned()))?,
                    unit_price: product.price,
                    recurrence: product.recurrence,
                    term_months: product.term_months,
                    ..Default::default()
                });
            }
            port.replace(
                ctx,
                &RecordRef::new(RecordType::parse(deal_type.clone())?, *deal_id),
                items,
            )
            .await?;
        }
        session.add_count("line_items", open_deals.len() as u64);
        session.set_value(
            DEALS_WITH_ITEMS_VALUE,
            open_deals
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>()
                .join(","),
        );
        Ok(())
    }

    async fn cleanup(
        &self,
        tx: &mut MySqlTx,
        records: Option<&[RecordRef]>,
    ) -> Result<bool, AppError> {
        let Some(records) = records else {
            sqlx::query("DELETE FROM line_items")
                .execute(&mut **tx)
                .await?;
            return Ok(true);
        };
        let mut delete = QueryBuilder::new("DELETE FROM line_items WHERE ");
        push_record_tuples(&mut delete, records);
        delete.build().execute(&mut **tx).await?;
        Ok(true)
    }
}
