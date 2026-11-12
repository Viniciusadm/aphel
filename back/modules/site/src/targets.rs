use std::collections::BTreeMap;

use aphel_contract_site::PageSource;
use aphel_kernel::errors::AppError;
use aphel_sdk::platform::api::state::AppState;
use serde::Serialize;
use serde_json::Value;
use utoipa::ToSchema;

use crate::cache;
use crate::data;
use crate::prefixes::SITE_PREFIX_FIELD;
use crate::runtime::SiteRuntime;

const RECORD_KIND: &str = "record";
const PAGE_KIND: &str = "page";
const TERM_KIND: &str = "term";
const LABEL_FIELDS: [&str; 3] = ["title", "name", "label"];
const MAX_LISTED_RECORDS: u64 = 200;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Target {
    Record { entity: String, id: String },
    Page { owner: String, key: String },
    Term { vocabulary: String, slug: String },
}

fn identifier(text: &str) -> bool {
    !text.is_empty()
        && text
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || "._-".contains(character))
}

impl Target {
    pub fn parse(text: &str) -> Option<Self> {
        let mut parts = text.splitn(3, ':');
        let (kind, first, second) = (parts.next()?, parts.next()?, parts.next()?);
        if !identifier(first) || !identifier(second) {
            return None;
        }
        match kind {
            RECORD_KIND => Some(Self::Record {
                entity: first.to_owned(),
                id: second.to_owned(),
            }),
            PAGE_KIND => Some(Self::Page {
                owner: first.to_owned(),
                key: second.to_owned(),
            }),
            TERM_KIND => Some(Self::Term {
                vocabulary: first.to_owned(),
                slug: second.to_owned(),
            }),
            _ => None,
        }
    }

    pub fn key(&self) -> String {
        match self {
            Self::Record { entity, id } => format!("{RECORD_KIND}:{entity}:{id}"),
            Self::Page { owner, key } => format!("{PAGE_KIND}:{owner}:{key}"),
            Self::Term { vocabulary, slug } => format!("{TERM_KIND}:{vocabulary}:{slug}"),
        }
    }

    pub fn tag(&self) -> Option<String> {
        match self {
            Self::Record { entity, .. } => Some(cache::entity_tag(entity)),
            Self::Page { .. } => None,
            Self::Term { .. } => crate::terms::cache_tags().into_iter().next(),
        }
    }
}

pub(crate) struct Resolved {
    pub path: String,
    pub label: Option<String>,
}

fn label_of(record: &Value) -> Option<String> {
    LABEL_FIELDS.iter().find_map(|field| {
        record
            .get(field)
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|label| !label.is_empty())
            .map(str::to_owned)
    })
}

fn built(
    pattern: &aphel_contract_site::PathPattern,
    field: &str,
    record: &Value,
) -> Option<String> {
    let value = record.get(field).and_then(Value::as_str)?;
    pattern.build(&BTreeMap::from([(field.to_owned(), value.to_owned())]))
}

fn needs_prefix(runtime: &SiteRuntime, entity: &str) -> bool {
    runtime.prefixed.iter().any(|route| {
        matches!(&route.page.source, PageSource::Record { entity: shown, .. } if shown == entity)
    })
}

pub(crate) fn record_path(runtime: &SiteRuntime, entity: &str, record: &Value) -> Option<String> {
    for route in &runtime.routes {
        if let PageSource::Record {
            entity: shown,
            match_field,
        } = &route.page.source
            && shown == entity
            && let Some(path) = built(&route.pattern, match_field, record)
        {
            return Some(path);
        }
    }
    for route in &runtime.prefixed {
        let Some(prefix) = &route.page.prefix else {
            continue;
        };
        if prefix.entity == entity && route.page.path.trim_end_matches('/').is_empty() {
            let segment = record.get(&prefix.field).and_then(Value::as_str)?;
            return Some(format!("/{segment}"));
        }
        if let PageSource::Record {
            entity: shown,
            match_field,
        } = &route.page.source
            && shown == entity
        {
            let segment = record.get(SITE_PREFIX_FIELD).and_then(Value::as_str)?;
            let detail = built(&route.pattern, match_field, record)?;
            return Some(format!("/{segment}{}", detail.trim_end_matches('/')));
        }
    }
    None
}

fn page_path(runtime: &SiteRuntime, owner: &str, key: &str) -> Option<String> {
    runtime
        .routes
        .iter()
        .find(|route| route.owner == owner && route.page.key == key)
        .and_then(|route| route.pattern.build(&BTreeMap::new()))
}

pub(crate) async fn resolve(
    state: &AppState,
    runtime: &SiteRuntime,
    target: &Target,
    translation: Option<&str>,
) -> Result<Option<Resolved>, AppError> {
    match target {
        Target::Page { owner, key } => {
            Ok(page_path(runtime, owner, key).map(|path| Resolved { path, label: None }))
        }
        Target::Term { vocabulary, slug } => {
            let Some(term) = crate::terms::find_public(state, vocabulary, slug).await? else {
                return Ok(None);
            };
            Ok(crate::terms::route_path(state, runtime, vocabulary, slug)
                .await?
                .map(|path| Resolved {
                    path,
                    label: Some(term.name),
                }))
        }
        Target::Record { entity, id } => {
            let Some(exposure) = runtime.entity_exposure(entity) else {
                return Ok(None);
            };
            let Some(mut record) =
                data::record(state, exposure, "id", id, None, translation).await?
            else {
                return Ok(None);
            };
            if needs_prefix(runtime, entity) {
                crate::prefixes::annotate(
                    state,
                    runtime,
                    entity,
                    std::slice::from_mut(&mut record),
                )
                .await?;
            }
            Ok(record_path(runtime, entity, &record).map(|path| Resolved {
                path,
                label: label_of(&record),
            }))
        }
    }
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub(crate) struct ListedTarget {
    #[schema(example = "record:pages.page:0192f0c4-0000-7000-8000-000000000000")]
    pub target: String,
    #[schema(example = "About us")]
    pub label: String,
    #[schema(example = "pages.page")]
    pub group: String,
    #[schema(example = "/about-us")]
    pub path: String,
}

fn linkable(runtime: &SiteRuntime, entity: &str) -> bool {
    runtime.routes.iter().any(|route| {
        matches!(&route.page.source, PageSource::Record { entity: shown, .. } if shown == entity)
    }) || runtime.prefixed.iter().any(|route| {
        route.page.prefix.as_ref().is_some_and(|prefix| {
            prefix.entity == entity && route.page.path.trim_end_matches('/').is_empty()
        }) || matches!(&route.page.source, PageSource::Record { entity: shown, .. } if shown == entity)
    })
}

pub(crate) async fn list(
    state: &AppState,
    runtime: &SiteRuntime,
) -> Result<Vec<ListedTarget>, AppError> {
    let mut listed: Vec<ListedTarget> = runtime
        .routes
        .iter()
        .filter(|route| {
            !matches!(route.page.source, PageSource::Record { .. })
                && !route.page.needs_customer()
                && route.page.path != "/"
        })
        .filter_map(|route| {
            let path = route.pattern.build(&BTreeMap::new())?;
            Some(ListedTarget {
                target: Target::Page {
                    owner: route.owner.clone(),
                    key: route.page.key.clone(),
                }
                .key(),
                label: path.clone(),
                group: runtime.owner_name(&route.owner).to_owned(),
                path,
            })
        })
        .collect();
    let registry = state.registry().await?;
    for (owner, exposure) in runtime.exposures() {
        let entity = exposure.entity.as_str();
        if !linkable(runtime, entity) {
            continue;
        }
        let group = registry
            .require(entity)
            .ok()
            .and_then(|definition| {
                definition
                    .custom
                    .as_ref()
                    .map(|labels| labels.plural.clone())
            })
            .unwrap_or_else(|| runtime.owner_name(owner).to_owned());
        let mut records = data::matching(
            state,
            exposure,
            &BTreeMap::new(),
            None,
            MAX_LISTED_RECORDS,
            None,
        )
        .await?;
        if needs_prefix(runtime, entity) {
            crate::prefixes::annotate(state, runtime, entity, &mut records).await?;
        }
        for record in &records {
            let (Some(id), Some(path)) = (
                record.get("id").and_then(Value::as_str),
                record_path(runtime, entity, record),
            ) else {
                continue;
            };
            listed.push(ListedTarget {
                target: Target::Record {
                    entity: entity.to_owned(),
                    id: id.to_owned(),
                }
                .key(),
                label: label_of(record).unwrap_or_else(|| path.clone()),
                group: group.clone(),
                path,
            });
        }
    }
    for (vocabulary, group, terms) in crate::terms::linkable(state, runtime).await? {
        for term in terms {
            let Some(path) =
                crate::terms::route_path(state, runtime, &vocabulary, &term.slug).await?
            else {
                continue;
            };
            listed.push(ListedTarget {
                target: Target::Term {
                    vocabulary: vocabulary.clone(),
                    slug: term.slug,
                }
                .key(),
                label: term.name,
                group: group.clone(),
                path,
            });
        }
    }
    Ok(listed)
}

fn term_path(runtime: &SiteRuntime, entity: &str, term: &Value, record: &Value) -> Option<String> {
    let vocabulary = term.get("vocabulary").and_then(Value::as_str)?;
    let slug = term.get("slug").and_then(Value::as_str)?;
    let params = BTreeMap::from([(aphel_contract_site::TERM_PARAM.to_owned(), slug.to_owned())]);
    let shows = |route: &&crate::runtime::Route| {
        route
            .page
            .term
            .as_ref()
            .is_some_and(|page_term| page_term.vocabulary == vocabulary)
            && route.page.source.entity() == Some(entity)
    };
    if let Some(route) = runtime.routes.iter().find(shows) {
        return route.pattern.build(&params);
    }
    let route = runtime.prefixed.iter().find(shows)?;
    let segment = record.get(SITE_PREFIX_FIELD).and_then(Value::as_str)?;
    let detail = route.pattern.build(&params)?;
    Some(format!("/{segment}{}", detail.trim_end_matches('/')))
}

pub(crate) fn annotate_urls(
    runtime: &SiteRuntime,
    entity: &str,
    records: &mut [Value],
    site_prefix: &str,
) {
    for record in records.iter_mut() {
        let url = record_path(runtime, entity, record).map(|path| format!("{site_prefix}{path}"));
        let terms: Option<Vec<Value>> = record
            .get(crate::terms::SITE_TERMS_FIELD)
            .and_then(Value::as_array)
            .map(|terms| {
                terms
                    .iter()
                    .map(|term| {
                        let mut term = term.clone();
                        if let Some(path) = term_path(runtime, entity, &term, record) {
                            term["url"] = Value::String(format!("{site_prefix}{path}"));
                        }
                        term
                    })
                    .collect()
            });
        if let Some(fields) = record.as_object_mut() {
            fields.insert("record_type".to_owned(), Value::String(entity.to_owned()));
            if let Some(url) = url {
                fields.insert("url".to_owned(), Value::String(url));
            }
            if let Some(terms) = terms {
                fields.insert(
                    crate::terms::SITE_TERMS_FIELD.to_owned(),
                    Value::Array(terms),
                );
            }
        }
    }
}
