use axum::Router;
use axum::body::Bytes;
use axum::http::header;
use axum::routing::get;
use utoipa::Modify;
use utoipa::OpenApi;
use utoipa::openapi::security::{
    ApiKey, ApiKeyValue, HttpAuthScheme, HttpBuilder, SecurityRequirement, SecurityScheme,
};
use utoipa::openapi::tag::TagBuilder;
use utoipa::openapi::{ContentBuilder, Ref, ResponseBuilder};
use utoipa_scalar::{Scalar, Servable};

use crate::api::dto::common::{ErrorBody, ErrorEnvelope};
use crate::api::session_cookie::SESSION_COOKIE;
use crate::modules::App;

pub const OPENAPI_PATH: &str = "/api/openapi.json";
pub const DOCS_PATH: &str = "/api/docs";
const API_INTRODUCTION: &str = include_str!("introduction.md");

#[derive(OpenApi)]
#[openapi(
    info(title = "Aphel API"),
    modifiers(&SessionCookieSecurity, &ApiTokenSecurity),
    components(schemas(ErrorEnvelope, ErrorBody)),
)]
pub struct ApiDoc;

pub fn base_document(app: &App) -> utoipa::openapi::OpenApi {
    let mut document = ApiDoc::openapi();
    document.tags = Some(
        app.catalog()
            .tag_contributions()
            .iter()
            .map(|contribution| {
                TagBuilder::new()
                    .name(contribution.item.name)
                    .description(Some(contribution.item.description))
                    .extensions(Some(
                        utoipa::openapi::extensions::ExtensionsBuilder::new()
                            .add("x-aphel-module", contribution.module)
                            .build(),
                    ))
                    .build()
            })
            .collect(),
    );
    document.info.license = None;
    document.info.version = env!("CARGO_PKG_VERSION").to_owned();
    document.info.description = Some(API_INTRODUCTION.to_owned());
    document
}

struct SessionCookieSecurity;

impl Modify for SessionCookieSecurity {
    fn modify(&self, openapi: &mut utoipa::openapi::OpenApi) {
        let components = openapi.components.get_or_insert_with(Default::default);
        components.add_security_scheme(
            "session_cookie",
            SecurityScheme::ApiKey(ApiKey::Cookie(ApiKeyValue::new(SESSION_COOKIE))),
        );
    }
}

const SESSION_ONLY_TAGS: [&str; 4] = ["auth", "api-tokens", "installer", "two-factor"];

struct ApiTokenSecurity;

impl Modify for ApiTokenSecurity {
    fn modify(&self, openapi: &mut utoipa::openapi::OpenApi) {
        let components = openapi.components.get_or_insert_with(Default::default);
        components.add_security_scheme(
            "api_token",
            SecurityScheme::Http(
                HttpBuilder::new()
                    .scheme(HttpAuthScheme::Bearer)
                    .bearer_format("aphel_<prefix>_<secret>")
                    .description(Some(
                        "API token created in Settings → API tokens, sent as `Authorization: Bearer aphel_…`",
                    ))
                    .build(),
            ),
        );
    }
}

pub fn add_api_token_security(openapi: &mut utoipa::openapi::OpenApi) {
    for item in openapi.paths.paths.values_mut() {
        for operation in [
            &mut item.get,
            &mut item.post,
            &mut item.put,
            &mut item.patch,
            &mut item.delete,
        ]
        .into_iter()
        .flatten()
        {
            let session_only = operation
                .tags
                .iter()
                .flatten()
                .any(|tag| SESSION_ONLY_TAGS.contains(&tag.as_str()));
            let Some(security) = operation.security.as_mut() else {
                continue;
            };
            if session_only || security.is_empty() {
                continue;
            }
            security.push(SecurityRequirement::new("api_token", Vec::<String>::new()));
            operation
                .responses
                .responses
                .entry("429".to_owned())
                .or_insert_with(|| {
                    ResponseBuilder::new()
                        .description("API token rate limit exceeded (`rate_limited`); see `Retry-After` and `X-RateLimit-*`")
                        .content(
                            "application/json",
                            ContentBuilder::new()
                                .schema(Some(Ref::from_schema_name("ErrorEnvelope")))
                                .build(),
                        )
                        .build()
                        .into()
                });
        }
    }
}

pub fn router<S>(openapi: utoipa::openapi::OpenApi) -> Router<S>
where
    S: Clone + Send + Sync + 'static,
{
    let document = match serde_json::to_vec(&openapi) {
        Ok(document) => Bytes::from(document),
        Err(error) => {
            tracing::error!("could not serialize the OpenAPI document: {error}");
            Bytes::new()
        }
    };
    Router::new()
        .route(
            OPENAPI_PATH,
            get(move || {
                let document = document.clone();
                async move { ([(header::CONTENT_TYPE, "application/json")], document) }
            }),
        )
        .merge(Scalar::with_url(DOCS_PATH, openapi))
}
