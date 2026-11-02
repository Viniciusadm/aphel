use aphel_contract_crm::CrmRecordTypes;
use serde_json::Value;
use uuid::Uuid;

use crate::http::ActivityCallResponse;
use crate::store::CallStore;
use aphel_kernel::application::shared::RecordType;
use aphel_kernel::errors::AppError;
use aphel_kernel::modules::HookCtx;
use aphel_kernel::records::SerializedRecords;

pub(crate) async fn attach_calls(
    ctx: &mut HookCtx<'_>,
    mut records: SerializedRecords,
) -> Result<SerializedRecords, AppError> {
    if records.record_type != RecordType::ACTIVITY {
        return Ok(records);
    }
    let ids: Vec<Uuid> = records
        .items
        .iter()
        .filter_map(|item| item.get("id").and_then(Value::as_str))
        .filter_map(|id| Uuid::parse_str(id).ok())
        .collect();
    let mut calls = CallStore::new(ctx.pool.clone()).find_many(&ids).await?;
    for item in &mut records.items {
        let call = item
            .get("id")
            .and_then(Value::as_str)
            .and_then(|id| Uuid::parse_str(id).ok())
            .and_then(|id| calls.remove(&id));
        let Some(object) = item.as_object_mut() else {
            continue;
        };
        let call = match call {
            Some(call) => {
                serde_json::to_value(ActivityCallResponse::from(call)).map_err(|error| {
                    AppError::Internal(format!("the call could not be serialized: {error}"))
                })?
            }
            None => Value::Null,
        };
        object.insert("call".to_owned(), call);
    }
    Ok(records)
}
