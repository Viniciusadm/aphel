use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;

pub const APPEARANCE_ENTITY: &str = "site.appearance";
pub const PRESENTATION_ENTITY: &str = "site.presentation";
pub const APPEARANCE_EDIT: &str = "site.appearance.edit";
pub const PRESENTATION_EDIT: &str = "site.presentation.edit";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AppearanceProperty {
    pub key: String,
    pub group: String,
    pub kind: String,
    pub default: Value,
    #[serde(default)]
    pub options: Vec<String>,
    #[serde(default)]
    pub collection: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct AppearanceValues(pub BTreeMap<String, Value>);

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AppearancePreset {
    pub key: String,
    pub values: AppearanceValues,
}

pub fn appearance_properties() -> Vec<AppearanceProperty> {
    let mut properties = Vec::new();
    for (key, default) in [
        ("dark_background", "#111116"),
        ("dark_surface", "#1b1b24"),
        ("dark_text", "#ececf1"),
        ("dark_muted", "#b4b4c3"),
        ("dark_border", "#41414b"),
        ("dark_link", "#aeb7ff"),
        ("background", "#fbfbfd"),
        ("surface", "#ffffff"),
        ("text", "#1d1d24"),
        ("muted", "#5d5d6b"),
        ("accent", "#3847a1"),
        ("link", "#3847a1"),
        ("border", "#e4e4ea"),
    ] {
        properties.push(AppearanceProperty {
            key: key.into(),
            group: "identity".into(),
            kind: "color".into(),
            default: default.into(),
            options: vec![],
            collection: false,
        });
    }
    for (key, default, options, group, collection) in [
        ("mode", "system", "light,dark,system", "identity", false),
        (
            "heading_font",
            "sans",
            "sans,serif,mono",
            "typography",
            false,
        ),
        ("body_font", "sans", "sans,serif", "typography", false),
        (
            "heading_scale",
            "normal",
            "small,normal,large",
            "typography",
            false,
        ),
        (
            "reading_size",
            "normal",
            "normal,large",
            "typography",
            false,
        ),
        (
            "line_height",
            "comfortable",
            "compact,comfortable,relaxed",
            "typography",
            false,
        ),
        (
            "content_width",
            "comfortable",
            "comfortable,wide",
            "spacing",
            false,
        ),
        (
            "density",
            "comfortable",
            "compact,comfortable,relaxed",
            "spacing",
            true,
        ),
        ("radius", "soft", "none,soft,round", "spacing", false),
        ("shadow", "none", "none,soft,strong", "spacing", false),
        ("header_size", "compact", "compact,wide", "frame", false),
        ("navigation", "inline", "inline,below", "frame", false),
        ("layout", "grid", "list,grid,featured", "collection", true),
        ("columns", "2", "1,2,3,4", "collection", true),
        ("per_page", "12", "6,12,24,48", "collection", true),
        ("card_image", "top", "top,side,none", "cards", true),
        (
            "image_ratio",
            "landscape",
            "landscape,square,portrait",
            "cards",
            true,
        ),
        ("image_fit", "cover", "cover,contain", "cards", true),
        ("card_size", "normal", "small,normal,large", "cards", true),
        (
            "cover_position",
            "below",
            "above,below,none",
            "detail",
            true,
        ),
        (
            "reading_width",
            "comfortable",
            "comfortable,wide",
            "detail",
            true,
        ),
        ("title_align", "left", "left,center", "detail", true),
    ] {
        properties.push(AppearanceProperty {
            key: key.into(),
            group: group.into(),
            kind: "select".into(),
            default: default.into(),
            options: options.split(',').map(str::to_owned).collect(),
            collection,
        });
    }
    for (key, default, group, collection) in [
        ("sticky_header", false, "frame", false),
        ("footer_brand", true, "frame", false),
        ("show_title", true, "collection", true),
        ("show_description", true, "collection", true),
        ("show_summary", true, "cards", true),
        ("show_byline", true, "detail", true),
        ("show_date", true, "detail", true),
        ("show_updated", true, "detail", true),
        ("show_terms", true, "detail", true),
        ("show_share", false, "detail", true),
        ("show_toc", false, "detail", true),
    ] {
        properties.push(AppearanceProperty {
            key: key.into(),
            group: group.into(),
            kind: "boolean".into(),
            default: default.into(),
            options: vec![],
            collection,
        });
    }
    for key in ["name", "tagline", "logo", "dark_logo", "icon"] {
        properties.push(AppearanceProperty {
            key: key.into(),
            group: "identity".into(),
            kind: if key == "name" || key == "tagline" {
                "text"
            } else {
                "file"
            }
            .into(),
            default: Value::Null,
            options: vec![],
            collection: false,
        });
    }
    properties.push(AppearanceProperty {
        key: "opening_image".into(),
        group: "collection".into(),
        kind: "file".into(),
        default: Value::Null,
        options: vec![],
        collection: true,
    });
    properties.push(AppearanceProperty {
        key: "theme".into(),
        group: "identity".into(),
        kind: "theme".into(),
        default: "".into(),
        options: vec![],
        collection: false,
    });
    for key in ["header", "footer", "translations"] {
        properties.push(AppearanceProperty {
            key: key.into(),
            group: "frame".into(),
            kind: "frame".into(),
            default: Value::Null,
            options: vec![],
            collection: false,
        });
    }
    properties
}

pub fn appearance_presets() -> Vec<AppearancePreset> {
    [
        ("editorial", "grid", "2", "top", "serif"),
        ("magazine", "featured", "3", "top", "sans"),
        ("minimal", "list", "1", "none", "sans"),
    ]
    .into_iter()
    .map(|(key, layout, columns, image, _font)| AppearancePreset {
        key: key.into(),
        values: AppearanceValues(
            [
                ("layout".into(), layout.into()),
                ("columns".into(), columns.into()),
                ("card_image".into(), image.into()),
            ]
            .into(),
        ),
    })
    .collect()
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ThemeAppearance {
    #[serde(default)]
    pub defaults: AppearanceValues,
    #[serde(default)]
    pub properties: Vec<String>,
    #[serde(default)]
    pub presentations: BTreeMap<String, String>,
}
