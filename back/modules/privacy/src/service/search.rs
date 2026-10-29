use serde_json::json;

use crate::domain::{PRIVACY_SUBJECT_SEARCHED, SEARCH_LIMIT, SubjectSearchResult};
use crate::service::subject::parse_subject;
use crate::service::{audit_only, personal_fields, require_privacy};
use crate::store::PrivacyStore;
use aphel_kernel::Ctx;
use aphel_kernel::application::shared::{FieldChange, RecordRef};
use aphel_kernel::errors::AppError;

pub(crate) async fn search_subject(
    ctx: &Ctx<'_>,
    term: &str,
    ip: Option<String>,
) -> Result<SubjectSearchResult, AppError> {
    require_privacy(ctx.scope)?;
    let matcher = parse_subject(term)?;
    let store = PrivacyStore::of(ctx);
    let mut records = Vec::new();
    let definitions: Vec<_> = ctx
        .registry()
        .definitions()
        .filter(|definition| !definition.is_custom())
        .cloned()
        .collect();
    for definition in &definitions {
        let fields = personal_fields(ctx, definition).await?;
        if fields.is_empty() {
            continue;
        }
        records.extend(
            store
                .search_records(definition, &fields, &matcher, SEARCH_LIMIT)
                .await?,
        );
    }
    let refs: Vec<RecordRef> = records.iter().map(|record| record.record.clone()).collect();
    let notes = store.search_notes(&matcher, &refs, SEARCH_LIMIT).await?;
    let activities = store
        .search_activities(&matcher, &refs, SEARCH_LIMIT)
        .await?;
    let attachments = store.attachments(&refs).await?;
    let import_rows = store.search_import_rows(&matcher, SEARCH_LIMIT).await?;
    let result = SubjectSearchResult {
        kind: matcher.kind,
        label: matcher.label(),
        records,
        notes,
        attachments,
        activities,
        import_rows,
    };

    let mut tx = ctx.begin().await?;
    audit_only(
        ctx,
        &mut tx,
        PRIVACY_SUBJECT_SEARCHED,
        RecordRef::organization(),
        vec![
            FieldChange::new("subject_kind", json!(null), json!(matcher.kind.as_str())),
            FieldChange::new(
                "found",
                json!(null),
                json!({
                    "records": result.records.len(),
                    "notes": result.notes.len(),
                    "attachments": result.attachments.len(),
                    "activities": result.activities.len(),
                    "import_rows": result.import_rows.len(),
                }),
            ),
        ],
        ip,
    )
    .await?;
    tx.commit().await?;
    Ok(result)
}
