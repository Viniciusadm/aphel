use std::sync::Arc;

use aphel_contract_site::{PageSource, SiteBundle, SiteContentSource};
use aphel_kernel::errors::AppError;
use aphel_sdk::platform::application::shared::{DomainEvent, EventConsumer};
use async_trait::async_trait;
use serde_json::Value;
use sqlx::MySqlPool;

use crate::cache::{self, ALL_TAG};
use crate::redirects;

const CONSUMER_NAME: &str = "site.invalidate";
const RECORD_ACTIONS: &[&str] = &[
    "created",
    "updated",
    "deleted",
    "restored",
    "purged",
    "archived",
    "unarchived",
];
const UPDATED_ACTION: &str = "updated";
const ATTACHMENT_EVENTS: &[&str] = &["attachment.added", "attachment.removed"];
const SITE_WIDE_EVENTS: &[&str] = &[
    "organization.updated",
    "organization.settings_updated",
    "module.enabled",
    "module.disabled",
];

pub(crate) struct SiteInvalidationConsumer {
    pool: MySqlPool,
}

impl SiteInvalidationConsumer {
    pub(crate) fn shared(pool: MySqlPool) -> Arc<dyn EventConsumer> {
        Arc::new(Self { pool })
    }
}

pub(crate) async fn invalidate_in_tx(
    tx: &mut aphel_sdk::platform::database::transaction::MySqlTx,
    record: &aphel_sdk::platform::application::shared::RecordRef,
) -> Result<(), AppError> {
    let record_type = record.record_type.as_str();
    let tags = if [
        aphel_contract_site::APPEARANCE_ENTITY,
        aphel_contract_site::DESIGN_ENTITY,
    ]
    .contains(&record_type)
    {
        vec![ALL_TAG.to_owned()]
    } else {
        match site_bundles().await {
            Ok(bundles)
                if !bundles
                    .iter()
                    .flat_map(|bundle| &bundle.exposures)
                    .any(|exposure| exposure.entity == record_type) =>
            {
                return Ok(());
            }
            Ok(_) => vec![
                cache::record_tag(record_type, &record.record_id.to_string()),
                cache::entity_tag(record_type),
            ],
            Err(error) => {
                tracing::warn!(%error, "site exposures unavailable; invalidating the whole site cache");
                vec![ALL_TAG.to_owned()]
            }
        }
    };
    for tag in tags {
        sqlx::query("INSERT INTO site_cache_invalidations (tag) VALUES (?)")
            .bind(tag)
            .execute(&mut **tx)
            .await?;
    }
    Ok(())
}

pub(crate) async fn site_bundles() -> Result<Vec<SiteBundle>, AppError> {
    let mut bundles = Vec::new();
    let Some(app) = aphel_sdk::platform::modules::try_app() else {
        return Ok(bundles);
    };
    for source in app.contributions::<dyn SiteContentSource>() {
        bundles.extend(source.bundles().await?);
    }
    Ok(bundles)
}

fn is_record_event(event_type: &str) -> bool {
    event_type
        .rsplit_once('.')
        .is_some_and(|(_, action)| RECORD_ACTIONS.contains(&action))
}

fn changed_value(value: Option<&Value>) -> Option<String> {
    match value? {
        Value::String(text) => Some(text.clone()),
        Value::Number(number) => Some(number.to_string()),
        _ => None,
    }
}

async fn record_redirects(
    pool: &MySqlPool,
    bundles: &[SiteBundle],
    record_type: &str,
    payload: &Value,
) -> Result<(), AppError> {
    let Some(changes) = payload.get("changes").and_then(Value::as_array) else {
        return Ok(());
    };
    let match_fields = bundles
        .iter()
        .flat_map(|bundle| &bundle.pages)
        .filter_map(|page| match &page.source {
            PageSource::Record {
                entity,
                match_field,
            } if entity == record_type => Some(match_field.as_str()),
            _ => None,
        })
        .chain(
            bundles
                .iter()
                .flat_map(|bundle| &bundle.pages)
                .filter_map(|page| page.prefix.as_ref())
                .filter(|prefix| prefix.entity == record_type)
                .map(|prefix| prefix.field.as_str()),
        );
    for field in match_fields {
        let Some(change) = changes
            .iter()
            .find(|change| change.get("field").and_then(Value::as_str) == Some(field))
        else {
            continue;
        };
        if let (Some(old), Some(new)) = (
            changed_value(change.get("old")),
            changed_value(change.get("new")),
        ) {
            redirects::record_change(pool, record_type, field, &old, &new).await?;
        }
    }
    Ok(())
}

#[async_trait]
impl EventConsumer for SiteInvalidationConsumer {
    fn name(&self) -> &str {
        CONSUMER_NAME
    }

    fn accepts(&self, event_type: &str) -> bool {
        SITE_WIDE_EVENTS.contains(&event_type)
            || ATTACHMENT_EVENTS.contains(&event_type)
            || is_record_event(event_type)
    }

    async fn consume(&self, event: &DomainEvent) -> Result<(), AppError> {
        if SITE_WIDE_EVENTS.contains(&event.event_type.as_str()) {
            return cache::invalidate(&self.pool, &[ALL_TAG.to_owned()]).await;
        }
        let Some(record) = &event.record else {
            return Ok(());
        };
        if event.event_type == crate::people_link::PERSON_PURGED_EVENT {
            return crate::people_link::forget_person(&self.pool, &record.record_id.to_string())
                .await;
        }
        let record_type = record.record_type.as_str();
        if crate::terms::TERM_ENTITIES.contains(&record_type) {
            return cache::invalidate(&self.pool, &[cache::entity_tag(record_type)]).await;
        }
        let bundles = site_bundles().await?;
        let exposed = bundles
            .iter()
            .flat_map(|bundle| &bundle.exposures)
            .any(|exposure| exposure.entity == record_type);
        if !exposed {
            return Ok(());
        }
        if event.event_type.rsplit_once('.').map(|(_, action)| action) == Some(UPDATED_ACTION) {
            record_redirects(&self.pool, &bundles, record_type, &event.payload).await?;
        }
        let record_id = record.record_id.to_string();
        cache::invalidate(
            &self.pool,
            &[
                cache::record_tag(record_type, &record_id),
                cache::entity_tag(record_type),
            ],
        )
        .await
    }
}
