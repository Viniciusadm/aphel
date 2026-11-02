use std::sync::Arc;

use utoipa_axum::router::OpenApiRouter;

use crate::api::records::custom_record_handler;
use crate::api::routes::{health, v1};
use crate::api::state::AppState;
use crate::application::custom_objects::RecordLinkRepository;
use crate::application::fields::formulas::formulas_daily_schedule;
use crate::application::idempotency::idempotency_purge_schedule;
use crate::application::integrations::GlobalMailerResolver;
use crate::domain::access::{Permission, builtin_permission_catalog};
use crate::domain::notifications::builtin_notification_kinds;
use crate::errors::AppError;
use crate::infrastructure::custom_objects::MySqlRecordLinkRepository;
use crate::infrastructure::fields::formulas::{
    formula_daily_handler, formula_events_consumer, formula_recalculate_handler,
};
use crate::infrastructure::fields::validation_rules::validation_rule_guard;
use crate::infrastructure::idempotency::idempotency_purge_handler;
use crate::infrastructure::notifications::notification_worker_parts;
use crate::infrastructure::search::{search_index_consumer, search_reindex_handler};
use crate::infrastructure::security::security_email_handler;
use crate::modules::builder::{APHEL_VERSION, KERNEL_MODULE};
use crate::modules::{AppBuilder, HookUsage, Manifest, Module, WorkerEnv, WorkerParts};
use crate::records::{RecordDeleting, RecordValidating};

const TAGS: &[(&str, &str)] = &[
    (
        "communication",
        "Communication opt-outs: addresses that must not receive messages sent on behalf of plugins and modules",
    ),
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
        "views",
        "Saved views (filter, sort and columns) of each record type",
    ),
    (
        "layouts",
        "Form layouts (tabs, sections, columns and conditional rules) of each record type",
    ),
    (
        "directory",
        "Directory of the active organization's users, readable by every member",
    ),
    (
        "validation-rules",
        "Validation rules: boolean conditions per record type that block writes (forms, API, imports, automations) with a custom message",
    ),
    (
        "notifications",
        "In-app notifications and notification preferences",
    ),
    (
        "config-packages",
        "Configuration packages (`aphel-config`): export, business templates, validation and installation plan, application and history",
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
        "search",
        "Global search across every record type the caller can read, grouped by type",
    ),
    (
        "preferences",
        "Personal preferences of the current user, such as keyboard shortcuts",
    ),
    (
        "custom-objects",
        "Custom objects of the organization (record types `custom.<key>` with their own fields, permissions and relationships)",
    ),
    (
        "objects",
        "Records of custom objects through a generic API; `GET /objects/{key}/schema` documents the fields of each object at runtime",
    ),
    (
        "relationships",
        "Configurable relationships between any pair of record types and the links between records",
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
        declare_record_hooks(app);
        register_record_hooks(app);
        for definition in crate::application::organizations::ORGANIZATION_PUBLIC_EVENTS {
            app.public_event(*definition);
        }
        app.openapi_tag("health", "Service health")
            .openapi_tag(
                v1::demo::TAG,
                "Demonstration mode status and the public credentials of the demo visitor",
            )
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
        for definition in builtin_notification_kinds() {
            app.notification_kind(*definition);
        }
        app.worker(maintenance)
            .worker(records)
            .worker(notifications)
            .worker(security);
        Ok(())
    }
}

fn routes() -> OpenApiRouter<AppState> {
    OpenApiRouter::new()
        .merge(v1::modules::router())
        .merge(v1::demo::router())
        .merge(v1::auth::router())
        .merge(v1::api_tokens::router())
        .merge(v1::installer::router())
        .merge(v1::organization::router())
        .merge(v1::organization_settings::router())
        .merge(v1::members::router())
        .merge(v1::roles::router())
        .merge(v1::teams::router())
        .merge(v1::system_jobs::router())
        .merge(v1::system::router())
        .merge(v1::platform_catalog::router())
        .merge(v1::entities::router())
        .merge(v1::audit::router())
        .merge(v1::files::router())
        .merge(v1::fields::router())
        .merge(v1::formulas::router())
        .merge(v1::views::router())
        .merge(v1::layouts::router())
        .merge(v1::directory::router())
        .merge(v1::communication::router())
        .merge(v1::notifications::router())
        .merge(v1::push::router())
        .merge(v1::custom_objects::router())
        .merge(v1::config_packages::router())
        .merge(v1::config_package_versions::router())
        .merge(v1::catalog::router())
        .merge(v1::two_factor::router())
        .merge(v1::security::router())
        .merge(v1::field_permissions::router())
        .merge(v1::aggregates::router())
        .merge(v1::search::router())
        .merge(v1::preferences::router())
        .merge(v1::record_shares::router())
        .merge(v1::validation_rules::router())
}

fn maintenance(env: &WorkerEnv) -> WorkerParts {
    let pool = &env.pool;
    WorkerParts::default()
        .handler(idempotency_purge_handler(pool.clone()))
        .schedule(idempotency_purge_schedule())
}

fn records(env: &WorkerEnv) -> WorkerParts {
    let pool = &env.pool;
    let record_types = env.record_types.clone();
    WorkerParts::default()
        .consumer(formula_events_consumer(pool.clone(), record_types.clone()))
        .handler(formula_recalculate_handler(
            pool.clone(),
            record_types.clone(),
        ))
        .handler(formula_daily_handler(pool.clone()))
        .schedule(formulas_daily_schedule())
        .handler(search_reindex_handler(pool.clone(), record_types.clone()))
        .consumer(search_index_consumer(pool.clone(), record_types))
}

fn notifications(env: &WorkerEnv) -> WorkerParts {
    let pool = &env.pool;
    let mut parts = WorkerParts::default();
    match crate::infrastructure::notifications::push::push_delivery_handler(
        pool.clone(),
        env.secrets.clone(),
        &env.public_url.get(),
    ) {
        Ok(handler) => parts = parts.handler(handler),
        Err(error) => tracing::warn!(%error, "web push delivery disabled"),
    }
    let mailers = Arc::new(GlobalMailerResolver::new(env.mailer.clone()));
    let notifications = notification_worker_parts(pool.clone(), mailers, env.public_url.clone());
    parts
        .consumer(notifications.consumer)
        .handlers(notifications.handlers)
        .schedules(notifications.schedules)
}

fn security(env: &WorkerEnv) -> WorkerParts {
    WorkerParts::default().handler(security_email_handler(
        env.pool.clone(),
        env.mailer.clone(),
        env.public_url.clone(),
    ))
}

pub const VALIDATION_RULE_PRIORITY: i32 = 30;
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
    app.on(RecordValidating, VALIDATION_RULE_PRIORITY, |ctx, args| {
        Box::pin(async move {
            validation_rule_guard(ctx.pool, &args.registry)
                .check(ctx.tx, ctx.scope, &args.write())
                .await
        })
    })
    .on(RecordDeleting, RESTRICTED_LINKS_PRIORITY, |ctx, args| {
        Box::pin(async move {
            MySqlRecordLinkRepository::new(ctx.pool.clone(), args.registry.clone())
                .ensure_deletable(ctx.tx, &args.record)
                .await
        })
    });
}
