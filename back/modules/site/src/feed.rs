use aphel_contract_site::{PageSource, TERM_PARAM};
use aphel_kernel::errors::AppError;
use aphel_sdk::platform::api::state::AppState;
use axum::http::{HeaderValue, StatusCode, header};
use axum::response::{IntoResponse, Response};
use serde_json::Value;

use crate::prefixes::Found;
use crate::runtime::SiteRuntime;
use crate::{cache, data};

const FEED_ITEMS: u64 = 20;
const FEED_CACHE_CONTROL: &str = "public, max-age=600";
const FEED_CONTENT_TYPE: &str = "application/rss+xml; charset=utf-8";

fn escape(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

fn first_text<'a>(record: &'a Value, fields: &[&str]) -> Option<&'a str> {
    fields.iter().find_map(|field| {
        record
            .get(*field)
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|text| !text.is_empty())
    })
}

fn rfc2822(value: Option<&str>) -> Option<String> {
    chrono::DateTime::parse_from_rfc3339(value?)
        .ok()
        .map(|instant| instant.to_rfc2822())
}

pub(crate) struct FeedRequest<'a> {
    pub site_name: &'a str,
    pub origin: &'a str,
    pub language_prefix: &'a str,
    pub locale: &'a str,
    pub list_path: &'a str,
    pub translation: Option<&'a str>,
}

async fn render(
    state: &AppState,
    runtime: &SiteRuntime,
    found: &Found<'_>,
    request: &FeedRequest<'_>,
) -> Result<Option<(String, Vec<String>)>, AppError> {
    let route = found.route;
    let PageSource::List { entity, sort, .. } = &route.page.source else {
        return Ok(None);
    };
    if !route.page.feed {
        return Ok(None);
    }
    let Some(exposure) = runtime.exposure(&route.owner, entity) else {
        return Ok(None);
    };
    let mut tags = vec![cache::entity_tag(entity)];
    let mut title_parts = vec![request.site_name.to_owned()];
    if let Some(prefix) = &found.prefix
        && let Some(name) = first_text(&prefix.record, &["name", "title"])
    {
        title_parts.push(name.to_owned());
    }
    let term_filter = match &route.page.term {
        Some(page_term) => {
            let slug = found.params.get(TERM_PARAM).cloned().unwrap_or_default();
            let Some(term) = crate::terms::find(state, page_term, &slug).await? else {
                return Ok(None);
            };
            tags.extend(crate::terms::cache_tags());
            title_parts.push(term.term.name.clone());
            Some(data::any_of(&page_term.field, &term.ids))
        }
        None => None,
    };
    let restriction = found
        .prefix
        .as_ref()
        .map(|prefix| (prefix.scope.as_str(), prefix.id.as_str()));
    let mut listed = data::list_in(
        state,
        exposure,
        (sort.as_deref(), 1, FEED_ITEMS),
        restriction,
        term_filter,
        request.translation,
    )
    .await?;
    crate::prefixes::annotate(state, runtime, entity, &mut listed.records).await?;
    let base = format!("{}{}", request.origin, request.language_prefix);
    let title = title_parts.join(" · ");
    let mut xml = String::from(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<rss version=\"2.0\" xmlns:atom=\"http://www.w3.org/2005/Atom\"><channel>",
    );
    xml.push_str(&format!(
        "<title>{}</title><link>{}</link><description>{}</description><language>{}</language><atom:link href=\"{}\" rel=\"self\" type=\"application/rss+xml\"/>",
        escape(&title),
        escape(&format!("{base}{}", request.list_path)),
        escape(
            found
                .prefix
                .as_ref()
                .and_then(|prefix| first_text(&prefix.record, &["description"]))
                .unwrap_or(&title)
        ),
        escape(request.locale),
        escape(&format!(
            "{base}{}/{}",
            request.list_path.trim_end_matches('/'),
            aphel_contract_site::FEED_SEGMENT
        )),
    ));
    for record in &listed.records {
        let Some(path) = crate::targets::record_path(runtime, entity, record) else {
            continue;
        };
        let link = format!("{base}{path}");
        xml.push_str("<item>");
        xml.push_str(&format!(
            "<title>{}</title><link>{}</link><guid isPermaLink=\"true\">{}</guid>",
            escape(first_text(record, &["title", "name"]).unwrap_or(&path)),
            escape(&link),
            escape(&link)
        ));
        if let Some(description) =
            first_text(record, &["seo_description", "summary", "description"])
        {
            xml.push_str(&format!(
                "<description>{}</description>",
                escape(description)
            ));
        }
        if let Some(date) = rfc2822(first_text(record, &["published_at", "created_at"])) {
            xml.push_str(&format!("<pubDate>{}</pubDate>", escape(&date)));
        }
        if let Some(author) = first_text(record, &["byline"]) {
            xml.push_str(&format!(
                "<dc:creator xmlns:dc=\"http://purl.org/dc/elements/1.1/\">{}</dc:creator>",
                escape(author)
            ));
        }
        xml.push_str("</item>");
    }
    xml.push_str("</channel></rss>\n");
    Ok(Some((xml, tags)))
}

pub(crate) async fn respond(
    state: &AppState,
    runtime: &SiteRuntime,
    found: &Found<'_>,
    request: &FeedRequest<'_>,
    cache_key: String,
) -> Option<Response> {
    if let Some(cached) = cache::get(&cache_key) {
        return Some(feed_response(cached.html.to_string()));
    }
    match render(state, runtime, found, request).await {
        Ok(Some((xml, tags))) => {
            let page = cache::put(cache_key, cache::page(xml, String::new()), tags, None);
            Some(feed_response(page.html.to_string()))
        }
        Ok(None) => None,
        Err(error) => {
            tracing::error!(%error, "could not render the site feed");
            Some(StatusCode::SERVICE_UNAVAILABLE.into_response())
        }
    }
}

fn feed_response(body: String) -> Response {
    let mut response = (StatusCode::OK, body).into_response();
    let headers = response.headers_mut();
    headers.insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static(FEED_CONTENT_TYPE),
    );
    headers.insert(
        header::CACHE_CONTROL,
        HeaderValue::from_static(FEED_CACHE_CONTROL),
    );
    response
}
