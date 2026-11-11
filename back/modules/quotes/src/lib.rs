mod dataset_seed;
pub(crate) mod document;
pub mod domain;
pub(crate) mod exposed_operations;
pub(crate) mod gateway;
pub(crate) mod hooks;
pub mod http;
pub(crate) mod pdf;
pub(crate) mod pricing;
pub(crate) mod public;
mod reference_cleanup;
pub(crate) mod runtime;
pub mod service;
pub(crate) mod store;
pub(crate) mod sync;
pub(crate) mod token;
pub(crate) mod workflow;

use aphel_contract_crm::{CrmPermissions, CrmRecordTypes};
use std::sync::Arc;

use aphel_kernel::modules::Migration;
use utoipa_axum::router::OpenApiRouter;

use crate::domain::QuoteStatus;
use aphel_kernel::errors::AppError;
use aphel_kernel::modules::{AppBuilder, Manifest, Module};
use aphel_sdk::platform::application::fields::builtin::{field, record_field};
use aphel_sdk::platform::application::fields::{
    EntityDefinition, EntityTraits, RecordRelation, SystemField,
};
use aphel_sdk::platform::application::public_events::{
    PreviousValues, PublicEventDefinition, PublicResource, changed, deleted, event,
};
use aphel_sdk::platform::application::shared::RecordType;
use aphel_sdk::platform::domain::access::{Permission, PermissionDefinition};
use aphel_sdk::platform::domain::fields::FieldType;
use aphel_sdk::platform::modules::builder::APHEL_VERSION;

pub use hooks::{AcceptingQuote, PricedQuote, QuoteAccepting, QuoteTotals};

pub(crate) const QUOTE_RESOURCE: PublicResource =
    PublicResource::new("quotes.quote", "QuoteDetailResponse");

pub const KEY: &str = "quotes";

const RECORD_PERMISSIONS: [Permission; 5] = [
    Permission::QUOTES_READ,
    Permission::QUOTES_CREATE,
    Permission::QUOTES_UPDATE,
    Permission::QUOTES_DELETE,
    Permission::QUOTES_SEND,
];

const PUBLIC_EVENTS: &[PublicEventDefinition] = &[
    event(
        "quote.created",
        QUOTE_RESOURCE,
        "A quote was created for a deal",
    ),
    changed(
        "quote.updated",
        QUOTE_RESOURCE,
        "Fields or items of a quote changed",
        PreviousValues::Changes,
    ),
    deleted(
        "quote.deleted",
        QUOTE_RESOURCE,
        "A quote version was deleted",
    ),
    event(
        "quote.version_created",
        QUOTE_RESOURCE,
        "A new version of a quote was created from the current one",
    ),
    event(
        "quote.submitted",
        QUOTE_RESOURCE,
        "A quote was sent for approval",
    ),
    event("quote.approved", QUOTE_RESOURCE, "A quote was approved"),
    event(
        "quote.returned_to_draft",
        QUOTE_RESOURCE,
        "An approval of the quote was rejected or cancelled and it went back to draft",
    ),
    event(
        "quote.sent",
        QUOTE_RESOURCE,
        "A quote was sent to the customer by e-mail with its public link",
    ),
    event(
        "quote.viewed",
        QUOTE_RESOURCE,
        "The customer opened the public link of the quote for the first time",
    ),
    event(
        "quote.accepted",
        QUOTE_RESOURCE,
        "The quote was accepted: by the customer through its public link, or recorded manually by a user (`manual: true`)",
    ),
    event(
        "quote.rejected",
        QUOTE_RESOURCE,
        "The customer rejected the quote through its public link",
    ),
    event(
        "quote.expired",
        QUOTE_RESOURCE,
        "A sent quote passed its validity date",
    ),
    event(
        "quote.deal_synced",
        QUOTE_RESOURCE,
        "The deal received the items and the amount of the accepted quote",
    ),
    event(
        "quote.deal_sync_failed",
        QUOTE_RESOURCE,
        "The deal could not be fully updated after the quote was accepted",
    ),
];

pub(crate) const QUOTE: RecordType = RecordType::from_static("quotes.quote");
pub(crate) const QUOTES_TABLE: &str = "quotes";

fn status_label(status: QuoteStatus) -> (&'static str, &'static str) {
    match status {
        QuoteStatus::Draft => ("Draft", "#6e7781"),
        QuoteStatus::PendingApproval => ("Pending approval", "#bf8700"),
        QuoteStatus::Approved => ("Approved", "#1f6feb"),
        QuoteStatus::Sent => ("Sent", "#8250df"),
        QuoteStatus::Accepted => ("Accepted", "#1a7f37"),
        QuoteStatus::Rejected => ("Rejected", "#cf222e"),
        QuoteStatus::Expired => ("Expired", "#57606a"),
    }
}

fn status_field() -> Result<SystemField, AppError> {
    Ok(QuoteStatus::ALL.into_iter().fold(
        field("status", crate::labels("Status"), FieldType::Select)?,
        |field, status| {
            let (label, color) = status_label(status);
            field.with_option(status.as_str(), crate::labels(label), Some(color))
        },
    ))
}

pub(crate) fn quote_definition() -> Result<EntityDefinition, AppError> {
    Ok(
        EntityDefinition::new(QUOTE, QUOTES_TABLE, Permission::QUOTES_READ)
            .capabilities(EntityTraits::default().validation_rules())
            .update_permission(Permission::QUOTES_UPDATE)
            .title_field("title")
            .owner_column("owner_id")
            .soft_delete_column("deleted_at")
            .system_field(
                field("title", crate::labels("Title"), FieldType::Text)?
                    .searchable()
                    .required(),
            )
            .system_field(field("number", crate::labels("Number"), FieldType::Number)?)
            .system_field(field(
                "version",
                crate::labels("Version"),
                FieldType::Number,
            )?)
            .system_field(field(
                "is_current",
                crate::labels("Current version"),
                FieldType::Checkbox,
            )?)
            .system_field(status_field()?)
            .system_field(record_field(
                "deal_id",
                crate::labels("Deal"),
                RecordType::DEAL.as_str(),
            )?)
            .system_field(record_field(
                "contact_id",
                crate::labels("Contact"),
                RecordType::CONTACT.as_str(),
            )?)
            .system_field(record_field(
                "company_id",
                crate::labels("Company"),
                RecordType::COMPANY.as_str(),
            )?)
            .system_field(field("owner_id", crate::labels("Owner"), FieldType::User)?)
            .system_field(field(
                "currency",
                crate::labels("Currency"),
                FieldType::Text,
            )?)
            .system_field(field(
                "subtotal",
                crate::labels("Subtotal"),
                FieldType::Currency,
            )?)
            .system_field(field(
                "discount_total",
                crate::labels("Discounts"),
                FieldType::Currency,
            )?)
            .system_field(field(
                "tax_total",
                crate::labels("Taxes"),
                FieldType::Currency,
            )?)
            .system_field(field("total", crate::labels("Total"), FieldType::Currency)?)
            .system_field(field(
                "valid_until",
                crate::labels("Valid until"),
                FieldType::Date,
            )?)
            .system_field(field(
                "exceeds_discount_limit",
                crate::labels("Discount above limit"),
                FieldType::Checkbox,
            )?)
            .system_field(field(
                "sent_at",
                crate::labels("Sent at"),
                FieldType::DateTime,
            )?)
            .system_field(field(
                "accepted_at",
                crate::labels("Accepted at"),
                FieldType::DateTime,
            )?)
            .system_field(field(
                "rejected_at",
                crate::labels("Rejected at"),
                FieldType::DateTime,
            )?)
            .system_field(field(
                "created_by",
                crate::labels("Created by"),
                FieldType::User,
            )?)
            .system_field(field(
                "created_at",
                crate::labels("Created at"),
                FieldType::DateTime,
            )?)
            .system_field(field(
                "updated_at",
                crate::labels("Updated at"),
                FieldType::DateTime,
            )?)
            .relation(RecordRelation::belongs_to(
                "deal",
                crate::labels("Deal"),
                RecordType::DEAL,
                "deal_id",
            ))
            .relation(RecordRelation::belongs_to(
                "contact",
                crate::labels("Contact"),
                RecordType::CONTACT,
                "contact_id",
            ))
            .relation(RecordRelation::belongs_to(
                "company",
                crate::labels("Company"),
                RecordType::COMPANY,
                "company_id",
            )),
    )
}

pub struct QuotesModule;

impl Module for QuotesModule {
    fn manifest(&self) -> Manifest {
        Manifest::new(KEY, "Quotes", APHEL_VERSION)
            .depends_on(aphel_sdk::documents::KEY, "*")
            .depends_on(aphel_sdk::workflow::KEY, "*")
            .description(
                "Quotes of deals: versions, items, discounts and taxes, approval, sending, the signed public page and acceptance",
            )
            .enabled_by_default()
            .owned_tables(&["quotes", "quote_counters", "quote_items", "quote_settings"])
            .migration(Migration::new(
                1,
                "create quotes",
                include_str!("../migrations/0001_create_quotes.sql"),
            ))
            .migration(Migration::new(
                2,
                "add quote accepted_by",
                include_str!("../migrations/0002_add_quote_accepted_by.sql"),
            ))
    }

    fn register(&self, app: &mut AppBuilder) -> Result<(), AppError> {
        app.contribute(std::sync::Arc::new(aphel_sdk::module::PublicAppPath(
            "quote",
        )));
        reference_cleanup::register(app);
        app.contribute::<dyn aphel_sdk::maintenance::DatasetSeed>(std::sync::Arc::new(
            dataset_seed::QuotesSeed,
        ));
        app.contribute::<dyn aphel_sdk::documents::DocumentSource>(Arc::new(
            document::QuoteDocumentSource,
        ));
        app.declare_hook(crate::hooks::QuoteAccepting, &[aphel_kernel::modules::HookUsage::Veto],
            "Before a quote is accepted (on its public page or manually by a user); an error blocks the acceptance",
        );
        app.forward_hook(
            crate::hooks::QuoteAccepting,
            aphel_kernel::modules::PluginHook::new(
                crate::hooks::ACCEPTING,
                "quotes.quote",
                aphel_kernel::modules::PluginHookKind::Veto,
                "Before a quote is accepted (public page or manually); values: { quote, deal_id, acceptor: { name, email }, manual }. An error blocks the acceptance",
            ),
        );
        app.declare_hook(
            crate::hooks::QuoteTotals,
            &[aphel_kernel::modules::HookUsage::Filter],
            "Totals of a quote (items, discounts and taxes) before they are saved",
        );
        app.entity(quote_definition()?.handler(Arc::new(http::QuoteHandler)))
            .openapi_tag(
                http::TAG,
                "Quotes of deals: versions, items, discounts and taxes, approval, sending, the signed public page and acceptance",
            )
            .routes(|_| OpenApiRouter::new().merge(http::router()).merge(http::public_router()))
            .worker(runtime::quote_worker);
        for permission in RECORD_PERMISSIONS {
            app.permission(PermissionDefinition::record(permission))
                .grant_to(["admin", "manager", "member"]);
        }
        for definition in PUBLIC_EVENTS {
            app.public_event(*definition);
        }
        exposed_operations::register(app);
        Ok(())
    }
}

pub(crate) fn labels(key: &str) -> aphel_sdk::platform::application::fields::LocalizedLabels {
    aphel_sdk::platform::application::fields::LocalizedLabels::from_catalog(
        include_str!("../locales/labels.json"),
        key,
    )
}
