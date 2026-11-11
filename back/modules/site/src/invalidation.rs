use std::collections::BTreeSet;
use std::sync::Arc;

use aphel_contract_site::SiteContentSource;
use aphel_kernel::errors::AppError;
use aphel_sdk::platform::application::shared::{DomainEvent, EventConsumer};
use async_trait::async_trait;
use sqlx::MySqlPool;

use crate::cache::{self, ALL_TAG};

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
const ATTACHMENT_EVENTS: &[&str] = &["attachment.added", "attachment.removed"];
const SITE_WIDE_EVENTS: &[&str] = &["organization.updated", "organization.settings_updated"];

pub(crate) struct SiteInvalidationConsumer {
    pool: MySqlPool,
}

impl SiteInvalidationConsumer {
    pub(crate) fn shared(pool: MySqlPool) -> Arc<dyn EventConsumer> {
        Arc::new(Self { pool })
    }
}

async fn exposed_entities() -> Result<BTreeSet<String>, AppError> {
    let mut exposed = BTreeSet::new();
    let Some(app) = aphel_sdk::platform::modules::try_app() else {
        return Ok(exposed);
    };
    for source in app.contributions::<dyn SiteContentSource>() {
        for bundle in source.bundles().await? {
            exposed.extend(bundle.exposures.into_iter().map(|exposure| exposure.entity));
        }
    }
    Ok(exposed)
}

fn is_record_event(event_type: &str) -> bool {
    event_type
        .rsplit_once('.')
        .is_some_and(|(_, action)| RECORD_ACTIONS.contains(&action))
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
        let record_type = record.record_type.as_str();
        if !exposed_entities().await?.contains(record_type) {
            return Ok(());
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
