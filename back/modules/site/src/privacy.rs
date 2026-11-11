use aphel_contract_site::CUSTOMER_RECORD_TYPE;
use aphel_kernel::errors::AppError;
use aphel_sdk::platform::domain::access::AccessScope;
use aphel_sdk::privacy::{DataSubjectOutcome, DataSubjectRequest, DataSubjectSource};
use async_trait::async_trait;
use serde_json::{Value, json};

use crate::customers;

const SOURCE: &str = "site.customers";

pub(crate) struct CustomerPrivacy;

async fn handle(request: &DataSubjectRequest<'_>, ids: &[String]) -> Result<Value, AppError> {
    let runtime = aphel_sdk::runtime()?;
    let pool = runtime.db()?;
    match request.action.as_str() {
        "export" => {
            let mut items = Vec::new();
            for id in ids {
                if let Some(customer) = customers::export(&pool, id).await? {
                    items.push(customer);
                }
            }
            Ok(json!({ "affected": items.len(), "items": items }))
        }
        _ => {
            let scope = AccessScope::system().acting_as("privacy", request.id.to_string());
            let ctx = runtime.ctx(&scope).await?;
            let mut affected = 0;
            for id in ids {
                if customers::export(&pool, id).await?.is_some() {
                    customers::close(&ctx, id, None).await?;
                    affected += 1;
                }
            }
            Ok(json!({ "affected": affected }))
        }
    }
}

#[async_trait]
impl DataSubjectSource for CustomerPrivacy {
    async fn run(&self, request: &DataSubjectRequest<'_>) -> Vec<DataSubjectOutcome> {
        let ids: Vec<String> = request
            .records
            .iter()
            .filter(|record| record.record_type.as_str() == CUSTOMER_RECORD_TYPE)
            .map(|record| record.record_id.to_string())
            .collect();
        if ids.is_empty() {
            return Vec::new();
        }
        vec![DataSubjectOutcome {
            source: SOURCE.to_owned(),
            result: handle(request, &ids).await,
        }]
    }
}
