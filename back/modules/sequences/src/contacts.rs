use crate::model::{ContactTarget, EnrollmentView};
use aphel_kernel::{Ctx, errors::AppError};
use aphel_sdk::platform::application::organization_admin::ListWindow;
use aphel_sdk::platform::application::query::{
    ArchivedFilter, Filter, FilterOperator, RecordQuery,
};
use serde_json::json;
use std::collections::{BTreeSet, HashMap};
use uuid::Uuid;

fn text_of(value: &serde_json::Value, key: &str) -> Option<String> {
    value
        .get(key)
        .or_else(|| {
            value
                .get("custom_fields")
                .and_then(|fields| fields.get(key))
        })
        .and_then(|value| value.as_str())
        .map(str::to_owned)
}

pub(crate) async fn list(
    ctx: &Ctx<'_>,
    record_type: &str,
    ids: &[Uuid],
    archived: ArchivedFilter,
) -> Result<HashMap<Uuid, ContactTarget>, AppError> {
    use aphel_sdk::party::{ContactPointKind, PartyTraits};
    let Ok(definition) = ctx.registry().require(record_type) else {
        return Ok(HashMap::new());
    };
    if !aphel_sdk::records::enabled(ctx, &definition.record_type)
        || ctx.scope.scope_for(&definition.read_permission).is_none()
    {
        return Ok(HashMap::new());
    }
    let email_fields: Vec<String> = definition
        .capabilities
        .contact_points()
        .map(|points| {
            points
                .fields(ContactPointKind::Email)
                .map(str::to_owned)
                .collect()
        })
        .unwrap_or_default();
    let ids: Vec<_> = ids
        .iter()
        .copied()
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect();
    let mut contacts = HashMap::new();
    for batch in ids.chunks(100) {
        let listing = aphel_sdk::record_context::list(
            ctx,
            definition.record_type.as_str(),
            RecordQuery {
                filter: Some(Filter::condition("id", FilterOperator::In, json!(batch))),
                archived,
                window: ListWindow {
                    offset: 0,
                    limit: 100,
                },
                ..RecordQuery::default()
            },
        )
        .await?;
        for value in listing.items {
            let id = value
                .get("id")
                .and_then(|value| value.as_str())
                .and_then(|value| Uuid::parse_str(value).ok())
                .ok_or_else(|| AppError::Internal("record snapshot has no identity".into()))?;
            let email = std::iter::once("email".to_owned())
                .chain(email_fields.iter().cloned())
                .find_map(|field| text_of(&value, &field));
            contacts.insert(
                id,
                ContactTarget {
                    id,
                    name: text_of(&value, "name")
                        .or_else(|| text_of(&value, "title"))
                        .unwrap_or_default(),
                    email: email
                        .map(|value| value.trim().to_lowercase())
                        .filter(|value| !value.is_empty()),
                    deleted: false,
                },
            );
        }
    }
    Ok(contacts)
}
pub(crate) async fn hydrate(
    ctx: &Ctx<'_>,
    enrollments: &mut [EnrollmentView],
) -> Result<(), AppError> {
    let record_types: BTreeSet<String> = enrollments
        .iter()
        .map(|entry| entry.record_type.clone())
        .collect();
    let mut contacts: HashMap<Uuid, ContactTarget> = HashMap::new();
    for record_type in record_types {
        let ids: Vec<_> = enrollments
            .iter()
            .filter(|entry| entry.record_type == record_type)
            .map(|entry| entry.contact_id)
            .collect();
        contacts.extend(list(ctx, &record_type, &ids, ArchivedFilter::Include).await?);
    }
    for enrollment in enrollments {
        enrollment.contact_name.clear();
        enrollment.contact_email = None;
        if let Some(contact) = contacts.get(&enrollment.contact_id) {
            enrollment.contact_name.clone_from(&contact.name);
            enrollment.contact_email.clone_from(&contact.email);
        }
    }
    Ok(())
}
