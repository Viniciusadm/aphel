use aphel_kernel::{Ctx, errors::AppError};
pub use aphel_sdk::timeline::{
    KindDefinition, TimelineCursor, TimelineEntry, TimelineEventExtension, TimelineRequest,
    TimelineSource,
};
use std::collections::BTreeMap;
pub(crate) const DEFAULT_TIMELINE_PAGE_SIZE: u32 = 25;
pub(crate) const MAX_TIMELINE_PAGE_SIZE: u32 = 100;
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) struct TimelineQuery {
    pub(crate) record_type: String,
    pub(crate) record_id: String,
    pub(crate) cursor: Option<String>,
    pub(crate) kinds: Option<String>,
    pub(crate) per_page: Option<u32>,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct TimelinePage {
    pub(crate) items: Vec<TimelineEntry>,
    pub(crate) next_cursor: Option<String>,
}
pub(crate) fn kinds(ctx: &Ctx<'_>) -> Result<Vec<KindDefinition>, AppError> {
    let mut result: BTreeMap<String, KindDefinition> = BTreeMap::new();
    let mut definitions: Vec<_> = ctx
        .app()
        .contributions::<dyn TimelineSource>()
        .into_iter()
        .flat_map(|source| source.kinds())
        .collect();
    definitions.extend(
        ctx.app()
            .contributions::<KindDefinition>()
            .into_iter()
            .map(|definition| (*definition).clone()),
    );
    for definition in definitions {
        if !aphel_sdk::timeline::valid_kind(&definition.key) {
            return Err(AppError::Configuration("invalid timeline kind".to_owned()));
        }
        if result.insert(definition.key.clone(), definition).is_some() {
            return Err(AppError::Configuration(
                "duplicate timeline kind".to_owned(),
            ));
        }
    }
    let assigned: Vec<String> = result
        .values()
        .filter(|definition| definition.key != "record_change")
        .flat_map(|definition| definition.event_types.iter().cloned())
        .collect();
    if let Some(changes) = result.get_mut("record_change") {
        for definition in ctx
            .registry()
            .definitions()
            .filter(|definition| aphel_sdk::records::enabled(ctx, &definition.record_type))
        {
            for action in [
                "created",
                "updated",
                "deleted",
                "restored",
                "archived",
                "unarchived",
                "purged",
            ] {
                let event = format!("{}.{}", definition.event_prefix, action);
                if !assigned.contains(&event) {
                    changes.event_types.push(event);
                }
            }
        }
        changes.event_types.sort();
        changes.event_types.dedup();
    }
    Ok(result.into_values().collect())
}
pub(crate) fn hidden_prefixes(ctx: &Ctx<'_>) -> Vec<String> {
    ctx.app()
        .all_contributions::<dyn TimelineSource>()
        .into_iter()
        .flat_map(|(_, source)| source.kinds())
        .chain(
            ctx.app()
                .all_contributions::<KindDefinition>()
                .into_iter()
                .map(|(_, definition)| (*definition).clone()),
        )
        .flat_map(|definition| definition.hidden_event_prefixes)
        .collect()
}
pub(crate) fn event_kind(event_type: &str, kinds: &[KindDefinition]) -> String {
    kinds
        .iter()
        .find(|kind| kind.event_types.iter().any(|event| event == event_type))
        .or_else(|| {
            kinds.iter().find(|kind| {
                kind.event_prefixes
                    .iter()
                    .any(|prefix| event_type.starts_with(prefix))
            })
        })
        .map_or_else(|| "event".to_owned(), |kind| kind.key.clone())
}
