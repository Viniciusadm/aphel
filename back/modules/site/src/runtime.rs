use std::collections::BTreeMap;
use std::sync::{Arc, LazyLock, RwLock};

use aphel_contract_site::{
    HEAD_PARTIAL, IMAGE_WIDTHS, LAYOUT_TEMPLATE, PathPattern, SiteBundle, SiteContentSource,
    SiteCsp, SiteExposure, SitePage, THEME_ERROR, THEME_LAYOUT, THEME_NOT_FOUND, check_bundle,
    join_template_path, menu_item_id, template_name,
};
use aphel_kernel::errors::AppError;
use aphel_kernel::modules::App;
use minijinja::value::Kwargs;
use minijinja::{Environment, Error, ErrorKind, State, Value};

pub(crate) const NOT_FOUND_TEMPLATE: &str = "aphel/not_found.html";
pub(crate) const ERROR_TEMPLATE: &str = "aphel/error.html";
pub(crate) const THEMES_SEGMENT: &str = "themes";
pub(crate) const ISLANDS_SEGMENT: &str = "islands";
const IMPORT_MAP: &str = r#"{"imports":{"react":"/_aphel/shared/react.js","react/jsx-runtime":"/_aphel/shared/jsx-runtime.js","react-dom":"/_aphel/shared/react-dom.js","react-dom/client":"/_aphel/shared/react-dom-client.js","@aphel/ui":"/_aphel/shared/ui.js"}}"#;

static IMPORT_MAP_HASH: LazyLock<String> = LazyLock::new(|| {
    use base64::Engine;
    use sha2::Digest;
    let digest = sha2::Sha256::digest(IMPORT_MAP.as_bytes());
    format!(
        "'sha256-{}'",
        base64::engine::general_purpose::STANDARD.encode(digest)
    )
});

fn import_map_tag() -> String {
    format!("<script type=\"importmap\">{IMPORT_MAP}</script>")
}
const RENDER_FUEL: u64 = 2_000_000;
const MAX_JSON_BYTES: usize = 512 * 1024;
const MAX_JSON_DEPTH: usize = 16;
const DEFAULT_DATE_FORMAT: &str = "%d/%m/%Y";

pub(crate) struct Route {
    pub owner: String,
    pub page: SitePage,
    pub pattern: PathPattern,
}

pub(crate) struct MenuCandidate {
    pub id: String,
    pub owner: String,
    pub item: aphel_contract_site::SiteMenuItem,
    pub path: String,
    pub source: aphel_contract_site::PageSource,
}

impl MenuCandidate {
    pub fn label(&self, language: &str) -> &str {
        self.item.label_for(language)
    }
}

#[derive(Clone, serde::Serialize)]
pub(crate) struct ThemeInfo {
    pub key: String,
    pub name: String,
    pub description: Option<String>,
}

pub(crate) struct SiteRuntime {
    revision: String,
    pub routes: Vec<Route>,
    exposures: BTreeMap<(String, String), SiteExposure>,
    pub environment: Environment<'static>,
    pub menu: Vec<MenuCandidate>,
    pub themes: Vec<ThemeInfo>,
    pub theme: Option<String>,
    csp: BTreeMap<String, SiteCsp>,
    pub uses_customers: bool,
    pub account_links: Vec<MenuCandidate>,
}

impl SiteRuntime {
    pub fn revision(&self) -> &str {
        &self.revision
    }

    pub fn image_exposure(&self, entity: &str) -> Option<&SiteExposure> {
        self.exposures
            .iter()
            .find(|((_, exposed), exposure)| exposed == entity && exposure.images)
            .map(|(_, exposure)| exposure)
    }

    pub fn entity_exposure(&self, entity: &str) -> Option<&SiteExposure> {
        self.exposures
            .iter()
            .find(|((_, exposed), _)| exposed == entity)
            .map(|(_, exposure)| exposure)
    }

    pub fn exposure(&self, owner: &str, entity: &str) -> Option<&SiteExposure> {
        self.exposures.get(&(owner.to_owned(), entity.to_owned()))
    }

    pub fn content_security_policy(&self, owner: Option<&str>) -> String {
        let extras: Vec<&SiteCsp> = [owner, self.theme.as_deref()]
            .into_iter()
            .flatten()
            .filter_map(|owner| self.csp.get(owner))
            .collect();
        let sources = |base: &[&str], pick: fn(&SiteCsp) -> &Vec<String>| {
            let mut all: Vec<String> = base.iter().map(|source| (*source).to_owned()).collect();
            for csp in &extras {
                for source in pick(csp) {
                    if !all.contains(source) {
                        all.push(source.clone());
                    }
                }
            }
            all.join(" ")
        };
        let script = sources(&["'self'", IMPORT_MAP_HASH.as_str()], |csp| &csp.script);
        let style = sources(&["'self'", "'unsafe-inline'"], |csp| &csp.style);
        let font = sources(&["'self'"], |csp| &csp.font);
        let connect = sources(&["'self'"], |csp| &csp.connect);
        let frame = sources(&["'self'"], |csp| &csp.frame);
        format!(
            "default-src 'self'; script-src {script}; style-src {style}; img-src 'self' data: https:; font-src {font}; connect-src {connect}; frame-src {frame}; object-src 'none'; base-uri 'self'; form-action 'self'; frame-ancestors 'self'"
        )
    }

    pub fn find(&self, path: &str) -> Option<(&Route, BTreeMap<String, String>)> {
        self.routes
            .iter()
            .find_map(|route| route.pattern.matches(path).map(|params| (route, params)))
    }
}

static CURRENT: RwLock<Option<Arc<SiteRuntime>>> = RwLock::new(None);

pub(crate) async fn current(app: &App, theme: Option<&str>) -> Result<Arc<SiteRuntime>, AppError> {
    let mut bundles = Vec::new();
    for source in app.contributions::<dyn SiteContentSource>() {
        bundles.extend(source.bundles().await?);
    }
    bundles.sort_by(|left, right| left.owner.cmp(&right.owner));
    let revision = bundles
        .iter()
        .map(|bundle| format!("{}@{}", bundle.owner, bundle.revision))
        .collect::<Vec<_>>()
        .join(",")
        + "|theme="
        + theme.unwrap_or_default();
    if let Some(runtime) = CURRENT
        .read()
        .ok()
        .and_then(|current| current.clone())
        .filter(|runtime| runtime.revision == revision)
    {
        return Ok(runtime);
    }
    let theme = theme
        .filter(|theme| {
            bundles
                .iter()
                .any(|bundle| bundle.owner == *theme && bundle.theme.is_some())
        })
        .map(str::to_owned);
    let runtime = Arc::new(build(revision, bundles, theme));
    if let Ok(mut current) = CURRENT.write() {
        *current = Some(runtime.clone());
    }
    Ok(runtime)
}

fn build(revision: String, bundles: Vec<SiteBundle>, theme: Option<String>) -> SiteRuntime {
    let mut environment = base_environment(theme.clone());
    let mut menu = Vec::new();
    let mut themes = Vec::new();
    let mut csp = BTreeMap::new();
    let mut uses_customers = false;
    let mut account_links = Vec::new();
    let mut islands: BTreeMap<String, Vec<String>> = BTreeMap::new();
    let mut routes: Vec<Route> = Vec::new();
    let mut exposures = BTreeMap::new();
    let mut paths: BTreeMap<String, BTreeMap<String, String>> = BTreeMap::new();
    for bundle in bundles {
        let problems = check_bundle(&bundle);
        if !problems.is_empty() {
            tracing::warn!(owner = %bundle.owner, ?problems, "skipping an invalid site bundle");
            continue;
        }
        for template in &bundle.templates {
            if let Err(error) = environment.add_template_owned(
                template_name(&bundle.owner, &template.name),
                template.source.clone(),
            ) {
                tracing::warn!(owner = %bundle.owner, template = %template.name, %error, "skipping a site template");
            }
        }
        if !bundle.csp.is_empty() {
            csp.insert(bundle.owner.clone(), bundle.csp.clone());
        }
        if !bundle.islands.is_empty() {
            islands.insert(bundle.owner.clone(), bundle.islands.clone());
        }
        if let Some(declared) = &bundle.theme {
            themes.push(ThemeInfo {
                key: bundle.owner.clone(),
                name: declared.name.clone(),
                description: declared.description.clone(),
            });
        }
        if theme.as_deref() == Some(bundle.owner.as_str()) {
            for (source, target) in [
                (THEME_LAYOUT, LAYOUT_TEMPLATE),
                (THEME_NOT_FOUND, NOT_FOUND_TEMPLATE),
                (THEME_ERROR, ERROR_TEMPLATE),
            ] {
                let Some(template) = bundle
                    .templates
                    .iter()
                    .find(|template| template.name == source)
                else {
                    continue;
                };
                if let Err(error) = environment.add_template_owned(target, template.source.clone())
                {
                    tracing::warn!(owner = %bundle.owner, template = %source, %error, "skipping a theme template");
                }
            }
        }
        uses_customers |=
            !bundle.account.is_empty() || bundle.pages.iter().any(|page| page.needs_customer());
        for item in &bundle.account {
            if let Some(page) = bundle.pages.iter().find(|page| page.key == item.page) {
                account_links.push(MenuCandidate {
                    id: menu_item_id(&bundle.owner, &item.key),
                    owner: bundle.owner.clone(),
                    item: item.clone(),
                    path: page.path.clone(),
                    source: page.source.clone(),
                });
            }
        }
        for item in &bundle.menu {
            if let Some(page) = bundle.pages.iter().find(|page| page.key == item.page) {
                menu.push(MenuCandidate {
                    id: menu_item_id(&bundle.owner, &item.key),
                    owner: bundle.owner.clone(),
                    item: item.clone(),
                    path: page.path.clone(),
                    source: page.source.clone(),
                });
            }
        }
        for exposure in &bundle.exposures {
            exposures.insert(
                (bundle.owner.clone(), exposure.entity.clone()),
                exposure.clone(),
            );
        }
        for page in bundle.pages {
            let Ok(pattern) = PathPattern::parse(&page.path) else {
                continue;
            };
            if let Some(taken) = routes
                .iter()
                .find(|route| route.pattern.shape() == pattern.shape())
            {
                tracing::warn!(owner = %bundle.owner, page = %page.key, taken_by = %taken.owner, "skipping a site page whose path is taken");
                continue;
            }
            paths
                .entry(bundle.owner.clone())
                .or_default()
                .insert(page.key.clone(), page.path.clone());
            routes.push(Route {
                owner: bundle.owner.clone(),
                page,
                pattern,
            });
        }
    }
    routes.sort_by_key(|route| std::cmp::Reverse(route.pattern.specificity()));
    environment.add_global("__site_pages", Value::from_serialize(&paths));
    environment.add_global("__site_islands", Value::from_serialize(&islands));
    SiteRuntime {
        revision,
        routes,
        exposures,
        environment,
        menu,
        themes,
        theme,
        csp,
        uses_customers,
        account_links,
    }
}

fn base_environment(theme: Option<String>) -> Environment<'static> {
    let mut environment = Environment::new();
    environment.set_fuel(Some(RENDER_FUEL));
    environment.add_global(
        "__site_theme",
        Value::from(theme.clone().unwrap_or_default()),
    );
    environment.add_function("theme_asset", theme_asset);
    environment.add_function("island", island);
    environment.add_global(
        "__site_import_map",
        Value::from_safe_string(import_map_tag()),
    );
    environment.set_path_join_callback(move |name, parent| {
        join_template_path(name, parent, theme.as_deref()).into()
    });
    for (name, source) in [
        (LAYOUT_TEMPLATE, include_str!("../theme/layout.html")),
        (NOT_FOUND_TEMPLATE, include_str!("../theme/not_found.html")),
        (ERROR_TEMPLATE, include_str!("../theme/error.html")),
        (HEAD_PARTIAL, include_str!("../theme/head.html")),
        (
            crate::account::ACCOUNT_TEMPLATE,
            include_str!("../theme/account.html"),
        ),
    ] {
        if let Err(error) = environment.add_template(name, source) {
            tracing::error!(template = name, %error, "the built-in site theme does not compile");
        }
    }
    environment.add_filter("markdown", markdown);
    environment.add_filter("date", date);
    environment.add_filter("image_url", image_url);
    environment.add_filter("from_json", from_json);
    environment.add_function("url_for", url_for);
    environment
}

fn markdown(text: Option<String>) -> Value {
    let text = text.unwrap_or_default();
    let parser = pulldown_cmark::Parser::new_ext(&text, pulldown_cmark::Options::all());
    let mut html = String::new();
    pulldown_cmark::html::push_html(&mut html, parser);
    Value::from_safe_string(ammonia::clean(&html))
}

fn depth_of(value: &serde_json::Value) -> usize {
    match value {
        serde_json::Value::Array(items) => 1 + items.iter().map(depth_of).max().unwrap_or(0),
        serde_json::Value::Object(fields) => 1 + fields.values().map(depth_of).max().unwrap_or(0),
        _ => 0,
    }
}

fn from_json(text: Option<String>) -> Result<Value, Error> {
    let Some(text) = text.filter(|text| !text.trim().is_empty()) else {
        return Ok(Value::from(()));
    };
    if text.len() > MAX_JSON_BYTES {
        return Err(Error::new(
            ErrorKind::InvalidOperation,
            format!("from_json: the text is larger than {MAX_JSON_BYTES} bytes"),
        ));
    }
    let parsed: serde_json::Value = serde_json::from_str(&text)
        .map_err(|error| Error::new(ErrorKind::InvalidOperation, format!("from_json: {error}")))?;
    if depth_of(&parsed) > MAX_JSON_DEPTH {
        return Err(Error::new(
            ErrorKind::InvalidOperation,
            format!("from_json: nested deeper than {MAX_JSON_DEPTH} levels"),
        ));
    }
    Ok(Value::from_serialize(&parsed))
}

fn date(value: Option<String>, format: Option<String>) -> String {
    let Some(value) = value.filter(|value| !value.is_empty()) else {
        return String::new();
    };
    let format = format.unwrap_or_else(|| DEFAULT_DATE_FORMAT.to_owned());
    let parsed = chrono::DateTime::parse_from_rfc3339(&value)
        .map(|moment| moment.naive_utc())
        .or_else(|_| chrono::NaiveDateTime::parse_from_str(&value, "%Y-%m-%dT%H:%M:%S%.f"))
        .or_else(|_| chrono::NaiveDateTime::parse_from_str(&value, "%Y-%m-%d %H:%M:%S%.f"))
        .or_else(|_| {
            chrono::NaiveDate::parse_from_str(&value, "%Y-%m-%d")
                .map(|day| day.and_time(chrono::NaiveTime::MIN))
        });
    match parsed {
        Ok(moment) => {
            let mut formatted = String::new();
            match std::fmt::write(&mut formatted, format_args!("{}", moment.format(&format))) {
                Ok(()) => formatted,
                Err(_) => value,
            }
        }
        Err(_) => value,
    }
}

fn image_url(image: Value, width: Option<u32>) -> String {
    let url = image
        .get_attr("url")
        .ok()
        .and_then(|url| url.as_str().map(str::to_owned))
        .unwrap_or_default();
    let Some(width) = width else {
        return url;
    };
    let chosen = IMAGE_WIDTHS
        .iter()
        .copied()
        .find(|candidate| *candidate >= width)
        .unwrap_or(IMAGE_WIDTHS[IMAGE_WIDTHS.len() - 1]);
    match url.strip_suffix("/original") {
        Some(base) => format!("{base}/w{chosen}"),
        None => url,
    }
}

fn escape_attribute(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

fn island(state: &State, name: String, props: Kwargs) -> Result<Value, Error> {
    let owner = state
        .lookup("page")
        .and_then(|current| current.get_attr("owner").ok())
        .and_then(|owner| owner.as_str().map(str::to_owned))
        .unwrap_or_default();
    let known = state
        .lookup("__site_islands")
        .and_then(|islands| islands.get_attr(&owner).ok())
        .and_then(|names| names.try_iter().ok().map(|names| names.collect::<Vec<_>>()))
        .is_some_and(|names| {
            names
                .iter()
                .any(|candidate| candidate.as_str() == Some(name.as_str()))
        });
    if !known {
        return Err(Error::new(
            ErrorKind::InvalidOperation,
            format!("island: {owner} has no site/islands/{name}.js"),
        ));
    }
    let mut values = serde_json::Map::new();
    for key in props.args() {
        let value: Value = props.get(key)?;
        values.insert(
            key.to_owned(),
            serde_json::to_value(&value).map_err(|error| {
                Error::new(
                    ErrorKind::InvalidOperation,
                    format!("island props: {error}"),
                )
            })?,
        );
    }
    props.assert_all_used()?;
    let encoded = serde_json::Value::Object(values).to_string();
    Ok(Value::from_safe_string(format!(
        "<div data-island=\"{}:{}\" data-props=\"{}\"></div>",
        escape_attribute(&owner),
        escape_attribute(&name),
        escape_attribute(&encoded)
    )))
}

fn theme_asset(state: &State, path: String) -> String {
    let theme = state
        .lookup("__site_theme")
        .and_then(|theme| theme.as_str().map(str::to_owned))
        .unwrap_or_default();
    if theme.is_empty() {
        return String::new();
    }
    format!(
        "/_aphel/site/{THEMES_SEGMENT}/{theme}/{}",
        path.trim_start_matches('/')
    )
}

fn url_for(state: &State, page: String, params: Kwargs) -> Result<String, Error> {
    let owner = state
        .lookup("page")
        .and_then(|current| current.get_attr("owner").ok())
        .and_then(|owner| owner.as_str().map(str::to_owned))
        .unwrap_or_default();
    let prefix = state
        .lookup("site")
        .and_then(|site| site.get_attr("prefix").ok())
        .and_then(|prefix| prefix.as_str().map(str::to_owned))
        .unwrap_or_default();
    let path = state
        .lookup("__site_pages")
        .and_then(|pages| pages.get_attr(&owner).ok())
        .and_then(|pages| pages.get_attr(&page).ok())
        .and_then(|path| path.as_str().map(str::to_owned))
        .ok_or_else(|| {
            Error::new(
                ErrorKind::InvalidOperation,
                format!("url_for: unknown page {page}"),
            )
        })?;
    let pattern = PathPattern::parse(&path)
        .map_err(|problem| Error::new(ErrorKind::InvalidOperation, problem))?;
    let mut values = BTreeMap::new();
    for name in pattern.params() {
        let value: Value = params.get(name)?;
        values.insert(name.to_owned(), value.to_string());
    }
    params.assert_all_used()?;
    let built = pattern.build(&values).ok_or_else(|| {
        Error::new(
            ErrorKind::InvalidOperation,
            format!("url_for: missing parameters for {page}"),
        )
    })?;
    Ok(format!("{prefix}{built}"))
}
