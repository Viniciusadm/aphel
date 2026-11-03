use aphel_kernel::application::fields::builtin::field;
use aphel_kernel::application::fields::{RecordRelation, RelationPath, SystemField};
use aphel_kernel::application::shared::RecordType;
use aphel_kernel::domain::fields::FieldType;
use aphel_kernel::errors::AppError;

use crate::records::CrmRecordTypes;

pub fn activities_relation() -> RecordRelation {
    RecordRelation::new("tasks", crate::labels("Activities"), RecordType::ACTIVITY)
        .path(RelationPath::PolymorphicMany {
            type_column: "record_type".to_owned(),
            id_column: "record_id".to_owned(),
        })
        .path(RelationPath::Linked {
            table: "task_links".to_owned(),
            source_type_column: Some("record_type".to_owned()),
            source_id_column: "record_id".to_owned(),
            target_type_column: None,
            target_id_column: "activity_id".to_owned(),
        })
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
