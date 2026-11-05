use crate::model::{ContactTarget, EnrollmentView};
use aphel_kernel::{Ctx, errors::AppError};
use aphel_sdk::platform::application::organization_admin::ListWindow;
use aphel_sdk::platform::application::query::{
    ArchivedFilter, Filter, FilterOperator, RecordQuery,
};
use serde_json::json;
use std::collections::{BTreeSet, HashMap};
use uuid::Uuid;

pub(crate) async fn list(
    ctx: &Ctx<'_>,
    ids: &[Uuid],
    archived: ArchivedFilter,
) -> Result<HashMap<Uuid, ContactTarget>, AppError> {
    let Ok(definition) = ctx.registry().require(aphel_sdk::people::RECORD_TYPE) else {
        return Ok(HashMap::new());
    };
    if !aphel_sdk::records::enabled(ctx, &definition.record_type)
        || ctx.scope.scope_for(&definition.read_permission).is_none()
    {
        return Ok(HashMap::new());
    }
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
            aphel_sdk::people::RECORD_TYPE,
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
                .ok_or_else(|| AppError::Internal("person snapshot has no identity".into()))?;
            contacts.insert(
                id,
                ContactTarget {
                    id,
                    name: value
                        .get("name")
                        .and_then(|value| value.as_str())
                        .unwrap_or_default()
                        .to_owned(),
                    email: value
                        .get("email")
                        .and_then(|value| value.as_str())
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
    let ids: Vec<_> = enrollments.iter().map(|entry| entry.contact_id).collect();
    let contacts = list(ctx, &ids, ArchivedFilter::Include).await?;
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
