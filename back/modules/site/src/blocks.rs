use aphel_blocks::style::{Scope, classes_of, declarations, normalize};
use aphel_contract_site::{
    BLOCK_TYPES_DIR, BLOCKS_ICONS, BLOCKS_ITEMS, BLOCKS_MACROS, BLOCKS_STYLE, BLOCKS_TEMPLATE,
};
use minijinja::{Environment, Value};

const BLOCK_TYPES: &[(&str, &str)] = &[
    (
        "site_logo",
        include_str!("../theme/blocks/types/site_logo.html"),
    ),
    (
        "site_title",
        include_str!("../theme/blocks/types/site_title.html"),
    ),
    (
        "site_tagline",
        include_str!("../theme/blocks/types/site_tagline.html"),
    ),
    (
        "navigation",
        include_str!("../theme/blocks/types/navigation.html"),
    ),
    ("search", include_str!("../theme/blocks/types/search.html")),
    (
        "languages",
        include_str!("../theme/blocks/types/languages.html"),
    ),
    (
        "account",
        include_str!("../theme/blocks/types/account.html"),
    ),
    (
        "site_cta",
        include_str!("../theme/blocks/types/site_cta.html"),
    ),
    (
        "breadcrumbs",
        include_str!("../theme/blocks/types/breadcrumbs.html"),
    ),
    (
        "social_links",
        include_str!("../theme/blocks/types/social_links.html"),
    ),
    (
        "site_contact",
        include_str!("../theme/blocks/types/site_contact.html"),
    ),
    (
        "site_footer_text",
        include_str!("../theme/blocks/types/site_footer_text.html"),
    ),
    (
        "copyright",
        include_str!("../theme/blocks/types/copyright.html"),
    ),
    (
        "template_part",
        include_str!("../theme/blocks/types/template_part.html"),
    ),
    (
        "pattern",
        include_str!("../theme/blocks/types/pattern.html"),
    ),
    (
        "record_title",
        include_str!("../theme/blocks/types/record_title.html"),
    ),
    (
        "record_field",
        include_str!("../theme/blocks/types/record_field.html"),
    ),
    (
        "record_image",
        include_str!("../theme/blocks/types/record_image.html"),
    ),
    (
        "record_terms",
        include_str!("../theme/blocks/types/record_terms.html"),
    ),
    (
        "record_body",
        include_str!("../theme/blocks/types/record_body.html"),
    ),
    (
        "record_link",
        include_str!("../theme/blocks/types/record_link.html"),
    ),
    (
        "archive_title",
        include_str!("../theme/blocks/types/archive_title.html"),
    ),
    (
        "archive_description",
        include_str!("../theme/blocks/types/archive_description.html"),
    ),
    ("query", include_str!("../theme/blocks/types/query.html")),
    (
        "query_pagination",
        include_str!("../theme/blocks/types/query_pagination.html"),
    ),
    ("group", include_str!("../theme/blocks/types/group.html")),
    ("row", include_str!("../theme/blocks/types/row.html")),
    ("stack", include_str!("../theme/blocks/types/stack.html")),
    ("grid", include_str!("../theme/blocks/types/grid.html")),
    ("cover", include_str!("../theme/blocks/types/cover.html")),
    (
        "details",
        include_str!("../theme/blocks/types/details.html"),
    ),
    (
        "booking",
        include_str!("../theme/blocks/types/booking.html"),
    ),
    (
        "buttons",
        include_str!("../theme/blocks/types/buttons.html"),
    ),
    (
        "collection",
        include_str!("../theme/blocks/types/collection.html"),
    ),
    (
        "columns",
        include_str!("../theme/blocks/types/columns.html"),
    ),
    (
        "contact",
        include_str!("../theme/blocks/types/contact.html"),
    ),
    ("embed", include_str!("../theme/blocks/types/embed.html")),
    ("faq", include_str!("../theme/blocks/types/faq.html")),
    ("form", include_str!("../theme/blocks/types/form.html")),
    (
        "gallery",
        include_str!("../theme/blocks/types/gallery.html"),
    ),
    ("hero", include_str!("../theme/blocks/types/hero.html")),
    ("image", include_str!("../theme/blocks/types/image.html")),
    ("spacer", include_str!("../theme/blocks/types/spacer.html")),
    ("quote", include_str!("../theme/blocks/types/quote.html")),
    ("code", include_str!("../theme/blocks/types/code.html")),
    (
        "callout",
        include_str!("../theme/blocks/types/callout.html"),
    ),
    (
        "testimonials",
        include_str!("../theme/blocks/types/testimonials.html"),
    ),
    ("text", include_str!("../theme/blocks/types/text.html")),
    (
        "text_image",
        include_str!("../theme/blocks/types/text_image.html"),
    ),
    ("video", include_str!("../theme/blocks/types/video.html")),
];

fn style_of(block: &Value) -> serde_json::Value {
    let block = serde_json::to_value(block).unwrap_or_default();
    normalize(block.get("style"), Scope::Block).unwrap_or_default()
}

fn block_classes(block: Value) -> String {
    classes_of(&style_of(&block))
        .into_iter()
        .map(|class| format!(" {class}"))
        .collect()
}

fn block_style(block: Value) -> Value {
    let rules = declarations(&style_of(&block));
    if rules.is_empty() {
        return Value::from("");
    }
    Value::from_safe_string(format!(" style=\"{}\"", rules.join(";")))
}

pub(crate) fn register(environment: &mut Environment<'static>) {
    environment.add_filter("block_classes", block_classes);
    environment.add_filter("block_style", block_style);
    for (name, source) in [
        (BLOCKS_TEMPLATE, include_str!("../theme/blocks/blocks.html")),
        (BLOCKS_ITEMS, include_str!("../theme/blocks/items.html")),
        (
            "aphel/site_macros.html",
            include_str!("../theme/blocks/site_macros.html"),
        ),
        (
            "aphel/query_item.html",
            include_str!("../theme/blocks/query_item.html"),
        ),
        (BLOCKS_STYLE, include_str!("../theme/blocks/style.html")),
        (BLOCKS_MACROS, include_str!("../theme/blocks/macros.html")),
        (BLOCKS_ICONS, include_str!("../theme/blocks/icons.html")),
    ] {
        if let Err(error) = environment.add_template(name, source) {
            tracing::error!(template = name, %error, "a built-in block template does not compile");
        }
    }
    for (block_type, source) in BLOCK_TYPES {
        let name = format!("{BLOCK_TYPES_DIR}/{block_type}.html");
        if let Err(error) = environment.add_template_owned(name, *source) {
            tracing::error!(block_type, %error, "a built-in block template does not compile");
        }
    }
}
