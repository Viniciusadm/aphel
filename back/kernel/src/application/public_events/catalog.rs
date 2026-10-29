use crate::application::shared::RecordType;
use crate::domain::custom_objects::is_custom_record_type;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PublicResource {
    key: &'static str,
    schema_name: &'static str,
    record: bool,
}

pub const CUSTOM_RECORD_RESOURCE: RecordType = RecordType::from_static("custom_record");

impl PublicResource {
    pub const CUSTOM_RECORD: Self = Self::new("custom_record", "CustomRecordResponse");

    pub const fn new(key: &'static str, schema_name: &'static str) -> Self {
        Self {
            key,
            schema_name,
            record: true,
        }
    }

    pub const fn payload(key: &'static str) -> Self {
        Self {
            key,
            schema_name: "",
            record: false,
        }
    }

    pub fn has_record(self) -> bool {
        self.record
    }

    pub fn key(self) -> &'static str {
        self.key
    }

    pub fn record_type(self) -> RecordType {
        RecordType::from_static(self.key)
    }

    pub fn schema_name(self) -> &'static str {
        self.schema_name
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PreviousValues {
    None,
    Changes,
    FromTo {
        field: &'static str,
        from: &'static str,
    },
    OldNew {
        field: &'static str,
    },
    Pipeline,
    Fields(&'static [(&'static str, &'static str)]),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PublicEventDefinition {
    pub event_type: &'static str,
    pub resource: PublicResource,
    pub description: &'static str,
    pub previous: PreviousValues,
    pub deleted: bool,
    pub verbatim_payload: bool,
    pub context: &'static [&'static str],
}

impl PublicEventDefinition {
    pub fn with_verbatim_payload(mut self) -> Self {
        self.verbatim_payload = true;
        self
    }

    pub const fn with_context(mut self, keys: &'static [&'static str]) -> Self {
        self.context = keys;
        self
    }
}

pub const fn event(
    event_type: &'static str,
    resource: PublicResource,
    description: &'static str,
) -> PublicEventDefinition {
    PublicEventDefinition {
        event_type,
        resource,
        description,
        previous: PreviousValues::None,
        deleted: false,
        verbatim_payload: false,
        context: &[],
    }
}

pub const fn changed(
    event_type: &'static str,
    resource: PublicResource,
    description: &'static str,
    previous: PreviousValues,
) -> PublicEventDefinition {
    PublicEventDefinition {
        event_type,
        resource,
        description,
        previous,
        deleted: false,
        verbatim_payload: false,
        context: &[],
    }
}

pub const fn deleted(
    event_type: &'static str,
    resource: PublicResource,
    description: &'static str,
) -> PublicEventDefinition {
    PublicEventDefinition {
        event_type,
        resource,
        description,
        previous: PreviousValues::None,
        deleted: true,
        verbatim_payload: false,
        context: &[],
    }
}

pub const CUSTOM_EVENT_PATTERN_PREFIX: &str = "co_<key>";

const CUSTOM_CATALOG: &[PublicEventDefinition] = &[
    event(
        "co_<key>.created",
        PublicResource::CUSTOM_RECORD,
        "A record of a custom object was created",
    ),
    changed(
        "co_<key>.updated",
        PublicResource::CUSTOM_RECORD,
        "Fields of a record of a custom object changed",
        PreviousValues::Changes,
    ),
    deleted(
        "co_<key>.deleted",
        PublicResource::CUSTOM_RECORD,
        "A record of a custom object was deleted",
    ),
    event(
        "co_<key>.archived",
        PublicResource::CUSTOM_RECORD,
        "A record of a custom object was archived",
    ),
    event(
        "co_<key>.unarchived",
        PublicResource::CUSTOM_RECORD,
        "A record of a custom object was unarchived",
    ),
];

pub fn public_event_catalog() -> impl Iterator<Item = PublicEventDefinition> {
    crate::modules::app()
        .catalog()
        .public_events()
        .copied()
        .chain(crate::modules::runtime_extensions::public_events())
}

pub fn custom_public_events() -> &'static [PublicEventDefinition] {
    CUSTOM_CATALOG
}

pub fn custom_event_record_type(event_type: &str) -> Option<&str> {
    event_type
        .split_once('.')
        .map(|(prefix, _)| prefix)
        .filter(|prefix| is_custom_record_type(prefix))
}

pub fn find_public_event(event_type: &str) -> Option<PublicEventDefinition> {
    if let Some(definition) =
        public_event_catalog().find(|definition| definition.event_type == event_type)
    {
        return Some(definition);
    }
    let (_, action) = event_type.split_once('.')?;
    custom_event_record_type(event_type)?;
    CUSTOM_CATALOG.iter().copied().find(|definition| {
        definition
            .event_type
            .split_once('.')
            .is_some_and(|(_, candidate)| candidate == action)
    })
}

pub fn is_public_event(event_type: &str) -> bool {
    find_public_event(event_type).is_some()
}

#[async_trait::async_trait]
pub trait RuntimePublicEventSource: Send + Sync {
    fn recognizes(&self, event_type: &str) -> bool;
    async fn refresh(&self, pool: &sqlx::MySqlPool) -> Result<(), crate::errors::AppError>;
}

pub async fn refresh_public_events(pool: &sqlx::MySqlPool) -> Result<(), crate::errors::AppError> {
    for source in crate::modules::app().contributions::<dyn RuntimePublicEventSource>() {
        source.refresh(pool).await?;
    }
    Ok(())
}
