use aphel_contract_crm::CrmRecordTypes;
use serde_json::Value;
use uuid::Uuid;

use crate::domain::ActivityData;
use aphel_kernel::application::fields::permissions::{FieldAccessPolicy, is_unrestricted};
use aphel_kernel::application::shared::RecordType;
use aphel_kernel::errors::AppError;
use aphel_kernel::infrastructure::fields::permissions::role_access_rules;
use aphel_kernel::modules::{Hook, HookCtx};
use aphel_kernel::records::{RemovedRecord, SerializedRecords};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CompletingActivity {
    pub(crate) id: Uuid,
    pub(crate) before: ActivityData,
    pub(crate) data: ActivityData,
    pub(crate) type_key: String,
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct ActivityCompleting;

impl Hook for ActivityCompleting {
    type Args = CompletingActivity;
}

const LOCATION_DETAIL_KEYS: [(&str, &str); 2] = [
    ("location_accuracy_m", "accuracy_m"),
    ("location_captured_at", "captured_at"),
];

pub(crate) async fn mask_location(
    ctx: &mut HookCtx<'_>,
    mut records: SerializedRecords,
) -> Result<SerializedRecords, AppError> {
    if records.record_type != RecordType::ACTIVITY || is_unrestricted(ctx.scope) {
        return Ok(records);
    }
    let rules = role_access_rules(
        ctx.pool,
        &RecordType::ACTIVITY,
        ctx.scope.role_key().as_str(),
    )
    .await?;
    let hidden = FieldAccessPolicy::from_rules(&RecordType::ACTIVITY, rules).hidden_keys();
    let hides_point = hidden.contains("location_latitude") || hidden.contains("location_longitude");
    let hidden_details: Vec<&str> = LOCATION_DETAIL_KEYS
        .iter()
        .filter(|(field, _)| hidden.contains(*field))
        .map(|(_, key)| *key)
        .collect();
    if !hides_point && hidden_details.is_empty() {
        return Ok(records);
    }
    for item in &mut records.items {
        let Some(object) = item.as_object_mut() else {
            continue;
        };
        if hides_point {
            object.remove("location");
            continue;
        }
        if let Some(location) = object.get_mut("location").and_then(Value::as_object_mut) {
            for key in &hidden_details {
                location.remove(*key);
            }
        }
    }
    Ok(records)
}

pub(crate) async fn unlink_purged_record(
    ctx: &mut HookCtx<'_>,
    removed: &RemovedRecord,
) -> Result<(), AppError> {
    let record_type = removed.record_type.as_str();
    let record_id = removed.record.record_id.to_string();
    sqlx::query("DELETE FROM activity_links WHERE record_type = ? AND record_id = ?")
        .bind(record_type)
        .bind(&record_id)
        .execute(&mut **ctx.tx)
        .await?;
    for table in ["activities", "activity_recurrences"] {
        sqlx::query(sqlx::AssertSqlSafe(format!(
            "UPDATE {table} SET record_type = NULL, record_id = NULL \
             WHERE record_type = ? AND record_id = ?"
        )))
        .bind(record_type)
        .bind(&record_id)
        .execute(&mut **ctx.tx)
        .await?;
    }
    Ok(())
}
