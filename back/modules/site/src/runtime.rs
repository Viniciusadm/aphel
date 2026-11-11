use std::collections::BTreeMap;
use std::sync::{Arc, RwLock};

use aphel_contract_site::{
    IMAGE_WIDTHS, LAYOUT_TEMPLATE, PathPattern, SEO_PARTIAL, SiteBundle, SiteContentSource,
    SiteExposure, SitePage, THEME_ERROR, THEME_LAYOUT, THEME_NOT_FOUND, check_bundle,
    join_template_path, menu_item_id, template_name,
};
use aphel_kernel::errors::AppError;
use aphel_kernel::modules::App;
use minijinja::value::Kwargs;
use minijinja::{Environment, Error, ErrorKind, State, Value};

pub(crate) const NOT_FOUND_TEMPLATE: &str = "aphel/not_found.html";
pub(crate) const ERROR_TEMPLATE: &str = "aphel/error.html";
pub(crate) const THEMES_SEGMENT: &str = "themes";
const RENDER_FUEL: u64 = 2_000_000;
const DEFAULT_DATE_FORMAT: &str = "%d/%m/%Y";

pub(crate) struct Route {
    pub owner: String,
    pub page: SitePage,
    pub pattern: PathPattern,
}

pub(crate) struct MenuCandidate {
    pub id: String,
    pub label: String,
    pub path: String,
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

    pub fn exposure(&self, owner: &str, entity: &str) -> Option<&SiteExposure> {
        self.exposures.get(&(owner.to_owned(), entity.to_owned()))
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
        for item in &bundle.menu {
            if let Some(page) = bundle.pages.iter().find(|page| page.key == item.page) {
                menu.push(MenuCandidate {
                    id: menu_item_id(&bundle.owner, &item.key),
                    label: item.label.clone(),
                    path: page.path.clone(),
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
    SiteRuntime {
        revision,
        routes,
        exposures,
        environment,
        menu,
        themes,
        theme,
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
    environment.set_path_join_callback(move |name, parent| {
        join_template_path(name, parent, theme.as_deref()).into()
    });
    for (name, source) in [
        (LAYOUT_TEMPLATE, include_str!("../theme/layout.html")),
        (NOT_FOUND_TEMPLATE, include_str!("../theme/not_found.html")),
        (ERROR_TEMPLATE, include_str!("../theme/error.html")),
        (SEO_PARTIAL, include_str!("../theme/seo.html")),
    ] {
        if let Err(error) = environment.add_template(name, source) {
            tracing::error!(template = name, %error, "the built-in site theme does not compile");
        }
    }
    environment.add_filter("markdown", markdown);
    environment.add_filter("date", date);
    environment.add_filter("image_url", image_url);
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
