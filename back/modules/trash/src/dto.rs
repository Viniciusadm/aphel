use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};

use crate::domain::{TrashEntry, TrashItem, TrashQuery, TrashSettings};
use aphel_sdk::platform::api::dto::common::{OwnerResponse, timestamp};
use aphel_sdk::platform::application::fields::EntityDefinition;

#[derive(Debug, Serialize, ToSchema)]
pub(crate) struct TrashItemResponse {
    #[schema(example = "people.person")]
    pub(crate) record_type: String,
    pub(crate) id: String,
    pub(crate) title: String,
    pub(crate) subtitle: Option<String>,
    pub(crate) owner: Option<OwnerResponse>,
    pub(crate) deleted_at: String,
    pub(crate) expires_at: Option<String>,
}

impl From<TrashEntry> for TrashItemResponse {
    fn from(entry: TrashEntry) -> Self {
        let mut response = Self::from(entry.item);
        response.expires_at = Some(timestamp(entry.expires_at));
        response
    }
}

impl From<TrashItem> for TrashItemResponse {
    fn from(item: TrashItem) -> Self {
        Self {
            record_type: item.record_type.as_str().to_owned(),
            id: item.id.to_string(),
            title: item.title,
            subtitle: item.subtitle,
            owner: item.owner.map(Into::into),
            deleted_at: timestamp(item.deleted_at),
            expires_at: None,
        }
    }
}

#[derive(Debug, Deserialize, IntoParams)]
#[into_params(parameter_in = Query)]
pub(crate) struct TrashListParams {
    #[param(example = "people.person")]
    pub(crate) record_type: Option<String>,
    pub(crate) search: Option<String>,
}

impl From<TrashListParams> for TrashQuery {
    fn from(params: TrashListParams) -> Self {
        Self {
            record_type: params.record_type,
            search: params.search,
        }
    }
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub(crate) struct TrashSettingsBody {
    #[schema(example = 30, minimum = 1, maximum = 365)]
    pub(crate) retention_days: u32,
}

impl From<TrashSettings> for TrashSettingsBody {
    fn from(settings: TrashSettings) -> Self {
        Self {
            retention_days: settings.retention_days,
        }
    }
}

#[derive(Debug, Serialize, ToSchema)]
pub(crate) struct TrashTypeResponse {
    #[schema(example = "people.person")]
    pub(crate) record_type: String,
    pub(crate) label: Option<String>,
    pub(crate) custom: bool,
}

impl From<&EntityDefinition> for TrashTypeResponse {
    fn from(definition: &EntityDefinition) -> Self {
        Self {
            record_type: definition.record_type.as_str().to_owned(),
            label: definition
                .custom
                .as_ref()
                .map(|labels| labels.plural.clone())
                .or_else(|| {
                    definition
                        .capabilities
                        .plural_labels_value()
                        .map(|labels| labels.default_text().to_owned())
                }),
            custom: definition.is_custom(),
        }
    }
}
