use aphel_contract_crm::CrmRecordTypes;
use serde_json::{Value, json};
use uuid::Uuid;

use crate::domain::ActivityData;
use aphel_kernel::errors::AppError;
use aphel_kernel::modules::{Hook, HookCtx, PluginHookArgs};
use aphel_kernel::records::{RemovedRecord, SerializedRecords};
use aphel_sdk::platform::application::fields::permissions::{FieldAccessPolicy, is_unrestricted};
use aphel_sdk::platform::application::shared::{RecordRef, RecordType};
use aphel_sdk::platform::infrastructure::fields::permissions::role_access_rules;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CompletingActivity {
    pub(crate) id: Uuid,
    pub(crate) before: ActivityData,
    pub(crate) data: ActivityData,
    pub(crate) type_key: String,
}

pub(crate) const COMPLETING: &str = "activity.completing";

fn record_json(record: &RecordRef) -> Value {
    json!({
        "record_type": record.record_type.as_str(),
        "record_id": record.record_id,
    })
}

fn activity_json(data: &ActivityData) -> Value {
    json!({
        "title": data.title,
        "activity_type_id": data.activity_type_id,
        "status": data.status.as_str(),
        "priority": data.priority.as_str(),
        "due_at": data.due_at,
        "completed_at": data.completed_at,
        "owner_id": data.owner_id.as_ref().map(|owner| owner.as_str()),
        "record": data.record.as_ref().map(record_json),
        "links": data.links.iter().map(record_json).collect::<Vec<_>>(),
        "duration_minutes": data.duration_minutes,
        "description": data.description,
    })
}

impl PluginHookArgs for CompletingActivity {
    fn plugin_record_id(&self) -> String {
        self.id.to_string()
    }

    fn plugin_payload(&self) -> Value {
        let mut activity = activity_json(&self.data);
        if let Value::Object(map) = &mut activity {
            map.insert("id".to_owned(), json!(self.id));
        }
        json!({
            "activity": activity,
            "previous": activity_json(&self.before),
            "type_key": self.type_key,
        })
    }
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
    crate::store::unlink_record(ctx.tx, record_type, &record_id).await?;
    Ok(())
}
