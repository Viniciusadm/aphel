use aphel_contract_site::{
    BLOCK_TYPES_DIR, BLOCKS_ICONS, BLOCKS_MACROS, BLOCKS_STYLE, BLOCKS_TEMPLATE,
};
use minijinja::Environment;

const BLOCK_TYPES: &[(&str, &str)] = &[
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

pub(crate) fn register(environment: &mut Environment<'static>) {
    for (name, source) in [
        (BLOCKS_TEMPLATE, include_str!("../theme/blocks/blocks.html")),
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
