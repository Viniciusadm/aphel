use std::collections::BTreeMap;
use std::sync::LazyLock;
use std::time::Duration;

use aphel_contract_site::SiteCustomer;
use aphel_contract_site::{
    MAX_MENU_RECORDS, MAX_PER_PAGE, PageSource, PathPattern, ROBOTS_PATH, SITEMAP_PATH,
    decode_segment, menu_record_item_id, template_name,
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
use crate::{
    account, assets, data, deferred, dynamic, images, preview, redirects, session, sitemap,
};

const LIVE_DATA_LIFETIME: Duration = Duration::from_secs(60);
const DEFAULT_COLOR: &str = "#4f46e5";
const PAGE_CACHE_CONTROL: &str = "public, max-age=60, stale-while-revalidate=600";
const RENDERS_PER_ADDRESS: u32 = 120;
const RENDER_WINDOW: Duration = Duration::from_secs(60);
const DESCRIPTION_MAX_CHARS: usize = 200;
const MAX_MENU_LABEL_CHARS: usize = 60;
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
    customer: Option<SiteCustomer>,
    suffix: Option<String>,
    tags: Vec<String>,
}

pub(crate) struct RecordMenuLine {
    pub id: String,
    pub parent: String,
    pub label: String,
    pub path: String,
}

fn value_text(value: Option<&serde_json::Value>) -> Option<String> {
    match value? {
        serde_json::Value::String(text) if !text.trim().is_empty() => Some(text.trim().to_owned()),
        serde_json::Value::Number(number) => Some(number.to_string()),
        _ => None,
    }
}

pub(crate) async fn record_menu(
    state: &AppState,
    runtime: &SiteRuntime,
    suffix: Option<&str>,
) -> Result<(Vec<RecordMenuLine>, Vec<String>), AppError> {
    let mut lines = Vec::new();
    let mut tags = Vec::new();
    for candidate in &runtime.menu {
        let Some(records) = &candidate.item.records else {
            continue;
        };
        let PageSource::Record {
            entity,
            match_field,
        } = &candidate.source
        else {
            continue;
        };
        let (Some(exposure), Ok(pattern)) = (
            runtime.exposure(&candidate.owner, entity),
            PathPattern::parse(&candidate.path),
        ) else {
            continue;
        };
        tags.push(cache::entity_tag(entity));
        let found = data::matching(
            state,
            exposure,
            &records.conditions,
            records.sort.as_deref(),
            MAX_MENU_RECORDS,
            suffix,
        )
        .await?;
        for record in found {
            let (Some(id), Some(label), Some(value)) = (
                value_text(record.get("id")),
                value_text(record.get(&records.label_field)),
                value_text(record.get(match_field)),
            ) else {
                continue;
            };
            let Some(path) = pattern.build(&BTreeMap::from([(match_field.clone(), value)])) else {
                continue;
            };
            lines.push(RecordMenuLine {
                id: menu_record_item_id(&candidate.owner, &candidate.item.key, &id),
                parent: candidate.id.clone(),
                label: label.chars().take(MAX_MENU_LABEL_CHARS).collect(),
                path,
            });
        }
    }
    Ok((lines, tags))
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
        .strip_prefix(&format!("/{}/", preview::PREVIEW_SEGMENT))
        .map(str::to_owned)
    {
        return preview::serve(state, &rest).await;
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

fn cache_key(parts: &Parts, path: &str, runtime: &SiteRuntime, route: &Route) -> String {
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
    format!("{host}|{}|{path}|{page}", runtime.revision())
}

const ACCOUNT_BODY_LIMIT: usize = 64 * 1024;
const ACCOUNT_ROOTS: [&str; 2] = ["conta", "account"];

async fn serve(state: &AppState, request: Request) -> Response {
    let (mut parts, body) = request.into_parts();
    let prefix = parts
        .extensions
        .get::<SiteMountPrefix>()
        .map(|prefix| prefix.0.clone())
        .unwrap_or_default();
    let config = state.organization_config.snapshot();
    let settings = site_settings(&config);
    let previewing = preview::active(state, &parts.headers).await;
    let theme = match &previewing {
        Some(previewing) if previewing.record.is_none() => previewing.theme.clone(),
        _ => settings.theme.clone(),
    };
    let runtime = match runtime::current(&state.app, theme.as_deref()).await {
        Ok(runtime) => runtime,
        Err(error) => {
            tracing::error!(%error, "could not load the site pages");
            return StatusCode::SERVICE_UNAVAILABLE.into_response();
        }
    };
    let accounts = settings.accounts || runtime.uses_customers;
    let full_path = parts.uri.path().to_owned();
    if readable_method(&parts.method) && full_path == SITEMAP_PATH {
        return sitemap::respond(state, &runtime, &parts, &prefix).await;
    }
    if readable_method(&parts.method) && full_path == ROBOTS_PATH && prefix.is_empty() {
        return sitemap::robots(state);
    }
    let mount = prefix.clone();
    let (language, path) = Language::resolve(&prefix, &full_path, &settings.languages);
    let prefix = language.prefix();
    let customer = match accounts {
        true => session::customer(state, &parts.headers).await,
        false => None,
    };
    let client = ClientInfo::from_request_parts(&mut parts, state)
        .await
        .ok()
        .and_then(|client| client.0.ip);
    let first_segment = path
        .trim_start_matches('/')
        .split('/')
        .next()
        .unwrap_or_default();
    if accounts && ACCOUNT_ROOTS.contains(&first_segment) {
        let context = match site_context(
            state,
            &language,
            &runtime,
            &path,
            customer.as_ref(),
            accounts,
        )
        .await
        {
            Ok(context) => context,
            Err(error) => {
                tracing::error!(%error, "could not load the site context");
                return StatusCode::SERVICE_UNAVAILABLE.into_response();
            }
        };
        let locale = context.site["locale"]
            .as_str()
            .unwrap_or_default()
            .to_owned();
        let paths = account::paths(&locale);
        let Some(segments) = account::segments(&paths, &path) else {
            return fallback(&context, StatusCode::NOT_FOUND, Some(&runtime));
        };
        let Ok(body) = axum::body::to_bytes(body, ACCOUNT_BODY_LIMIT).await else {
            return StatusCode::PAYLOAD_TOO_LARGE.into_response();
        };
        let links = runtime
            .account_links
            .iter()
            .map(|link| json!({ "label": link.label(&locale), "url": format!("{prefix}{}", link.path) }))
            .collect();
        let segments: Vec<&str> = segments;
        return account::handle(
            account::AccountRequest {
                state,
                runtime: &runtime,
                site: context.site.clone(),
                locale,
                prefix: prefix.clone(),
                base_url: config.site_base_url(),
                parts,
                body,
                customer,
                client,
                links,
            },
            segments,
        )
        .await;
    }
    if !readable_method(&parts.method) {
        return StatusCode::METHOD_NOT_ALLOWED.into_response();
    }
    let found = runtime.find(&path, settings.home.as_deref());
    let personal = customer.is_some() || previewing.is_some();
    let key = found
        .as_ref()
        .filter(|_| !personal)
        .map(|(route, _)| cache_key(&parts, &full_path, &runtime, route));
    if let Some(key) = &key {
        if let Ok(pool) = state.db()
            && let Err(error) = cache::sync(&pool).await
        {
            tracing::warn!(%error, "could not read the site cache invalidations");
        }
        if let Some(page) = cache::get(key)
            && !found
                .as_ref()
                .is_some_and(|(route, _)| route.page.needs_customer())
        {
            return page_response(&page, &parts.headers, false);
        }
    }
    let address = client.as_deref().unwrap_or("unknown");
    if RENDER_LIMIT.check(address, RATE_LIMITED).await.is_err() {
        return (StatusCode::TOO_MANY_REQUESTS, RATE_LIMITED).into_response();
    }
    let context = match site_context(
        state,
        &language,
        &runtime,
        &path,
        customer.as_ref(),
        accounts,
    )
    .await
    {
        Ok(context) => context,
        Err(error) => {
            tracing::error!(%error, "could not load the site context");
            return StatusCode::SERVICE_UNAVAILABLE.into_response();
        }
    };
    let Some((route, params)) = found else {
        return not_found(
            state,
            &mount,
            &full_path,
            (&prefix, &path),
            &context,
            &runtime,
        )
        .await;
    };
    if route.page.needs_customer() && customer.is_none() {
        if !accounts {
            return fallback(&context, StatusCode::NOT_FOUND, Some(&runtime));
        }
        let locale = context.site["locale"].as_str().unwrap_or_default();
        let login = account::urls(&prefix, &account::paths(locale))["login"]
            .as_str()
            .unwrap_or_default()
            .to_owned();
        let current = match parts.uri.query() {
            Some(query) => format!("{prefix}{path}?{query}"),
            None => format!("{prefix}{path}"),
        };
        return see_other(&format!("{login}?next={}", account::urlencode(&current)));
    }
    let query = parts.uri.query().unwrap_or_default();
    let owner = route.owner.clone();
    let cacheable = !personal && !route.page.needs_customer();
    match render_route(
        state,
        &runtime,
        &context,
        RouteRequest {
            route,
            params,
            path: &path,
            query,
            preview: previewing.as_ref(),
        },
    )
    .await
    {
        Ok(Outcome::Page {
            html,
            mut tags,
            live,
        }) => {
            tags.extend(context.tags.iter().cloned());
            let locale = context.site["locale"].as_str().unwrap_or_default();
            let html = match &previewing {
                Some(previewing) if previewing.record.is_some() => {
                    preview::with_banner(html, preview::BannerSubject::Draft, locale)
                }
                Some(_) => preview::with_banner(
                    html,
                    preview::BannerSubject::Theme(&theme_name(&runtime)),
                    locale,
                ),
                None => html,
            };
            let page = cache::page(html, runtime.content_security_policy(Some(&owner)));
            let page = match key.filter(|_| cacheable) {
                Some(key) => cache::put(key, page, tags, live.then_some(LIVE_DATA_LIFETIME)),
                None => page,
            };
            page_response(&page, &parts.headers, !cacheable)
        }
        Ok(Outcome::Redirect(location)) => moved_permanently(&location),
        Ok(Outcome::NotFound) => {
            not_found(
                state,
                &mount,
                &full_path,
                (&prefix, &path),
                &context,
                &runtime,
            )
            .await
        }
        Err(error) => {
            tracing::warn!(%error, path = %parts.uri.path(), "could not render a site page");
            fallback(&context, StatusCode::INTERNAL_SERVER_ERROR, Some(&runtime))
        }
    }
}

fn theme_name(runtime: &SiteRuntime) -> String {
    runtime
        .theme
        .as_ref()
        .and_then(|key| runtime.themes.iter().find(|theme| &theme.key == key))
        .map(|theme| theme.name.clone())
        .unwrap_or_else(|| "Aphel".to_owned())
}

async fn not_found(
    state: &AppState,
    mount: &str,
    full_path: &str,
    (language_prefix, path): (&str, &str),
    context: &SiteContext,
    runtime: &SiteRuntime,
) -> Response {
    let found = match state.db() {
        Ok(pool) => match redirects::find_path(&pool, full_path).await {
            Ok(Some(redirect)) => Ok(Some((mount, redirect))),
            Ok(None) if path != full_path => redirects::find_path(&pool, path)
                .await
                .map(|redirect| redirect.map(|redirect| (language_prefix, redirect))),
            other => other.map(|redirect| redirect.map(|redirect| (mount, redirect))),
        },
        Err(error) => Err(error),
    };
    match found {
        Ok(Some((base, redirect))) => {
            let location = match redirect.target.starts_with('/') {
                true => format!("{base}{}", redirect.target),
                false => redirect.target,
            };
            match redirect.permanent {
                true => moved_permanently(&location),
                false => found_elsewhere(&location),
            }
        }
        Ok(None) => fallback(context, StatusCode::NOT_FOUND, Some(runtime)),
        Err(error) => {
            tracing::warn!(%error, "could not read the site redirects");
            fallback(context, StatusCode::NOT_FOUND, Some(runtime))
        }
    }
}

fn found_elsewhere(location: &str) -> Response {
    let mut response = StatusCode::FOUND.into_response();
    if let Ok(value) = HeaderValue::from_str(location) {
        response.headers_mut().insert(header::LOCATION, value);
    }
    response
        .headers_mut()
        .insert(header::CACHE_CONTROL, HeaderValue::from_static("no-store"));
    response
}

fn see_other(location: &str) -> Response {
    let mut response = StatusCode::SEE_OTHER.into_response();
    if let Ok(value) = HeaderValue::from_str(location) {
        response.headers_mut().insert(header::LOCATION, value);
    }
    response
        .headers_mut()
        .insert(header::CACHE_CONTROL, HeaderValue::from_static("no-store"));
    response
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
            "sign_in": "Entrar",
            "languages": "Idiomas",
            "my_account": "Minha conta",
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
            "sign_in": "Sign in",
            "languages": "Languages",
            "my_account": "My account",
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

pub(crate) struct Language {
    mount: String,
    slug: Option<String>,
    code: Option<String>,
    available: Vec<String>,
}

impl Language {
    fn resolve(mount: &str, path: &str, languages: &[String]) -> (Self, String) {
        let first = path
            .trim_start_matches('/')
            .split('/')
            .next()
            .unwrap_or_default();
        let code = languages
            .iter()
            .find(|language| crate::settings::language_slug(language) == first && !first.is_empty())
            .cloned();
        let rest = match &code {
            Some(_) => {
                let rest = &path.trim_start_matches('/')[first.len()..];
                if rest.is_empty() {
                    "/".to_owned()
                } else {
                    rest.to_owned()
                }
            }
            None => path.to_owned(),
        };
        (
            Self {
                mount: mount.to_owned(),
                slug: code.as_deref().map(crate::settings::language_slug),
                code,
                available: languages.to_vec(),
            },
            rest,
        )
    }

    fn prefix(&self) -> String {
        match &self.slug {
            Some(slug) => format!("{}/{slug}", self.mount),
            None => self.mount.clone(),
        }
    }

    fn suffix(&self) -> Option<String> {
        self.code.as_deref().map(crate::settings::field_suffix)
    }
}

struct MenuLine<'a> {
    id: String,
    parent: Option<&'a str>,
    label: &'a str,
    url: String,
    path: Option<&'a str>,
}

struct MenuPrefixes<'a> {
    language: &'a str,
    mount: &'a str,
    tools: &'a [&'static str],
}

impl MenuPrefixes<'_> {
    fn for_path(&self, path: &str) -> &str {
        let first = path
            .trim_start_matches('/')
            .split('/')
            .next()
            .unwrap_or_default();
        match self.tools.contains(&first) {
            true => self.mount,
            false => self.language,
        }
    }
}

fn menu_of(
    runtime: &SiteRuntime,
    records: &[RecordMenuLine],
    settings: &crate::settings::SiteSettings,
    prefixes: &MenuPrefixes<'_>,
    current: &str,
    locale: &str,
) -> Vec<serde_json::Value> {
    let prefix = prefixes.language;
    let position = |id: &str| {
        settings
            .menu
            .iter()
            .position(|entry| entry.item == id)
            .unwrap_or(usize::MAX)
    };
    let hidden = |id: &str| {
        settings
            .menu
            .iter()
            .any(|entry| entry.item == id && entry.hidden)
    };
    let plugin_items = runtime
        .menu
        .iter()
        .filter(|item| item.item.records.is_none())
        .map(|item| MenuLine {
            id: item.id.clone(),
            parent: None,
            label: item.label(locale),
            url: format!("{prefix}{}", item.path),
            path: Some(item.path.as_str()),
        });
    let record_items = records.iter().map(|line| MenuLine {
        id: line.id.clone(),
        parent: Some(line.parent.as_str()),
        label: line.label.as_str(),
        url: format!("{prefix}{}", line.path),
        path: Some(line.path.as_str()),
    });
    let links = settings.links.iter().map(|link| {
        let url = link.url.trim();
        MenuLine {
            id: format!("{}{}", crate::settings::LINK_PREFIX, link.id),
            parent: None,
            label: link.label.trim(),
            url: match url.starts_with('/') {
                true => format!("{}{url}", prefixes.for_path(url)),
                false => url.to_owned(),
            },
            path: url.starts_with('/').then_some(url),
        }
    });
    let mut items: Vec<_> = plugin_items
        .chain(record_items)
        .chain(links)
        .filter(|line| !hidden(&line.id) && !line.parent.is_some_and(hidden))
        .enumerate()
        .collect();
    items.sort_by_key(|(index, line)| (position(&line.id), *index));
    items
        .into_iter()
        .map(|(_, line)| {
            let active = line.path.is_some_and(|path| {
                current == path || (path != "/" && current.starts_with(&format!("{path}/")))
            });
            json!({
                "label": line.label,
                "url": line.url,
                "active": active,
            })
        })
        .collect()
}

async fn site_context(
    state: &AppState,
    language: &Language,
    runtime: &SiteRuntime,
    current: &str,
    customer: Option<&SiteCustomer>,
    accounts: bool,
) -> Result<SiteContext, AppError> {
    let config = state.organization_config.snapshot();
    let settings = site_settings(&config);
    let organization = MySqlOrganizationRepository::new(state.db()?)
        .find()
        .await?
        .ok_or_else(|| AppError::NotFound("the organization is not installed".to_owned()))?;
    let prefix = language.prefix();
    let prefix = prefix.as_str();
    let locale = language
        .code
        .clone()
        .unwrap_or_else(|| organization.locale.clone());
    let mount_url = config.site_base_url().trim_end_matches('/').to_owned();
    let origin = origin_of(&mount_url).to_owned();
    let base_url = format!("{origin}{prefix}");
    let (record_lines, tags) = record_menu(state, runtime, language.suffix().as_deref()).await?;
    let alternates: Vec<serde_json::Value> = if language.available.is_empty() {
        Vec::new()
    } else {
        std::iter::once((organization.locale.clone(), language.mount.clone()))
            .chain(
                language
                    .available
                    .iter()
                    .filter(|code| **code != organization.locale)
                    .map(|code| {
                        (
                            code.clone(),
                            format!(
                                "{}/{}",
                                language.mount,
                                crate::settings::language_slug(code)
                            ),
                        )
                    }),
            )
            .map(|(code, base)| {
                json!({
                    "code": code,
                    "url": format!("{origin}{base}{current}"),
                    "path": format!("{base}{current}"),
                    "current": code == locale,
                })
            })
            .collect()
    };
    Ok(SiteContext {
        suffix: language.suffix(),
        site: json!({
            "name": organization.name.as_str(),
            "locale": locale,
            "language": locale,
            "alternates": alternates,
            "color": safe_color(organization.primary_color.as_deref()),
            "prefix": prefix,
            "mount": language.mount,
            "tools": aphel_sdk::module::public_app_paths(state),
            "home": format!("{prefix}/"),
            "year": chrono::Utc::now().format("%Y").to_string(),
            "texts": texts(&locale),
            "base_url": base_url,
            "indexing": settings.indexing,
            "tagline": settings.tagline.as_deref().map(str::trim).filter(|tagline| !tagline.is_empty()),
            "menu": menu_of(
                runtime,
                &record_lines,
                &settings,
                &MenuPrefixes {
                    language: prefix,
                    mount: &language.mount,
                    tools: &aphel_sdk::module::public_app_paths(state),
                },
                current,
                &locale,
            ),
            "logo_url": organization
                .logo_file_id
                .as_ref()
                .map(|file| format!("/_aphel/site{LOGO_PATH}?v={file}")),
            "account": accounts.then(|| account::urls(prefix, &account::paths(&locale))),
        }),
        customer: customer.cloned(),
        tags,
    })
}

struct RouteRequest<'a> {
    route: &'a Route,
    params: BTreeMap<String, String>,
    path: &'a str,
    query: &'a str,
    preview: Option<&'a preview::Preview>,
}

async fn render_route(
    state: &AppState,
    runtime: &SiteRuntime,
    context: &SiteContext,
    request: RouteRequest<'_>,
) -> Result<Outcome, AppError> {
    let RouteRequest {
        route,
        params,
        path,
        query,
        preview,
    } = request;
    let draft_of = |entity: &str| preview.and_then(|preview| preview.draft_for(entity));
    let customer = context.customer.as_ref();
    let params: BTreeMap<String, String> = params
        .into_iter()
        .map(|(name, value)| (name, decode_segment(&value)))
        .collect();
    let mut values = serde_json::Map::new();
    values.insert("site".to_owned(), context.site.clone());
    values.insert("customer".to_owned(), json!(customer));
    let restriction = route
        .page
        .customer_field
        .as_deref()
        .zip(customer.map(|customer| customer.id.as_str()));
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
        let Some(loaded) =
            dynamic::page_data(state, &route.owner, &path, customer.cloned()).await?
        else {
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
            let drafted = match draft_of(entity) {
                Some(draft) => {
                    data::draft(
                        state,
                        exposure,
                        &draft,
                        &BTreeMap::from([(match_field.clone(), json!(value))]),
                        context.suffix.as_deref(),
                    )
                    .await?
                }
                None => None,
            };
            let published = match drafted {
                Some(record) => Some(record),
                None => {
                    data::record(
                        state,
                        exposure,
                        match_field,
                        &value,
                        restriction,
                        context.suffix.as_deref(),
                    )
                    .await?
                }
            };
            let Some(mut record) = published else {
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
        PageSource::Single { entity, conditions } => {
            let exposure = runtime
                .exposure(&route.owner, entity)
                .ok_or_else(|| AppError::Internal(format!("{entity} is not public")))?;
            let drafted = match draft_of(entity) {
                Some(draft) => {
                    data::draft(
                        state,
                        exposure,
                        &draft,
                        conditions,
                        context.suffix.as_deref(),
                    )
                    .await?
                }
                None => None,
            };
            let found = match drafted {
                Some(record) => Some(record),
                None => {
                    data::single(state, exposure, conditions, context.suffix.as_deref()).await?
                }
            };
            tags.push(cache::entity_tag(entity));
            let Some(mut record) = found else {
                return Ok(Outcome::NotFound);
            };
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
            let mut listed = data::list(
                state,
                exposure,
                sort.as_deref(),
                current,
                per_page,
                restriction,
                context.suffix.as_deref(),
            )
            .await?;
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
    let extra = render(&route.page.seo.structured_data).unwrap_or_else(|error| {
        tracing::warn!(%error, page = %route.page.key, "could not render the structured data");
        None
    });
    let image = image.map(|image| absolute_url(base_url, &image));
    let json_ld = structured_data(StructuredPage {
        site: &context.site,
        schema: route.page.seo.schema.as_deref(),
        source: &route.page.source,
        home: path == "/",
        title: title.as_deref(),
        description: description.as_deref(),
        image: image.as_deref(),
        canonical: &canonical,
        extra: extra.as_deref(),
    });
    page["title"] = json!(title);
    page["seo"] = json!({
        "canonical": canonical,
        "description": description,
        "image": image,
        "noindex": route.page.seo.noindex || !indexing,
        "type": if matches!(route.page.source, PageSource::Record { .. }) { "article" } else { "website" },
        "json_ld": json_ld,
    });
    values.insert("page".to_owned(), page);
    let html = runtime
        .environment
        .get_template(&template_name(&route.owner, &route.page.template))
        .and_then(|template| template.render(Value::from_serialize(&values)))
        .map_err(render_error)?;
    let filled = deferred::resolve(
        html,
        &deferred::ResolveContext {
            state,
            runtime,
            site: &context.site,
            suffix: context.suffix.as_deref(),
        },
    )
    .await;
    tags.extend(filled.tags);
    Ok(Outcome::Page {
        html: filled.html,
        tags,
        live: live || filled.live,
    })
}

struct StructuredPage<'a> {
    site: &'a serde_json::Value,
    schema: Option<&'a str>,
    source: &'a PageSource,
    home: bool,
    title: Option<&'a str>,
    description: Option<&'a str>,
    image: Option<&'a str>,
    canonical: &'a str,
    extra: Option<&'a str>,
}

fn structured_data(page: StructuredPage<'_>) -> String {
    let site_name = page.site["name"].as_str().unwrap_or_default();
    let base_url = page.site["base_url"].as_str().unwrap_or_default();
    let home_url = format!("{base_url}/");
    let mut publisher = json!({ "@type": "Organization", "name": site_name, "url": home_url });
    if let Some(logo) = page.site["logo_url"].as_str() {
        publisher["logo"] = json!(absolute_url(base_url, logo));
    }
    let default_schema = match (page.home, page.source) {
        (true, PageSource::Static | PageSource::Single { .. }) => "WebSite",
        (_, PageSource::Record { .. }) => "Article",
        (_, PageSource::List { .. }) => "CollectionPage",
        _ => "WebPage",
    };
    let schema = page.schema.unwrap_or(default_schema);
    let name = page.title.unwrap_or(site_name);
    let mut data = json!({
        "@context": "https://schema.org",
        "@type": schema,
        "name": name,
        "url": page.canonical,
        "inLanguage": page.site["locale"],
    });
    if let Some(description) = page.description {
        data["description"] = json!(description);
    }
    if let Some(image) = page.image {
        data["image"] = json!(image);
    }
    match schema {
        "WebSite" => data["publisher"] = publisher,
        "Article" | "BlogPosting" | "NewsArticle" => {
            data["headline"] = json!(name);
            data["publisher"] = publisher;
            data["mainEntityOfPage"] = json!(page.canonical);
        }
        _ => {
            data["isPartOf"] = json!({ "@type": "WebSite", "name": site_name, "url": home_url });
        }
    }
    if let Some(serde_json::Value::Object(extra)) = page
        .extra
        .and_then(|extra| serde_json::from_str::<serde_json::Value>(extra.trim()).ok())
        && let Some(target) = data.as_object_mut()
    {
        for (key, value) in extra {
            if key != "@context" {
                target.insert(key, value);
            }
        }
    }
    serde_json::to_string(&data)
        .unwrap_or_default()
        .replace('<', "\\u003c")
        .replace('>', "\\u003e")
        .replace('&', "\\u0026")
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
    let values = json!({ "site": context.site, "page": {}, "customer": context.customer });
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

fn page_response(page: &CachedPage, request_headers: &HeaderMap, personal: bool) -> Response {
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
    if personal {
        headers.insert(
            header::CACHE_CONTROL,
            HeaderValue::from_static("private, no-store"),
        );
    } else {
        headers.insert(
            header::CACHE_CONTROL,
            HeaderValue::from_static(PAGE_CACHE_CONTROL),
        );
        headers.insert(header::VARY, HeaderValue::from_static("Cookie"));
    }
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
