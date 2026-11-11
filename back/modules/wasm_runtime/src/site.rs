use std::sync::{Arc, RwLock};

use aphel_contract_site::{
    ALWAYS_PUBLIC_FIELDS, PageSource, PathPattern, SiteBundle, SiteContentSource, SiteExposure,
    SitePage, SiteTemplate, check_bundle,
};
use aphel_kernel::errors::AppError;
use aphel_sdk::platform::state::runtime_pool;
use async_trait::async_trait;

use crate::manifest::{PluginManifest, SitePageSpec};
use crate::package::PackageFile;
use crate::{registry, store};

pub(crate) const SITE_DIR: &str = "site";
const TEMPLATE_EXTENSION: &str = ".html";
const SITE_INVALID: &str = "plugin_site_invalid";
const SITE_ROUTE_CONFLICT: &str = "site_route_conflict";

fn source_of(manifest: &PluginManifest, page: &SitePageSpec) -> Result<PageSource, String> {
    let label = format!("site.pages {}", page.key);
    match (&page.entity, &page.match_field, &page.list) {
        (None, None, None) => Ok(PageSource::Static),
        (Some(entity), Some(match_field), None) => Ok(PageSource::Record {
            entity: manifest.entity_record_type(entity),
            match_field: match_field.clone(),
        }),
        (Some(entity), None, Some(list)) => Ok(PageSource::List {
            entity: manifest.entity_record_type(entity),
            sort: list.sort.clone(),
            per_page: list.per_page,
        }),
        (Some(_), None, None) => Err(format!(
            "{label}: a page with an entity needs match (one record) or list"
        )),
        (Some(_), Some(_), Some(_)) => Err(format!("{label}: use either match or list")),
        (None, _, _) => Err(format!("{label}: match and list need an entity")),
    }
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

fn bundle_of(
    manifest: &PluginManifest,
    revision: String,
    templates: Vec<SiteTemplate>,
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
            }),
            Err(problem) => {
                problems.push(problem);
                None
            }
        })
        .collect();
    let bundle = SiteBundle {
        owner: manifest.key.clone(),
        revision,
        pages,
        exposures,
        templates,
    };
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
) -> Result<(), AppError> {
    let templates = templates_of(
        files
            .iter()
            .map(|file| (file.path.as_str(), file.content.as_slice())),
    );
    if let Err(problems) = bundle_of(manifest, String::new(), templates) {
        return Err(AppError::validation_code(SITE_INVALID, problems.join("; ")));
    }
    for (shape, path) in shapes(manifest) {
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

static BUNDLES: RwLock<CachedBundles> = RwLock::new(None);

pub(crate) struct PluginSiteContent;

#[async_trait]
impl SiteContentSource for PluginSiteContent {
    async fn bundles(&self) -> Result<Vec<SiteBundle>, AppError> {
        let pool = runtime_pool()?;
        let snapshot = registry::current(&pool).await?;
        if let Some((_, bundles)) = BUNDLES
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
            let templates = templates_of(
                files
                    .iter()
                    .map(|(path, content)| (path.as_str(), content.as_slice())),
            );
            match bundle_of(&plugin.manifest, revision.clone(), templates) {
                Ok(Some(bundle)) => bundles.push(bundle),
                Ok(None) => {}
                Err(problems) => {
                    tracing::warn!(plugin = %plugin.key, ?problems, "the plugin site pages are invalid")
                }
            }
        }
        let bundles = Arc::new(bundles);
        if let Ok(mut cached) = BUNDLES.write() {
            *cached = Some((snapshot.version, bundles.clone()));
        }
        Ok(bundles.as_ref().clone())
    }
}
