use aphel_kernel::application::fields::builtin::field;
use aphel_kernel::application::fields::{RecordRelation, RelationPath, SystemField};
use aphel_kernel::application::shared::RecordType;
use aphel_kernel::domain::fields::{FieldConfig, FieldType};
use aphel_kernel::errors::AppError;

use crate::records::CrmRecordTypes;

pub fn activities_relation() -> RecordRelation {
    RecordRelation::new(
        "activities",
        crate::labels("Activities"),
        RecordType::ACTIVITY,
    )
    .path(RelationPath::PolymorphicMany {
        type_column: "record_type".to_owned(),
        id_column: "record_id".to_owned(),
    })
    .path(RelationPath::Linked {
        table: "activity_links".to_owned(),
        source_type_column: Some("record_type".to_owned()),
        source_id_column: "record_id".to_owned(),
        target_type_column: None,
        target_id_column: "activity_id".to_owned(),
    })
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
    .map(|(key, label)| field(key, crate::labels(label), FieldType::Text))
    .collect()
}

pub fn email_engagement_fields() -> Result<Vec<SystemField>, AppError> {
    Ok(vec![
        field(
            "last_contacted_at",
            crate::labels("Last contact"),
            FieldType::DateTime,
        )?,
        field(
            "last_email_received_at",
            crate::labels("Last email received"),
            FieldType::DateTime,
        )?,
        field(
            "last_email_sent_at",
            crate::labels("Last email sent"),
            FieldType::DateTime,
        )?,
        field(
            "interaction_count",
            crate::labels("Email interactions"),
            FieldType::Number,
        )?,
        field(
            "last_contacted_by",
            crate::labels("Last contacted by"),
            FieldType::User,
        )?,
    ])
}
