use std::sync::{Arc, RwLock};

use aphel_contract_site::{
    ALWAYS_PUBLIC_FIELDS, HOME_PATH, ISLANDS_DIR, PageSource, PathPattern, SiteApiHandler,
    SiteApiRequest, SiteBundle, SiteContentSource, SiteCsp, SiteExposure, SiteMenuItem,
    SiteMenuRecords, SitePage, SitePageSeo, SiteTemplate, SiteTheme, THEME_ASSETS_DIR,
    check_bundle, is_island_name,
};
use aphel_kernel::errors::AppError;
use aphel_sdk::platform::api::state::AppState;
use aphel_sdk::platform::state::runtime_pool;
use aphel_sdk::platform::tenancy::TenantLocal;
use async_trait::async_trait;
use axum::response::Response;

use crate::manifest::{
    PluginManifest, RouteAudience, SiteCspSpec, SiteMenuSpec, SitePageSpec, match_route,
};
use crate::package::PackageFile;
use crate::{registry, store};

pub(crate) const SITE_DIR: &str = "site";
const TEMPLATE_EXTENSION: &str = ".html";
const ISLAND_EXTENSION: &str = ".js";
const SITE_INVALID: &str = "plugin_site_invalid";
const SITE_ROUTE_CONFLICT: &str = "site_route_conflict";

fn source_of(manifest: &PluginManifest, page: &SitePageSpec) -> Result<PageSource, String> {
    let label = format!("site.pages {}", page.key);
    match (
        &page.entity,
        &page.match_field,
        &page.list,
        &page.conditions,
    ) {
        (None, None, None, None) => Ok(PageSource::Static),
        (Some(entity), Some(match_field), None, None) => Ok(PageSource::Record {
            entity: manifest.entity_record_type(entity),
            match_field: match_field.clone(),
        }),
        (Some(entity), None, Some(list), None) => Ok(PageSource::List {
            entity: manifest.entity_record_type(entity),
            sort: list.sort.clone(),
            per_page: list.per_page,
        }),
        (Some(entity), None, None, Some(conditions)) => Ok(PageSource::Single {
            entity: manifest.entity_record_type(entity),
            conditions: conditions.clone(),
        }),
        (Some(_), None, None, None) => Err(format!(
            "{label}: a page with an entity needs match (one record by address), where (one record by condition) or list"
        )),
        (Some(_), _, _, _) => Err(format!("{label}: use only one of match, where or list")),
        (None, _, _, _) => Err(format!("{label}: match, where and list need an entity")),
    }
}

fn menu_item_of(spec: &SiteMenuSpec, label: &str) -> Result<SiteMenuItem, String> {
    let records = match (&spec.conditions, &spec.label_field, &spec.sort) {
        (None, None, None) => None,
        (conditions, Some(label_field), sort) => Some(SiteMenuRecords {
            conditions: conditions.clone().unwrap_or_default(),
            label_field: label_field.clone(),
            sort: sort.clone(),
        }),
        (_, None, _) => {
            return Err(format!(
                "{label} {}: where and sort need label_field",
                spec.key
            ));
        }
    };
    Ok(SiteMenuItem {
        key: spec.key.clone(),
        label: spec.label.clone(),
        page: spec.page.clone(),
        labels: spec.labels.clone(),
        records,
    })
}

fn templates_of<'a>(files: impl Iterator<Item = (&'a str, &'a [u8])>) -> Vec<SiteTemplate> {
    files
        .filter(|(path, _)| {
            path.starts_with(&format!("{SITE_DIR}/")) && path.ends_with(TEMPLATE_EXTENSION)
        })
        .filter_map(|(path, content)| {
            std::str::from_utf8(content)
                .ok()
                .map(|source| SiteTemplate {
                    name: path.to_owned(),
                    source: source.to_owned(),
                })
        })
        .collect()
}

fn islands_of<'a>(files: impl Iterator<Item = (&'a str, &'a [u8])>) -> Vec<String> {
    files
        .filter_map(|(path, _)| {
            path.strip_prefix(&format!("{ISLANDS_DIR}/"))
                .and_then(|name| name.strip_suffix(ISLAND_EXTENSION))
                .filter(|name| !name.contains('/'))
                .map(str::to_owned)
        })
        .collect()
}

fn csp_of(spec: &SiteCspSpec) -> SiteCsp {
    SiteCsp {
        script: spec.script.clone(),
        style: spec.style.clone(),
        img: spec.img.clone(),
        font: spec.font.clone(),
        connect: spec.connect.clone(),
        frame: spec.frame.clone(),
    }
}

fn dummy_path(path: &str) -> String {
    path.split('/')
        .map(|segment| {
            if segment.starts_with('{') && segment.ends_with('}') {
                "x"
            } else {
                segment
            }
        })
        .collect::<Vec<_>>()
        .join("/")
}

fn bundle_of(
    manifest: &PluginManifest,
    revision: String,
    templates: Vec<SiteTemplate>,
    islands: Vec<String>,
) -> Result<Option<SiteBundle>, Vec<String>> {
    let Some(site) = &manifest.site else {
        return Ok(None);
    };
    let mut problems = Vec::new();
    let exposures = site
        .public
        .iter()
        .map(|public| {
            match manifest
                .entities
                .iter()
                .find(|entity| entity.key == public.entity)
            {
                None => problems.push(format!(
                    "site.public: {} is not an entity of this plugin",
                    public.entity
                )),
                Some(entity) => {
                    for field in &public.fields {
                        let known = ALWAYS_PUBLIC_FIELDS.contains(&field.as_str())
                            || entity.fields.iter().any(|declared| &declared.key == field);
                        if !known {
                            problems.push(format!(
                                "site.public {}: {field} is not a field of the entity",
                                public.entity
                            ));
                        }
                    }
                }
            }
            SiteExposure {
                entity: manifest.entity_record_type(&public.entity),
                fields: public.fields.clone(),
                conditions: public.conditions.clone(),
                images: public.images,
                translated: public.translated.clone(),
            }
        })
        .collect();
    let pages = site
        .pages
        .iter()
        .filter_map(|page| match source_of(manifest, page) {
            Ok(source) => Some(SitePage {
                key: page.key.clone(),
                path: page.path.clone(),
                template: page.template.clone(),
                source,
                title: page.title.clone(),
                seo: SitePageSeo {
                    description: page.seo.description.clone(),
                    image: page.seo.image.clone(),
                    noindex: page.seo.noindex,
                    schema: page.seo.schema.clone(),
                    structured_data: page.seo.structured_data.clone(),
                },
                data: page.data.clone(),
                requires_customer: page.requires_customer,
                customer_field: page.customer_field.clone(),
            }),
            Err(problem) => {
                problems.push(problem);
                None
            }
        })
        .collect();
    let bundle = SiteBundle {
        owner: manifest.key.clone(),
        name: Some(manifest.name.clone()),
        revision,
        pages,
        exposures,
        templates,
        menu: site
            .menu
            .iter()
            .filter_map(|item| match menu_item_of(item, "site.menu") {
                Ok(item) => Some(item),
                Err(problem) => {
                    problems.push(problem);
                    None
                }
            })
            .collect(),
        theme: site.theme.as_ref().map(|theme| SiteTheme {
            name: theme.name.clone(),
            description: theme.description.clone(),
        }),
        islands,
        csp: csp_of(&site.csp),
        account: site
            .account
            .iter()
            .filter_map(|item| match menu_item_of(item, "site.account") {
                Ok(item) if item.records.is_some() => {
                    problems.push(format!(
                        "site.account {}: account links cannot use where",
                        item.key
                    ));
                    None
                }
                Ok(item) => Some(item),
                Err(problem) => {
                    problems.push(problem);
                    None
                }
            })
            .collect(),
    };
    for page in &site.pages {
        if let Some(data) = &page.data
            && match_route(
                &manifest.routes,
                RouteAudience::Site,
                "GET",
                &dummy_path(data),
            )
            .is_none()
        {
            problems.push(format!(
                "site.pages {}: data {data} is not a GET route with site = true",
                page.key
            ));
        }
    }
    problems.extend(check_bundle(&bundle));
    if problems.is_empty() {
        Ok(Some(bundle))
    } else {
        Err(problems)
    }
}

fn shapes(manifest: &PluginManifest) -> Vec<(String, String)> {
    manifest
        .site
        .iter()
        .flat_map(|site| &site.pages)
        .filter_map(|page| {
            PathPattern::parse(&page.path)
                .ok()
                .map(|pattern| (pattern.shape(), page.path.clone()))
        })
        .collect()
}

pub(crate) fn validate_package(
    manifest: &PluginManifest,
    files: &[PackageFile],
    others: &[&PluginManifest],
    tool_paths: &[&str],
) -> Result<(), AppError> {
    let entries = || {
        files
            .iter()
            .map(|file| (file.path.as_str(), file.content.as_slice()))
    };
    if let Err(problems) = bundle_of(
        manifest,
        String::new(),
        templates_of(entries()),
        islands_of(entries()),
    ) {
        return Err(AppError::validation_code(SITE_INVALID, problems.join("; ")));
    }
    for (shape, path) in shapes(manifest) {
        let first = path
            .trim_start_matches('/')
            .split('/')
            .next()
            .unwrap_or_default();
        if tool_paths.contains(&first) {
            return Err(AppError::conflict_code(
                SITE_ROUTE_CONFLICT,
                format!("the site page {path} uses /{first}, reserved for public app pages"),
            ));
        }
        if shape == HOME_PATH {
            continue;
        }
        if let Some(other) = others
            .iter()
            .filter(|other| other.key != manifest.key)
            .find(|other| shapes(other).iter().any(|(taken, _)| taken == &shape))
        {
            return Err(AppError::conflict_code(
                SITE_ROUTE_CONFLICT,
                format!(
                    "the site page {path} is already served by the plugin {}",
                    other.key
                ),
            ));
        }
    }
    Ok(())
}

pub(crate) fn public_routes(manifest: &PluginManifest) -> Vec<String> {
    let Some(site) = &manifest.site else {
        return Vec::new();
    };
    site.pages
        .iter()
        .map(|page| format!("GET <site>{}", page.path))
        .chain(
            manifest
                .routes
                .iter()
                .filter(|route| route.site)
                .map(|route| {
                    format!(
                        "{} /_aphel/site/api/{}{}",
                        route.method, manifest.key, route.path
                    )
                }),
        )
        .chain(
            csp_of(&site.csp)
                .directives()
                .into_iter()
                .flat_map(|(directive, sources)| {
                    sources
                        .iter()
                        .map(move |source| format!("site csp: {directive} {source}"))
                        .collect::<Vec<_>>()
                }),
        )
        .chain(site.public.iter().map(|public| {
            format!(
                "site: {} ({})",
                manifest.entity_record_type(&public.entity),
                public.fields.join(", ")
            )
        }))
        .collect()
}

type CachedBundles = Option<(Option<u64>, Arc<Vec<SiteBundle>>)>;

static BUNDLES: TenantLocal<RwLock<CachedBundles>> = TenantLocal::new(RwLock::default);

pub(crate) struct PluginSiteContent;

#[async_trait]
impl SiteContentSource for PluginSiteContent {
    async fn bundles(&self) -> Result<Vec<SiteBundle>, AppError> {
        let pool = runtime_pool()?;
        let snapshot = registry::current(&pool).await?;
        let cache = BUNDLES.try_get()?;
        if let Some((_, bundles)) = cache
            .read()
            .ok()
            .and_then(|cached| cached.clone())
            .filter(|(version, _)| *version == snapshot.version)
        {
            return Ok(bundles.as_ref().clone());
        }
        let revision = snapshot.version.unwrap_or_default().to_string();
        let mut bundles = Vec::new();
        for plugin in &snapshot.plugins {
            if plugin.manifest.site.is_none() {
                continue;
            }
            let files = store::files_under(&pool, &plugin.key, SITE_DIR).await?;
            let entries = || {
                files
                    .iter()
                    .map(|(path, content)| (path.as_str(), content.as_slice()))
            };
            match bundle_of(
                &plugin.manifest,
                revision.clone(),
                templates_of(entries()),
                islands_of(entries()),
            ) {
                Ok(Some(bundle)) => bundles.push(bundle),
                Ok(None) => {}
                Err(problems) => {
                    tracing::warn!(plugin = %plugin.key, ?problems, "the plugin site pages are invalid")
                }
            }
        }
        let bundles = Arc::new(bundles);
        if let Ok(mut cached) = cache.write() {
            *cached = Some((snapshot.version, bundles.clone()));
        }
        Ok(bundles.as_ref().clone())
    }

    async fn asset(&self, owner: &str, path: &str) -> Result<Option<(String, Vec<u8>)>, AppError> {
        let path = format!("{THEME_ASSETS_DIR}/{}", path.trim_start_matches('/'));
        if path
            .split('/')
            .any(|part| part.is_empty() || part == "." || part == "..")
        {
            return Ok(None);
        }
        let pool = runtime_pool()?;
        let snapshot = registry::current(&pool).await?;
        let is_theme = snapshot
            .plugin(owner)
            .and_then(|plugin| plugin.manifest.site.as_ref())
            .is_some_and(|site| site.theme.is_some());
        if !is_theme {
            return Ok(None);
        }
        store::file(&pool, owner, &path).await
    }

    async fn island(&self, owner: &str, name: &str) -> Result<Option<Vec<u8>>, AppError> {
        if !is_island_name(name) {
            return Ok(None);
        }
        let pool = runtime_pool()?;
        let snapshot = registry::current(&pool).await?;
        if snapshot
            .plugin(owner)
            .is_none_or(|plugin| plugin.manifest.site.is_none())
        {
            return Ok(None);
        }
        Ok(store::file(
            &pool,
            owner,
            &format!("{ISLANDS_DIR}/{name}{ISLAND_EXTENSION}"),
        )
        .await?
        .map(|(_, content)| content))
    }
}

pub(crate) struct PluginSiteApi;

#[async_trait]
impl SiteApiHandler for PluginSiteApi {
    async fn handle(
        &self,
        state: &AppState,
        request: SiteApiRequest,
    ) -> Result<Option<Response>, AppError> {
        let snapshot = registry::current(&state.db()?).await?;
        let serves_site = snapshot
            .plugin(&request.owner)
            .is_some_and(|plugin| plugin.manifest.routes.iter().any(|route| route.site));
        if !serves_site {
            return Ok(None);
        }
        crate::http::check_public_limits(
            &request.owner,
            request.client.as_deref().unwrap_or("unknown"),
        )
        .await?;
        let call = crate::http::RouteCall {
            method: request.method,
            key: request.owner,
            path: request.path.trim_start_matches('/').to_owned(),
            query: request.query,
            headers: request.headers,
            body: request.body,
            customer: request.customer,
        };
        crate::http::serve_route(state, call, None, RouteAudience::Site)
            .await
            .map(Some)
    }
}
