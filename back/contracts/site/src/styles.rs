use aphel_blocks::style::{self as block_style, Scope, is_hex_color, is_length, is_slug};
use serde_json::{Map, Value, json};

pub const FONT_STACKS: &[(&str, &str)] = &[
    (
        "sans",
        r#"system-ui,-apple-system,"Segoe UI",Roboto,"Helvetica Neue",sans-serif"#,
    ),
    ("serif", r#"Georgia,"Times New Roman",serif"#),
    (
        "mono",
        r#"ui-monospace,"SF Mono","Cascadia Code","JetBrains Mono",Menlo,Consolas,monospace"#,
    ),
    (
        "humanist",
        r#"Seravek,"Gill Sans Nova",Ubuntu,Calibri,"DejaVu Sans",source-sans-pro,sans-serif"#,
    ),
    (
        "geometric",
        r#"Avenir,"Avenir Next",Montserrat,"Century Gothic","URW Gothic",Futura,sans-serif"#,
    ),
    (
        "transitional",
        r#"Charter,"Bitstream Charter","Sitka Text",Cambria,"Noto Serif",Georgia,serif"#,
    ),
    (
        "old_style",
        r#""Iowan Old Style","Palatino Linotype","URW Palladio L",P052,"Book Antiqua",Palatino,serif"#,
    ),
];
pub const REQUIRED_COLORS: &[&str] = &[
    "base",
    "surface",
    "contrast",
    "muted",
    "border",
    "accent",
    "link",
    "on-accent",
];
pub const ELEMENTS: &[(&str, &str)] = &[
    ("body", "body"),
    ("heading", "h1,h2,h3,h4,h5,h6"),
    ("h1", "h1"),
    ("h2", "h2"),
    ("h3", "h3"),
    ("h4", "h4"),
    ("h5", "h5"),
    ("h6", "h6"),
    ("link", "a"),
    (
        "button",
        ".blk-button-primary,a.cta,.site-search-form button,.blk-form button,.site-booking-button",
    ),
    ("caption", "figcaption"),
    ("quote", ".blk-slot[data-type=\"quote\"]"),
    ("code", ".blk-code"),
];
pub const MODES: &[&str] = &["system", "light", "dark"];
pub const SHAPE_RADII: &[&str] = &["soft", "none", "round"];
pub const SHAPE_SHADOWS: &[&str] = &["none", "soft", "strong"];
const MAX_PALETTE: usize = 24;
const MAX_GRADIENTS: usize = 12;
const MAX_FAMILIES: usize = 8;
const MAX_SIZES: usize = 12;
const MAX_VARIATIONS: usize = 8;
const MAX_NAME_CHARS: usize = 40;
const SPACE_SCALE: usize = block_style::SPACE_STEPS;

pub fn default_styles() -> Value {
    json!({
        "mode": "system",
        "palette": [
            { "slug": "base", "name": "Fundo", "light": "#fbfbfd", "dark": "#111116" },
            { "slug": "surface", "name": "Superfície", "light": "#ffffff", "dark": "#1b1b24" },
            { "slug": "contrast", "name": "Texto", "light": "#1d1d24", "dark": "#ececf1" },
            { "slug": "muted", "name": "Texto secundário", "light": "#5d5d6b", "dark": "#b4b4c3" },
            { "slug": "border", "name": "Bordas", "light": "#e4e4ea", "dark": "#41414b" },
            { "slug": "accent", "name": "Destaque", "light": "#3847a1", "dark": "#3847a1" },
            { "slug": "link", "name": "Links", "light": "#3847a1", "dark": "#aeb7ff" },
            { "slug": "on-accent", "name": "Texto sobre destaque", "light": "#ffffff", "dark": "#ffffff" }
        ],
        "gradients": [
            { "slug": "accent-fade", "name": "Destaque suave", "angle": 135, "stops": ["accent", "surface"] }
        ],
        "families": [
            { "slug": "heading", "name": "Títulos", "stack": "sans" },
            { "slug": "body", "name": "Texto", "stack": "sans" },
            { "slug": "mono", "name": "Código", "stack": "mono" }
        ],
        "sizes": [
            { "slug": "xs", "name": "Muito pequeno", "min": "0.75rem", "max": "0.75rem" },
            { "slug": "sm", "name": "Pequeno", "min": "0.875rem", "max": "0.875rem" },
            { "slug": "md", "name": "Normal", "min": "1rem", "max": "1rem" },
            { "slug": "lg", "name": "Médio", "min": "1.125rem", "max": "1.25rem" },
            { "slug": "xl", "name": "Grande", "min": "1.25rem", "max": "1.5rem" },
            { "slug": "2xl", "name": "Maior", "min": "1.5rem", "max": "1.875rem" },
            { "slug": "3xl", "name": "Título 3", "min": "1.75rem", "max": "2.25rem" },
            { "slug": "4xl", "name": "Título 2", "min": "2rem", "max": "2.75rem" },
            { "slug": "5xl", "name": "Título 1", "min": "2.25rem", "max": "3.25rem" }
        ],
        "spacing": {
            "scale": ["0", "0.25rem", "0.5rem", "0.75rem", "1rem", "1.5rem", "2.5rem", "4rem", "6rem"],
            "content": "46rem",
            "wide": "68rem"
        },
        "shape": { "radius": "soft", "shadow": "none" },
        "elements": {
            "body": { "font_family": "body", "font_size": "md", "line_height": "normal", "text": "contrast", "background": "base" },
            "heading": { "font_family": "heading", "font_weight": "700", "line_height": "tight", "letter_spacing": "tight" },
            "h1": { "font_size": "5xl" },
            "h2": { "font_size": "4xl" },
            "h3": { "font_size": "2xl" },
            "h4": { "font_size": "xl" },
            "link": { "text": "link", "hover": { "text": "contrast" } },
            "button": { "background": "accent", "text": "on-accent", "radius": "full", "font_weight": "600" },
            "caption": { "text": "muted", "font_size": "sm" },
            "quote": { "font_family": "heading", "font_size": "xl" },
            "code": { "font_family": "mono" }
        },
        "blocks": {
            "quote": { "variations": [
                { "slug": "large", "name": "Grande", "style": { "font_size": "3xl", "text_align": "center" } }
            ] },
            "image": { "variations": [
                { "slug": "frame", "name": "Moldura", "style": { "padding": { "top": "2", "right": "2", "bottom": "2", "left": "2" }, "background": "surface", "shadow": "md" } }
            ] },
            "buttons": { "variations": [
                { "slug": "square", "name": "Cantos retos", "style": { "radius": "none" } }
            ] }
        }
    })
}

fn palette(entries: &[(&str, &str, &str)]) -> Value {
    Value::Array(
        entries
            .iter()
            .map(|(slug, light, dark)| json!({ "slug": slug, "light": light, "dark": dark }))
            .collect(),
    )
}

pub fn style_variations() -> Vec<(&'static str, Value)> {
    vec![
        ("default", json!({})),
        (
            "editorial",
            json!({
                "palette": palette(&[
                    ("base", "#faf7f2", "#16120f"), ("surface", "#ffffff", "#201b17"),
                    ("contrast", "#1f1a17", "#f1ebe3"), ("muted", "#675c53", "#b8ad9f"),
                    ("border", "#e6ddd1", "#3a322b"), ("accent", "#8a1c2b", "#8a1c2b"),
                    ("link", "#8a1c2b", "#efa3ad"), ("on-accent", "#ffffff", "#ffffff"),
                ]),
                "families": [{ "slug": "heading", "stack": "transitional" }, { "slug": "body", "stack": "transitional" }],
                "shape": { "radius": "none", "shadow": "none" },
                "elements": {
                    "body": { "font_size": "lg", "line_height": "relaxed" },
                    "h1": { "font_size": "5xl" },
                    "heading": { "letter_spacing": "tighter" }
                }
            }),
        ),
        (
            "magazine",
            json!({
                "palette": palette(&[
                    ("base", "#ffffff", "#0e0e0e"), ("surface", "#ffffff", "#181818"),
                    ("contrast", "#111111", "#f2f2f2"), ("muted", "#595959", "#a8a8a8"),
                    ("border", "#e6e6e6", "#2e2e2e"), ("accent", "#c1121f", "#c1121f"),
                    ("link", "#a50f1a", "#ff8f87"), ("on-accent", "#ffffff", "#ffffff"),
                ]),
                "families": [{ "slug": "heading", "stack": "geometric" }, { "slug": "body", "stack": "sans" }],
                "shape": { "radius": "soft", "shadow": "none" },
                "elements": { "heading": { "font_weight": "800", "letter_spacing": "tighter" } }
            }),
        ),
        (
            "minimal",
            json!({
                "palette": palette(&[
                    ("base", "#ffffff", "#0b0b0c"), ("surface", "#ffffff", "#141416"),
                    ("contrast", "#18181b", "#ededf0"), ("muted", "#5f5f68", "#a1a1aa"),
                    ("border", "#ececef", "#27272a"), ("accent", "#18181b", "#ededf0"),
                    ("link", "#18181b", "#ededf0"), ("on-accent", "#ffffff", "#18181b"),
                ]),
                "shape": { "radius": "none", "shadow": "none" },
                "elements": {
                    "body": { "line_height": "relaxed" },
                    "heading": { "font_weight": "600" },
                    "link": { "decoration": "underline" }
                }
            }),
        ),
        (
            "modern",
            json!({
                "palette": palette(&[
                    ("base", "#f7f7fb", "#0f1020"), ("surface", "#ffffff", "#181a2e"),
                    ("contrast", "#1b1b2b", "#ececf6"), ("muted", "#565670", "#a7a9c4"),
                    ("border", "#e5e5f0", "#2c2f4a"), ("accent", "#4f46e5", "#4f46e5"),
                    ("link", "#4338ca", "#a5b4fc"), ("on-accent", "#ffffff", "#ffffff"),
                ]),
                "families": [{ "slug": "heading", "stack": "geometric" }, { "slug": "body", "stack": "sans" }],
                "shape": { "radius": "round", "shadow": "soft" }
            }),
        ),
        (
            "night",
            json!({
                "mode": "dark",
                "palette": palette(&[
                    ("base", "#fffaf0", "#14120f"), ("surface", "#ffffff", "#1f1b17"),
                    ("contrast", "#1c1917", "#f3eee6"), ("muted", "#5f574d", "#b5ac9e"),
                    ("border", "#eadfcc", "#38322b"), ("accent", "#b45309", "#b45309"),
                    ("link", "#92400e", "#f5b84a"), ("on-accent", "#ffffff", "#ffffff"),
                ]),
                "families": [{ "slug": "heading", "stack": "humanist" }, { "slug": "body", "stack": "humanist" }],
                "shape": { "radius": "soft", "shadow": "strong" },
                "elements": { "body": { "font_size": "lg" } }
            }),
        ),
    ]
}

pub fn variation(key: &str) -> Option<Value> {
    style_variations()
        .into_iter()
        .find(|(name, _)| *name == key)
        .map(|(_, styles)| styles)
}

fn name(value: Option<&Value>, label: &str) -> Result<Option<Value>, String> {
    match value {
        None | Some(Value::Null) => Ok(None),
        Some(Value::String(text))
            if text.chars().count() <= MAX_NAME_CHARS && !text.chars().any(char::is_control) =>
        {
            Ok(Some(Value::String(text.trim().to_owned())))
        }
        Some(_) => Err(format!(
            "{label}: the name has more than {MAX_NAME_CHARS} characters"
        )),
    }
}

fn entry_slug(entry: &Map<String, Value>, label: &str) -> Result<String, String> {
    entry
        .get("slug")
        .and_then(Value::as_str)
        .filter(|slug| is_slug(slug))
        .map(str::to_owned)
        .ok_or_else(|| {
            format!("{label}: every entry needs a slug with lowercase letters, digits and -")
        })
}

fn object<'a>(value: &'a Value, label: &str) -> Result<&'a Map<String, Value>, String> {
    value
        .as_object()
        .ok_or_else(|| format!("{label} must be an object"))
}

fn list<'a>(value: &'a Value, label: &str, max: usize) -> Result<&'a Vec<Value>, String> {
    let entries = value
        .as_array()
        .ok_or_else(|| format!("{label} must be a list"))?;
    if entries.len() > max {
        return Err(format!("{label} has more than {max} entries"));
    }
    Ok(entries)
}

fn keep(
    entry: &Map<String, Value>,
    clean: &mut Map<String, Value>,
    key: &str,
    label: &str,
) -> Result<(), String> {
    if let Some(text) = name(entry.get(key), label)? {
        clean.insert(key.to_owned(), text);
    }
    Ok(())
}

fn hex(entry: &Map<String, Value>, key: &str, label: &str) -> Result<Option<Value>, String> {
    match entry.get(key) {
        None | Some(Value::Null) => Ok(None),
        Some(Value::String(color)) if is_hex_color(color) => {
            Ok(Some(Value::String(color.to_ascii_lowercase())))
        }
        Some(_) => Err(format!("{label}: {key} must be a #rrggbb code")),
    }
}

fn rem(value: Option<&Value>, label: &str) -> Result<Option<Value>, String> {
    match value {
        None | Some(Value::Null) => Ok(None),
        Some(Value::String(length)) if length.ends_with("rem") && is_length(length) => {
            Ok(Some(Value::String(length.clone())))
        }
        Some(_) => Err(format!("{label} must be a size in rem")),
    }
}

fn length(value: Option<&Value>, label: &str) -> Result<Option<Value>, String> {
    match value {
        None | Some(Value::Null) => Ok(None),
        Some(Value::String(text)) if is_length(text) => Ok(Some(Value::String(text.clone()))),
        Some(_) => Err(format!("{label} must be a length such as 1rem or 46rem")),
    }
}

fn element(value: &Value, label: &str) -> Result<Value, String> {
    block_style::normalize(Some(value), Scope::Element)
        .map_err(|error| format!("{label}: {}", error.message))
}

fn palette_entries(value: &Value) -> Result<Value, String> {
    let mut clean = Vec::new();
    for entry in list(value, "palette", MAX_PALETTE)? {
        let entry = object(entry, "palette entry")?;
        let mut item = Map::new();
        item.insert("slug".into(), Value::String(entry_slug(entry, "palette")?));
        keep(entry, &mut item, "name", "palette")?;
        for key in ["light", "dark"] {
            if let Some(color) = hex(entry, key, "palette")? {
                item.insert(key.into(), color);
            }
        }
        clean.push(Value::Object(item));
    }
    Ok(Value::Array(clean))
}

fn gradient_entries(value: &Value) -> Result<Value, String> {
    let mut clean = Vec::new();
    for entry in list(value, "gradients", MAX_GRADIENTS)? {
        let entry = object(entry, "gradient")?;
        let mut item = Map::new();
        item.insert(
            "slug".into(),
            Value::String(entry_slug(entry, "gradients")?),
        );
        keep(entry, &mut item, "name", "gradients")?;
        if let Some(angle) = entry.get("angle").filter(|angle| !angle.is_null()) {
            let angle = angle
                .as_u64()
                .filter(|angle| *angle <= 360)
                .ok_or("gradients: the angle goes from 0 to 360")?;
            item.insert("angle".into(), Value::from(angle));
        }
        if let Some(stops) = entry.get("stops").filter(|stops| !stops.is_null()) {
            let stops = list(stops, "gradient stops", 4)?;
            if stops.len() < 2 {
                return Err("gradients: use from 2 to 4 colors".into());
            }
            let colors = stops
                .iter()
                .map(|stop| {
                    block_style::color(stop, "gradient color")
                        .map(Value::String)
                        .map_err(|error| error.message)
                })
                .collect::<Result<Vec<_>, _>>()?;
            item.insert("stops".into(), Value::Array(colors));
        }
        clean.push(Value::Object(item));
    }
    Ok(Value::Array(clean))
}

fn family_entries(value: &Value) -> Result<Value, String> {
    let mut clean = Vec::new();
    for entry in list(value, "families", MAX_FAMILIES)? {
        let entry = object(entry, "font family")?;
        let mut item = Map::new();
        item.insert("slug".into(), Value::String(entry_slug(entry, "families")?));
        keep(entry, &mut item, "name", "families")?;
        if let Some(stack) = entry.get("stack").filter(|stack| !stack.is_null()) {
            let stack = stack
                .as_str()
                .filter(|stack| {
                    FONT_STACKS.iter().any(|(key, _)| key == stack)
                        || stack.strip_prefix("font:").is_some_and(is_slug)
                })
                .ok_or("families: unknown font")?;
            item.insert("stack".into(), Value::String(stack.to_owned()));
        }
        clean.push(Value::Object(item));
    }
    Ok(Value::Array(clean))
}

fn size_entries(value: &Value) -> Result<Value, String> {
    let mut clean = Vec::new();
    for entry in list(value, "sizes", MAX_SIZES)? {
        let entry = object(entry, "font size")?;
        let mut item = Map::new();
        item.insert("slug".into(), Value::String(entry_slug(entry, "sizes")?));
        keep(entry, &mut item, "name", "sizes")?;
        for key in ["min", "max"] {
            if let Some(size) = rem(entry.get(key), "sizes")? {
                item.insert(key.into(), size);
            }
        }
        clean.push(Value::Object(item));
    }
    Ok(Value::Array(clean))
}

fn spacing(value: &Value) -> Result<Value, String> {
    let source = object(value, "spacing")?;
    let mut clean = Map::new();
    for (key, item) in source {
        match key.as_str() {
            "scale" => {
                let steps = list(item, "spacing scale", SPACE_SCALE)?;
                if steps.len() != SPACE_SCALE {
                    return Err(format!("spacing scale needs exactly {SPACE_SCALE} sizes"));
                }
                let lengths = steps
                    .iter()
                    .map(|step| length(Some(step), "spacing scale").map(Option::unwrap_or_default))
                    .collect::<Result<Vec<_>, _>>()?;
                clean.insert(key.clone(), Value::Array(lengths));
            }
            "content" | "wide" => {
                if let Some(width) = length(Some(item), "spacing width")? {
                    clean.insert(key.clone(), width);
                }
            }
            _ => return Err(format!("spacing has an unknown property {key}")),
        }
    }
    Ok(Value::Object(clean))
}

fn choice(value: &Value, label: &str, options: &[&str]) -> Result<Value, String> {
    value
        .as_str()
        .filter(|text| options.contains(text))
        .map(|text| Value::String(text.to_owned()))
        .ok_or_else(|| format!("{label} is not one of the allowed values"))
}

fn shape(value: &Value) -> Result<Value, String> {
    let mut clean = Map::new();
    for (key, item) in object(value, "shape")? {
        let options = match key.as_str() {
            "radius" => SHAPE_RADII,
            "shadow" => SHAPE_SHADOWS,
            _ => return Err(format!("shape has an unknown property {key}")),
        };
        clean.insert(key.clone(), choice(item, key, options)?);
    }
    Ok(Value::Object(clean))
}

fn elements(value: &Value) -> Result<Value, String> {
    let mut clean = Map::new();
    for (key, item) in object(value, "elements")? {
        if !ELEMENTS.iter().any(|(element, _)| element == key) {
            return Err(format!("elements has an unknown element {key}"));
        }
        clean.insert(key.clone(), element(item, key)?);
    }
    Ok(Value::Object(clean))
}

fn block_types(value: &Value) -> Result<Value, String> {
    let mut clean = Map::new();
    for (key, item) in object(value, "blocks")? {
        let valid = !key.is_empty()
            && key.len() <= 40
            && key.chars().all(|character| {
                character.is_ascii_lowercase()
                    || character.is_ascii_digit()
                    || matches!(character, '_' | ':')
            });
        if !valid {
            return Err(format!("blocks has an invalid block type {key}"));
        }
        let source = object(item, "block style")?;
        let mut entry = Map::new();
        for (field, setting) in source {
            match field.as_str() {
                "style" => {
                    entry.insert(field.clone(), element(setting, key)?);
                }
                "variations" => {
                    let mut variations = Vec::new();
                    for variation in list(setting, "variations", MAX_VARIATIONS)? {
                        let variation = object(variation, "variation")?;
                        let mut clean_variation = Map::new();
                        clean_variation.insert(
                            "slug".into(),
                            Value::String(entry_slug(variation, "variations")?),
                        );
                        keep(variation, &mut clean_variation, "name", "variations")?;
                        if let Some(style) = variation.get("style") {
                            clean_variation.insert("style".into(), element(style, key)?);
                        }
                        variations.push(Value::Object(clean_variation));
                    }
                    entry.insert(field.clone(), Value::Array(variations));
                }
                _ => return Err(format!("blocks.{key} has an unknown property {field}")),
            }
        }
        clean.insert(key.clone(), Value::Object(entry));
    }
    Ok(Value::Object(clean))
}

pub const MAX_CSS_CHARS: usize = 64_000;

pub fn validate_css(css: &str) -> Result<String, String> {
    if css.chars().count() > MAX_CSS_CHARS {
        return Err(format!(
            "the additional CSS has more than {MAX_CSS_CHARS} characters"
        ));
    }
    let lower = css.to_ascii_lowercase();
    if lower.contains("</")
        || lower.contains("<!--")
        || lower.contains("@import")
        || lower.contains("@charset")
        || lower.contains("expression(")
        || lower.contains("javascript:")
        || lower.contains("behavior:")
    {
        return Err("the additional CSS cannot use @import, </, expression or javascript".into());
    }
    for part in lower.split("url(").skip(1) {
        let target = part
            .trim_start()
            .trim_start_matches(['"', '\''])
            .trim_start();
        let local = target.starts_with('/') && !target.starts_with("//");
        if !local && !target.starts_with("data:image/") && !target.starts_with('#') {
            return Err("url() in the additional CSS can only point to the site itself".into());
        }
    }
    let balance = css.chars().fold(0_i64, |depth, character| match character {
        '{' => depth + 1,
        '}' => depth - 1,
        _ => depth,
    });
    if balance != 0 {
        return Err("the additional CSS has unbalanced braces".into());
    }
    Ok(css.replace('\r', ""))
}

pub fn validate_styles(value: &Value) -> Result<Value, String> {
    let mut clean = Map::new();
    let source = match value {
        Value::Null => return Ok(Value::Object(clean)),
        other => object(other, "styles")?,
    };
    for (key, item) in source {
        if item.is_null() {
            continue;
        }
        let cleaned = match key.as_str() {
            "mode" => choice(item, "mode", MODES)?,
            "palette" => palette_entries(item)?,
            "gradients" => gradient_entries(item)?,
            "families" => family_entries(item)?,
            "sizes" => size_entries(item)?,
            "spacing" => spacing(item)?,
            "shape" => shape(item)?,
            "elements" => elements(item)?,
            "blocks" => block_types(item)?,
            "css" => Value::String(validate_css(item.as_str().ok_or("css must be text")?)?),
            _ => return Err(format!("styles has an unknown property {key}")),
        };
        clean.insert(key.clone(), cleaned);
    }
    Ok(Value::Object(clean))
}

fn slug_of(value: &Value) -> Option<&str> {
    value.get("slug").and_then(Value::as_str)
}

pub fn merge_styles(base: &mut Value, over: &Value) {
    if over.is_null() {
        return;
    }
    match (base, over) {
        (Value::Object(base), Value::Object(over)) => {
            for (key, value) in over {
                match base.get_mut(key) {
                    Some(existing) => merge_styles(existing, value),
                    None => {
                        base.insert(key.clone(), value.clone());
                    }
                }
            }
        }
        (Value::Array(base), Value::Array(over))
            if over.iter().all(|entry| slug_of(entry).is_some())
                && base.iter().all(|entry| slug_of(entry).is_some()) =>
        {
            for entry in over {
                match base
                    .iter_mut()
                    .find(|existing| slug_of(existing) == slug_of(entry))
                {
                    Some(existing) => merge_styles(existing, entry),
                    None => base.push(entry.clone()),
                }
            }
        }
        (base, over) => *base = over.clone(),
    }
}

fn entries(styles: &Value, key: &str) -> Vec<Map<String, Value>> {
    styles[key]
        .as_array()
        .map(|list| list.iter().filter_map(Value::as_object).cloned().collect())
        .unwrap_or_default()
}

fn text<'a>(entry: &'a Map<String, Value>, key: &str) -> &'a str {
    entry.get(key).and_then(Value::as_str).unwrap_or_default()
}

fn color_reference(reference: &str) -> String {
    if is_hex_color(reference) {
        reference.to_owned()
    } else {
        format!("var(--site-color-{reference})")
    }
}

fn rule(selector: &str, declarations: &[String]) -> String {
    if declarations.is_empty() {
        String::new()
    } else {
        format!("{selector}{{{}}}\n", declarations.join(";"))
    }
}

fn fluid(min: &str, max: &str) -> String {
    let number = |text: &str| text.trim_end_matches("rem").parse::<f64>().unwrap_or(1.0);
    let (low, high) = (number(min), number(max));
    if (high - low).abs() < f64::EPSILON {
        return min.to_owned();
    }
    format!(
        "clamp({min},calc({min} + {:.4} * ((100vw - 20rem) / 60)),{max})",
        high - low
    )
}

pub fn styles_mode(styles: &Value) -> &str {
    styles["mode"].as_str().unwrap_or("system")
}

pub fn compile_css(styles: &Value) -> String {
    let palette = entries(styles, "palette");
    let mut root = Vec::new();
    let mut dark = Vec::new();
    for color in &palette {
        let slug = text(color, "slug");
        let light = text(color, "light");
        let dark_value = color.get("dark").and_then(Value::as_str).unwrap_or(light);
        root.push(format!("--site-color-{slug}:{light}"));
        dark.push(format!("--site-color-{slug}:{dark_value}"));
    }
    for gradient in entries(styles, "gradients") {
        let stops: Vec<String> = gradient
            .get("stops")
            .and_then(Value::as_array)
            .map(|stops| {
                stops
                    .iter()
                    .filter_map(Value::as_str)
                    .map(color_reference)
                    .collect()
            })
            .unwrap_or_default();
        if stops.len() >= 2 {
            let angle = gradient.get("angle").and_then(Value::as_u64).unwrap_or(135);
            root.push(format!(
                "--site-gradient-{}:linear-gradient({angle}deg,{})",
                text(&gradient, "slug"),
                stops.join(",")
            ));
        }
    }
    let mut faces = String::new();
    for family in entries(styles, "families") {
        let chosen = text(&family, "stack");
        let stack = match chosen.strip_prefix("font:") {
            Some(key) => {
                let font = &styles["uploaded_fonts"][key];
                let name = font["family"].as_str().unwrap_or_default().replace('"', "");
                let fallback = font["fallback"].as_str().unwrap_or("sans-serif");
                for face in font["faces"].as_array().into_iter().flatten() {
                    let url = face["url"].as_str().unwrap_or_default();
                    let format = if url.is_empty() { "" } else { "woff2" };
                    faces.push_str(&format!(
                        "@font-face{{font-family:\"{name}\";src:url(\"{url}\") format(\"{format}\"),url(\"{url}\") format(\"woff\");font-weight:{};font-style:{};font-display:swap}}\n",
                        face["weight"].as_u64().unwrap_or(400),
                        face["style"].as_str().unwrap_or("normal"),
                    ));
                }
                if name.is_empty() {
                    FONT_STACKS[0].1.to_owned()
                } else {
                    format!("\"{name}\",{fallback}")
                }
            }
            None => FONT_STACKS
                .iter()
                .find(|(key, _)| *key == chosen)
                .map(|(_, stack)| (*stack).to_owned())
                .unwrap_or_else(|| FONT_STACKS[0].1.to_owned()),
        };
        root.push(format!("--site-font-{}:{stack}", text(&family, "slug")));
    }
    for size in entries(styles, "sizes") {
        let min = text(&size, "min");
        let max = size.get("max").and_then(Value::as_str).unwrap_or(min);
        if !min.is_empty() {
            root.push(format!(
                "--site-size-{}:{}",
                text(&size, "slug"),
                fluid(min, max)
            ));
        }
    }
    if let Some(scale) = styles["spacing"]["scale"].as_array() {
        for (step, size) in scale.iter().enumerate() {
            root.push(format!(
                "--site-space-{step}:{}",
                size.as_str().unwrap_or("0")
            ));
        }
    }
    root.push(format!(
        "--site-content:{}",
        styles["spacing"]["content"].as_str().unwrap_or("46rem")
    ));
    root.push(format!(
        "--site-wide:{}",
        styles["spacing"]["wide"].as_str().unwrap_or("68rem")
    ));
    let radii = match styles["shape"]["radius"].as_str().unwrap_or("soft") {
        "none" => ["0", "0", "0"],
        "round" => ["0.5rem", "1rem", "1.75rem"],
        _ => ["0.25rem", "0.5rem", "1rem"],
    };
    for (name, radius) in ["sm", "md", "lg"].iter().zip(radii) {
        root.push(format!("--site-radius-{name}:{radius}"));
    }
    root.push("--site-shadow-sm:0 1px 3px rgb(0 0 0/.1)".into());
    root.push("--site-shadow-md:0 6px 18px -6px rgb(0 0 0/.18)".into());
    root.push("--site-shadow-lg:0 18px 40px -12px rgb(0 0 0/.28)".into());
    root.push(format!(
        "--site-shadow-card:{}",
        match styles["shape"]["shadow"].as_str().unwrap_or("none") {
            "soft" => "0 2px 12px rgb(0 0 0/.07)",
            "strong" => "0 8px 28px rgb(0 0 0/.19)",
            _ => "none",
        }
    ));
    root.extend(
        [
            ("--bg", "var(--site-color-base)"),
            ("--surface", "var(--site-color-surface)"),
            ("--fg", "var(--site-color-contrast)"),
            ("--muted", "var(--site-color-muted)"),
            ("--line", "var(--site-color-border)"),
            ("--accent", "var(--site-color-accent)"),
            ("--link", "var(--site-color-link)"),
            ("--on-accent", "var(--site-color-on-accent)"),
            ("--site-surface", "var(--site-color-surface)"),
            ("--site-muted", "var(--site-color-muted)"),
            ("--site-border", "var(--site-color-border)"),
            ("--site-accent", "var(--site-color-accent)"),
            ("--site-radius", "var(--site-radius-md)"),
            ("--site-shadow", "var(--site-shadow-card)"),
            ("--site-content-width", "var(--site-wide)"),
            ("--site-reading-width", "var(--site-content)"),
            ("--site-gap", "var(--site-space-5)"),
            ("--site-body-font", "var(--site-font-body)"),
            ("--site-heading-font", "var(--site-font-heading)"),
            ("--site-heading-scale", "var(--site-size-5xl)"),
        ]
        .map(|(name, value)| format!("{name}:{value}")),
    );
    let mode = styles_mode(styles);
    root.push(format!(
        "color-scheme:{}",
        if mode == "system" { "light dark" } else { mode }
    ));
    let mut css = faces;
    css.push_str(&rule(":root", &root));
    match mode {
        "dark" => css.push_str(&rule(":root", &dark)),
        "system" => css.push_str(&format!(
            "@media (prefers-color-scheme:dark){{{}}}\n",
            rule(":root", &dark).trim_end()
        )),
        _ => {}
    }
    for (key, selector) in ELEMENTS {
        let style = &styles["elements"][*key];
        css.push_str(&rule(selector, &block_style::declarations(style)));
        let hover = &style["hover"];
        if !hover.is_null() {
            let hovered: Vec<String> = selector
                .split(',')
                .map(|part| format!("{part}:hover"))
                .collect();
            css.push_str(&rule(&hovered.join(","), &block_style::declarations(hover)));
        }
    }
    let button_radius = block_style::declarations(
        &json!({ "radius": styles["elements"]["button"]["radius"].clone() }),
    );
    if let Some(value) = button_radius
        .iter()
        .find_map(|line| line.strip_prefix("border-radius:"))
    {
        css.push_str(&rule(":root", &[format!("--site-button-radius:{value}")]));
    }
    if let Some(blocks) = styles["blocks"].as_object() {
        for (block_type, settings) in blocks {
            let selector = format!(".blk-slot[data-type=\"{block_type}\"]");
            css.push_str(&rule(
                &selector,
                &block_style::declarations(&settings["style"]),
            ));
            for variation in settings["variations"].as_array().into_iter().flatten() {
                if let Some(slug) = slug_of(variation) {
                    css.push_str(&rule(
                        &format!("{selector}.is-style-{slug}"),
                        &block_style::declarations(&variation["style"]),
                    ));
                }
            }
        }
    }
    if let Some(extra) = styles["css"].as_str() {
        css.push('\n');
        css.push_str(extra);
        css.push('\n');
    }
    css
}

pub fn resolve_styles(layers: &[&Value]) -> Value {
    let mut merged = default_styles();
    for layer in layers {
        merge_styles(&mut merged, layer);
    }
    let palette = merged["palette"].as_array().cloned().unwrap_or_default();
    for slug in REQUIRED_COLORS {
        if !palette.iter().any(|entry| slug_of(entry) == Some(slug)) {
            merge_styles(
                &mut merged,
                &json!({ "palette": default_styles()["palette"].clone() }),
            );
            break;
        }
    }
    merged
}
