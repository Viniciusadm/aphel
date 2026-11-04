use aphel_contract_crm::CrmRecordTypes;
use std::collections::{BTreeMap, HashMap};

use chrono::{DateTime, Duration, Utc};
use serde_json::{Map, Value, json};
use uuid::Uuid;

use crate::revenue;
use crate::store::DemoStore;
use crate::{
    DemoCatalog, DemoCleanup, DemoDataset, DemoDealOutcome, DemoRemovalReport, DemoResetReport,
    DemoSeedCommand, DemoSeedReport, DemoTarget, DemoVisitor, NewDemoVisitor, generate,
};
use aphel_kernel::Ctx;
use aphel_kernel::application::access::{AccessRepository, build_access_scope};
use aphel_kernel::application::fields::{CreateFieldCommand, FieldOptionInput, FieldUseCases};
use aphel_kernel::application::security::{PasswordHasher, password_violation};
use aphel_kernel::application::shared::{
    Actor, AuditEntry, AuditLog, DomainEvent, EventPublisher, FileStorage, RecordRef, RecordType,
};
use aphel_kernel::domain::access::{AccessScope, Permission, permission_catalog};
use aphel_kernel::domain::{Email, UserId};
use aphel_kernel::errors::{AppError, FieldViolation};
use aphel_kernel::infrastructure::auth::Argon2PasswordHasher;
use aphel_kernel::infrastructure::fields::MySqlFieldPorts;
use aphel_kernel::infrastructure::repositories::mysql_access::MySqlAccessRepository;
use aphel_kernel::records::AddressInput;
use aphel_sdk::configuration::{ApplyCommand, PackageSource, configuration_packages};
use aphel_sdk::notes::CreateNoteCommand;
use aphel_sdk::tasks::CreateActivityCommand;
use deals::domain::{DealInput, LoseDealCommand, MoveDealCommand};
use onboarding::domain::CompleteOnboardingCommand;
use onboarding::service::ONBOARDING_NOT_PENDING;
use organizations::domain::{CompanyInput, SaveCompanyCommand};
use people::domain::{ContactInput, SaveContactCommand};
use workflow::seed::{STAGE_KIND_LOST, STAGE_KIND_OPEN, STAGE_KIND_WON};

pub(crate) const DEMO_SEEDED: &str = "demo.seeded";
pub(crate) const DEMO_TEMPLATE: &str = "b2b";
pub(crate) const DEMO_RESET: &str = "demo.reset";
pub(crate) const DEMO_REMOVED: &str = "demo.removed";
pub(crate) const DEMO_ALREADY_SEEDED: &str = "demo_already_seeded";
pub(crate) const DEMO_NOT_INSTALLED: &str = "demo_not_installed";
pub(crate) const DEMO_VISITOR_ROLE: &str = "demo_visitor";
pub(crate) const DEMO_VISITOR_ROLE_NAME: &str = "Visitante da demonstração";
pub(crate) const DEMO_VISITOR_NAME: &str = "Visitante Demo";
pub(crate) const DEMO_FIELD_RECORD_TYPE: RecordType = RecordType::from_static("field");
const FOREIGN_CURRENCY_EVERY: usize = 5;

struct SeedState {
    organizations: Vec<Uuid>,
    people: Vec<Uuid>,
    deals: Vec<Uuid>,
    counts: BTreeMap<String, u64>,
}

impl SeedState {
    fn target(&self, target: DemoTarget) -> (RecordType, Uuid) {
        match target {
            DemoTarget::Company(index) => (RecordType::COMPANY, self.organizations[index]),
            DemoTarget::Contact(index) => (RecordType::CONTACT, self.people[index]),
            DemoTarget::Deal(index) => (RecordType::DEAL, self.deals[index]),
        }
    }
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
    install_template(&ctx, DEMO_TEMPLATE).await?;
    let catalog = store.catalog().await?;
    let now = ctx.now();
    let open_stages: Vec<&str> = catalog
        .stages
        .iter()
        .filter(|stage| stage.kind == STAGE_KIND_OPEN)
        .map(|stage| stage.id.as_str())
        .collect();
    if open_stages.is_empty()
        || !catalog
            .stages
            .iter()
            .any(|stage| stage.kind == STAGE_KIND_WON)
        || !catalog
            .stages
            .iter()
            .any(|stage| stage.kind == STAGE_KIND_LOST)
    {
        return Err(AppError::Validation(
            "the default pipeline needs open, won and lost stages".to_owned(),
        ));
    }
    let dataset = generate(now, open_stages.len(), catalog.lost_reason_ids.len());

    let mut state = SeedState {
        organizations: Vec::new(),
        people: Vec::new(),
        deals: Vec::new(),
        counts: BTreeMap::new(),
    };
    seed_fields(&ctx, &store, &mut state).await?;
    seed_companies(&ctx, &store, &dataset, &mut state).await?;
    seed_contacts(&ctx, &store, &dataset, &mut state).await?;
    let foreign_currency = revenue::seed_currency(&ctx).await?;
    if let Some(currency) = foreign_currency {
        for id in revenue::seed_rates(&ctx, currency).await? {
            track(&store, &mut state, revenue::EXCHANGE_RATE_RECORD_TYPE, id).await?;
        }
    }
    seed_deals(
        &ctx,
        &store,
        &dataset,
        &catalog,
        &open_stages,
        foreign_currency.map(|(code, _)| code),
        &mut state,
    )
    .await?;
    seed_activities(&ctx, &store, &dataset, &catalog, &mut state).await?;
    seed_notes(&ctx, &store, &dataset, &mut state).await?;
    seed_revenue(&ctx, &store, &dataset, &catalog, &open_stages, &mut state).await?;

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
    match onboarding::service::complete(
        ctx,
        CompleteOnboardingCommand {
            template_key: Some(template_key.to_owned()),
            install_id: Some(result.install.id.to_string()),
            ip: None,
        },
    )
    .await
    {
        Ok(_)
        | Err(AppError::Coded {
            code: ONBOARDING_NOT_PENDING,
            ..
        }) => Ok(()),
        Err(error) => Err(error),
    }
}

async fn track(
    store: &DemoStore,
    state: &mut SeedState,
    record_type: RecordType,
    id: Uuid,
) -> Result<RecordRef, AppError> {
    let record = RecordRef::new(record_type, id);
    store.track(&record).await?;
    *state
        .counts
        .entry(record.record_type.as_str().to_owned())
        .or_default() += 1;
    Ok(record)
}

async fn seed_fields(
    ctx: &Ctx<'_>,
    store: &DemoStore,
    state: &mut SeedState,
) -> Result<(), AppError> {
    let fields = FieldUseCases::new(MySqlFieldPorts::new(
        ctx.pool().clone(),
        ctx.registry().clone(),
        ctx.events().clone(),
        ctx.audit().clone(),
    ));
    for (record_type, command) in demo_fields() {
        let id = fields
            .create_field(ctx.scope, record_type, command)
            .await?
            .definition
            .id;
        track(store, state, DEMO_FIELD_RECORD_TYPE, id).await?;
    }
    Ok(())
}

async fn seed_companies(
    ctx: &Ctx<'_>,
    store: &DemoStore,
    dataset: &DemoDataset,
    state: &mut SeedState,
) -> Result<(), AppError> {
    for company in &dataset.organizations {
        let id = organizations::service::create(
            ctx,
            SaveCompanyCommand {
                input: CompanyInput {
                    name: text(&company.name),
                    domain: text(&company.domain),
                    website: text(&format!("https://{}", company.domain)),
                    industry: text(&company.industry),
                    size_range: text(&company.size_range),
                    phone: text(&company.phone),
                    email: text(&format!("contato@{}", company.domain)),
                    address: AddressInput {
                        city: text(&company.city),
                        state: text(&company.state),
                        country: text("BR"),
                        ..AddressInput::default()
                    },
                    source: text(&company.source),
                    ..CompanyInput::default()
                },
                custom_fields: custom(&[("customer_tier", json!(company.tier))]),
                ip: None,
                expected_updated_at: None,
            },
        )
        .await?
        .view
        .company
        .id;
        let record = track(store, state, RecordType::COMPANY, id).await?;
        store
            .backdate(&record, company.created_at, company.created_at)
            .await?;
        state.organizations.push(id);
    }
    Ok(())
}

async fn seed_contacts(
    ctx: &Ctx<'_>,
    store: &DemoStore,
    dataset: &DemoDataset,
    state: &mut SeedState,
) -> Result<(), AppError> {
    for contact in &dataset.people {
        let id = people::service::create(
            ctx,
            SaveContactCommand {
                input: ContactInput {
                    first_name: text(&contact.first_name),
                    last_name: text(&contact.last_name),
                    email: text(&contact.email),
                    mobile: text(&contact.mobile),
                    job_title: text(&contact.job_title),
                    company_id: text(&state.organizations[contact.company_index].to_string()),
                    source: text(&contact.source),
                    ..ContactInput::default()
                },
                custom_fields: custom(&[("preferred_channel", json!(contact.preferred_channel))]),
                ip: None,
                expected_updated_at: None,
            },
        )
        .await?
        .view
        .contact
        .id;
        let record = track(store, state, RecordType::CONTACT, id).await?;
        store
            .backdate(&record, contact.created_at, contact.created_at)
            .await?;
        state.people.push(id);
    }
    Ok(())
}

async fn seed_deals(
    ctx: &Ctx<'_>,
    store: &DemoStore,
    dataset: &DemoDataset,
    catalog: &DemoCatalog,
    open_stages: &[&str],
    foreign_currency: Option<&str>,
    state: &mut SeedState,
) -> Result<(), AppError> {
    let today = ctx.now().date_naive();
    for (index, deal) in dataset.deals.iter().enumerate() {
        let currency = foreign_currency
            .filter(|_| index % FOREIGN_CURRENCY_EVERY == FOREIGN_CURRENCY_EVERY - 1)
            .map(str::to_owned);
        let id = deals::service::create(
            ctx,
            DealInput {
                title: Some(deal.title.clone()),
                amount: Some(Some(format!("{}.00", deal.amount))),
                stage_id: Some(open_stages[0].to_owned()),
                expected_close_date: Some(Some(
                    (today + Duration::days(deal.expected_close_in_days))
                        .format("%Y-%m-%d")
                        .to_string(),
                )),
                contact_id: Some(Some(state.people[deal.contact_index].to_string())),
                company_id: Some(Some(state.organizations[deal.company_index].to_string())),
                source: Some(Some("Demonstração".to_owned())),
                currency,
                custom_fields: custom(&[("contract_months", json!(deal.contract_months))]),
                ..DealInput::default()
            },
        )
        .await?
        .record
        .deal
        .id;
        let record = track(store, state, RecordType::DEAL, id).await?;
        let deal_id = id.to_string();
        let reached = match deal.outcome {
            DemoDealOutcome::Open { stage_index } => stage_index,
            DemoDealOutcome::Won { reached_stage }
            | DemoDealOutcome::Lost { reached_stage, .. } => reached_stage,
        };
        for stage_id in open_stages.iter().take(reached + 1).skip(1) {
            deals::service::move_to_stage(
                ctx,
                &deal_id,
                MoveDealCommand {
                    stage_id: (*stage_id).to_owned(),
                    ..MoveDealCommand::default()
                },
            )
            .await?;
        }
        match deal.outcome {
            DemoDealOutcome::Won { .. } => {
                deals::service::win(ctx, &deal_id, None).await?;
            }
            DemoDealOutcome::Lost { reason_index, .. } => {
                deals::service::lose(
                    ctx,
                    &deal_id,
                    LoseDealCommand {
                        lost_reason_id: catalog.lost_reason_ids.get(reason_index).cloned(),
                        lost_reason_note: Some("Registrado pelos dados de demonstração".to_owned()),
                        ip: None,
                    },
                )
                .await?;
            }
            DemoDealOutcome::Open { .. } => {}
        }
        store
            .backdate(&record, deal.created_at, deal.last_change_at)
            .await?;
        state.deals.push(id);
    }
    Ok(())
}

async fn seed_activities(
    ctx: &Ctx<'_>,
    store: &DemoStore,
    dataset: &DemoDataset,
    catalog: &DemoCatalog,
    state: &mut SeedState,
) -> Result<(), AppError> {
    let fallback_type = catalog.task_types.values().next().cloned();
    for activity in &dataset.tasks {
        let Some(activity_type_id) = catalog
            .task_types
            .get(&activity.type_key)
            .cloned()
            .or_else(|| fallback_type.clone())
        else {
            return Err(AppError::Validation(
                "the organization has no activity types".to_owned(),
            ));
        };
        let (record_type, record_id) = state.target(activity.target);
        let id = aphel_sdk::tasks::writer()?
            .create(
                ctx,
                CreateActivityCommand {
                    title: activity.title.clone(),
                    activity_type_id,
                    status: Some(
                        if activity.completed_at.is_some() {
                            "done"
                        } else {
                            "open"
                        }
                        .to_owned(),
                    ),
                    priority: Some(activity.priority.clone()),
                    due_at: Some(rfc3339(activity.due_at)),
                    completed_at: activity.completed_at.map(rfc3339),
                    record_type: Some(record_type.as_str().to_owned()),
                    record_id: Some(record_id.to_string()),
                    ..CreateActivityCommand::default()
                },
            )
            .await?;
        let record = track(store, state, RecordType::ACTIVITY, id).await?;
        let changed_at = activity.completed_at.unwrap_or(activity.created_at);
        store
            .backdate(&record, activity.created_at, changed_at)
            .await?;
    }
    Ok(())
}

async fn seed_notes(
    ctx: &Ctx<'_>,
    store: &DemoStore,
    dataset: &DemoDataset,
    state: &mut SeedState,
) -> Result<(), AppError> {
    for note in &dataset.notes {
        let (record_type, record_id) = state.target(note.target);
        let id = aphel_sdk::notes::writer()?
            .create(
                ctx,
                CreateNoteCommand {
                    record_type: record_type.as_str().to_owned(),
                    record_id: record_id.to_string(),
                    body: note.body.clone(),
                    visibility: Some("team".to_owned()),
                    pinned: Some(note.pinned),
                    ip: None,
                },
            )
            .await?;
        let record = track(store, state, RecordType::NOTE, id).await?;
        store
            .backdate(&record, note.created_at, note.created_at)
            .await?;
    }
    Ok(())
}

async fn seed_revenue(
    ctx: &Ctx<'_>,
    store: &DemoStore,
    dataset: &DemoDataset,
    catalog: &DemoCatalog,
    open_stages: &[&str],
    state: &mut SeedState,
) -> Result<(), AppError> {
    revenue::seed_rotting(ctx, open_stages).await?;
    let products = revenue::seed_products(ctx).await?;
    for product in &products {
        track(store, state, RecordType::PRODUCT, product.id).await?;
    }
    if let Some(id) = revenue::seed_price_list(ctx, store, &products).await? {
        track(store, state, revenue::PRICE_LIST_RECORD_TYPE, id).await?;
    }
    let open_deals: Vec<Uuid> = dataset
        .deals
        .iter()
        .zip(&state.deals)
        .filter(|(deal, _)| matches!(deal.outcome, DemoDealOutcome::Open { .. }))
        .map(|(_, id)| *id)
        .collect();
    let with_items = revenue::seed_line_items(ctx, &open_deals, &products).await?;
    if !with_items.is_empty() {
        *state.counts.entry("line_items".to_owned()).or_default() += with_items.len() as u64;
    }
    for id in revenue::seed_quotes(ctx, &with_items).await? {
        track(store, state, revenue::QUOTE_RECORD_TYPE, id).await?;
    }
    let members = revenue::seed_deal_team(ctx, store, &state.deals).await?;
    if members > 0 {
        *state.counts.entry("deal_members".to_owned()).or_default() += members;
    }
    if let Some(won_stage) = catalog
        .stages
        .iter()
        .find(|stage| stage.kind == STAGE_KIND_WON)
        && let Some(id) = revenue::seed_approval_policy(ctx, store, &won_stage.id).await?
    {
        track(store, state, revenue::APPROVAL_POLICY_RECORD_TYPE, id).await?;
    }
    if let Some(id) = revenue::seed_validation_rule(ctx, store).await? {
        track(store, state, revenue::VALIDATION_RULE_RECORD_TYPE, id).await?;
    }
    Ok(())
}

fn demo_fields() -> Vec<(&'static str, CreateFieldCommand)> {
    let field = |key: &str,
                 label: aphel_kernel::application::fields::LocalizedLabels,
                 field_type: &str,
                 options: &[(&str, &str)]| CreateFieldCommand {
        key: key.to_owned(),
        label: label.default_text().to_owned(),
        field_type: field_type.to_owned(),
        is_required: false,
        is_unique: false,
        is_searchable: false,
        is_personal: false,
        is_sensitive: false,
        config: Value::Null,
        options: options
            .iter()
            .map(|(value, label)| FieldOptionInput {
                value: Some((*value).to_owned()),
                label: (*label).to_owned(),
                color: None,
            })
            .collect(),
        ip: None,
    };
    vec![
        (
            "organizations.organization",
            field(
                "customer_tier",
                crate::labels("Customer category"),
                "select",
                &[("gold", "Ouro"), ("silver", "Prata"), ("bronze", "Bronze")],
            ),
        ),
        (
            "people.person",
            field(
                "preferred_channel",
                crate::labels("Preferred channel"),
                "select",
                &[
                    ("email", "E-mail"),
                    ("phone", "Telefone"),
                    ("whatsapp", "WhatsApp"),
                ],
            ),
        ),
        (
            "deals.deal",
            field(
                "contract_months",
                crate::labels("Contract duration (months)"),
                "number",
                &[],
            ),
        ),
    ]
}

fn text(value: &str) -> Option<Option<String>> {
    Some(Some(value.to_owned()))
}

fn custom(values: &[(&str, Value)]) -> Map<String, Value> {
    values
        .iter()
        .map(|(key, value)| ((*key).to_owned(), value.clone()))
        .collect()
}

fn rfc3339(value: DateTime<Utc>) -> String {
    value.to_rfc3339()
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
