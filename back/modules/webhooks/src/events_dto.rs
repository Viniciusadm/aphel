use serde::Serialize;
use utoipa::ToSchema;

use aphel_kernel::application::public_events::{PreviousValues, PublicEventDefinition};

#[derive(Debug, Serialize, ToSchema)]
pub(crate) struct PublicEventTypeResponse {
    #[serde(rename = "type")]
    #[schema(example = "deal.stage_changed")]
    pub(crate) event_type: String,
    #[schema(example = "deals.deal")]
    pub(crate) resource: String,
    #[schema(example = "A deal moved to another stage")]
    pub(crate) description: String,
    #[schema(example = "#/components/schemas/DealResponse")]
    pub(crate) data_schema: Option<String>,
    pub(crate) has_previous: bool,
    pub(crate) data_is_id_only: bool,
}

impl From<&PublicEventDefinition> for PublicEventTypeResponse {
    fn from(definition: &PublicEventDefinition) -> Self {
        Self {
            event_type: definition.event_type.to_owned(),
            resource: definition.resource.key().to_owned(),
            description: definition.description.to_owned(),
            data_schema: definition
                .resource
                .has_record()
                .then(|| format!("#/components/schemas/{}", definition.resource.schema_name())),
            has_previous: definition.previous != PreviousValues::None,
            data_is_id_only: definition.deleted,
        }
    }
}
