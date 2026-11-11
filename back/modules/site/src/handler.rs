use std::collections::BTreeMap;
use std::sync::LazyLock;
use std::time::Duration;

use aphel_contract_site::{
    MAX_PER_PAGE, PageSource, ROBOTS_PATH, SITEMAP_PATH, decode_segment, template_name,
};
use aphel_kernel::errors::AppError;
use aphel_kernel::modules::{SiteMountPrefix, SiteRequestHandler};
use aphel_sdk::platform::api::extractors::ClientInfo;
use aphel_sdk::platform::api::rate_limit::SharedRateLimiter;
use aphel_sdk::platform::api::state::AppState;
use aphel_sdk::platform::application::organization_profile::OrganizationRepository;
use aphel_sdk::platform::infrastructure::repositories::mysql_organization_profile::MySqlOrganizationRepository;
use axum::extract::{FromRequestParts, Request};
use axum::http::request::Parts;
use axum::http::{HeaderMap, HeaderValue, Method, StatusCode, header};
use axum::response::{IntoResponse, Response};
use futures::future::BoxFuture;
use minijinja::Value;
use serde_json::json;

use crate::assets::LOGO_PATH;
use crate::cache::{self, CachedPage};
use crate::dynamic::{API_SEGMENT, ISLANDS_LOADER_PATH};
use crate::runtime::{
    self, ERROR_TEMPLATE, ISLANDS_SEGMENT, NOT_FOUND_TEMPLATE, Route, SiteRuntime, THEMES_SEGMENT,
};
use crate::settings::site_settings;
use crate::{assets, data, dynamic, images, redirects, sitemap};

const LIVE_DATA_LIFETIME: Duration = Duration::from_secs(60);
const DEFAULT_COLOR: &str = "#4f46e5";
const PAGE_CACHE_CONTROL: &str = "public, max-age=60, stale-while-revalidate=600";
const RENDERS_PER_ADDRESS: u32 = 120;
const RENDER_WINDOW: Duration = Duration::from_secs(60);
const DESCRIPTION_MAX_CHARS: usize = 200;
const RATE_LIMITED: &str = "muitas páginas em sequência; aguarde um pouco e tente novamente";

static RENDER_LIMIT: LazyLock<SharedRateLimiter> = LazyLock::new(|| {
    SharedRateLimiter::new(
        "site.render_per_address",
        RENDERS_PER_ADDRESS,
        RENDER_WINDOW,
    )
});

pub(crate) struct SiteHandler;

impl SiteRequestHandler for SiteHandler {
    fn serve<'a>(&'a self, state: &'a AppState, request: Request) -> BoxFuture<'a, Response> {
        Box::pin(serve(state, request))
    }

    fn serve_reserved<'a>(
        &'a self,
        state: &'a AppState,
        request: Request,
    ) -> BoxFuture<'a, Response> {
        Box::pin(serve_reserved(state, request))
    }
}

struct SiteContext {
    site: serde_json::Value,
}

enum Outcome {
    Page {
        html: String,
        tags: Vec<String>,
        live: bool,
    },
    Redirect(String),
    NotFound,
}

fn readable_method(method: &Method) -> bool {
    method == Method::GET || method == Method::HEAD
}

async fn serve_reserved(state: &AppState, request: Request) -> Response {
    if let Some(rest) = request
        .uri()
        .path()
        .strip_prefix(&format!("/{API_SEGMENT}/"))
        .map(str::to_owned)
    {
        return dynamic::api(state, &rest, request).await;
    }
    if !readable_method(request.method()) {
        return StatusCode::METHOD_NOT_ALLOWED.into_response();
    }
    if request.uri().path() == ISLANDS_LOADER_PATH {
        return dynamic::islands_loader();
    }
    if let Some(rest) = request
        .uri()
        .path()
        .strip_prefix(&format!("/{ISLANDS_SEGMENT}/"))
        .map(str::to_owned)
    {
        return dynamic::island(state, &rest).await;
    }
    let theme = site_settings(&state.organization_config.snapshot()).theme;
    let runtime = match runtime::current(&state.app, theme.as_deref()).await {
        Ok(runtime) => runtime,
        Err(error) => {
            tracing::error!(%error, "could not load the site pages");
            return StatusCode::SERVICE_UNAVAILABLE.into_response();
        }
    };
    let path = request.uri().path().to_owned();
    if path == LOGO_PATH {
        return assets::logo(state).await;
    }
    if let Some(rest) = path.strip_prefix(&format!("/{THEMES_SEGMENT}/")) {
        return assets::theme_file(state, rest).await;
    }
    match images::serve(state, &runtime, request.uri().path(), request.headers()).await {
        Ok(response) => response,
        Err(error) => {
            tracing::warn!(%error, path = %request.uri().path(), "could not serve a site file");
            StatusCode::NOT_FOUND.into_response()
        }
    }
}

fn requested_page(query: &str) -> u64 {
    query
        .split('&')
        .filter_map(|pair| pair.split_once('='))
        .find(|(name, _)| *name == "page")
        .and_then(|(_, value)| value.parse::<u64>().ok())
        .filter(|page| *page >= 1)
        .unwrap_or(1)
}

fn cache_key(parts: &Parts, prefix: &str, runtime: &SiteRuntime, route: &Route) -> String {
    let host = parts
        .headers
        .get(header::HOST)
        .and_then(|value| value.to_str().ok())
        .unwrap_or_default()
        .to_ascii_lowercase();
    let page = match route.page.source {
        PageSource::List { .. } => requested_page(parts.uri.query().unwrap_or_default()),
        _ => 1,
    };
    format!(
        "{host}|{prefix}|{}|{}|{page}",
        runtime.revision(),
        parts.uri.path()
    )
}

async fn serve(state: &AppState, request: Request) -> Response {
    if !readable_method(request.method()) {
        return StatusCode::METHOD_NOT_ALLOWED.into_response();
    }
    let (mut parts, _) = request.into_parts();
    let prefix = parts
        .extensions
        .get::<SiteMountPrefix>()
        .map(|prefix| prefix.0.clone())
        .unwrap_or_default();
    let theme = site_settings(&state.organization_config.snapshot()).theme;
    let runtime = match runtime::current(&state.app, theme.as_deref()).await {
        Ok(runtime) => runtime,
        Err(error) => {
            tracing::error!(%error, "could not load the site pages");
            return StatusCode::SERVICE_UNAVAILABLE.into_response();
        }
    };
    let path = parts.uri.path().to_owned();
    if path == SITEMAP_PATH {
        return sitemap::respond(state, &runtime, &parts, &prefix).await;
    }
    if path == ROBOTS_PATH && prefix.is_empty() {
        return sitemap::robots(state);
    }
    let found = runtime.find(parts.uri.path());
    let key = found
        .as_ref()
        .map(|(route, _)| cache_key(&parts, &prefix, &runtime, route));
    if let Some(key) = &key {
        if let Ok(pool) = state.db()
            && let Err(error) = cache::sync(&pool).await
        {
            tracing::warn!(%error, "could not read the site cache invalidations");
        }
        if let Some(page) = cache::get(key) {
            return page_response(&page, &parts.headers);
        }
    }
    let client = ClientInfo::from_request_parts(&mut parts, state).await.ok();
    let address = client
        .as_ref()
        .map(ClientInfo::ip_or_unknown)
        .unwrap_or("unknown");
    if RENDER_LIMIT.check(address, RATE_LIMITED).await.is_err() {
        return (StatusCode::TOO_MANY_REQUESTS, RATE_LIMITED).into_response();
    }
    let context = match site_context(state, &prefix, &runtime, &path).await {
        Ok(context) => context,
        Err(error) => {
            tracing::error!(%error, "could not load the site context");
            return StatusCode::SERVICE_UNAVAILABLE.into_response();
        }
    };
    let Some((route, params)) = found else {
        return fallback(&context, StatusCode::NOT_FOUND, Some(&runtime));
    };
    let query = parts.uri.query().unwrap_or_default();
    let owner = route.owner.clone();
    match render_route(
        state,
        &runtime,
        &context,
        route,
        params,
        parts.uri.path(),
        query,
    )
    .await
    {
        Ok(Outcome::Page { html, tags, live }) => {
            let page = cache::page(html, runtime.content_security_policy(Some(&owner)));
            let page = match key {
                Some(key) => cache::put(key, page, tags, live.then_some(LIVE_DATA_LIFETIME)),
                None => page,
            };
            page_response(&page, &parts.headers)
        }
        Ok(Outcome::Redirect(location)) => moved_permanently(&location),
        Ok(Outcome::NotFound) => fallback(&context, StatusCode::NOT_FOUND, Some(&runtime)),
        Err(error) => {
            tracing::warn!(%error, path = %parts.uri.path(), "could not render a site page");
            fallback(&context, StatusCode::INTERNAL_SERVER_ERROR, Some(&runtime))
        }
    }
}

fn texts(locale: &str) -> serde_json::Value {
    if locale.starts_with("pt") {
        json!({
            "not_found_title": "Página não encontrada",
            "not_found_body": "O endereço não existe ou o conteúdo não está mais publicado.",
            "error_title": "Algo deu errado",
            "error_body": "Não foi possível mostrar esta página agora. Tente de novo em instantes.",
            "back_home": "Voltar ao início",
            "previous": "Anterior",
            "next": "Próxima",
            "menu": "Menu principal",
        })
    } else {
        json!({
            "not_found_title": "Page not found",
            "not_found_body": "The address does not exist or the content is no longer published.",
            "error_title": "Something went wrong",
            "error_body": "This page could not be shown right now. Try again in a moment.",
            "back_home": "Back to the home page",
            "previous": "Previous",
            "next": "Next",
            "menu": "Main menu",
        })
    }
}

fn safe_color(color: Option<&str>) -> &str {
    color
        .filter(|color| {
            color.len() <= 9
                && color.starts_with('#')
                && color[1..]
                    .chars()
                    .all(|character| character.is_ascii_hexdigit())
        })
        .unwrap_or(DEFAULT_COLOR)
}

fn menu_of(
    runtime: &SiteRuntime,
    settings: &crate::settings::SiteSettings,
    prefix: &str,
    current: &str,
) -> Vec<serde_json::Value> {
    let position = |id: &str| {
        settings
            .menu
            .iter()
            .position(|entry| entry.item == id)
            .unwrap_or(usize::MAX)
    };
    let mut items: Vec<_> = runtime
        .menu
        .iter()
        .filter(|item| {
            !settings
                .menu
                .iter()
                .any(|entry| entry.item == item.id && entry.hidden)
        })
        .enumerate()
        .collect();
    items.sort_by_key(|(index, item)| (position(&item.id), *index));
    items
        .into_iter()
        .map(|(_, item)| {
            let active = current == item.path
                || (item.path != "/" && current.starts_with(&format!("{}/", item.path)));
            json!({
                "label": item.label,
                "url": format!("{prefix}{}", item.path),
                "active": active,
            })
        })
        .collect()
}

async fn site_context(
    state: &AppState,
    prefix: &str,
    runtime: &SiteRuntime,
    current: &str,
) -> Result<SiteContext, AppError> {
    let config = state.organization_config.snapshot();
    let settings = site_settings(&config);
    let organization = MySqlOrganizationRepository::new(state.db()?)
        .find()
        .await?
        .ok_or_else(|| AppError::NotFound("the organization is not installed".to_owned()))?;
    Ok(SiteContext {
        site: json!({
            "name": organization.name.as_str(),
            "locale": organization.locale,
            "color": safe_color(organization.primary_color.as_deref()),
            "prefix": prefix,
            "home": format!("{prefix}/"),
            "year": chrono::Utc::now().format("%Y").to_string(),
            "texts": texts(&organization.locale),
            "base_url": config.site_base_url().trim_end_matches('/'),
            "indexing": settings.indexing,
            "tagline": settings.tagline.as_deref().map(str::trim).filter(|tagline| !tagline.is_empty()),
            "menu": menu_of(runtime, &settings, prefix, current),
            "logo_url": organization
                .logo_file_id
                .as_ref()
                .map(|file| format!("/_aphel/site{LOGO_PATH}?v={file}")),
        }),
    })
}

async fn render_route(
    state: &AppState,
    runtime: &SiteRuntime,
    context: &SiteContext,
    route: &Route,
    params: BTreeMap<String, String>,
    path: &str,
    query: &str,
) -> Result<Outcome, AppError> {
    let params: BTreeMap<String, String> = params
        .into_iter()
        .map(|(name, value)| (name, decode_segment(&value)))
        .collect();
    let mut values = serde_json::Map::new();
    values.insert("site".to_owned(), context.site.clone());
    let mut page = json!({
        "owner": route.owner,
        "key": route.page.key,
        "path": path,
        "params": params,
    });
    let mut tags = Vec::new();
    let mut live = false;
    if let Some(data) = &route.page.data {
        let path = aphel_contract_site::PathPattern::parse(data)
            .ok()
            .and_then(|pattern| pattern.build(&params))
            .ok_or_else(|| AppError::Internal(format!("the data route {data} cannot be built")))?;
        let Some(loaded) = dynamic::page_data(state, &route.owner, &path).await? else {
            return Ok(Outcome::NotFound);
        };
        values.insert("data".to_owned(), loaded);
        live = true;
    }
    match &route.page.source {
        PageSource::Static => {}
        PageSource::Record {
            entity,
            match_field,
        } => {
            let exposure = runtime
                .exposure(&route.owner, entity)
                .ok_or_else(|| AppError::Internal(format!("{entity} is not public")))?;
            let value = params.get(match_field).cloned().unwrap_or_default();
            let Some(mut record) = data::record(state, exposure, match_field, &value).await? else {
                let moved = redirects::lookup(&state.db()?, entity, match_field, &value).await?;
                let location = moved.and_then(|moved| {
                    route
                        .pattern
                        .build(&BTreeMap::from([(match_field.clone(), moved)]))
                });
                return Ok(match location {
                    Some(location) => Outcome::Redirect(format!(
                        "{}{location}",
                        context.site["prefix"].as_str().unwrap_or_default()
                    )),
                    None => Outcome::NotFound,
                });
            };
            if let Some(id) = record.get("id").and_then(serde_json::Value::as_str) {
                tags.push(cache::record_tag(entity, id));
            }
            images::attach_images(state, exposure, &mut record).await?;
            values.insert("record".to_owned(), record);
        }
        PageSource::List {
            entity,
            sort,
            per_page,
        } => {
            let exposure = runtime
                .exposure(&route.owner, entity)
                .ok_or_else(|| AppError::Internal(format!("{entity} is not public")))?;
            let per_page = (*per_page).clamp(1, MAX_PER_PAGE);
            let current = requested_page(query);
            let mut listed =
                data::list(state, exposure, sort.as_deref(), current, per_page).await?;
            let pages = listed.total.div_ceil(per_page).max(1);
            if current > pages {
                return Ok(Outcome::NotFound);
            }
            tags.push(cache::entity_tag(entity));
            for record in &mut listed.records {
                images::attach_images(state, exposure, record).await?;
            }
            let base = format!(
                "{}{}",
                context.site["prefix"].as_str().unwrap_or_default(),
                path.trim_end_matches('/')
            );
            let link = |number: u64| match number {
                1 => base.clone(),
                number => format!("{base}?page={number}"),
            };
            values.insert("records".to_owned(), json!(listed.records));
            values.insert(
                "pagination".to_owned(),
                json!({
                    "page": current,
                    "pages": pages,
                    "total": listed.total,
                    "previous_url": (current > 1).then(|| link(current - 1)),
                    "next_url": (current < pages).then(|| link(current + 1)),
                }),
            );
        }
    }
    values.insert("page".to_owned(), page.clone());
    let render = |expression: &Option<String>| -> Result<Option<String>, AppError> {
        expression
            .as_ref()
            .map(|expression| {
                runtime
                    .environment
                    .render_str(expression, Value::from_serialize(&values))
                    .map_err(render_error)
            })
            .transpose()
    };
    let title = render(&route.page.title)?;
    let description = render(&route.page.seo.description)?
        .map(|description| plain_summary(&description))
        .filter(|description| !description.is_empty());
    let image = render(&route.page.seo.image)?
        .map(|image| image.trim().to_owned())
        .filter(|image| !image.is_empty());
    let base_url = context.site["base_url"].as_str().unwrap_or_default();
    let canonical = match requested_page(query) {
        number if number > 1 && matches!(route.page.source, PageSource::List { .. }) => {
            format!("{base_url}{path}?page={number}")
        }
        _ => format!("{base_url}{path}"),
    };
    let indexing = context.site["indexing"].as_bool().unwrap_or(true);
    page["title"] = json!(title);
    page["seo"] = json!({
        "canonical": canonical,
        "description": description,
        "image": image.map(|image| absolute_url(base_url, &image)),
        "noindex": route.page.seo.noindex || !indexing,
        "type": if matches!(route.page.source, PageSource::Record { .. }) { "article" } else { "website" },
    });
    values.insert("page".to_owned(), page);
    let html = runtime
        .environment
        .get_template(&template_name(&route.owner, &route.page.template))
        .and_then(|template| template.render(Value::from_serialize(&values)))
        .map_err(render_error)?;
    Ok(Outcome::Page { html, tags, live })
}

fn plain_summary(text: &str) -> String {
    let collapsed = text.split_whitespace().collect::<Vec<_>>().join(" ");
    if collapsed.chars().count() <= DESCRIPTION_MAX_CHARS {
        return collapsed;
    }
    let cut: String = collapsed.chars().take(DESCRIPTION_MAX_CHARS - 1).collect();
    format!("{}…", cut.trim_end())
}

pub(crate) fn origin_of(base_url: &str) -> &str {
    let after_scheme = base_url.find("://").map(|index| index + 3).unwrap_or(0);
    match base_url[after_scheme..].find('/') {
        Some(index) => &base_url[..after_scheme + index],
        None => base_url,
    }
}

fn absolute_url(base_url: &str, url: &str) -> String {
    if url.starts_with("http://") || url.starts_with("https://") {
        url.to_owned()
    } else if url.starts_with('/') {
        format!("{}{url}", origin_of(base_url))
    } else {
        format!("{base_url}/{url}")
    }
}

fn moved_permanently(location: &str) -> Response {
    let mut response = StatusCode::MOVED_PERMANENTLY.into_response();
    if let Ok(value) = HeaderValue::from_str(location) {
        response.headers_mut().insert(header::LOCATION, value);
    }
    response.headers_mut().insert(
        header::CACHE_CONTROL,
        HeaderValue::from_static("public, max-age=3600"),
    );
    response
}

fn render_error(error: minijinja::Error) -> AppError {
    AppError::Internal(format!("template error: {error:#}"))
}

fn fallback(context: &SiteContext, status: StatusCode, runtime: Option<&SiteRuntime>) -> Response {
    let template = if status == StatusCode::NOT_FOUND {
        NOT_FOUND_TEMPLATE
    } else {
        ERROR_TEMPLATE
    };
    let values = json!({ "site": context.site, "page": {} });
    let rendered = runtime.and_then(|runtime| {
        runtime
            .environment
            .get_template(template)
            .and_then(|template| template.render(Value::from_serialize(&values)))
            .ok()
            .map(|html| (html, runtime.content_security_policy(None)))
    });
    match rendered {
        Some((html, csp)) => {
            let mut response = html_response(status, html, &csp);
            response
                .headers_mut()
                .insert(header::CACHE_CONTROL, HeaderValue::from_static("no-store"));
            response
        }
        None => status.into_response(),
    }
}

fn page_response(page: &CachedPage, request_headers: &HeaderMap) -> Response {
    let matches = request_headers
        .get(header::IF_NONE_MATCH)
        .and_then(|value| value.to_str().ok())
        .is_some_and(|value| {
            value
                .split(',')
                .any(|candidate| candidate.trim() == page.etag)
        });
    let mut response = if matches {
        StatusCode::NOT_MODIFIED.into_response()
    } else {
        html_response(StatusCode::OK, page.html.to_string(), &page.csp)
    };
    let headers = response.headers_mut();
    if let Ok(etag) = HeaderValue::from_str(&page.etag) {
        headers.insert(header::ETAG, etag);
    }
    headers.insert(
        header::CACHE_CONTROL,
        HeaderValue::from_static(PAGE_CACHE_CONTROL),
    );
    response
}

fn html_response(status: StatusCode, html: String, csp: &str) -> Response {
    let mut response = (status, html).into_response();
    let headers = response.headers_mut();
    headers.insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static("text/html; charset=utf-8"),
    );
    if let Ok(value) = HeaderValue::from_str(csp) {
        headers.insert(header::CONTENT_SECURITY_POLICY, value);
    }
    headers.insert(
        header::X_CONTENT_TYPE_OPTIONS,
        HeaderValue::from_static("nosniff"),
    );
    headers.insert(
        header::REFERRER_POLICY,
        HeaderValue::from_static("strict-origin-when-cross-origin"),
    );
    response
}
