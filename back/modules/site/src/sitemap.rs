use std::collections::BTreeMap;

use aphel_contract_site::{PageSource, SITEMAP_PATH, SiteExposure};
use aphel_kernel::errors::AppError;
use aphel_sdk::platform::api::state::AppState;
use axum::http::request::Parts;
use axum::http::{HeaderValue, StatusCode, header};
use axum::response::{IntoResponse, Response};
use serde_json::Value;

use crate::cache;
use crate::data::{self, Restriction};
use crate::prefixes;
use crate::runtime::{Route, SiteRuntime};
use crate::settings::site_settings;

const MAX_URLS: usize = 50_000;
const BATCH: u64 = 500;
const SITEMAP_CACHE_CONTROL: &str = "public, max-age=3600";
const RECORD_ID_SORT: &str = "id";

struct SitemapUrl {
    location: String,
    modified: Option<String>,
}

fn escape_xml(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

fn day_of(value: Option<&Value>) -> Option<String> {
    value
        .and_then(Value::as_str)
        .map(|text| text.chars().take(10).collect::<String>())
        .filter(|day| day.len() == 10)
}

fn value_text(value: Option<&Value>) -> Option<String> {
    match value? {
        Value::String(text) if !text.is_empty() => Some(text.clone()),
        Value::Number(number) => Some(number.to_string()),
        _ => None,
    }
}

async fn record_urls(
    state: &AppState,
    exposure: &SiteExposure,
    route: &Route,
    match_field: &str,
    restriction: Restriction<'_>,
    base: &str,
    urls: &mut Vec<SitemapUrl>,
) -> Result<(), AppError> {
    let mut page = 1;
    loop {
        let listed = data::list(
            state,
            exposure,
            Some(RECORD_ID_SORT),
            page,
            BATCH,
            restriction,
            None,
        )
        .await?;
        for record in &listed.records {
            let Some(value) = value_text(record.get(match_field)) else {
                continue;
            };
            if let Some(path) = route
                .pattern
                .build(&BTreeMap::from([(match_field.to_owned(), value)]))
            {
                urls.push(SitemapUrl {
                    location: joined(base, &path),
                    modified: day_of(record.get("updated_at")),
                });
            }
        }
        if listed.records.len() < BATCH as usize
            || page * BATCH >= listed.total
            || urls.len() >= MAX_URLS
        {
            return Ok(());
        }
        page += 1;
    }
}

async fn page_params(
    state: &AppState,
    route: &Route,
    tags: &mut Vec<String>,
) -> Result<Vec<BTreeMap<String, String>>, AppError> {
    let Some(page_term) = &route.page.term else {
        return Ok(vec![BTreeMap::new()]);
    };
    tags.extend(crate::terms::cache_tags());
    Ok(crate::terms::slugs(state, page_term)
        .await?
        .into_iter()
        .map(|slug| BTreeMap::from([(aphel_contract_site::TERM_PARAM.to_owned(), slug)]))
        .collect())
}

fn joined(base: &str, path: &str) -> String {
    if base.is_empty() {
        return path.to_owned();
    }
    format!("{base}{}", path.trim_end_matches('/'))
}

async fn prefixed_urls(
    state: &AppState,
    runtime: &SiteRuntime,
    urls: &mut Vec<SitemapUrl>,
    tags: &mut Vec<String>,
) -> Result<(), AppError> {
    for route in &runtime.prefixed {
        let Some(prefix) = &route.page.prefix else {
            continue;
        };
        if route.page.seo.noindex || route.page.needs_customer() {
            continue;
        }
        tags.push(cache::entity_tag(&prefix.entity));
        for candidate in prefixes::candidates(state, runtime, &route.owner, prefix).await? {
            if urls.len() >= MAX_URLS {
                return Ok(());
            }
            let (Some(segment), Some(id)) = (
                value_text(candidate.get(&prefix.field)),
                candidate.get("id").and_then(Value::as_str),
            ) else {
                continue;
            };
            let base = format!("/{segment}");
            match &route.page.source {
                PageSource::Record {
                    entity,
                    match_field,
                } => {
                    let Some(exposure) = runtime.exposure(&route.owner, entity) else {
                        continue;
                    };
                    tags.push(cache::entity_tag(entity));
                    record_urls(
                        state,
                        exposure,
                        route,
                        match_field,
                        Some((prefix.scope.as_str(), id)),
                        &base,
                        urls,
                    )
                    .await?;
                }
                _ => {
                    for params in page_params(state, route, tags).await? {
                        if let Some(path) = route.pattern.build(&params) {
                            urls.push(SitemapUrl {
                                location: joined(&base, &path),
                                modified: day_of(candidate.get("updated_at")),
                            });
                        }
                    }
                }
            }
        }
    }
    Ok(())
}

async fn collect(
    state: &AppState,
    runtime: &SiteRuntime,
    base_url: &str,
) -> Result<(Vec<SitemapUrl>, Vec<String>), AppError> {
    let mut urls = Vec::new();
    let mut tags = Vec::new();
    let home = crate::settings::site_settings(&state.organization_config.snapshot()).home;
    for route in runtime.effective_routes(home.as_deref()) {
        if route.page.seo.noindex || route.page.needs_customer() || urls.len() >= MAX_URLS {
            continue;
        }
        match &route.page.source {
            PageSource::Static => {
                if let Some(path) = route.pattern.build(&BTreeMap::new()) {
                    urls.push(SitemapUrl {
                        location: format!("{base_url}{path}"),
                        modified: None,
                    });
                }
            }
            PageSource::Single { entity, conditions } => {
                let Some(exposure) = runtime.exposure(&route.owner, entity) else {
                    continue;
                };
                tags.push(cache::entity_tag(entity));
                let found = data::single(state, exposure, conditions, None).await?;
                if let (Some(record), Some(path)) = (found, route.pattern.build(&BTreeMap::new())) {
                    urls.push(SitemapUrl {
                        location: format!("{base_url}{path}"),
                        modified: day_of(record.get("updated_at")),
                    });
                }
            }
            PageSource::List { entity, .. } => {
                tags.push(cache::entity_tag(entity));
                for params in page_params(state, route, &mut tags).await? {
                    if let Some(path) = route.pattern.build(&params) {
                        urls.push(SitemapUrl {
                            location: format!("{base_url}{path}"),
                            modified: None,
                        });
                    }
                }
            }
            PageSource::Record {
                entity,
                match_field,
            } => {
                let Some(exposure) = runtime.exposure(&route.owner, entity) else {
                    continue;
                };
                tags.push(cache::entity_tag(entity));
                record_urls(state, exposure, route, match_field, None, "", &mut urls).await?;
            }
        }
    }
    prefixed_urls(state, runtime, &mut urls, &mut tags).await?;
    urls.truncate(MAX_URLS);
    Ok((urls, tags))
}

fn render(urls: &[SitemapUrl]) -> String {
    let mut xml = String::from(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<urlset xmlns=\"http://www.sitemaps.org/schemas/sitemap/0.9\">\n",
    );
    for url in urls {
        xml.push_str("  <url><loc>");
        xml.push_str(&escape_xml(&url.location));
        xml.push_str("</loc>");
        if let Some(modified) = &url.modified {
            xml.push_str("<lastmod>");
            xml.push_str(&escape_xml(modified));
            xml.push_str("</lastmod>");
        }
        xml.push_str("</url>\n");
    }
    xml.push_str("</urlset>\n");
    xml
}

fn text_response(content_type: &'static str, body: String, etag: Option<&str>) -> Response {
    let mut response = (StatusCode::OK, body).into_response();
    let headers = response.headers_mut();
    headers.insert(header::CONTENT_TYPE, HeaderValue::from_static(content_type));
    headers.insert(
        header::CACHE_CONTROL,
        HeaderValue::from_static(SITEMAP_CACHE_CONTROL),
    );
    if let Some(value) = etag.and_then(|etag| HeaderValue::from_str(etag).ok()) {
        headers.insert(header::ETAG, value);
    }
    response
}

pub(crate) async fn respond(
    state: &AppState,
    runtime: &SiteRuntime,
    parts: &Parts,
    prefix: &str,
) -> Response {
    let config = state.organization_config.snapshot();
    if !site_settings(&config).indexing {
        return StatusCode::NOT_FOUND.into_response();
    }
    let host = parts
        .headers
        .get(header::HOST)
        .and_then(|value| value.to_str().ok())
        .unwrap_or_default()
        .to_ascii_lowercase();
    let key = format!("sitemap|{host}|{prefix}|{}", runtime.revision());
    if let Ok(pool) = state.db()
        && let Err(error) = cache::sync(&pool).await
    {
        tracing::warn!(%error, "could not read the site cache invalidations");
        cache::forget_all();
    }
    if let Some(cached) = cache::get(&key) {
        return text_response(
            "application/xml; charset=utf-8",
            cached.html.to_string(),
            Some(&cached.etag),
        );
    }
    let base_url = config.site_base_url().trim_end_matches('/').to_owned();
    let bases: Vec<String> = std::iter::once(base_url.clone())
        .chain(
            site_settings(&config)
                .languages
                .iter()
                .map(|language| format!("{base_url}/{}", crate::settings::language_slug(language))),
        )
        .collect();
    match collect(state, runtime, "").await {
        Ok((paths, tags)) => {
            let urls: Vec<SitemapUrl> = bases
                .iter()
                .flat_map(|base| {
                    paths.iter().map(move |url| SitemapUrl {
                        location: format!("{base}{}", url.location),
                        modified: url.modified.clone(),
                    })
                })
                .take(MAX_URLS)
                .collect();
            let cached = cache::put(key, cache::page(render(&urls), String::new()), tags, None);
            text_response(
                "application/xml; charset=utf-8",
                cached.html.to_string(),
                Some(&cached.etag),
            )
        }
        Err(error) => {
            tracing::warn!(%error, "could not build the sitemap");
            StatusCode::SERVICE_UNAVAILABLE.into_response()
        }
    }
}

pub(crate) fn robots(state: &AppState) -> Response {
    let config = state.organization_config.snapshot();
    let mut body = String::from("User-agent: *\n");
    if site_settings(&config).indexing {
        body.push_str("Disallow: /api/\nDisallow: /_aphel/app/\n");
        if !config.app_prefix().is_empty() {
            body.push_str(&format!("Disallow: {}/\n", config.app_prefix()));
        }
        body.push_str(&format!(
            "\nSitemap: {}{SITEMAP_PATH}\n",
            config.site_base_url().trim_end_matches('/')
        ));
    } else {
        body.push_str("Disallow: /\n");
    }
    text_response("text/plain; charset=utf-8", body, None)
}
