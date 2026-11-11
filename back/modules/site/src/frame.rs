use serde_json::{Value, json};

use crate::settings::{SiteSettings, SiteTexts, filled, phone_digits, social_network};

pub(crate) struct SitePrefixes<'a> {
    pub language: &'a str,
    pub mount: &'a str,
    pub tools: &'a [&'static str],
}

impl SitePrefixes<'_> {
    pub(crate) fn for_path(&self, path: &str) -> &str {
        let first = path
            .trim_start_matches('/')
            .split('/')
            .next()
            .unwrap_or_default();
        match self.tools.contains(&first) {
            true => self.mount,
            false => self.language,
        }
    }

    pub(crate) fn link(&self, url: &str) -> String {
        match url.starts_with('/') {
            true => format!("{}{url}", self.for_path(url)),
            false => url.to_owned(),
        }
    }
}

pub(crate) fn is_active(path: &str, current: &str) -> bool {
    current == path || (path != "/" && current.starts_with(&format!("{path}/")))
}

pub(crate) fn link_label<'a>(texts: Option<&'a SiteTexts>, id: &str, label: &'a str) -> &'a str {
    texts
        .and_then(|texts| filled(texts.links.get(id).map(String::as_str)))
        .unwrap_or(label.trim())
}

fn translated<'a>(base: Option<&'a str>, translation: Option<&'a str>) -> Option<&'a str> {
    filled(translation).or(filled(base))
}

pub(crate) fn tagline<'a>(
    settings: &'a SiteSettings,
    texts: Option<&'a SiteTexts>,
) -> Option<&'a str> {
    translated(
        settings.tagline.as_deref(),
        texts.and_then(|texts| texts.tagline.as_deref()),
    )
}

pub(crate) fn header(
    settings: &SiteSettings,
    texts: Option<&SiteTexts>,
    prefixes: &SitePrefixes<'_>,
) -> Value {
    let cta = settings.header.cta.as_ref().map(|cta| {
        json!({
            "label": translated(Some(cta.label.as_str()), texts.and_then(|texts| texts.cta_label.as_deref())),
            "url": prefixes.link(cta.url.trim()),
        })
    });
    json!({ "align": settings.header.align, "cta": cta })
}

fn lines(text: &str) -> Vec<&str> {
    text.lines().map(str::trim_end).collect()
}

fn encoded(value: &str) -> String {
    form_urlencoded::byte_serialize(value.as_bytes()).collect()
}

pub(crate) fn footer(
    settings: &SiteSettings,
    texts: Option<&SiteTexts>,
    prefixes: &SitePrefixes<'_>,
    current: &str,
) -> Value {
    let footer = &settings.footer;
    let text = translated(
        footer.text.as_deref(),
        texts.and_then(|texts| texts.footer_text.as_deref()),
    )
    .map(lines)
    .unwrap_or_default();
    let contact = &footer.contact;
    let phone = filled(contact.phone.as_deref())
        .map(|phone| json!({ "label": phone, "url": format!("tel:+{}", phone_digits(phone)) }));
    let whatsapp = filled(contact.whatsapp.as_deref()).map(
        |phone| json!({ "label": phone, "url": format!("https://wa.me/{}", phone_digits(phone)) }),
    );
    let email = filled(contact.email.as_deref())
        .map(|email| json!({ "label": email, "url": format!("mailto:{email}") }));
    let address = filled(contact.address.as_deref()).map(|address| {
        json!({
            "lines": lines(address),
            "url": format!("https://www.google.com/maps/search/?api=1&query={}", encoded(address)),
        })
    });
    let has_contact = phone.is_some() || whatsapp.is_some() || email.is_some() || address.is_some();
    let social: Vec<Value> = footer
        .social
        .iter()
        .filter_map(|link| {
            let (network, label, _) = social_network(&link.network)?;
            Some(json!({ "network": network, "label": label, "url": link.url.trim() }))
        })
        .collect();
    let links: Vec<Value> = footer
        .links
        .iter()
        .map(|link| {
            let url = link.url.trim();
            json!({
                "label": link_label(texts, &link.id, &link.label),
                "url": prefixes.link(url),
                "active": url.starts_with('/') && is_active(url, current),
            })
        })
        .collect();
    let empty = text.is_empty() && !has_contact && social.is_empty() && links.is_empty();
    json!({
        "text": text,
        "contact": has_contact.then(|| json!({
            "phone": phone,
            "whatsapp": whatsapp,
            "email": email,
            "address": address,
        })),
        "social": social,
        "links": links,
        "credit": footer.credit,
        "empty": empty,
    })
}
