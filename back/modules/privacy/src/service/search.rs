use serde_json::json;

use crate::domain::{PRIVACY_SUBJECT_SEARCHED, SEARCH_LIMIT, SubjectSearchResult};
use crate::service::subject::parse_subject;
use crate::service::{audit_only, personal_fields, require_privacy};
use crate::store::PrivacyStore;
use aphel_kernel::Ctx;
use aphel_kernel::errors::AppError;
use aphel_sdk::platform::application::shared::{FieldChange, RecordRef};

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
        .filter(|definition| definition.capabilities.has_trait("privacy.personal_data"))
        .cloned()
        .collect();
    for definition in &definitions {
        if !aphel_sdk::privacy::sql::table_exists(ctx, &definition.table).await? {
            continue;
        }
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
    let mut items = Vec::new();
    let mut sources = Vec::new();
    for source in super::sources::available(ctx).await? {
        let found = source.search(ctx, &matcher, &refs, SEARCH_LIMIT).await?;
        super::sources::validate_items(source.as_ref(), &found, SEARCH_LIMIT)?;
        sources.push(crate::domain::PrivacySourceInfo {
            key: source.key().into(),
            labels: source.labels().0,
        });
        items.extend(found);
    }
    let attachments = store.attachments(&refs).await?;
    let result = SubjectSearchResult {
        kind: matcher.kind,
        label: matcher.label(),
        records,
        items,
        sources,
        attachments,
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
                    "items": result.items.len(),
                    "attachments": result.attachments.len(),
                }),
            ),
        ],
        ip,
    )
    .await?;
    tx.commit().await?;
    Ok(result)
}
