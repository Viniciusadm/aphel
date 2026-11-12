use std::sync::Arc;

use aphel_contract_site::{
    CARDS_DIR, SiteEmbedContext, SiteEmbedSource, is_embed_key, template_name,
};
use aphel_kernel::errors::AppError;
use aphel_sdk::platform::api::state::AppState;
use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use minijinja::Value;
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::runtime::SiteRuntime;
use crate::{cache, data, images};

const MARKER_START: &str = "<!--aphel:defer:";
const MARKER_END: &str = "-->";
const MAX_DEFERRED: usize = 20;
pub(crate) const MAX_LIST_LIMIT: u64 = 24;
const DEFAULT_LIST_SORT: &str = "-created_at";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub(crate) enum Deferred {
    Embed {
        source: String,
        key: String,
    },
    List {
        owner: String,
        entity: String,
        sort: Option<String>,
        limit: u64,
        #[serde(default)]
        conditions: std::collections::BTreeMap<String, serde_json::Value>,
    },
}

pub(crate) fn marker(deferred: &Deferred) -> Value {
    let encoded = serde_json::to_vec(deferred)
        .map(|bytes| URL_SAFE_NO_PAD.encode(bytes))
        .unwrap_or_default();
    Value::from_safe_string(format!("{MARKER_START}{encoded}{MARKER_END}"))
}

pub(crate) struct Resolution {
    pub html: String,
    pub tags: Vec<String>,
    pub live: bool,
}

pub(crate) struct ResolveContext<'a> {
    pub state: &'a AppState,
    pub runtime: &'a SiteRuntime,
    pub site: &'a serde_json::Value,
    pub appearance: Option<&'a crate::appearance::AppearancePreview>,
    pub translation: Option<&'a str>,
}

fn embed_sources(state: &AppState) -> Vec<Arc<dyn SiteEmbedSource>> {
    state.app.contributions::<dyn SiteEmbedSource>()
}

pub(crate) async fn resolve(html: String, context: &ResolveContext<'_>) -> Resolution {
    if !html.contains(MARKER_START) {
        return Resolution {
            html,
            tags: Vec::new(),
            live: false,
        };
    }
    let mut output = String::with_capacity(html.len());
    let mut tags = Vec::new();
    let mut live = false;
    let mut rest = html.as_str();
    let mut count = 0;
    while let Some(start) = rest.find(MARKER_START) {
        output.push_str(&rest[..start]);
        let after = &rest[start + MARKER_START.len()..];
        let Some(end) = after.find(MARKER_END) else {
            rest = after;
            break;
        };
        let encoded = &after[..end];
        rest = &after[end + MARKER_END.len()..];
        count += 1;
        if count > MAX_DEFERRED {
            continue;
        }
        let Some(deferred) = URL_SAFE_NO_PAD
            .decode(encoded)
            .ok()
            .and_then(|bytes| serde_json::from_slice::<Deferred>(&bytes).ok())
        else {
            continue;
        };
        match resolve_one(&deferred, context).await {
            Ok(resolution) => {
                output.push_str(&resolution.html);
                tags.extend(resolution.tags);
                live |= resolution.live;
            }
            Err(error) => {
                tracing::warn!(%error, ?deferred, "could not fill a deferred part of a site page");
            }
        }
    }
    output.push_str(rest);
    Resolution {
        html: output,
        tags,
        live,
    }
}

async fn resolve_one(
    deferred: &Deferred,
    context: &ResolveContext<'_>,
) -> Result<Resolution, AppError> {
    match deferred {
        Deferred::Embed { source, key } => embed(source, key, context).await,
        Deferred::List {
            owner,
            entity,
            sort,
            limit,
            conditions,
        } => list(owner, entity, sort.as_deref(), *limit, conditions, context).await,
    }
}

async fn embed(
    kind: &str,
    key: &str,
    context: &ResolveContext<'_>,
) -> Result<Resolution, AppError> {
    let empty = Resolution {
        html: String::new(),
        tags: Vec::new(),
        live: true,
    };
    if !is_embed_key(key) {
        return Ok(empty);
    }
    let Some(source) = embed_sources(context.state)
        .into_iter()
        .find(|source| source.kind() == kind)
    else {
        return Ok(empty);
    };
    let text = |name: &str| context.site[name].as_str().unwrap_or_default().to_owned();
    let embed_context = SiteEmbedContext {
        mount: text("mount"),
        prefix: text("prefix"),
        locale: text("locale"),
    };
    Ok(
        match source.render(context.state, key, &embed_context).await? {
            Some(embed) => Resolution {
                html: embed.html,
                tags: embed.tags,
                live: true,
            },
            None => empty,
        },
    )
}

pub(crate) fn card_template(owner: &str, entity: &str) -> String {
    template_name(owner, &format!("{CARDS_DIR}/{entity}.html"))
}

fn sort_of(sort: Option<&str>, exposes: impl Fn(&str) -> bool) -> String {
    sort.filter(|sort| {
        !sort.is_empty()
            && sort.split(',').all(|field| {
                let field = field.trim().trim_start_matches(['-', '+']);
                !field.is_empty() && exposes(field)
            })
    })
    .unwrap_or(DEFAULT_LIST_SORT)
    .to_owned()
}

async fn list(
    owner: &str,
    entity: &str,
    sort: Option<&str>,
    limit: u64,
    conditions: &std::collections::BTreeMap<String, serde_json::Value>,
    context: &ResolveContext<'_>,
) -> Result<Resolution, AppError> {
    let record_type = format!("{owner}.{entity}");
    let empty = Resolution {
        html: String::new(),
        tags: Vec::new(),
        live: false,
    };
    let Some(exposure) = context.runtime.exposure(owner, &record_type) else {
        return Ok(empty);
    };
    let Ok(template) = context
        .runtime
        .environment
        .get_template(&card_template(owner, entity))
    else {
        return Ok(empty);
    };
    let sort = sort_of(sort, |field| exposure.exposes(field));
    if conditions.keys().any(|field| !exposure.exposes(field)) {
        return Ok(empty);
    }
    let mut records = data::matching(
        context.state,
        exposure,
        conditions,
        Some(&sort),
        limit.clamp(1, MAX_LIST_LIMIT),
        context.translation,
    )
    .await?;
    crate::prefixes::annotate(context.state, context.runtime, &record_type, &mut records).await?;
    let mut tags = vec![cache::entity_tag(&record_type)];
    crate::terms::annotate(context.state, exposure, &mut records, &mut tags).await?;
    let mut cards = String::new();
    let mut presentations = std::collections::BTreeMap::new();
    for record in &mut records {
        images::attach_images(context.state, exposure, record).await?;
        let target = context.runtime.prefixed.iter().find_map(|route| {
            let prefix = route.page.prefix.as_ref()?;
            let matches = match &route.page.source {
                aphel_contract_site::PageSource::List { entity, .. } => entity == &record_type,
                _ => false,
            };
            matches
                .then(|| {
                    record
                        .get(&prefix.scope)
                        .and_then(serde_json::Value::as_str)
                        .map(|id| format!("{}:{id}", prefix.entity))
                })
                .flatten()
        });
        let mut site = context.site.clone();
        if let Some(target) = target {
            if let std::collections::btree_map::Entry::Vacant(entry) =
                presentations.entry(target.clone())
            {
                let values = crate::appearance::presentation_values(
                    context.state,
                    &target,
                    context.appearance,
                )
                .await?;
                entry.insert(values);
            }
            if let Some(values) = site["appearance"].as_object_mut() {
                values.extend(presentations[&target].clone());
            }
        }
        let values = json!({
            "site": site,
            "record": record,
            "page": { "owner": owner, "key": "card" },
            "customer": null,
        });
        match template.render(Value::from_serialize(&values)) {
            Ok(card) => cards.push_str(&format!("<li class=\"site-list-item\">{card}</li>")),
            Err(error) => {
                tracing::warn!(%error, owner, entity, "could not render a site card");
            }
        }
    }
    if cards.is_empty() {
        return Ok(Resolution {
            html: String::new(),
            tags,
            live: false,
        });
    }
    Ok(Resolution {
        html: format!("<ul class=\"site-list site-list-{owner}-{entity}\">{cards}</ul>"),
        tags,
        live: false,
    })
}
