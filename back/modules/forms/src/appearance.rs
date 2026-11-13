use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::Row;
use utoipa::ToSchema;

use crate::store::public_path;
use aphel_kernel::Ctx;
use aphel_kernel::errors::{AppError, FieldViolation};
use aphel_kernel::modules::{SettingField, SettingKind, SettingOption};

pub(crate) const DEFAULT_ACCENT: &str = "#2563eb";
pub(crate) const MAX_INTRO_LENGTH: usize = 200;

pub(crate) const SETTING_THEME: &str = "default_theme";
pub(crate) const SETTING_RADIUS: &str = "default_radius";
pub(crate) const SETTING_FONT: &str = "default_font";
pub(crate) const SETTING_WIDTH: &str = "default_width";
pub(crate) const SETTING_LOGO: &str = "use_logo";

macro_rules! choice {
    ($name:ident { $($variant:ident => $text:literal),+ $(,)? }) => {
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
        #[serde(rename_all = "snake_case")]
        pub(crate) enum $name {
            $($variant),+
        }

        impl $name {
            pub(crate) fn as_str(&self) -> &'static str {
                match self {
                    $(Self::$variant => $text),+
                }
            }
        }
    };
}

choice!(Theme { Clean => "clean", Soft => "soft", Dark => "dark", Outline => "outline" });
choice!(Radius { Sharp => "sharp", Rounded => "rounded", Pill => "pill" });
choice!(Font { System => "system", Serif => "serif", Rounded => "rounded" });
choice!(Width { Narrow => "narrow", Medium => "medium", Wide => "wide" });
choice!(Surface { Plain => "plain", Card => "card" });
choice!(ButtonStyle { Solid => "solid", Outline => "outline" });
choice!(ButtonWidth { Auto => "auto", Full => "full" });
choice!(EmbedMode { Theme => "theme", Inherit => "inherit" });

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub(crate) struct LeadFormAppearance {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) theme: Option<Theme>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schema(example = "#2563eb")]
    pub(crate) accent: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) radius: Option<Radius>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) font: Option<Font>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) width: Option<Width>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) surface: Option<Surface>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) button: Option<ButtonStyle>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) button_width: Option<ButtonWidth>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) show_title: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) show_logo: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) intro: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) embed_mode: Option<EmbedMode>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Placement {
    Page,
    Embed,
    Site,
}

#[derive(Debug, Clone)]
pub(crate) struct Branding {
    pub(crate) accent: Option<String>,
    pub(crate) has_logo: bool,
    pub(crate) logo_file_id: Option<String>,
    pub(crate) theme: Theme,
    pub(crate) radius: Radius,
    pub(crate) font: Font,
    pub(crate) width: Width,
    pub(crate) use_logo: bool,
}

impl Default for Branding {
    fn default() -> Self {
        Self {
            accent: None,
            has_logo: false,
            logo_file_id: None,
            theme: Theme::Clean,
            radius: Radius::Rounded,
            font: Font::System,
            width: Width::Medium,
            use_logo: true,
        }
    }
}

fn choice<T: serde::de::DeserializeOwned>(settings: &Value, key: &str) -> Option<T> {
    settings
        .get(key)
        .and_then(|value| serde_json::from_value(value.clone()).ok())
}

pub(crate) async fn branding(ctx: &Ctx<'_>) -> Result<Branding, AppError> {
    let settings = aphel_sdk::platform::modules::module_settings(ctx.pool(), crate::KEY).await?;
    let row =
        sqlx::query("SELECT logo_file_id, primary_color FROM organization_settings WHERE id = 1")
            .fetch_optional(ctx.pool())
            .await?;
    let (logo, color) = match row {
        Some(row) => (
            row.try_get::<Option<String>, _>("logo_file_id")?,
            row.try_get::<Option<String>, _>("primary_color")?,
        ),
        None => (None, None),
    };
    let fallback = Branding::default();
    Ok(Branding {
        accent: color.filter(|value| is_hex_color(value)),
        has_logo: logo.is_some(),
        logo_file_id: logo,
        theme: choice(&settings, SETTING_THEME).unwrap_or(fallback.theme),
        radius: choice(&settings, SETTING_RADIUS).unwrap_or(fallback.radius),
        font: choice(&settings, SETTING_FONT).unwrap_or(fallback.font),
        width: choice(&settings, SETTING_WIDTH).unwrap_or(fallback.width),
        use_logo: settings
            .get(SETTING_LOGO)
            .and_then(Value::as_bool)
            .unwrap_or(fallback.use_logo),
    })
}

pub(crate) fn is_hex_color(value: &str) -> bool {
    value.len() == 7
        && value.starts_with('#')
        && value[1..]
            .chars()
            .all(|character| character.is_ascii_hexdigit())
}

pub(crate) fn validate(appearance: &LeadFormAppearance, violations: &mut Vec<FieldViolation>) {
    if appearance
        .accent
        .as_deref()
        .is_some_and(|value| !is_hex_color(value))
    {
        violations.push(FieldViolation::new(
            "appearance.accent",
            "must be a color like #2563eb",
        ));
    }
    if appearance
        .intro
        .as_deref()
        .is_some_and(|value| value.chars().count() > MAX_INTRO_LENGTH)
    {
        violations.push(FieldViolation::new(
            "appearance.intro",
            "must have at most 200 characters",
        ));
    }
}

pub(crate) fn clean(appearance: LeadFormAppearance) -> Option<LeadFormAppearance> {
    let appearance = LeadFormAppearance {
        accent: appearance
            .accent
            .map(|value| value.trim().to_ascii_lowercase())
            .filter(|value| !value.is_empty()),
        intro: appearance
            .intro
            .map(|value| value.trim().to_owned())
            .filter(|value| !value.is_empty()),
        ..appearance
    };
    (appearance != LeadFormAppearance::default()).then_some(appearance)
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub(crate) struct LeadFormPublicAppearance {
    pub(crate) vars: BTreeMap<String, String>,
    pub(crate) attributes: BTreeMap<String, String>,
    pub(crate) title: Option<String>,
    pub(crate) intro: Option<String>,
    pub(crate) logo_url: Option<String>,
}

#[derive(Debug, Clone)]
pub(crate) struct Resolved {
    pub(crate) theme: Theme,
    pub(crate) accent: String,
    pub(crate) radius: Radius,
    pub(crate) font: Font,
    pub(crate) width: Width,
    pub(crate) surface: Surface,
    pub(crate) button: ButtonStyle,
    pub(crate) button_width: ButtonWidth,
    pub(crate) show_title: bool,
    pub(crate) logo_url: Option<String>,
    pub(crate) intro: Option<String>,
    pub(crate) mode: EmbedMode,
}

pub(crate) fn resolve(
    appearance: Option<&LeadFormAppearance>,
    branding: &Branding,
    public_id: &str,
    placement: Placement,
) -> Resolved {
    let empty = LeadFormAppearance::default();
    let own = appearance.unwrap_or(&empty);
    let mode = match placement {
        Placement::Page => EmbedMode::Theme,
        Placement::Embed => own.embed_mode.unwrap_or(EmbedMode::Theme),
        Placement::Site => own.embed_mode.unwrap_or(EmbedMode::Inherit),
    };
    let show_logo = own.show_logo.unwrap_or(branding.use_logo) && branding.has_logo;
    Resolved {
        theme: own.theme.unwrap_or(branding.theme),
        accent: own
            .accent
            .clone()
            .or_else(|| branding.accent.clone())
            .unwrap_or_else(|| DEFAULT_ACCENT.to_owned()),
        radius: own.radius.unwrap_or(branding.radius),
        font: own.font.unwrap_or(branding.font),
        width: own.width.unwrap_or(branding.width),
        surface: own.surface.unwrap_or(match placement {
            Placement::Page => Surface::Card,
            Placement::Embed | Placement::Site => Surface::Plain,
        }),
        button: own.button.unwrap_or(ButtonStyle::Solid),
        button_width: own.button_width.unwrap_or(ButtonWidth::Auto),
        show_title: own.show_title.unwrap_or(placement == Placement::Page),
        logo_url: show_logo.then(|| public_path(public_id, "/logo")),
        intro: own.intro.clone(),
        mode,
    }
}

struct Palette {
    page: &'static str,
    surface: &'static str,
    fg: &'static str,
    muted: &'static str,
    line: &'static str,
    field: &'static str,
}

fn palette(theme: Theme) -> Palette {
    match theme {
        Theme::Clean => Palette {
            page: "#f3f4f6",
            surface: "#ffffff",
            fg: "#111827",
            muted: "#6b7280",
            line: "#d1d5db",
            field: "#ffffff",
        },
        Theme::Soft => Palette {
            page: "#f6f1ea",
            surface: "#fffdf9",
            fg: "#2b2522",
            muted: "#78706a",
            line: "#e6ddd2",
            field: "#ffffff",
        },
        Theme::Dark => Palette {
            page: "#0b1120",
            surface: "#111827",
            fg: "#f3f4f6",
            muted: "#9ca3af",
            line: "#374151",
            field: "#1f2937",
        },
        Theme::Outline => Palette {
            page: "#ffffff",
            surface: "#ffffff",
            fg: "#0f172a",
            muted: "#475569",
            line: "#0f172a",
            field: "#ffffff",
        },
    }
}

fn luminance(hex: &str) -> f64 {
    let channel = |range: std::ops::Range<usize>| {
        let value = u8::from_str_radix(&hex[range], 16).unwrap_or(0);
        let value = f64::from(value) / 255.0;
        if value <= 0.03928 {
            value / 12.92
        } else {
            ((value + 0.055) / 1.055).powf(2.4)
        }
    };
    0.2126 * channel(1..3) + 0.7152 * channel(3..5) + 0.0722 * channel(5..7)
}

pub(crate) fn contrast_text(background: &str) -> &'static str {
    if luminance(background) > 0.4 {
        "#111827"
    } else {
        "#ffffff"
    }
}

impl Resolved {
    pub(crate) fn vars(&self) -> BTreeMap<String, String> {
        let palette = palette(self.theme);
        let (field_radius, button_radius) = match self.radius {
            Radius::Sharp => ("2px", "2px"),
            Radius::Rounded => ("10px", "10px"),
            Radius::Pill => ("14px", "999px"),
        };
        let font = match self.font {
            Font::System => "system-ui,-apple-system,'Segoe UI',Roboto,'Helvetica Neue',sans-serif",
            Font::Serif => "Georgia,'Times New Roman',Times,serif",
            Font::Rounded => {
                "ui-rounded,'SF Pro Rounded','Hiragino Maru Gothic ProN',Quicksand,Nunito,system-ui,sans-serif"
            }
        };
        let width = match self.width {
            Width::Narrow => "480px",
            Width::Medium => "560px",
            Width::Wide => "720px",
        };
        let mut vars = BTreeMap::new();
        vars.insert("--lf-accent".into(), self.accent.clone());
        vars.insert("--lf-on-accent".into(), contrast_text(&self.accent).into());
        vars.insert("--lf-radius".into(), field_radius.into());
        vars.insert("--lf-radius-btn".into(), button_radius.into());
        vars.insert("--lf-width".into(), width.into());
        vars.insert("--lf-page".into(), palette.page.into());
        if self.mode == EmbedMode::Theme {
            vars.insert("--lf-font".into(), font.into());
            vars.insert("--lf-surface".into(), palette.surface.into());
            vars.insert("--lf-fg".into(), palette.fg.into());
            vars.insert("--lf-muted".into(), palette.muted.into());
            vars.insert("--lf-line".into(), palette.line.into());
            vars.insert("--lf-field".into(), palette.field.into());
        }
        vars
    }

    pub(crate) fn style(&self) -> String {
        self.vars()
            .into_iter()
            .map(|(name, value)| format!("{name}:{value}"))
            .collect::<Vec<_>>()
            .join(";")
    }

    pub(crate) fn attributes(&self) -> BTreeMap<String, String> {
        let mut attributes = BTreeMap::new();
        attributes.insert("theme".into(), self.theme.as_str().into());
        attributes.insert("radius".into(), self.radius.as_str().into());
        attributes.insert("font".into(), self.font.as_str().into());
        attributes.insert("width".into(), self.width.as_str().into());
        attributes.insert("surface".into(), self.surface.as_str().into());
        attributes.insert("button".into(), self.button.as_str().into());
        attributes.insert("button-width".into(), self.button_width.as_str().into());
        attributes.insert("mode".into(), self.mode.as_str().into());
        attributes
    }

    pub(crate) fn public(&self, title: &str) -> LeadFormPublicAppearance {
        LeadFormPublicAppearance {
            vars: self.vars(),
            attributes: self.attributes(),
            title: self.show_title.then(|| title.to_owned()),
            intro: self.intro.clone(),
            logo_url: self.logo_url.clone(),
        }
    }

    pub(crate) fn attribute_markup(&self) -> String {
        self.attributes()
            .into_iter()
            .map(|(name, value)| format!(" data-{name}=\"{value}\""))
            .collect()
    }
}

pub(crate) fn choice_setting(
    key: &'static str,
    label: &'static str,
    help: &'static str,
    options: &[(&'static str, &'static str)],
    default: &'static str,
) -> SettingField {
    SettingField::new(
        key,
        label,
        SettingKind::Select {
            options: options
                .iter()
                .map(|(value, label)| SettingOption { value, label })
                .collect(),
        },
    )
    .default_value(serde_json::json!(default))
    .help(help)
}
