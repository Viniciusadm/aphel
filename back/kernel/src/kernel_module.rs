use std::sync::Arc;

use utoipa_axum::router::OpenApiRouter;

use crate::api::records::custom_record_handler;
use crate::api::routes::{health, v1};
use crate::api::state::AppState;
use crate::application::custom_objects::RecordLinkRepository;
use crate::application::idempotency::idempotency_purge_schedule;
use crate::domain::access::{Permission, builtin_permission_catalog};
use crate::errors::AppError;
use crate::infrastructure::custom_objects::MySqlRecordLinkRepository;
use crate::infrastructure::idempotency::idempotency_purge_handler;
use crate::infrastructure::security::security_email_handler;
use crate::modules::builder::{APHEL_VERSION, KERNEL_MODULE};
use crate::modules::{AppBuilder, HookUsage, Manifest, Module, WorkerEnv, WorkerParts};
use crate::records::RecordDeleting;

const TAGS: &[(&str, &str)] = &[
    ("objects", "Dynamic entity records"),
    ("relationships", "Entity relations"),
    ("auth", "Authentication, sessions and invitations"),
    (
        "api-tokens",
        "Personal API tokens with scopes for integrations (session only)",
    ),
    ("installer", "First-run installation wizard"),
    (
        "support-access",
        "Temporary support access the organization grants to installation administrators",
    ),
    ("organization", "Active organization settings"),
    (
        "members",
        "Members and invitations of the active organization",
    ),
    ("roles", "Roles, permission matrix and permission catalog"),
    ("teams", "Teams, managers and team hierarchy"),
    ("system", "System administration: background jobs"),
    ("audit", "Audit log of the active organization"),
    ("files", "File uploads, downloads and record attachments"),
    (
        "fields",
        "Field registry of each record type: system and custom fields, options and the schema used by forms and filters",
    ),
    (
        "directory",
        "Directory of the active organization's users, readable by every member",
    ),
    (
        "two-factor",
        "Two-factor authentication (TOTP): setup, confirmation, recovery codes, login challenge and organization policy",
    ),
    (
        "security",
        "Account security: active sessions, personal API tokens, password change and the security event log",
    ),
    (
        "field-permissions",
        "Field permissions per role: hidden, read only or editable",
    ),
    (
        "aggregates",
        "Aggregation engine: grouped counts, sums and averages of records with periods, comparison with the previous period and filters on related records",
    ),
    (
        "preferences",
        "Personal preferences of the current user, such as keyboard shortcuts",
    ),
    (
        "record-sharing",
        "Sharing of any record with users or teams (read or edit access) on top of the role scopes",
    ),
];

pub struct KernelModule;

impl Module for KernelModule {
    fn manifest(&self) -> Manifest {
        Manifest::new(KERNEL_MODULE, "Kernel", APHEL_VERSION)
            .description(
                "Platform of Aphel: identity, records engine, events, jobs and the module runtime",
            )
            .required()
    }

    fn register(&self, app: &mut AppBuilder) -> Result<(), AppError> {
        for definition in crate::application::fields::field_types::native_definitions() {
            app.contribute(Arc::new(definition));
        }
        app.declare_hook(
            crate::application::security::SessionRevoked,
            &[HookUsage::Action],
            "A session or a user session set was revoked in the current transaction",
        );
        declare_record_hooks(app);
        register_record_hooks(app);
        for definition in crate::application::organization_profile::ORGANIZATION_PUBLIC_EVENTS {
            app.public_event(*definition);
        }
        app.openapi_tag("health", "Service health")

            .openapi_tag(
                v1::modules::TAG,
                "Modules compiled into this build: manifest, compatibility, enabling, settings and data removal",
            )
            .root_routes(|_| health::router())
            .routes(|_| routes());
        for (name, description) in TAGS {
            app.openapi_tag(name, description);
        }
        for definition in builtin_permission_catalog() {
            let shares = definition.permission == Permission::RECORDS_SHARE;
            let entry = app.permission(definition);
            if shares {
                entry.grant_to(["admin"]);
            }
        }
        app.worker(maintenance).worker(security);
        Ok(())
    }
}

fn routes() -> OpenApiRouter<AppState> {
    OpenApiRouter::new()
        .merge(v1::modules::router())
        .merge(v1::auth::router())
        .merge(v1::api_tokens::router())
        .merge(v1::installer::router())
        .merge(v1::organization::router())
        .merge(v1::organization_settings::router())
        .merge(v1::organization_experiences::router())
        .merge(v1::members::router())
        .merge(v1::roles::router())
        .merge(v1::teams::router())
        .merge(v1::system_jobs::router())
        .merge(v1::system::router())
        .merge(v1::platform_catalog::router())
        .merge(v1::entities::router())
        .merge(v1::audit::router())
        .merge(v1::files::router())
        .merge(v1::dynamic_records::router())
        .merge(v1::fields::router())
        .merge(v1::directory::router())
        .merge(v1::two_factor::router())
        .merge(v1::security::router())
        .merge(v1::field_permissions::router())
        .merge(v1::aggregates::router())
        .merge(v1::preferences::router())
        .merge(v1::record_shares::router())
}

fn maintenance(env: &WorkerEnv) -> WorkerParts {
    let pool = &env.pool;
    WorkerParts::default()
        .handler(idempotency_purge_handler(pool.clone()))
        .schedule(idempotency_purge_schedule())
}

fn security(env: &WorkerEnv) -> WorkerParts {
    WorkerParts::default().handler(security_email_handler(
        env.pool.clone(),
        env.mailer.clone(),
        env.public_url.clone(),
    ))
}

pub const RESTRICTED_LINKS_PRIORITY: i32 = 10;

fn declare_record_hooks(app: &mut AppBuilder) {
    app.declare_hook(crate::records::RecordSaving, &[HookUsage::Filter],
        "Values and custom fields of a record before they are written (create and update of every record type)",
    );
    app.declare_hook(crate::records::RecordValidating, &[HookUsage::Veto],
        "After a record was written and before the commit; an error rolls the write back (approvals, stage checklists, validation rules, lifecycle)",
    );
    app.declare_hook(
        crate::records::RecordSaved,
        &[HookUsage::Action],
        "After a record was written, inside the same transaction",
    );
    app.declare_hook(
        crate::records::RecordCommitted,
        &[HookUsage::Action],
        "After the transaction that wrote a record was committed",
    );
    app.declare_hook(
        crate::records::RecordArchiving,
        &[HookUsage::Veto],
        "Before a record is archived; an error blocks the archive",
    );
    app.declare_hook(
        crate::records::RecordDeleting,
        &[HookUsage::Veto],
        "Before a record is deleted; an error blocks the deletion (restrict relationships)",
    );
    app.declare_hook(
        crate::records::RecordDeleted,
        &[HookUsage::Action],
        "After a record was deleted, inside the same transaction",
    );
    app.declare_hook(crate::records::RecordPurged, &[HookUsage::Action],
        "After a record was permanently deleted (trash purge, erasure, retention), inside the same transaction, before the kernel removes its custom values, relationship links, attachments, shares and the record fields that pointed to it",
    );
    app.declare_hook(crate::records::RecordRestored, &[HookUsage::Action],
        "After a deleted record was restored from the trash, inside the same transaction, after RecordValidating and before RecordSaved",
    );
    app.declare_hook(
        crate::records::RecordQueryFilter,
        &[HookUsage::Filter],
        "Extra AND conditions and sorting applied to every query of a record type",
    );
    app.declare_hook(
        crate::records::RecordSerializing,
        &[HookUsage::Filter],
        "Records before they are returned by the API (after the field permissions)",
    );
    app.declare_hook(
        crate::records::RecordSchemaFilter,
        &[HookUsage::Filter],
        "The field schema of a record type returned to forms, lists and filters",
    );
    app.declare_hook(
        crate::records::SearchPlanFilter,
        &[HookUsage::Filter],
        "The query plan of the global search for one record type (extra restrictions)",
    );
}

fn register_record_hooks(app: &mut AppBuilder) {
    app.custom_record_handler(custom_record_handler);
    app.on(RecordDeleting, RESTRICTED_LINKS_PRIORITY, |ctx, args| {
        Box::pin(async move {
            MySqlRecordLinkRepository::new(ctx.pool.clone(), args.registry.clone())
                .ensure_deletable(ctx.tx, &args.record)
                .await
        })
    });
}
