pub(crate) mod account;
mod appearance;
mod appearance_http;
pub(crate) mod assets;
pub(crate) mod blocks;
pub(crate) mod cache;
pub(crate) mod customers;
pub(crate) mod data;
pub(crate) mod deferred;
pub(crate) mod dynamic;
pub(crate) mod feed;
pub(crate) mod frame;
pub(crate) mod handler;
pub(crate) mod http;
pub(crate) mod images;
pub(crate) mod invalidation;
pub(crate) mod paths;
pub(crate) mod people_link;
pub(crate) mod prefixes;
pub(crate) mod preview;
pub(crate) mod privacy;
pub(crate) mod provision;
pub(crate) mod redirects;
pub(crate) mod runtime;
pub(crate) mod search;
pub(crate) mod session;
pub(crate) mod settings;
pub(crate) mod sitemap;
pub(crate) mod targets;
mod terms;

use std::sync::Arc;

const INVALIDATION_PRIORITY: i32 = 90;
const PATH_PRIORITY: i32 = 30;

use aphel_kernel::errors::AppError;
use aphel_kernel::modules::{AppBuilder, Manifest, Migration, Module, SiteRequestHandler};
use aphel_sdk::platform::application::fields::{EntityDefinition, SystemField};
use aphel_sdk::platform::domain::access::{Permission, PermissionDefinition};
use aphel_sdk::platform::domain::fields::FieldType;
use aphel_sdk::platform::modules::builder::APHEL_VERSION;

pub(crate) const KEY: &str = "site";

pub(crate) static CUSTOMERS_READ: Permission = Permission::from_static("site.customers.read");

fn customer_entity() -> Result<EntityDefinition, AppError> {
    Ok(EntityDefinition::new(
        customers::record_type(),
        "site_customers",
        CUSTOMERS_READ.clone(),
    )
    .events(aphel_contract_site::CUSTOMER_RECORD_TYPE)
    .title_field("name")
    .soft_delete_column("deleted_at")
    .system_field(
        SystemField::new("name", "Name", FieldType::Text, "name")?
            .read_only()
            .searchable()
            .personal(),
    )
    .system_field(
        SystemField::new("email", "Email", FieldType::Email, "email")?
            .read_only()
            .searchable()
            .personal(),
    )
    .system_field(
        aphel_sdk::platform::application::fields::builtin::record_field(
            "person_id",
            "Person",
            aphel_sdk::people::RECORD_TYPE,
        )?
        .read_only(),
    )
    .system_field(
        SystemField::new(
            "email_verified_at",
            "Email verified at",
            FieldType::DateTime,
            "email_verified_at",
        )?
        .read_only(),
    )
    .system_field(
        SystemField::new(
            "last_login_at",
            "Last sign-in",
            FieldType::DateTime,
            "last_login_at",
        )?
        .read_only(),
    )
    .system_field(
        SystemField::new(
            "created_at",
            "Created at",
            FieldType::DateTime,
            "created_at",
        )?
        .read_only(),
    ))
}

pub struct SiteModule;

impl Module for SiteModule {
    fn manifest(&self) -> Manifest {
        Manifest::new(KEY, "Public site", APHEL_VERSION)
            .optional("publication")
            .optional("tagging")
            .description(
                "Public pages that plugins declare (blog posts, catalogs, landing pages), rendered on the server from their published records",
            )
            .owned_tables(&[
                "site_cache_invalidations",
                "site_redirects",
                "site_customers",
                "site_customer_sessions",
                "site_customer_tokens",
                "site_path_redirects",
                "site_previews",
                "site_path_lock",
            ])
            .migration(Migration::new(
                1,
                "create site cache invalidations",
                include_str!("../migrations/0001_create_site_cache_invalidations.sql"),
            ))
            .migration(Migration::new(
                2,
                "create site redirects",
                include_str!("../migrations/0002_create_site_redirects.sql"),
            ))
            .migration(Migration::new(
                3,
                "create site customers",
                include_str!("../migrations/0003_create_site_customers.sql"),
            ))
            .migration(Migration::new(
                4,
                "create site path redirects",
                include_str!("../migrations/0004_create_site_path_redirects.sql"),
            ))
            .migration(Migration::new(
                5,
                "create site previews",
                include_str!("../migrations/0005_create_site_previews.sql"),
            ))
            .migration(Migration::new(
                6,
                "add site customer person",
                include_str!("../migrations/0006_add_site_customer_person.sql"),
            ))
            .migration(Migration::new(
                7,
                "add site preview record",
                include_str!("../migrations/0007_add_site_preview_record.sql"),
            ))
            .migration(Migration::new(
                8,
                "create site path lock",
                include_str!("../migrations/0008_create_site_path_lock.sql"),
            ))
    }

    fn register(&self, app: &mut AppBuilder) -> Result<(), AppError> {
        app.contract(aphel_contract_site::DESCRIPTOR);
        appearance::register(app)?;
        app.entity(customer_entity()?.handler(Arc::new(
            aphel_kernel::records::GenericRecordHandler::new(customers::record_type()),
        )));
        app.permission(PermissionDefinition::record(CUSTOMERS_READ.clone()))
            .grant_to(["admin", "manager"]);
        app.contribute::<dyn SiteRequestHandler>(Arc::new(handler::SiteHandler));
        app.contribute::<dyn aphel_sdk::provisioning::Provisioner>(Arc::new(
            provision::LinkProvisioner,
        ));
        app.on(
            aphel_kernel::records::RecordSaved,
            INVALIDATION_PRIORITY,
            |ctx, args| Box::pin(invalidation::invalidate_in_tx(ctx.tx, &args.record)),
        )
        .on(
            aphel_kernel::records::RecordDeleted,
            INVALIDATION_PRIORITY,
            |ctx, args| Box::pin(invalidation::invalidate_in_tx(ctx.tx, &args.record)),
        )
        .on(
            aphel_kernel::records::RecordArchiving,
            INVALIDATION_PRIORITY,
            |ctx, args| Box::pin(invalidation::invalidate_in_tx(ctx.tx, &args.record)),
        )
        .on(
            aphel_kernel::records::RecordRestored,
            INVALIDATION_PRIORITY,
            |ctx, args| Box::pin(invalidation::invalidate_in_tx(ctx.tx, &args.record)),
        )
        .on(
            aphel_kernel::records::RecordValidating,
            PATH_PRIORITY,
            |ctx, args| Box::pin(paths::guard(ctx, args)),
        );
        app.contribute::<dyn aphel_sdk::merges::MergeReferences>(Arc::new(
            people_link::PersonMerges,
        ));
        app.contribute::<dyn aphel_sdk::privacy::DataSubjectSource>(Arc::new(
            privacy::CustomerPrivacy,
        ));
        app.contribute::<dyn aphel_sdk::platform::application::organization_profile::OrganizationConfigExtension>(
            Arc::new(settings::SiteConfiguration),
        );
        app.consumer(|env| invalidation::SiteInvalidationConsumer::shared(env.pool.clone()));
        app.openapi_tag(
            http::TAG,
            "Public site administration: themes, menu items, redirects and page cache",
        )
        .routes(|_| http::router());
        Ok(())
    }
}

pub fn probe(failures: &mut Vec<String>) {
    appearance::probe(failures);
    runtime::probe(failures);
}
