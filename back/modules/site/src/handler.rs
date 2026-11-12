use std::collections::BTreeMap;
use std::sync::LazyLock;
use std::time::Duration;

use aphel_contract_site::SiteCustomer;
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
    translation: Option<String>,
    tags: Vec<String>,
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
    if request.uri().path() == "/interactions.js" {
        return dynamic::interactions();
    }
    if request.uri().path() == ISLANDS_LOADER_PATH {
        return dynamic::islands_loader();
    }
    if request.uri().path() == dynamic::ACCOUNT_SCRIPT_PATH {
        return dynamic::account_script();
    }
    if request.uri().path() == dynamic::PREVIEW_SCRIPT_PATH {
        return dynamic::preview_script();
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
    let runtime = match runtime::active(state).await {
        Ok(runtime) => runtime,
        Err(error) => {
            tracing::error!(%error, "could not load the site pages");
            return StatusCode::SERVICE_UNAVAILABLE.into_response();
        }
    };
    let path = request.uri().path().to_owned();
    if let Some(key) = path.strip_prefix("/appearance/") {
        return assets::appearance_file(state, key).await;
    }
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
    let selected = match &previewing {
        Some(previewing) if previewing.record.is_none() => {
            runtime::current(&state.app, previewing.theme.as_deref()).await
        }
        _ => runtime::active(state).await,
    };
    let runtime = match selected {
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
    if let Some(list_path) = path.strip_suffix(aphel_contract_site::FEED_SEGMENT) {
        let list_path = match list_path.trim_end_matches('/') {
            "" => "/",
            trimmed => trimmed,
        };
        if let Ok(Some(found)) = crate::prefixes::find(state, &runtime, list_path).await {
            let organization = MySqlOrganizationRepository::new(match state.db() {
                Ok(pool) => pool,
                Err(_) => return StatusCode::SERVICE_UNAVAILABLE.into_response(),
            })
            .find()
            .await
            .ok()
            .flatten();
            let site_name = organization
                .as_ref()
                .map(|organization| organization.name.as_str().to_owned())
                .unwrap_or_default();
            let locale = language
                .code
                .clone()
                .or_else(|| {
                    organization
                        .as_ref()
                        .map(|organization| organization.locale.clone())
                })
                .unwrap_or_default();
            let mount_url = config.site_base_url();
            let translation = language.translation();
            let request = crate::feed::FeedRequest {
                site_name: &site_name,
                origin: origin_of(mount_url.trim_end_matches('/')),
                language_prefix: &prefix,
                locale: &locale,
                list_path,
                translation: translation.as_deref(),
            };
            let key = format!("feed|{full_path}|{}", runtime.revision());
            if let Some(response) =
                crate::feed::respond(state, &runtime, &found, &request, key).await
            {
                return response;
            }
        }
    }
    let home = home_path(
        state,
        &runtime,
        &settings,
        &path,
        language.translation().as_deref(),
    )
    .await;
    let searching = home.is_none() && crate::search::is_search_path(&path);
    let found = match match searching {
        true => Ok(None),
        false => crate::prefixes::find(state, &runtime, home.as_deref().unwrap_or(&path)).await,
    } {
        Ok(found) => found,
        Err(error) => {
            tracing::error!(%error, "could not resolve the site route");
            return StatusCode::SERVICE_UNAVAILABLE.into_response();
        }
    };
    let personal = customer.is_some() || previewing.is_some();
    let key = found
        .as_ref()
        .filter(|_| !personal)
        .map(|found| cache_key(&parts, &full_path, &runtime, found.route));
    if let Some(key) = &key {
        if let Ok(pool) = state.db()
            && let Err(error) = cache::sync(&pool).await
        {
            tracing::warn!(%error, "could not read the site cache invalidations");
            cache::forget_all();
        }
        if let Some(page) = cache::get(key)
            && !found
                .as_ref()
                .is_some_and(|found| found.route.page.needs_customer())
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
    let Some(crate::prefixes::Found {
        route,
        params,
        prefix: matched_prefix,
    }) = found
    else {
        if path == "/" {
            return welcome(&context, &runtime);
        }
        if searching {
            return search_page(state, &context, &runtime, &parts, (&prefix, &path)).await;
        }
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
            prefix: matched_prefix.as_ref(),
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

pub(crate) async fn render_draft(
    state: &AppState,
    runtime: &SiteRuntime,
    previewing: &preview::Preview,
    full_path: &str,
) -> Result<Option<String>, AppError> {
    let config = state.organization_config.snapshot();
    let settings = site_settings(&config);
    let mount = config.mounts.site.prefix().to_owned();
    let (language, path) = Language::resolve(&mount, full_path, &settings.languages);
    let accounts = settings.accounts || runtime.uses_customers;
    let context = site_context(state, &language, runtime, &path, None, accounts).await?;
    let home = home_path(
        state,
        runtime,
        &settings,
        &path,
        language.translation().as_deref(),
    )
    .await;
    let Some(crate::prefixes::Found {
        route,
        params,
        prefix: matched_prefix,
    }) = crate::prefixes::find(state, runtime, home.as_deref().unwrap_or(&path)).await?
    else {
        return Ok(None);
    };
    let outcome = render_route(
        state,
        runtime,
        &context,
        RouteRequest {
            route,
            params,
            path: &path,
            query: "",
            preview: Some(previewing),
            prefix: matched_prefix.as_ref(),
        },
    )
    .await?;
    Ok(match outcome {
        Outcome::Page { html, .. } => Some(html),
        Outcome::Redirect(_) | Outcome::NotFound => None,
    })
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
        Ok(None) => match crate::prefixes::moved(state, runtime, path).await {
            Ok(Some(location)) => moved_permanently(&format!("{language_prefix}{location}")),
            Ok(None) => fallback(context, StatusCode::NOT_FOUND, Some(runtime)),
            Err(error) => {
                tracing::warn!(%error, "could not read the site redirects");
                fallback(context, StatusCode::NOT_FOUND, Some(runtime))
            }
        },
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
    match locale.split('-').next().unwrap_or_default() {
        "pt" => json!({
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
            "footer_links": "Links do rodapé",
            "social": "Redes sociais",
            "contact": "Contato",
            "my_account": "Minha conta",
            "share": "Compartilhar",
            "copy_url": "Copiar endereço",
            "copied": "Endereço copiado",
            "toc": "Nesta publicação",
            "empty_collection": "Ainda não há publicações aqui.",
            "updated": "Atualizada em",
            "search": "Buscar",
            "search_label": "Buscar no site",
            "search_empty": "Nenhum resultado para esta busca.",
            "search_short": "Digite ao menos duas letras para buscar.",
            "reading_time": "min de leitura",
            "related": "Leia também",
            "categories": "Categorias",
            "feed": "Feed RSS",
        }),
        "es" => json!({
            "not_found_title": "Página no encontrada",
            "not_found_body": "La dirección no existe o el contenido ya no está publicado.",
            "error_title": "Algo salió mal",
            "error_body": "No se pudo mostrar esta página ahora. Inténtalo de nuevo en unos instantes.",
            "back_home": "Volver al inicio",
            "previous": "Anterior",
            "next": "Siguiente",
            "menu": "Menú principal",
            "sign_in": "Iniciar sesión",
            "languages": "Idiomas",
            "footer_links": "Enlaces del pie de página",
            "social": "Redes sociales",
            "contact": "Contacto",
            "my_account": "Mi cuenta",
            "share": "Compartir",
            "copy_url": "Copiar dirección",
            "copied": "Dirección copiada",
            "toc": "En esta publicación",
            "empty_collection": "Todavía no hay publicaciones aquí.",
            "updated": "Actualizado el",
            "search": "Buscar",
            "search_label": "Buscar en el sitio",
            "search_empty": "No hay resultados para esta búsqueda.",
            "search_short": "Escribe al menos dos letras para buscar.",
            "reading_time": "min de lectura",
            "related": "Lee también",
            "categories": "Categorías",
            "feed": "Feed RSS",
        }),
        "fr" => json!({
            "not_found_title": "Page introuvable",
            "not_found_body": "L’adresse n’existe pas ou le contenu n’est plus publié.",
            "error_title": "Une erreur s’est produite",
            "error_body": "Impossible d’afficher cette page pour le moment. Réessayez dans quelques instants.",
            "back_home": "Retour à l’accueil",
            "previous": "Précédent",
            "next": "Suivant",
            "menu": "Menu principal",
            "sign_in": "Se connecter",
            "languages": "Langues",
            "footer_links": "Liens du pied de page",
            "social": "Réseaux sociaux",
            "contact": "Contact",
            "my_account": "Mon compte",
            "share": "Partager",
            "copy_url": "Copier l’adresse",
            "copied": "Adresse copiée",
            "toc": "Dans cette publication",
            "empty_collection": "Il n’y a pas encore de publications ici.",
            "updated": "Mis à jour le",
            "search": "Rechercher",
            "search_label": "Rechercher sur le site",
            "search_empty": "Aucun résultat pour cette recherche.",
            "search_short": "Saisissez au moins deux lettres pour rechercher.",
            "reading_time": "min de lecture",
            "related": "À lire aussi",
            "categories": "Catégories",
            "feed": "Flux RSS",
        }),
        "de" => json!({
            "not_found_title": "Seite nicht gefunden",
            "not_found_body": "Die Adresse existiert nicht oder der Inhalt ist nicht mehr veröffentlicht.",
            "error_title": "Etwas ist schiefgelaufen",
            "error_body": "Diese Seite kann gerade nicht angezeigt werden. Versuchen Sie es gleich noch einmal.",
            "back_home": "Zurück zur Startseite",
            "previous": "Zurück",
            "next": "Weiter",
            "menu": "Hauptmenü",
            "sign_in": "Anmelden",
            "languages": "Sprachen",
            "footer_links": "Links in der Fußzeile",
            "social": "Soziale Netzwerke",
            "contact": "Kontakt",
            "my_account": "Mein Konto",
            "share": "Teilen",
            "copy_url": "Adresse kopieren",
            "copied": "Adresse kopiert",
            "toc": "In diesem Beitrag",
            "empty_collection": "Hier gibt es noch keine Beiträge.",
            "updated": "Aktualisiert am",
            "search": "Suchen",
            "search_label": "Website durchsuchen",
            "search_empty": "Keine Ergebnisse für diese Suche.",
            "search_short": "Geben Sie mindestens zwei Buchstaben ein.",
            "reading_time": "Min. Lesezeit",
            "related": "Weitere Beiträge",
            "categories": "Kategorien",
            "feed": "RSS-Feed",
        }),
        "it" => json!({
            "not_found_title": "Pagina non trovata",
            "not_found_body": "L’indirizzo non esiste o il contenuto non è più pubblicato.",
            "error_title": "Qualcosa è andato storto",
            "error_body": "Non è possibile mostrare questa pagina ora. Riprova tra qualche istante.",
            "back_home": "Torna alla home",
            "previous": "Precedente",
            "next": "Successiva",
            "menu": "Menu principale",
            "sign_in": "Accedi",
            "languages": "Lingue",
            "footer_links": "Link del piè di pagina",
            "social": "Social network",
            "contact": "Contatti",
            "my_account": "Il mio account",
            "share": "Condividi",
            "copy_url": "Copia indirizzo",
            "copied": "Indirizzo copiato",
            "toc": "In questo articolo",
            "empty_collection": "Non ci sono ancora pubblicazioni qui.",
            "updated": "Aggiornato il",
            "search": "Cerca",
            "search_label": "Cerca nel sito",
            "search_empty": "Nessun risultato per questa ricerca.",
            "search_short": "Digita almeno due lettere per cercare.",
            "reading_time": "min di lettura",
            "related": "Leggi anche",
            "categories": "Categorie",
            "feed": "Feed RSS",
        }),
        _ => json!({
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
            "footer_links": "Footer links",
            "social": "Social networks",
            "contact": "Contact",
            "my_account": "My account",
            "share": "Share",
            "copy_url": "Copy URL",
            "copied": "URL copied",
            "toc": "On this page",
            "empty_collection": "There are no publications here yet.",
            "updated": "Updated on",
            "search": "Search",
            "search_label": "Search this site",
            "search_empty": "No results for this search.",
            "search_short": "Type at least two letters to search.",
            "reading_time": "min read",
            "related": "Read next",
            "categories": "Categories",
            "feed": "RSS feed",
        }),
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

    fn translation(&self) -> Option<String> {
        self.code.clone()
    }
}

async fn navigation_of(
    state: &AppState,
    runtime: &SiteRuntime,
    items: &[crate::settings::NavigationItem],
    context: &NavigationContext<'_>,
    tags: &mut Vec<String>,
) -> Result<Vec<serde_json::Value>, AppError> {
    let mut lines = Vec::with_capacity(items.len());
    for item in items {
        let target = item
            .target
            .as_deref()
            .and_then(crate::targets::Target::parse);
        let (url, path, fallback) = match (&target, item.url.as_deref().map(str::trim)) {
            (Some(target), _) => {
                tags.extend(target.tag());
                let Some(resolved) =
                    crate::targets::resolve(state, runtime, target, context.translation).await?
                else {
                    continue;
                };
                (
                    Some(context.prefixes.link(&resolved.path)),
                    Some(resolved.path),
                    resolved.label,
                )
            }
            (None, Some(url)) if !url.is_empty() => (
                Some(context.prefixes.link(url)),
                url.starts_with('/').then(|| url.to_owned()),
                None,
            ),
            _ => (None, None, None),
        };
        let children =
            Box::pin(navigation_of(state, runtime, &item.children, context, tags)).await?;
        if url.is_none() && children.is_empty() {
            continue;
        }
        let label = crate::frame::link_label(context.texts, &item.id, &item.label);
        let label = match (label.is_empty(), fallback) {
            (true, Some(fallback)) => fallback,
            _ => label.to_owned(),
        };
        let active = path
            .as_deref()
            .is_some_and(|path| crate::frame::is_active(path, context.current))
            || children
                .iter()
                .any(|child| child["active"].as_bool().unwrap_or(false));
        lines.push(json!({
            "label": label.chars().take(MAX_MENU_LABEL_CHARS).collect::<String>(),
            "url": url,
            "active": active,
            "children": children,
        }));
    }
    Ok(lines)
}

struct NavigationContext<'a> {
    prefixes: &'a crate::frame::SitePrefixes<'a>,
    texts: Option<&'a crate::settings::SiteTexts>,
    current: &'a str,
    translation: Option<&'a str>,
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
    let mut tags = Vec::new();
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
    let tools = aphel_sdk::module::public_app_paths(state);
    let prefixes = crate::frame::SitePrefixes {
        language: prefix,
        mount: &language.mount,
        tools: &tools,
    };
    let site_texts = language
        .code
        .as_ref()
        .and_then(|code| settings.translations.get(code));
    let translation = language.translation();
    let navigation = navigation_of(
        state,
        runtime,
        &settings.navigation,
        &NavigationContext {
            prefixes: &prefixes,
            texts: site_texts,
            current,
            translation: translation.as_deref(),
        },
        &mut tags,
    )
    .await?;
    let mut context = SiteContext {
        translation: language.translation(),
        site: json!({
            "appearance": crate::appearance::resolved(state, None, None).await?,
            "name": organization.name.as_str(),
            "locale": locale,
            "language": locale,
            "alternates": alternates,
            "color": safe_color(organization.primary_color.as_deref()),
            "prefix": prefix,
            "mount": language.mount,
            "tools": tools,
            "home": format!("{prefix}/"),
            "search": format!("{prefix}{}", crate::search::search_path(&locale)),
            "year": chrono::Utc::now().format("%Y").to_string(),
            "texts": texts(&locale),
            "base_url": base_url,
            "indexing": settings.indexing,
            "scrollbar": settings.scrollbar,
            "tagline": crate::frame::tagline(&settings, site_texts),
            "header": crate::frame::header(&settings, site_texts, &prefixes),
            "footer": crate::frame::footer(&settings, site_texts, &prefixes, current),
            "menu": navigation,
            "logo_url": organization
                .logo_file_id
                .as_ref()
                .map(|file| format!("/_aphel/site{LOGO_PATH}?v={file}")),
            "account": accounts.then(|| account::urls(prefix, &account::paths(&locale))),
        }),
        customer: customer.cloned(),
        tags,
    };
    let appearance = context.site["appearance"].clone();
    crate::appearance::apply_frame(&mut context.site, &appearance, state)?;
    Ok(context)
}

struct RouteRequest<'a> {
    route: &'a Route,
    params: BTreeMap<String, String>,
    path: &'a str,
    query: &'a str,
    preview: Option<&'a preview::Preview>,
    prefix: Option<&'a crate::prefixes::PrefixMatch>,
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
        prefix,
    } = request;
    let draft_of = |entity: &str| preview.and_then(|preview| preview.draft_for(entity));
    let customer = context.customer.as_ref();
    let params: BTreeMap<String, String> = params
        .into_iter()
        .map(|(name, value)| (name, decode_segment(&value)))
        .collect();
    let mut values = serde_json::Map::new();
    let target = prefix.map(|prefix| format!("{}:{}", prefix.entity, prefix.id));
    let appearance = crate::appearance::resolved(
        state,
        target.as_deref(),
        preview.and_then(|preview| preview.appearance.as_ref()),
    )
    .await?;
    let mut site = context.site.clone();
    crate::appearance::apply_frame(&mut site, &appearance, state)?;
    site["appearance"] = appearance;
    values.insert("site".to_owned(), site);
    values.insert("customer".to_owned(), json!(customer));
    let restriction = route
        .page
        .customer_field
        .as_deref()
        .zip(customer.map(|customer| customer.id.as_str()))
        .or_else(|| prefix.map(|prefix| (prefix.scope.as_str(), prefix.id.as_str())));
    let mut page = json!({
        "owner": route.owner,
        "key": route.page.key,
        "path": path,
        "params": params,
    });
    let mut tags = Vec::new();
    if let Some(prefix) = prefix {
        values.insert("prefix".to_owned(), prefix.record.clone());
        tags.push(cache::entity_tag(&prefix.entity));
        tags.push(cache::record_tag(&prefix.entity, &prefix.id));
    }
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
                        context.translation.as_deref(),
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
                        context.translation.as_deref(),
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
                        "{}{}{location}",
                        context.site["prefix"].as_str().unwrap_or_default(),
                        params
                            .get(aphel_contract_site::PREFIX_PARAM)
                            .map(|segment| format!("/{segment}"))
                            .unwrap_or_default()
                    )),
                    None => Outcome::NotFound,
                });
            };
            if let Some(id) = record.get("id").and_then(serde_json::Value::as_str) {
                tags.push(cache::record_tag(entity, id));
            }
            images::attach_images(state, exposure, &mut record).await?;
            crate::terms::annotate(
                state,
                exposure,
                std::slice::from_mut(&mut record),
                &mut tags,
            )
            .await?;
            if exposure.exposes("published_at")
                && let (Some(published_at), Some(id)) = (
                    record
                        .get("published_at")
                        .and_then(serde_json::Value::as_str)
                        .map(str::to_owned),
                    record
                        .get("id")
                        .and_then(serde_json::Value::as_str)
                        .map(str::to_owned),
                )
            {
                tags.push(cache::entity_tag(entity));
                let mut adjacent = serde_json::Map::new();
                for (key, older) in [("previous", true), ("next", false)] {
                    let Some(mut neighbour) = data::neighbour(
                        state,
                        exposure,
                        (&published_at, &id),
                        older,
                        restriction,
                        context.translation.as_deref(),
                    )
                    .await?
                    else {
                        continue;
                    };
                    crate::prefixes::annotate(
                        state,
                        runtime,
                        entity,
                        std::slice::from_mut(&mut neighbour),
                    )
                    .await?;
                    let title = ["title", "name"].iter().find_map(|field| {
                        neighbour.get(*field).and_then(serde_json::Value::as_str)
                    });
                    if let (Some(path), Some(title)) = (
                        crate::targets::record_path(runtime, entity, &neighbour),
                        title,
                    ) {
                        adjacent.insert(
                            key.to_owned(),
                            json!({
                                "title": title,
                                "url": format!("{}{path}", context.site["prefix"].as_str().unwrap_or_default()),
                            }),
                        );
                    }
                }
                values.insert("adjacent".to_owned(), serde_json::Value::Object(adjacent));
            }
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
                        context.translation.as_deref(),
                    )
                    .await?
                }
                None => None,
            };
            let found = match drafted {
                Some(record) => Some(record),
                None => {
                    data::single(state, exposure, conditions, context.translation.as_deref())
                        .await?
                }
            };
            tags.push(cache::entity_tag(entity));
            let Some(mut record) = found else {
                return Ok(Outcome::NotFound);
            };
            images::attach_images(state, exposure, &mut record).await?;
            crate::terms::annotate(
                state,
                exposure,
                std::slice::from_mut(&mut record),
                &mut tags,
            )
            .await?;
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
            let per_page = values["site"]["appearance"]["per_page"]
                .as_str()
                .and_then(|value| value.parse::<u64>().ok())
                .filter(|_| prefix.is_some())
                .unwrap_or(*per_page)
                .clamp(1, MAX_PER_PAGE);
            let current = requested_page(query);
            let term_filter = match &route.page.term {
                Some(page_term) => {
                    let slug = params
                        .get(aphel_contract_site::TERM_PARAM)
                        .cloned()
                        .unwrap_or_default();
                    let Some(found) = crate::terms::find(state, page_term, &slug).await? else {
                        return Ok(Outcome::NotFound);
                    };
                    tags.extend(crate::terms::cache_tags());
                    values.insert("term".to_owned(), json!(found.term));
                    Some(data::any_of(&page_term.field, &found.ids))
                }
                None => None,
            };
            let mut listed = data::list_in(
                state,
                exposure,
                (sort.as_deref(), current, per_page),
                restriction,
                term_filter,
                context.translation.as_deref(),
            )
            .await?;
            if preview
                .and_then(|preview| preview.appearance.as_ref())
                .is_some_and(|appearance| appearance.empty)
            {
                listed.records.clear();
                listed.total = 0;
            }
            let pages = listed.total.div_ceil(per_page).max(1);
            if current > pages {
                return Ok(Outcome::NotFound);
            }
            tags.push(cache::entity_tag(entity));
            for record in &mut listed.records {
                images::attach_images(state, exposure, record).await?;
            }
            crate::prefixes::annotate(state, runtime, entity, &mut listed.records).await?;
            crate::terms::annotate(state, exposure, &mut listed.records, &mut tags).await?;
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
            let vocabularies = crate::terms::vocabularies(state, runtime, entity).await?;
            if !vocabularies.is_empty() {
                tags.extend(crate::terms::cache_tags());
                values.insert("vocabularies".to_owned(), json!(vocabularies));
            }
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
    let filled = |expression: &Option<String>| -> Result<Option<String>, AppError> {
        Ok(render(expression)?
            .map(|value| value.trim().to_owned())
            .filter(|value| !value.is_empty()))
    };
    let author = filled(&route.page.seo.author)?;
    let published = filled(&route.page.seo.published)?;
    let modified = filled(&route.page.seo.modified)?;
    let json_ld = structured_data(StructuredPage {
        site: values.get("site").unwrap_or(&context.site),
        schema: route.page.seo.schema.as_deref(),
        source: &route.page.source,
        home: path == "/",
        title: title.as_deref(),
        description: description.as_deref(),
        image: image.as_deref(),
        canonical: &canonical,
        author: author.as_deref(),
        published: published.as_deref(),
        modified: modified.as_deref(),
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
        "author": author,
        "published": published,
        "modified": modified,
        "breadcrumbs": breadcrumbs(prefix, base_url, path, title.as_deref(), &canonical),
    });
    if route.page.feed {
        page["feed_url"] = json!(format!(
            "{}{}/{}",
            context.site["prefix"].as_str().unwrap_or_default(),
            path.trim_end_matches('/'),
            aphel_contract_site::FEED_SEGMENT
        ));
    }
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
            site: &values["site"],
            appearance: preview.and_then(|preview| preview.appearance.as_ref()),
            translation: context.translation.as_deref(),
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

fn breadcrumbs(
    prefix: Option<&crate::prefixes::PrefixMatch>,
    base_url: &str,
    path: &str,
    title: Option<&str>,
    canonical: &str,
) -> Option<String> {
    let prefix = prefix?;
    let segment = path.trim_start_matches('/').split('/').next()?;
    let name = ["name", "title"].iter().find_map(|field| {
        prefix
            .record
            .get(*field)
            .and_then(serde_json::Value::as_str)
    })?;
    let mut items = vec![
        json!({ "@type": "ListItem", "position": 1, "name": name, "item": format!("{base_url}/{segment}") }),
    ];
    if path.trim_end_matches('/') != format!("/{segment}") {
        items.push(json!({ "@type": "ListItem", "position": 2, "name": title.unwrap_or(name), "item": canonical }));
    }
    serde_json::to_string(&json!({ "@context": "https://schema.org", "@type": "BreadcrumbList", "itemListElement": items }))
        .ok()
        .map(|text| text.replace('<', "\\u003c").replace('>', "\\u003e").replace('&', "\\u0026"))
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
    author: Option<&'a str>,
    published: Option<&'a str>,
    modified: Option<&'a str>,
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
    if let Some(author) = page.author {
        data["author"] = json!({ "@type": "Person", "name": author });
    }
    if let Some(published) = page.published {
        data["datePublished"] = json!(published);
    }
    if let Some(modified) = page.modified.or(page.published) {
        data["dateModified"] = json!(modified);
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

async fn home_path(
    state: &AppState,
    runtime: &SiteRuntime,
    settings: &crate::settings::SiteSettings,
    path: &str,
    translation: Option<&str>,
) -> Option<String> {
    if path != "/" {
        return None;
    }
    let target = crate::targets::Target::parse(settings.home.as_deref()?)?;
    match crate::targets::resolve(state, runtime, &target, translation).await {
        Ok(resolved) => resolved.map(|resolved| resolved.path),
        Err(error) => {
            tracing::warn!(%error, "could not resolve the site home page");
            None
        }
    }
}

async fn search_page(
    state: &AppState,
    context: &SiteContext,
    runtime: &SiteRuntime,
    parts: &Parts,
    (prefix, path): (&str, &str),
) -> Response {
    let query = crate::search::requested_query(parts.uri.query());
    let results = match crate::search::results(
        state,
        runtime,
        &query,
        prefix,
        context.translation.as_deref(),
    )
    .await
    {
        Ok(results) => results,
        Err(error) => {
            tracing::error!(%error, "could not search the site");
            return fallback(context, StatusCode::SERVICE_UNAVAILABLE, Some(runtime));
        }
    };
    let page = json!({
        "query": query,
        "short": !query.is_empty() && query.chars().count() < 2,
        "results": results,
        "action": format!("{prefix}{path}"),
    });
    let values = json!({ "site": context.site, "page": page, "customer": context.customer });
    match runtime
        .environment
        .get_template(crate::search::SEARCH_TEMPLATE)
        .and_then(|template| template.render(Value::from_serialize(&values)))
    {
        Ok(html) => {
            let mut response =
                html_response(StatusCode::OK, html, &runtime.content_security_policy(None));
            response
                .headers_mut()
                .insert(header::CACHE_CONTROL, HeaderValue::from_static("no-store"));
            response
        }
        Err(error) => {
            tracing::warn!(%error, "could not render the site search page");
            fallback(context, StatusCode::NOT_FOUND, Some(runtime))
        }
    }
}

fn welcome(context: &SiteContext, runtime: &SiteRuntime) -> Response {
    let values = json!({ "site": context.site, "page": {}, "customer": context.customer });
    match runtime
        .environment
        .get_template(crate::runtime::WELCOME_TEMPLATE)
        .and_then(|template| template.render(Value::from_serialize(&values)))
    {
        Ok(html) => {
            let mut response =
                html_response(StatusCode::OK, html, &runtime.content_security_policy(None));
            response
                .headers_mut()
                .insert(header::CACHE_CONTROL, HeaderValue::from_static("no-store"));
            response
        }
        Err(error) => {
            tracing::warn!(%error, "could not render the site welcome page");
            fallback(context, StatusCode::NOT_FOUND, Some(runtime))
        }
    }
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
