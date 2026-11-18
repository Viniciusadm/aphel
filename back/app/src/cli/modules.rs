use std::collections::{BTreeMap, HashSet};
use std::sync::Arc;

use chrono::Utc;
use clap::{Args, Subcommand};

use crate::cli::CliResult;
use crate::cli::doctor::connect_quietly;
use aphel_kernel::api::state::AppState;
use aphel_kernel::application::custom_objects::RecordTypeCatalog;
use aphel_kernel::application::fields::RecordTypeRegistry;
use aphel_kernel::application::organization_profile::{OrganizationConfigCatalog, PublicBaseUrl};
use aphel_kernel::config::AppConfig;
use aphel_kernel::database::Db;
use aphel_kernel::domain::access::AccessScope;
use aphel_kernel::errors::AppError;
use aphel_kernel::infrastructure::mail::SmtpMailerProvider;
use aphel_kernel::infrastructure::noop::{UnavailableFileStorage, UnavailableSecretVault};
use aphel_kernel::infrastructure::secrets::build_secret_vault;
use aphel_kernel::modules::runtime::module_use_cases;
use aphel_kernel::modules::worker::lazy_pool;
use aphel_kernel::modules::{App, LoadedModule, ModuleCtx, WorkerEnv, WorkerParts};

#[derive(Debug, Clone, PartialEq, Eq, Subcommand)]
pub(crate) enum ModulesCommand {
    #[command(about = "List the modules compiled into this build, in load order")]
    List,
    #[command(
        about = "Show what each module contributes (routes, hooks, services, catalogs, worker) and the conflicts between modules"
    )]
    Doctor,
    #[command(about = "Disable a module and run its uninstall step; --purge also removes its data")]
    Uninstall(UninstallArgs),
}

#[derive(Debug, Clone, PartialEq, Eq, Args)]
pub(crate) struct UninstallArgs {
    #[arg(help = "Module key, such as insights")]
    pub(crate) key: String,
    #[arg(
        long,
        help = "Export the data of the module to a JSON file and then remove it (tables, settings and secrets)"
    )]
    pub(crate) purge: bool,
}

pub(crate) async fn run(config: AppConfig, command: ModulesCommand) -> CliResult {
    let app = aphel_kernel::modules::app();
    match command {
        ModulesCommand::List => list(&config, app).await,
        ModulesCommand::Doctor => doctor(&config, app).await,
        ModulesCommand::Uninstall(args) => uninstall(&config, app, args).await,
    }
}

pub(crate) fn short_type(name: &str) -> String {
    let mut output = String::new();
    let mut segment = String::new();
    for character in name.chars() {
        if character.is_alphanumeric() || character == '_' || character == ':' {
            segment.push(character);
        } else {
            output.push_str(segment.rsplit("::").next().unwrap_or_default());
            segment.clear();
            output.push(character);
        }
    }
    output.push_str(segment.rsplit("::").next().unwrap_or_default());
    output
}

async fn enabled_from_database(config: &AppConfig, app: &App) -> Option<HashSet<String>> {
    let url = config.database_url.as_deref()?;
    match connect_quietly(url).await {
        Ok(pool) => match app.enabled().refresh(&pool).await {
            Ok(()) => Some(app.enabled().snapshot().into_iter().collect()),
            Err(error) => {
                eprintln!("could not read the enabled modules: {error}");
                None
            }
        },
        Err(error) => {
            eprintln!("could not connect to the database: {error}");
            None
        }
    }
}

fn dependencies(entry: &LoadedModule) -> String {
    let mut parts: Vec<String> = entry
        .manifest
        .dependencies
        .iter()
        .map(|dependency| format!("{} {}", dependency.key, dependency.version_req))
        .collect();
    parts.extend(
        entry
            .manifest
            .optional
            .iter()
            .map(|key| format!("{key} (optional)")),
    );
    if parts.is_empty() {
        "-".to_owned()
    } else {
        parts.join(", ")
    }
}

async fn list(config: &AppConfig, app: &App) -> CliResult {
    let enabled = enabled_from_database(config, app).await;
    println!(
        "{:<14} {:<10} {:<9} {:<9} {:<26} DEPENDENCIES",
        "KEY", "VERSION", "REQUIRED", "ENABLED", "COMPATIBILITY"
    );
    for entry in app.modules() {
        let state = match &enabled {
            Some(keys) if keys.contains(entry.key()) => "yes",
            Some(_) => "no",
            None => "?",
        };
        println!(
            "{:<14} {:<10} {:<9} {:<9} {:<26} {}",
            entry.key(),
            entry.manifest.version,
            if entry.manifest.required { "yes" } else { "no" },
            state,
            entry.compatibility.code(),
            dependencies(entry)
        );
    }
    Ok(())
}

fn doctor_env(config: &AppConfig) -> Result<WorkerEnv, AppError> {
    let pool = lazy_pool(config.database_url.as_deref())?;
    let secrets = Arc::new(UnavailableSecretVault);
    Ok(WorkerEnv {
        mailer: SmtpMailerProvider::shared(pool.clone(), secrets.clone()),
        secrets,
        storage: Arc::new(UnavailableFileStorage),
        public_url: PublicBaseUrl::live(Arc::new(OrganizationConfigCatalog::defaults_only())),
        outbound_http: config.outbound_http.clone(),
        record_types: Arc::new(RecordTypeCatalog::builtin_only(
            RecordTypeRegistry::builtin(),
        )),
        pool,
    })
}

fn route_paths(app: &App, state: &AppState, module: &str) -> Vec<String> {
    let mut paths = Vec::new();
    for contribution in app
        .route_contributions()
        .iter()
        .filter(|contribution| contribution.module == module)
    {
        let prefix = contribution.item.mount.prefix();
        let document = (contribution.item.factory)(state).into_openapi();
        for (path, item) in &document.paths.paths {
            let methods: Vec<&str> = [
                ("GET", item.get.is_some()),
                ("POST", item.post.is_some()),
                ("PUT", item.put.is_some()),
                ("PATCH", item.patch.is_some()),
                ("DELETE", item.delete.is_some()),
            ]
            .into_iter()
            .filter_map(|(method, present)| present.then_some(method))
            .collect();
            paths.push(format!(
                "{} {prefix}{path} [{}]",
                methods.join(","),
                contribution.item.mount.as_str()
            ));
        }
    }
    paths.sort();
    paths
}

fn section(title: &str, lines: &[String]) {
    if lines.is_empty() {
        return;
    }
    println!("  {title} ({}):", lines.len());
    for line in lines {
        println!("    {line}");
    }
}

async fn doctor(config: &AppConfig, app: &App) -> CliResult {
    let enabled = enabled_from_database(config, app).await;
    let state = AppState::new(None);
    let env = doctor_env(config)?;
    let mut job_owners: BTreeMap<String, Vec<&'static str>> = BTreeMap::new();
    let mut consumer_owners: BTreeMap<String, Vec<&'static str>> = BTreeMap::new();
    let mut schedule_owners: BTreeMap<String, Vec<&'static str>> = BTreeMap::new();

    for entry in app.modules() {
        let key = entry.key();
        let state_label = match &enabled {
            _ if entry.manifest.required => "required",
            Some(keys) if keys.contains(key) => "enabled",
            Some(_) => "disabled",
            None => "unknown",
        };
        println!(
            "{key} {} [{state_label}, {}] depends on: {}",
            entry.manifest.version,
            entry.compatibility.code(),
            dependencies(entry)
        );
        if !entry.is_compatible() {
            println!(
                "  not loaded: {}",
                entry.compatibility.detail().unwrap_or_default()
            );
            continue;
        }

        section("routes", &route_paths(app, &state, key));

        let hooks: Vec<String> = app
            .hooks()
            .describe()
            .iter()
            .filter(|hook| hook.module == key)
            .map(|hook| {
                format!(
                    "{} {} (priority {})",
                    hook.kind.as_str(),
                    hook.hook,
                    hook.priority
                )
            })
            .collect();
        section("hooks", &hooks);

        let records: Vec<String> = app
            .record_contributions()
            .iter()
            .filter(|record| record.module == key)
            .map(|record| record.description.clone())
            .collect();
        section("record types", &records);

        let services: Vec<String> = app
            .services()
            .describe()
            .into_iter()
            .filter(|service| service.module == key || service.decorators.contains(&key))
            .map(|service| {
                if service.module == key {
                    format!("provides {}", service.type_name)
                } else {
                    format!(
                        "decorates {} (provided by {})",
                        service.type_name, service.module
                    )
                }
            })
            .collect();
        section("services", &services);

        let catalog = app.catalog();
        let permissions: Vec<String> = catalog
            .permission_contributions()
            .iter()
            .filter(|contribution| contribution.module == key)
            .map(|contribution| {
                let roles: Vec<&str> = catalog
                    .grant_contributions()
                    .iter()
                    .filter(|grant| {
                        grant.module == key && grant.item.permission == contribution.item.permission
                    })
                    .flat_map(|grant| grant.item.roles.iter().copied())
                    .collect();
                if roles.is_empty() {
                    contribution.item.permission.to_string()
                } else {
                    format!("{} → {}", contribution.item.permission, roles.join(", "))
                }
            })
            .collect();
        section("permissions", &permissions);
        let events: Vec<String> = catalog
            .public_event_contributions()
            .iter()
            .filter(|contribution| contribution.module == key)
            .map(|contribution| contribution.item.event_type.to_owned())
            .collect();
        section("public events", &events);
        let tags: Vec<String> = catalog
            .tag_contributions()
            .iter()
            .filter(|contribution| contribution.module == key)
            .map(|contribution| contribution.item.name.to_owned())
            .collect();
        section("openapi tags", &tags);

        let parts: WorkerParts = app.worker_parts_of(key, &env);
        let jobs: Vec<String> = parts
            .handlers
            .iter()
            .map(|handler| handler.kind().to_owned())
            .collect();
        let consumers: Vec<String> = parts
            .consumers
            .iter()
            .map(|consumer| consumer.name().to_owned())
            .collect();
        let schedules: Vec<String> = parts
            .schedules
            .iter()
            .map(|schedule| format!("{} ({})", schedule.kind, schedule.cron.expression()))
            .collect();
        for job in &jobs {
            job_owners.entry(job.clone()).or_default().push(key);
        }
        for consumer in &consumers {
            consumer_owners
                .entry(consumer.clone())
                .or_default()
                .push(key);
        }
        for schedule in &parts.schedules {
            schedule_owners
                .entry(schedule.kind.clone())
                .or_default()
                .push(key);
        }
        section("jobs", &jobs);
        section("consumers", &consumers);
        section("schedules", &schedules);
    }

    let mut conflicts: Vec<String> = Vec::new();
    for service in app.services().describe() {
        if !service.replaced.is_empty() {
            conflicts.push(format!(
                "service {} provided by {} replaced {}",
                service.type_name,
                service.module,
                service.replaced.join(", ")
            ));
        }
        if !service.decorators.is_empty() {
            conflicts.push(format!(
                "service {} provided by {} decorated by {}",
                service.type_name,
                service.module,
                service.decorators.join(", ")
            ));
        }
    }
    for conflict in app.catalog().conflicts() {
        conflicts.push(format!(
            "{} {} contributed by {} (the last one wins)",
            conflict.catalog,
            conflict.key,
            conflict.modules.join(", ")
        ));
    }
    for (label, owners) in [
        ("job", &job_owners),
        ("consumer", &consumer_owners),
        ("schedule", &schedule_owners),
    ] {
        for (name, modules) in owners {
            if modules.len() > 1 {
                conflicts.push(format!(
                    "{label} {name} registered {} times by {} (the last one wins)",
                    modules.len(),
                    modules.join(", ")
                ));
            }
        }
    }
    println!();
    if conflicts.is_empty() {
        println!("conflicts: none");
    } else {
        println!("conflicts ({}):", conflicts.len());
        for conflict in conflicts {
            println!("  {conflict}");
        }
    }
    Ok(())
}

async fn platform_state(config: &AppConfig, pool: &Db) -> Result<AppState, AppError> {
    let secrets = build_secret_vault(pool, &config.encryption)?;
    let state = AppState::new(Some(pool.clone()))
        .with_secrets(secrets)
        .with_outbound_http(config.outbound_http.clone())
        .with_mysql_platform()
        .with_organization_storage();
    aphel_kernel::state::install_runtime_state(state.clone());
    Ok(state)
}

async fn uninstall(config: &AppConfig, app: &App, args: UninstallArgs) -> CliResult {
    let entry = app
        .module(&args.key)
        .ok_or_else(|| AppError::Configuration(format!("unknown module {}", args.key)))?;
    if entry.manifest.required {
        return Err(AppError::Configuration(format!(
            "the module {} is required by this build and cannot be uninstalled",
            entry.key()
        ))
        .into());
    }
    let database_url = config
        .database_url
        .as_deref()
        .ok_or_else(|| AppError::Configuration("DATABASE_URL is not configured".to_owned()))?;
    let pool = aphel_kernel::database::pool::connect(database_url).await?;
    app.enabled().refresh(&pool).await?;
    let state = platform_state(config, &pool).await?;
    let use_cases = module_use_cases(&state)?;
    let scope = AccessScope::system();
    use_cases.disable(&scope, entry.key()).await?;
    println!("module {} disabled", entry.key());
    entry
        .module
        .uninstall(&mut ModuleCtx {
            pool: &pool,
            scope: &scope,
            services: app.services(),
            module: entry.key(),
        })
        .await?;
    if !args.purge {
        println!(
            "data kept; run `aphel modules uninstall {} --purge` to remove it",
            entry.key()
        );
        return Ok(());
    }
    let export = use_cases.export_data(&scope, entry.key()).await?;
    let file = format!(
        "aphel-module-{}-{}.json",
        entry.key(),
        Utc::now().format("%Y%m%d%H%M%S")
    );
    let body = serde_json::to_vec_pretty(&export.data)
        .map_err(|error| AppError::Internal(format!("could not serialize the export: {error}")))?;
    std::fs::write(&file, body)?;
    println!("data exported to {file}");
    use_cases
        .purge_data(&scope, entry.key(), entry.key())
        .await?;
    println!("data of {} removed", entry.key());
    if aphel_kernel::modules::migrations::revert(&pool, entry).await? {
        println!("schema of {} reverted", entry.key());
    }
    Ok(())
}
