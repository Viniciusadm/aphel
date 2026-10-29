use crate::application::fields::{RecordRelation, RecordTypeDefinition, RelationPath, SystemField};
use crate::application::shared::RecordType;
use crate::domain::fields::{FieldConfig, FieldType};
use crate::errors::AppError;

pub fn activities_relation() -> RecordRelation {
    RecordRelation::new("activities", "Atividades", RecordType::ACTIVITY)
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

pub fn field(key: &str, label: &str, field_type: FieldType) -> Result<SystemField, AppError> {
    SystemField::new(key, label, field_type, key)
}

pub fn record_field(key: &str, label: &str, target: &str) -> Result<SystemField, AppError> {
    Ok(
        field(key, label, FieldType::Record)?.with_config(FieldConfig {
            target_record_type: Some(target.to_owned()),
            ..FieldConfig::default()
        }),
    )
}

pub fn phone_field(key: &str, label: &str) -> Result<SystemField, AppError> {
    Ok(field(key, label, FieldType::Phone)?
        .searchable()
        .with_config(FieldConfig {
            default_country: Some("BR".to_owned()),
            ..FieldConfig::default()
        }))
}

pub fn address_fields() -> Result<Vec<SystemField>, AppError> {
    [
        ("address_street", "Logradouro"),
        ("address_number", "Número"),
        ("address_complement", "Complemento"),
        ("address_district", "Bairro"),
        ("address_city", "Cidade"),
        ("address_state", "Estado"),
        ("address_postal_code", "CEP"),
        ("address_country", "País"),
    ]
    .into_iter()
    .map(|(key, label)| field(key, label, FieldType::Text))
    .collect()
}

pub fn timestamps() -> Result<Vec<SystemField>, AppError> {
    Ok(vec![
        field("created_at", "Criado em", FieldType::DateTime)?,
        field("updated_at", "Atualizado em", FieldType::DateTime)?,
    ])
}

pub fn email_engagement_fields() -> Result<Vec<SystemField>, AppError> {
    Ok(vec![
        field("last_contacted_at", "Último contato", FieldType::DateTime)?,
        field(
            "last_email_received_at",
            "Último e-mail recebido",
            FieldType::DateTime,
        )?,
        field(
            "last_email_sent_at",
            "Último e-mail enviado",
            FieldType::DateTime,
        )?,
        field(
            "interaction_count",
            "Interações por e-mail",
            FieldType::Number,
        )?,
        field("last_contacted_by", "Último contato por", FieldType::User)?,
    ])
}

pub fn with_fields(
    definition: RecordTypeDefinition,
    fields: Vec<SystemField>,
) -> RecordTypeDefinition {
    fields
        .into_iter()
        .fold(definition, RecordTypeDefinition::system_field)
}
