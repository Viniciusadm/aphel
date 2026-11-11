use std::collections::BTreeMap;

use aphel_contract_site::{MAX_PER_PAGE, PageSource, decode_segment, template_name};
use aphel_kernel::errors::AppError;
use aphel_kernel::modules::{SiteMountPrefix, SiteRequestHandler};
use aphel_sdk::platform::api::state::AppState;
use aphel_sdk::platform::application::organization_profile::OrganizationRepository;
use aphel_sdk::platform::infrastructure::repositories::mysql_organization_profile::MySqlOrganizationRepository;
use axum::extract::Request;
use axum::http::{HeaderValue, Method, StatusCode, header};
use axum::response::{IntoResponse, Response};
use futures::future::BoxFuture;
use minijinja::Value;
use serde_json::json;

use crate::data;
use crate::runtime::{self, ERROR_TEMPLATE, NOT_FOUND_TEMPLATE, SiteRuntime};

const SITE_CSP: &str = "default-src 'self'; script-src 'self'; style-src 'self' 'unsafe-inline'; img-src 'self' data: https:; font-src 'self'; connect-src 'self'; object-src 'none'; base-uri 'self'; form-action 'self'; frame-ancestors 'self'";
const DEFAULT_COLOR: &str = "#4f46e5";

pub(crate) struct SiteHandler;

impl SiteRequestHandler for SiteHandler {
    fn serve<'a>(&'a self, state: &'a AppState, request: Request) -> BoxFuture<'a, Response> {
        Box::pin(serve(state, request))
    }
}

struct SiteContext {
    site: serde_json::Value,
}

enum Outcome {
    Page(String),
    NotFound,
}

async fn serve(state: &AppState, request: Request) -> Response {
    if request.method() != Method::GET && request.method() != Method::HEAD {
        return StatusCode::METHOD_NOT_ALLOWED.into_response();
    }
    let prefix = request
        .extensions()
        .get::<SiteMountPrefix>()
        .map(|prefix| prefix.0.clone())
        .unwrap_or_default();
    let context = match site_context(state, &prefix).await {
        Ok(context) => context,
        Err(error) => {
            tracing::error!(%error, "could not load the site context");
            return StatusCode::SERVICE_UNAVAILABLE.into_response();
        }
    };
    let runtime = match runtime::current(&state.app).await {
        Ok(runtime) => runtime,
        Err(error) => {
            tracing::error!(%error, "could not load the site pages");
            return fallback(&context, StatusCode::SERVICE_UNAVAILABLE, None);
        }
    };
    let path = request.uri().path().to_owned();
    let query = request.uri().query().unwrap_or_default().to_owned();
    match render_route(state, &runtime, &context, &path, &query).await {
        Ok(Outcome::Page(html)) => html_response(StatusCode::OK, html),
        Ok(Outcome::NotFound) => fallback(&context, StatusCode::NOT_FOUND, Some(&runtime)),
        Err(error) => {
            tracing::warn!(%error, path = %path, "could not render a site page");
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

fn requested_page(query: &str) -> u64 {
    query
        .split('&')
        .filter_map(|pair| pair.split_once('='))
        .find(|(name, _)| *name == "page")
        .and_then(|(_, value)| value.parse::<u64>().ok())
        .filter(|page| *page >= 1)
        .unwrap_or(1)
}

async fn render_route(
    state: &AppState,
    runtime: &SiteRuntime,
    context: &SiteContext,
    path: &str,
    query: &str,
) -> Result<Outcome, AppError> {
    let Some((route, params)) = runtime.find(path) else {
        return Ok(Outcome::NotFound);
    };
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
            let Some(record) = data::record(state, exposure, match_field, &value).await? else {
                return Ok(Outcome::NotFound);
            };
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
            let listed = data::list(state, exposure, sort.as_deref(), current, per_page).await?;
            let pages = listed.total.div_ceil(per_page).max(1);
            if current > pages {
                return Ok(Outcome::NotFound);
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
    let template_context = Value::from_serialize(&values);
    if let Some(title) = &route.page.title {
        let rendered = runtime
            .environment
            .render_str(title, template_context.clone())
            .map_err(render_error)?;
        page["title"] = json!(rendered);
        values.insert("page".to_owned(), page);
    }
    let html = runtime
        .environment
        .get_template(&template_name(&route.owner, &route.page.template))
        .and_then(|template| template.render(Value::from_serialize(&values)))
        .map_err(render_error)?;
    Ok(Outcome::Page(html))
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
        Some(html) => html_response(status, html),
        None => status.into_response(),
    }
}

fn html_response(status: StatusCode, html: String) -> Response {
    let mut response = (status, html).into_response();
    let headers = response.headers_mut();
    headers.insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static("text/html; charset=utf-8"),
    );
    headers.insert(header::CACHE_CONTROL, HeaderValue::from_static("no-cache"));
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
