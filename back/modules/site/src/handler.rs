use std::collections::BTreeMap;
use std::sync::LazyLock;
use std::time::Duration;

use aphel_contract_site::{MAX_PER_PAGE, PageSource, decode_segment, template_name};
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

use crate::cache::{self, CachedPage};
use crate::runtime::{self, ERROR_TEMPLATE, NOT_FOUND_TEMPLATE, Route, SiteRuntime};
use crate::{data, images};

const SITE_CSP: &str = "default-src 'self'; script-src 'self'; style-src 'self' 'unsafe-inline'; img-src 'self' data: https:; font-src 'self'; connect-src 'self'; object-src 'none'; base-uri 'self'; form-action 'self'; frame-ancestors 'self'";
const DEFAULT_COLOR: &str = "#4f46e5";
const PAGE_CACHE_CONTROL: &str = "public, max-age=60, stale-while-revalidate=600";
const RENDERS_PER_ADDRESS: u32 = 120;
const RENDER_WINDOW: Duration = Duration::from_secs(60);
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
    Page { html: String, tags: Vec<String> },
    NotFound,
}

fn readable_method(method: &Method) -> bool {
    method == Method::GET || method == Method::HEAD
}

async fn serve_reserved(state: &AppState, request: Request) -> Response {
    if !readable_method(request.method()) {
        return StatusCode::METHOD_NOT_ALLOWED.into_response();
    }
    let runtime = match runtime::current(&state.app).await {
        Ok(runtime) => runtime,
        Err(error) => {
            tracing::error!(%error, "could not load the site pages");
            return StatusCode::SERVICE_UNAVAILABLE.into_response();
        }
    };
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
    let runtime = match runtime::current(&state.app).await {
        Ok(runtime) => runtime,
        Err(error) => {
            tracing::error!(%error, "could not load the site pages");
            return StatusCode::SERVICE_UNAVAILABLE.into_response();
        }
    };
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
    let context = match site_context(state, &prefix).await {
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
        Ok(Outcome::Page { html, tags }) => {
            let page = match key {
                Some(key) => cache::put(key, html, tags),
                None => CachedPage {
                    etag: cache::etag_of(&html),
                    html: html.into(),
                },
            };
            page_response(&page, &parts.headers)
        }
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

async fn site_context(state: &AppState, prefix: &str) -> Result<SiteContext, AppError> {
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
                return Ok(Outcome::NotFound);
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
    if let Some(title) = &route.page.title {
        let rendered = runtime
            .environment
            .render_str(title, Value::from_serialize(&values))
            .map_err(render_error)?;
        page["title"] = json!(rendered);
        values.insert("page".to_owned(), page);
    }
    let html = runtime
        .environment
        .get_template(&template_name(&route.owner, &route.page.template))
        .and_then(|template| template.render(Value::from_serialize(&values)))
        .map_err(render_error)?;
    Ok(Outcome::Page { html, tags })
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
    });
    match rendered {
        Some(html) => {
            let mut response = html_response(status, html);
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
        html_response(StatusCode::OK, page.html.to_string())
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

fn html_response(status: StatusCode, html: String) -> Response {
    let mut response = (status, html).into_response();
    let headers = response.headers_mut();
    headers.insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static("text/html; charset=utf-8"),
    );
    headers.insert(
        header::CONTENT_SECURITY_POLICY,
        HeaderValue::from_static(SITE_CSP),
    );
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
