use crate::AppError;
use crate::fields::{EntityTraits, LocalizedLabels, SystemField};
pub use aphel_contract_party::{
    CONTACT_POINTS, CONTRACT, ContactPoint, ContactPointKind, ContactPoints, DESCRIPTOR,
};
use aphel_kernel::application::fields::builtin::field;
use aphel_kernel::domain::fields::{FieldConfig, FieldType};

pub trait PartyTraits {
    fn with_contact_points(self, points: ContactPoints) -> Self;
    fn contact_points(&self) -> Option<ContactPoints>;
    fn contact_point_columns(&self) -> Vec<String>;
}
impl PartyTraits for EntityTraits {
    fn with_contact_points(self, points: ContactPoints) -> Self {
        self.with_trait_data(
            CONTACT_POINTS.key(),
            serde_json::to_value(points).expect("contact point metadata"),
        )
    }
    fn contact_points(&self) -> Option<ContactPoints> {
        serde_json::from_value(self.trait_data(CONTACT_POINTS.key())?.clone()).ok()
    }
    fn contact_point_columns(&self) -> Vec<String> {
        self.contact_points()
            .map(|points| {
                points
                    .columns()
                    .into_iter()
                    .filter(|column| aphel_kernel::application::fields::is_sql_identifier(column))
                    .collect()
            })
            .unwrap_or_default()
    }
}
fn labels(key: &str) -> LocalizedLabels {
    LocalizedLabels::from_catalog(include_str!("../locales/party.json"), key)
}

pub fn phone_field(
    key: &str,
    label: impl Into<aphel_kernel::application::fields::LocalizedLabels>,
) -> Result<SystemField, AppError> {
    Ok(field(key, label, FieldType::Phone)?
        .searchable()
        .with_config(FieldConfig {
            default_country: Some("BR".to_owned()),
            ..FieldConfig::default()
        }))
}

pub fn address_fields() -> Result<Vec<SystemField>, AppError> {
    [
        ("address_street", "Street"),
        ("address_number", "Number"),
        ("address_complement", "Address line 2"),
        ("address_district", "District"),
        ("address_city", "City"),
        ("address_state", "State"),
        ("address_postal_code", "Postal code"),
        ("address_country", "Country"),
    ]
    .into_iter()
    .map(|(key, label)| field(key, labels(label), FieldType::Text))
    .collect()
}

#[async_trait::async_trait]
pub trait PartyCaptureDefaults: Send + Sync {
    async fn values(
        &self,
        ctx: &crate::Ctx<'_>,
        record_type: &crate::shared::RecordType,
    ) -> Result<serde_json::Map<String, serde_json::Value>, AppError>;
}
pub async fn capture_defaults(
    ctx: &crate::Ctx<'_>,
    record_type: &crate::shared::RecordType,
) -> Result<serde_json::Map<String, serde_json::Value>, AppError> {
    let mut values = serde_json::Map::new();
    for source in ctx.app().contributions::<dyn PartyCaptureDefaults>() {
        for (key, value) in source.values(ctx, record_type).await? {
            if values.insert(key.clone(), value).is_some() {
                return Err(AppError::Configuration(format!(
                    "multiple party capture defaults for {key}"
                )));
            }
        }
    }
    Ok(values)
}

#[async_trait::async_trait]
pub trait PartyAssociationSource: Send + Sync {
    fn target_type(&self) -> &'static str;
    async fn linked_ids(
        &self,
        ctx: &crate::Ctx<'_>,
        party_type: &crate::shared::RecordType,
        ids: &[uuid::Uuid],
    ) -> Result<Vec<uuid::Uuid>, AppError>;
}
