use std::collections::{BTreeMap, HashMap};

use serde_json::{Value, json};
use uuid::Uuid;

use crate::DEMO_SEED;
use crate::store::DemoStore;
use crate::{
    DemoCleanup, DemoRemovalReport, DemoResetReport, DemoSeedCommand, DemoSeedReport, DemoVisitor,
    NewDemoVisitor,
};
use aphel_kernel::Ctx;
use aphel_kernel::domain::{Email, UserId};
use aphel_kernel::errors::{AppError, FieldViolation};
use aphel_sdk::configuration::{ApplyCommand, PackageSource, configuration_packages};
use aphel_sdk::maintenance::{DatasetSeed, DatasetSession, MaintenanceCatalog, TemplateOnboarding};
use aphel_sdk::platform::application::access::{AccessRepository, build_access_scope};
use aphel_sdk::platform::application::security::{PasswordHasher, password_violation};
use aphel_sdk::platform::application::shared::{
    Actor, AuditEntry, AuditLog, DomainEvent, EventPublisher, FileStorage,
};
use aphel_sdk::platform::domain::access::{AccessScope, Permission, permission_catalog};
use aphel_sdk::platform::infrastructure::auth::Argon2PasswordHasher;
use aphel_sdk::platform::infrastructure::repositories::mysql_access::MySqlAccessRepository;
const STAGE_KIND_OPEN: &str = "open";
const STAGE_KIND_WON: &str = "won";
const STAGE_KIND_LOST: &str = "lost";

pub(crate) const DEMO_SEEDED: &str = "demo.seeded";
pub(crate) const DEFAULTS_TEMPLATE: &str = "crm";
pub(crate) const DEMO_TEMPLATE: &str = "sales";
pub(crate) const DEMO_RESET: &str = "demo.reset";
pub(crate) const DEMO_REMOVED: &str = "demo.removed";
pub(crate) const DEMO_ALREADY_SEEDED: &str = "demo_already_seeded";
pub(crate) const DEMO_NOT_INSTALLED: &str = "demo_not_installed";
pub(crate) const DEMO_VISITOR_ROLE: &str = "demo_visitor";
pub(crate) const DEMO_VISITOR_ROLE_NAME: &str = "Visitante da demonstração";
pub(crate) const DEMO_VISITOR_NAME: &str = "Visitante Demo";

struct SeedState {
    counts: BTreeMap<String, u64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Cleanup {
    Tracked,
    Everything,
}

pub(crate) async fn seed(
    ctx: &Ctx<'_>,
    command: DemoSeedCommand,
) -> Result<DemoSeedReport, AppError> {
    let store = DemoStore::of(ctx);
    let admin_id = store.admin_user().await?.ok_or_else(|| {
        AppError::conflict_code(
            DEMO_NOT_INSTALLED,
            "the installation has no active administrator; run the installer first",
        )
    })?;
    if !command.force && (store.tracked_count().await? > 0 || store.has_business_records().await?) {
        return Err(AppError::conflict_code(
            DEMO_ALREADY_SEEDED,
            "the installation already has records; run `aphel demo remove` first or seed with --force",
        ));
    }
    let scope = admin_scope(ctx, &admin_id).await?;
    let ctx = ctx.with_scope(&scope);
    let visitor_email = match &command.visitor {
        Some(visitor) => Some(ensure_visitor(&ctx, &store, visitor).await?),
        None => None,
    };
    install_template(&ctx, DEFAULTS_TEMPLATE).await?;
    install_template(&ctx, DEMO_TEMPLATE).await?;
    let mut catalog = MaintenanceCatalog::default();
    for seed in ordered_seeds(&ctx) {
        seed.catalog(&ctx, &mut catalog).await?;
    }
    let has_kind = |kind: &str| {
        catalog
            .stages
            .iter()
            .any(|(_, stage_kind)| stage_kind == kind)
    };
    if !has_kind(STAGE_KIND_OPEN) || !has_kind(STAGE_KIND_WON) || !has_kind(STAGE_KIND_LOST) {
        return Err(AppError::Validation(
            "the default pipeline needs open, won and lost stages".to_owned(),
        ));
    }
    let mut state = SeedState {
        counts: BTreeMap::new(),
    };
    run_dataset_seeds(&ctx, &store, catalog, &mut state).await?;

    record_outcome(
        &ctx,
        &store,
        DEMO_SEEDED,
        None,
        json!({ "counts": state.counts }),
    )
    .await?;

    Ok(DemoSeedReport {
        visitor_email,
        counts: state.counts,
    })
}

pub(crate) async fn remove(
    ctx: &Ctx<'_>,
    storage: &dyn FileStorage,
) -> Result<DemoRemovalReport, AppError> {
    let store = DemoStore::of(ctx);
    let cleanup =
        record_outcome(ctx, &store, DEMO_REMOVED, Some(Cleanup::Tracked), json!({})).await?;
    let removed_files = delete_objects(storage, &cleanup.storage_keys).await;
    Ok(DemoRemovalReport {
        counts: cleanup.counts,
        removed_files,
    })
}

pub(crate) async fn reset(
    ctx: &Ctx<'_>,
    storage: &dyn FileStorage,
    full: bool,
    visitor: Option<DemoVisitor>,
) -> Result<DemoResetReport, AppError> {
    let store = DemoStore::of(ctx);
    let cleanup = if full {
        Cleanup::Everything
    } else {
        Cleanup::Tracked
    };
    let removed = record_outcome(
        ctx,
        &store,
        DEMO_RESET,
        Some(cleanup),
        json!({ "full": full }),
    )
    .await?;
    let removed_files = delete_objects(storage, &removed.storage_keys).await;
    let seeded = seed(
        ctx,
        DemoSeedCommand {
            force: true,
            visitor,
        },
    )
    .await?;
    Ok(DemoResetReport {
        removed: removed.counts,
        removed_files,
        seeded: seeded.counts,
        visitor_email: seeded.visitor_email,
    })
}

async fn delete_objects(storage: &dyn FileStorage, storage_keys: &[String]) -> u64 {
    let mut removed = 0;
    for key in storage_keys {
        match storage.delete(key).await {
            Ok(()) => removed += 1,
            Err(error) => {
                tracing::warn!(storage_key = %key, error = %error, "failed to remove a demo file from the storage");
            }
        }
    }
    removed
}

async fn ensure_visitor(
    ctx: &Ctx<'_>,
    store: &DemoStore,
    visitor: &DemoVisitor,
) -> Result<String, AppError> {
    let mut violations = Vec::new();
    let email = Email::new(visitor.email.trim().to_lowercase()).ok();
    if email.is_none() {
        violations.push(FieldViolation::new(
            "visitor_email",
            "must be a valid email address",
        ));
    }
    if let Some(message) = password_violation(visitor.password.expose()) {
        violations.push(FieldViolation::new("visitor_password", message));
    }
    let (Some(email), true) = (email, violations.is_empty()) else {
        return Err(AppError::invalid_fields("invalid demo visitor", violations));
    };
    store
        .ensure_visitor(&NewDemoVisitor {
            user_id: UserId::new(Uuid::now_v7().to_string())?,
            email: email.clone(),
            display_name: DEMO_VISITOR_NAME.to_owned(),
            password_hash: Argon2PasswordHasher.hash_password(visitor.password.expose())?,
            role_key: DEMO_VISITOR_ROLE.to_owned(),
            role_name: DEMO_VISITOR_ROLE_NAME.to_owned(),
            permissions: demo_visitor_permissions(),
            verified_at: ctx.now(),
        })
        .await?;
    Ok(email.as_str().to_owned())
}

async fn record_outcome(
    ctx: &Ctx<'_>,
    store: &DemoStore,
    action: &str,
    cleanup: Option<Cleanup>,
    payload: Value,
) -> Result<DemoCleanup, AppError> {
    let now = ctx.now();
    let mut tx = ctx.begin().await?;
    let removed = match cleanup {
        Some(Cleanup::Tracked) => store.remove_tracked(&mut tx).await?,
        Some(Cleanup::Everything) => store.wipe(&mut tx).await?,
        None => DemoCleanup::default(),
    };
    let mut payload = payload;
    if cleanup.is_some()
        && let Value::Object(object) = &mut payload
    {
        object.insert("counts".to_owned(), json!(removed.counts));
        object.insert("files".to_owned(), json!(removed.storage_keys.len()));
    }
    let event = DomainEvent::new(Actor::System, action, now).with_payload(payload);
    let entry = AuditEntry::new(Actor::System, action, now);
    ctx.events().publish(&mut tx, event).await?;
    ctx.audit().record(&mut tx, entry).await?;
    tx.commit().await?;
    Ok(removed)
}

async fn admin_scope(ctx: &Ctx<'_>, user_id: &UserId) -> Result<AccessScope, AppError> {
    let access = MySqlAccessRepository::new(ctx.pool().clone())
        .load_access(user_id)
        .await?
        .ok_or_else(|| AppError::Validation("the demo administrator has no access".to_owned()))?;
    Ok(build_access_scope(user_id.clone(), access))
}

async fn install_template(ctx: &Ctx<'_>, template_key: &str) -> Result<(), AppError> {
    let packages = configuration_packages(ctx)?;
    let source = PackageSource::Template(template_key.to_owned());
    let plan = packages.plan(ctx, source.clone()).await?;
    if !plan.valid {
        return Err(AppError::Configuration(format!(
            "template {template_key} is not valid: {:?}",
            plan.issues
        )));
    }
    let result = packages
        .apply(
            ctx,
            ApplyCommand {
                source,
                decisions: HashMap::new(),
                fingerprint: plan.fingerprint,
                ip: None,
            },
        )
        .await?;
    if let Some(onboarding) = ctx.services().get::<dyn TemplateOnboarding>() {
        onboarding
            .complete_with_template(ctx, template_key, &result.install.id.to_string())
            .await?;
    }
    Ok(())
}

fn ordered_seeds(ctx: &Ctx<'_>) -> Vec<std::sync::Arc<dyn DatasetSeed>> {
    let mut seeds = ctx.app().contributions::<dyn DatasetSeed>();
    seeds.sort_by_key(|seed| seed.order());
    seeds
}

async fn run_dataset_seeds(
    ctx: &Ctx<'_>,
    store: &DemoStore,
    catalog: MaintenanceCatalog,
    state: &mut SeedState,
) -> Result<(), AppError> {
    let mut session = DatasetSession::new(ctx.now(), DEMO_SEED, catalog);
    let seeds = ordered_seeds(ctx);
    for seed in &seeds {
        seed.seed(ctx, &mut session).await?;
        for entry in session.take_pending() {
            store.track(&entry.record).await?;
            *state
                .counts
                .entry(entry.record.record_type.as_str().to_owned())
                .or_default() += 1;
            let mut handled = false;
            for owner in &seeds {
                if owner.backdate(ctx, &entry).await? {
                    handled = true;
                    break;
                }
            }
            if !handled {
                store
                    .backdate(&entry.record, entry.created_at, entry.changed_at)
                    .await?;
            }
        }
        for (key, amount) in session.take_counts() {
            *state.counts.entry(key).or_default() += amount;
        }
    }
    Ok(())
}

pub(crate) fn demo_visitor_permissions() -> Vec<Permission> {
    permission_catalog()
        .into_iter()
        .filter(|definition| {
            definition.record_scoped && !definition.permission.as_str().ends_with(".export")
        })
        .map(|definition| definition.permission)
        .collect()
}
