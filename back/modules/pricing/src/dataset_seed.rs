use std::collections::BTreeMap;

use aphel_kernel::database::transaction::MySqlTx;
use aphel_sdk::maintenance::{DatasetSeed, DatasetSession, ids_of};
use aphel_sdk::shared::{RecordRef, RecordType};
use aphel_sdk::{AppError, Ctx};
use async_trait::async_trait;
use sqlx::QueryBuilder;

use crate::domain::{PriceListItemInput, ReplacePriceListItemsCommand, SavePriceListCommand};

const PRODUCT_RECORD_TYPE_VALUE: &str = "catalog_items.record_type";
const PRICE_LIST: RecordType = RecordType::from_static("pricing.price_list");
const PRICE_LIST_KEY: &str = "demo_partners";

pub(crate) struct PricingSeed;

#[async_trait]
impl DatasetSeed for PricingSeed {
    fn order(&self) -> i32 {
        160
    }

    fn entities(&self) -> Vec<RecordType> {
        vec![PRICE_LIST]
    }

    async fn seed(&self, ctx: &Ctx<'_>, session: &mut DatasetSession) -> Result<(), AppError> {
        let product_type = session
            .value(PRODUCT_RECORD_TYPE_VALUE)
            .unwrap_or_default()
            .to_owned();
        let products: Vec<(String, String)> = session
            .entries(&product_type)
            .iter()
            .map(|entry| {
                (
                    entry.attributes.get("kind").cloned().unwrap_or_default(),
                    entry.record.record_id.to_string(),
                )
            })
            .collect();
        if products.is_empty() {
            return Ok(());
        }
        let existing: Option<i64> =
            sqlx::query_scalar("SELECT 1 FROM price_lists WHERE `key` = ? LIMIT 1")
                .bind(PRICE_LIST_KEY)
                .fetch_optional(ctx.pool())
                .await?;
        if existing.is_some() {
            return Ok(());
        }
        let list = crate::service::create(
            ctx,
            SavePriceListCommand {
                key: Some(PRICE_LIST_KEY.to_owned()),
                name: Some("Parceiros (demonstração)".to_owned()),
                description: Some(Some(
                    "Preços com desconto para revendas e parceiros".to_owned(),
                )),
                applies_to: Some("all".to_owned()),
                is_active: Some(true),
                is_default: Some(false),
                ..SavePriceListCommand::default()
            },
        )
        .await?;
        let prices = [
            ("license", "129.00", Some("10")),
            ("support", "4200.00", None),
            ("setup", "2900.00", None),
        ];
        let items = prices
            .iter()
            .filter_map(|(kind, price, min_quantity)| {
                products
                    .iter()
                    .find(|(product_kind, _)| product_kind == kind)
                    .map(|(_, product_id)| PriceListItemInput {
                        product_id: product_id.clone(),
                        unit_price: (*price).to_owned(),
                        min_quantity: min_quantity.map(str::to_owned),
                    })
            })
            .collect();
        crate::service::replace_items(
            ctx,
            &list.id.to_string(),
            ReplacePriceListItemsCommand {
                items,
                ..ReplacePriceListItemsCommand::default()
            },
        )
        .await?;
        let now = session.now();
        session.record(
            RecordRef::new(PRICE_LIST, list.id),
            now,
            now,
            BTreeMap::new(),
        );
        Ok(())
    }

    async fn cleanup(
        &self,
        tx: &mut MySqlTx,
        records: Option<&[RecordRef]>,
    ) -> Result<bool, AppError> {
        let Some(records) = records else {
            sqlx::query("DELETE FROM price_lists")
                .execute(&mut **tx)
                .await?;
            return Ok(true);
        };
        let ids = ids_of(records, &PRICE_LIST);
        if !ids.is_empty() {
            let mut delete = QueryBuilder::new("DELETE FROM price_lists WHERE id IN (");
            let mut list = delete.separated(", ");
            for id in ids {
                list.push_bind(id);
            }
            delete.push(")");
            delete.build().execute(&mut **tx).await?;
        }
        Ok(true)
    }
}
