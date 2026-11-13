use std::collections::BTreeMap;
use std::sync::Arc;

use aphel_contract_publication::PublishableDeclaration;
use aphel_contract_site::{
    PageSource, SiteBundle, SiteContentSource, SiteExposure, SitePage, SitePageSeo, SiteTemplate,
};
use aphel_kernel::errors::FieldViolation;
use aphel_kernel::records::{RecordSaving, SavingRecord};
use aphel_sdk::AppError;
use aphel_sdk::module::{AppBuilder, HookCtx};
use aphel_sdk::platform::application::fields::{EntityDeclaration, LocalizedLabels};
use aphel_sdk::platform::application::shared::RecordType;
use aphel_sdk::platform::domain::access::{Permission, PermissionDefinition};
use aphel_sdk::records::GenericRecordHandler;
use serde_json::{Value, json};

pub(crate) const PAGE_ENTITY: &str = "site.page";
pub(crate) const PAGE_READ: &str = "site.page.read";
pub(crate) const PAGE_PUBLISH: &str = "site.page.publish";
pub(crate) const OWNER: &str = "site";
const MAX_TITLE_CHARS: usize = 160;
const SAVING_PRIORITY: i32 = 30;
const REVISION: &str = "pages-1";

const PAGE_TEMPLATE: &str = include_str!("../templates_builtin/page.json");
const LANDING_TEMPLATE: &str = include_str!("../templates_builtin/page@landing.json");

const STRUCTURED_DATA: &str = "{% set ns = namespace(items=[]) %}{% for block in (record.blocks | from_json) or [] %}{% if block.type == 'faq' %}{% for item in block['items'] %}{% if item.question and item.answer %}{% set ns.items = ns.items + [{'@type': 'Question', 'name': item.question, 'acceptedAnswer': {'@type': 'Answer', 'text': (item.answer | markdown) | string}}] %}{% endif %}{% endfor %}{% endif %}{% endfor %}{% if ns.items %}{{ {'@type': 'FAQPage', 'mainEntity': ns.items} | tojson }}{% endif %}";
const SEO_IMAGE: &str = "{% set ns = namespace(url='') %}{% for block in (record.blocks | from_json) or [] %}{% if not ns.url and block.image %}{% for picture in record.images or [] %}{% if picture.id == block.image %}{% set ns.url = picture | image_url(1280) %}{% endif %}{% endfor %}{% endif %}{% endfor %}{{ ns.url }}";

fn labels(key: &str) -> LocalizedLabels {
    LocalizedLabels::from_catalog(include_str!("../locales/labels.json"), key)
}

fn declaration() -> Result<EntityDeclaration, AppError> {
    serde_json::from_value(json!({
        "key": "page",
        "label": "Site page",
        "label_plural": "Site pages",
        "icon": "file-text",
        "storage": "managed",
        "owner_enabled": false,
        "permission_scope": "organization",
        "title_field": "title",
        "fields": [
            { "key": "title", "label": "Title", "type": "text", "required": true, "searchable": true,
              "max_length": MAX_TITLE_CHARS, "translatable": true, "published_by": "publication" },
            { "key": "slug", "label": "Address", "type": "text", "required": true, "unique": true,
              "max_length": 80, "pattern": "^[a-z0-9]+(-[a-z0-9]+)*$" },
            { "key": "blocks", "label": "Blocks", "type": "document", "max_length": 400000,
              "translatable": true, "published_by": "publication" },
            { "key": "seo_description", "label": "Search description", "type": "long_text",
              "max_length": 300, "translatable": true, "published_by": "publication" }
        ]
    }))
    .map_err(|error| AppError::Configuration(format!("site page declaration: {error}")))
}

pub(crate) fn register(app: &mut AppBuilder) -> Result<(), AppError> {
    for suffix in ["read", "create", "update", "delete"] {
        let permission = Permission::parse(format!("site.page.{suffix}"))?;
        app.permission(PermissionDefinition::record(permission))
            .grant_to(["admin", "manager"]);
    }
    app.permission(PermissionDefinition::administrative(Permission::from_static(
        PAGE_PUBLISH,
    )))
    .grant_to(["admin", "manager"]);
    let record_type = RecordType::from_static(PAGE_ENTITY);
    let definition = declaration()?
        .definition(PAGE_ENTITY, Permission::from_static(PAGE_READ))?
        .with_trait("ui.plural_labels", labels("Pages"))
        .handler(Arc::new(GenericRecordHandler::new(record_type)));
    app.entity(definition);
    app.contribute(Arc::new(PublishableDeclaration {
        entity: PAGE_ENTITY.into(),
        format: 1,
        permission: PAGE_PUBLISH.into(),
    }));
    app.contribute::<dyn SiteContentSource>(Arc::new(PagesSource));
    app.filter(RecordSaving, SAVING_PRIORITY, |ctx, saving| {
        Box::pin(saving_filter(ctx, saving))
    });
    Ok(())
}

fn invalid(field: &str, message: String) -> AppError {
    AppError::invalid_fields_code(
        "site_page_invalid",
        "invalid site page",
        vec![FieldViolation::new(field, message)],
    )
}

fn normalized_blocks(field: &str, text: &str) -> Result<String, AppError> {
    aphel_blocks::blocks::normalize(text).map_err(|error| invalid(field, error.message))
}

fn normalized_translations(
    field: &str,
    text: &str,
    each: impl Fn(&str) -> Result<String, AppError>,
) -> Result<String, AppError> {
    let parsed: Value =
        serde_json::from_str(text).map_err(|_| invalid(field, "the translations are not valid JSON".into()))?;
    let Value::Object(map) = parsed else {
        return Err(invalid(field, "the translations must map languages to texts".into()));
    };
    let mut clean = serde_json::Map::new();
    for (language, value) in map {
        let Value::String(content) = value else {
            return Err(invalid(field, format!("the {language} translation must be text")));
        };
        clean.insert(language, Value::String(each(&content)?));
    }
    Ok(Value::Object(clean).to_string())
}

async fn saving_filter(
    _ctx: &mut HookCtx<'_>,
    mut saving: SavingRecord,
) -> Result<SavingRecord, AppError> {
    if saving.record_type.as_str() != PAGE_ENTITY {
        return Ok(saving);
    }
    if let Some(Value::String(slug)) = saving.draft.values.get("slug").cloned() {
        let slug = aphel_blocks::slug::check(&slug).map_err(|error| invalid("slug", error.message))?;
        saving.draft.values.insert("slug".into(), Value::String(slug));
    }
    let entries: Vec<(String, String)> = saving
        .draft
        .values
        .iter()
        .filter_map(|(key, value)| value.as_str().map(|text| (key.clone(), text.to_owned())))
        .collect();
    for (key, text) in entries {
        let cleaned = match key.as_str() {
            "draft_blocks" => Some(normalized_blocks(&key, &text)?),
            "draft_title" => Some(text.trim().to_owned()),
            "draft_blocks_translations" if !text.trim().is_empty() => Some(
                normalized_translations(&key, &text, |blocks| normalized_blocks(&key, blocks))?,
            ),
            "draft_title_translations" if !text.trim().is_empty() => Some(
                normalized_translations(&key, &text, |title| Ok(title.trim().to_owned()))?,
            ),
            _ => None,
        };
        if let Some(cleaned) = cleaned {
            saving.draft.values.insert(key, Value::String(cleaned));
        }
    }
    Ok(saving)
}

struct PagesSource;

#[async_trait::async_trait]
impl SiteContentSource for PagesSource {
    async fn bundles(&self) -> Result<Vec<SiteBundle>, AppError> {
        Ok(vec![SiteBundle {
            owner: OWNER.into(),
            name: Some("Site".into()),
            revision: REVISION.into(),
            pages: vec![SitePage {
                key: "page".into(),
                path: "/{slug}".into(),
                template: String::new(),
                source: PageSource::Record {
                    entity: PAGE_ENTITY.into(),
                    match_field: "slug".into(),
                },
                title: Some("{{ record.title }}".into()),
                seo: SitePageSeo {
                    description: Some("{{ record.seo_description or '' }}".into()),
                    image: Some(SEO_IMAGE.into()),
                    schema: Some("WebPage".into()),
                    structured_data: Some(STRUCTURED_DATA.into()),
                    ..Default::default()
                },
                data: None,
                requires_customer: false,
                customer_field: None,
                prefix: None,
                term: None,
                feed: false,
            }],
            exposures: vec![SiteExposure {
                entity: PAGE_ENTITY.into(),
                fields: ["title", "slug", "blocks", "seo_description", "publication_state"]
                    .map(String::from)
                    .to_vec(),
                conditions: BTreeMap::from([("publication_state".to_owned(), json!("published"))]),
                filter: None,
                images: true,
            }],
            templates: vec![
                SiteTemplate {
                    name: "site/templates/page.json".into(),
                    source: PAGE_TEMPLATE.into(),
                },
                SiteTemplate {
                    name: "site/templates/page@landing.json".into(),
                    source: LANDING_TEMPLATE.into(),
                },
            ],
            theme: None,
            islands: Vec::new(),
            csp: Default::default(),
            account: Vec::new(),
        }])
    }

    async fn asset(&self, _owner: &str, _path: &str) -> Result<Option<(String, Vec<u8>)>, AppError> {
        Ok(None)
    }

    async fn island(&self, _owner: &str, _name: &str) -> Result<Option<Vec<u8>>, AppError> {
        Ok(None)
    }
}

