use std::collections::BTreeSet;

use aphel_kernel::errors::AppError;
use aphel_sdk::platform::api::state::AppState;
use aphel_sdk::platform::application::query::{Filter, FilterOperator};
use serde::Serialize;
use serde_json::Value;
use utoipa::ToSchema;

use crate::runtime::SiteRuntime;
use crate::{data, targets};

const STATE_FIELD: &str = "publication_state";
const RECENT_LIMIT: u64 = 5;
const TITLE_FIELDS: [&str; 2] = ["title", "name"];

#[derive(Debug, Serialize, ToSchema)]
pub(crate) struct SiteStatusResponse {
    #[schema(example = "https://www.example.com/s")]
    pub base_url: String,
    pub indexing: bool,
    pub home: Option<SiteStatusLink>,
    pub menu_items: usize,
    pub named: bool,
    pub has_logo: bool,
    pub languages: usize,
    pub content: Vec<SiteContentCount>,
    pub recent: Vec<SiteRecentRecord>,
}

#[derive(Debug, Serialize, ToSchema)]
pub(crate) struct SiteStatusLink {
    pub label: String,
    #[schema(example = "https://www.example.com/s/blog")]
    pub url: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub(crate) struct SiteContentCount {
    #[schema(example = "content.entry")]
    pub record_type: String,
    pub label: String,
    pub published: u64,
    pub drafts: u64,
}

#[derive(Debug, Serialize, ToSchema)]
pub(crate) struct SiteRecentRecord {
    #[schema(example = "content.entry")]
    pub record_type: String,
    pub record_id: String,
    pub title: String,
    #[schema(example = "https://www.example.com/s/blog/hello")]
    pub url: String,
    pub published_at: Option<String>,
}

fn text(record: &Value, fields: &[&str]) -> Option<String> {
    fields.iter().find_map(|field| {
        record
            .get(*field)
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|text| !text.is_empty())
            .map(str::to_owned)
    })
}

fn state_is(value: &str) -> Filter {
    Filter::condition(
        STATE_FIELD,
        FilterOperator::Eq,
        Value::String(value.to_owned()),
    )
}

pub(crate) async fn status(
    state: &AppState,
    runtime: &SiteRuntime,
) -> Result<SiteStatusResponse, AppError> {
    let config = state.organization_config.snapshot();
    let settings = crate::settings::site_settings(&config);
    let base_url = config.site_base_url();
    let base = base_url.trim_end_matches('/').to_owned();
    let home = match settings.home.as_deref().and_then(targets::Target::parse) {
        Some(target) => targets::resolve(state, runtime, &target, None)
            .await?
            .map(|resolved| SiteStatusLink {
                label: resolved.label.unwrap_or_default(),
                url: format!("{base}{}", resolved.path),
            }),
        None => None,
    };
    let appearance = crate::appearance::resolved(state, None, None).await?;
    let filled = |key: &str| {
        appearance
            .get(key)
            .and_then(Value::as_str)
            .is_some_and(|value| !value.trim().is_empty())
    };
    let organization = {
        use aphel_sdk::platform::application::organization_profile::OrganizationRepository;
        aphel_sdk::platform::infrastructure::repositories::mysql_organization_profile::MySqlOrganizationRepository::new(state.db()?)
            .find()
            .await?
    };
    let named = filled("name")
        || organization
            .as_ref()
            .is_some_and(|organization| !organization.name.as_str().trim().is_empty());
    let has_logo = filled("logo")
        || organization
            .as_ref()
            .is_some_and(|organization| organization.logo_file_id.is_some());
    let registry = state.registry().await?;
    let mut seen = BTreeSet::new();
    let mut content = Vec::new();
    let mut recent = Vec::new();
    for (_, exposure) in runtime.exposures() {
        if !seen.insert(exposure.entity.clone()) || !exposure.exposes(STATE_FIELD) {
            continue;
        }
        let label = registry
            .require(&exposure.entity)
            .ok()
            .and_then(|definition| {
                definition
                    .custom
                    .as_ref()
                    .map(|labels| labels.plural.clone())
            })
            .unwrap_or_else(|| exposure.entity.clone());
        content.push(SiteContentCount {
            record_type: exposure.entity.clone(),
            label,
            published: data::count(state, exposure, state_is("published")).await?,
            drafts: data::count(state, exposure, state_is("draft")).await?,
        });
        let sort = exposure.exposes("published_at").then_some("-published_at");
        let mut listed =
            data::list_in(state, exposure, (sort, 1, RECENT_LIMIT), None, None, None).await?;
        crate::prefixes::annotate(state, runtime, &exposure.entity, &mut listed.records).await?;
        for record in &listed.records {
            let (Some(path), Some(title), Some(id)) = (
                targets::record_path(runtime, &exposure.entity, record),
                text(record, &TITLE_FIELDS),
                text(record, &["id"]),
            ) else {
                continue;
            };
            recent.push(SiteRecentRecord {
                record_type: exposure.entity.clone(),
                record_id: id,
                title,
                url: format!("{base}{path}"),
                published_at: text(record, &["published_at"]),
            });
        }
    }
    recent.sort_by(|left, right| right.published_at.cmp(&left.published_at));
    recent.truncate(RECENT_LIMIT as usize);
    Ok(SiteStatusResponse {
        base_url,
        indexing: settings.indexing,
        home,
        menu_items: settings.navigation.len(),
        named,
        has_logo,
        languages: settings.languages.len(),
        content,
        recent,
    })
}
