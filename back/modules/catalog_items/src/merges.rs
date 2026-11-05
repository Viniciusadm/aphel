use crate::domain::{Product, ProductData, product_not_found, product_snapshot};
use crate::store::ProductStore;
use aphel_contract_crm::CrmPermissions;
use aphel_sdk::merges::{MergeOwner, PreparedMerge, use_duplicate};
use aphel_sdk::platform::application::access::Authorizer;
use aphel_sdk::platform::database::transaction::MySqlTx;
use aphel_sdk::platform::domain::access::Permission;
use aphel_sdk::{AppError, Ctx};
use chrono::{DateTime, Utc};
use serde_json::Value;
use std::collections::BTreeMap;
use uuid::Uuid;
pub(crate) struct Owner;
struct Prepared {
    primary_id: Uuid,
    primary: Product,
    duplicate: Product,
    merged: ProductData,
}
#[async_trait::async_trait]
impl MergeOwner for Owner {
    fn record_type(&self) -> &'static str {
        aphel_sdk::catalog_items::RECORD_TYPE
    }
    fn not_found(&self) -> AppError {
        product_not_found()
    }
    fn fields(&self) -> &'static [&'static str] {
        &[
            "name",
            "code",
            "price",
            "currency",
            "unit",
            "recurrence",
            "term_months",
            "is_active",
            "description",
        ]
    }
    async fn prepare(
        &self,
        ctx: &Ctx<'_>,
        primary_id: Uuid,
        duplicate_id: Uuid,
        choices: &BTreeMap<String, bool>,
    ) -> Result<Box<dyn PreparedMerge>, AppError> {
        ctx.scope.require(&Permission::PRODUCTS_READ)?;
        ctx.scope.require(&Permission::PRODUCTS_UPDATE)?;
        ctx.scope.require(&Permission::PRODUCTS_DELETE)?;
        let store = ProductStore::of(ctx);
        let primary = store
            .find(primary_id)
            .await?
            .ok_or_else(product_not_found)?;
        let duplicate = store
            .find(duplicate_id)
            .await?
            .ok_or_else(product_not_found)?;
        let before = product_snapshot(&primary.data);
        let source = product_snapshot(&duplicate.data);
        let mut merged = primary.data.clone();
        for key in self.fields() {
            let empty = |value: &Value| match value {
                Value::Null => true,
                Value::String(text) => text.trim().is_empty(),
                Value::Array(items) => items.is_empty(),
                _ => false,
            };
            if use_duplicate(choices, key, empty(&before[key]), empty(&source[key])) {
                match *key {
                    "name" => merged.name = duplicate.data.name.clone(),
                    "code" => merged.code = duplicate.data.code.clone(),
                    "price" => merged.price = duplicate.data.price,
                    "currency" => merged.currency = duplicate.data.currency.clone(),
                    "unit" => merged.unit = duplicate.data.unit.clone(),
                    "recurrence" => merged.recurrence = duplicate.data.recurrence,
                    "term_months" => merged.term_months = duplicate.data.term_months,
                    "is_active" => merged.is_active = duplicate.data.is_active,
                    "description" => merged.description = duplicate.data.description.clone(),
                    _ => unreachable!(),
                }
            }
        }
        if merged.name.trim().is_empty() {
            merged.name = primary.data.name.clone();
        }
        Ok(Box::new(Prepared {
            primary_id,
            primary,
            duplicate,
            merged,
        }))
    }
    async fn retire(
        &self,
        ctx: &Ctx<'_>,
        tx: &mut MySqlTx,
        duplicate: Uuid,
        now: DateTime<Utc>,
    ) -> Result<(), AppError> {
        ProductStore::of(ctx).soft_delete(tx, duplicate, now).await
    }
}
#[async_trait::async_trait]
impl PreparedMerge for Prepared {
    fn before(&self) -> Value {
        product_snapshot(&self.primary.data)
    }
    fn duplicate_name(&self) -> String {
        self.duplicate.data.name.clone()
    }
    async fn apply(
        &mut self,
        ctx: &Ctx<'_>,
        tx: &mut MySqlTx,
        now: DateTime<Utc>,
        _ip: Option<String>,
    ) -> Result<Value, AppError> {
        let _ = (ctx, tx, now);
        Ok(product_snapshot(&self.merged))
    }
    async fn persist(
        &mut self,
        ctx: &Ctx<'_>,
        tx: &mut MySqlTx,
        now: DateTime<Utc>,
    ) -> Result<(), AppError> {
        if self.merged != self.primary.data {
            ProductStore::of(ctx)
                .update(tx, self.primary_id, &self.merged, now)
                .await?;
        }
        Ok(())
    }
}
