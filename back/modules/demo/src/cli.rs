use std::collections::BTreeMap;
use std::sync::Arc;

use async_trait::async_trait;
use clap::{ArgMatches, Args, Command, FromArgMatches, Subcommand};
use tracing::info;

use crate::service;
use crate::settings::demo_settings;
use crate::{DemoSeedCommand, DemoVisitor};
use aphel_kernel::Ctx;
use aphel_kernel::application::fields::RecordTypeRegistry;
use aphel_kernel::config::AppConfig;
use aphel_kernel::database;
use aphel_kernel::domain::Secret;
use aphel_kernel::domain::access::AccessScope;
use aphel_kernel::errors::AppError;
use aphel_kernel::infrastructure::audit::MySqlAuditLog;
use aphel_kernel::infrastructure::defaults::sync_defaults;
use aphel_kernel::infrastructure::events::MySqlEventPublisher;
use aphel_kernel::infrastructure::fields::sync_system_fields;
use aphel_kernel::infrastructure::organization_profile::organization_config_catalog;
use aphel_kernel::infrastructure::secrets::build_secret_vault;
use aphel_kernel::infrastructure::storage::DynamicFileStorage;
use aphel_kernel::modules::{CommandResult, ModuleCommand, connect_with_modules};

#[derive(Debug, Clone, PartialEq, Eq, Subcommand)]
pub(crate) enum DemoCommand {
    #[command(
        about = "Fill the installation with fictional data (refuses when it already has records, unless --force)"
    )]
    Seed(DemoSeedArgs),
    #[command(about = "Delete the fictional data created by `aphel demo seed`")]
    Remove,
    #[command(
        about = "Delete the demo data and seed it again (with --full, for DEMO_MODE: delete every record, file and non-administrator user first)"
    )]
    Reset(DemoResetArgs),
}

#[derive(Debug, Clone, PartialEq, Eq, Args)]
pub(crate) struct DemoVisitorArgs {
    #[arg(
        long,
        help = "Email of the limited demo visitor (created or reset; defaults to DEMO_USER_EMAIL when DEMO_MODE=true)"
    )]
    pub(crate) visitor_email: Option<String>,
    #[arg(
        long,
        help = "Password of the demo visitor (defaults to DEMO_USER_PASSWORD)"
    )]
    pub(crate) visitor_password: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Args)]
pub(crate) struct DemoSeedArgs {
    #[arg(long, help = "Seed even when the installation already has records")]
    pub(crate) force: bool,
    #[command(flatten)]
    pub(crate) visitor: DemoVisitorArgs,
}

#[derive(Debug, Clone, PartialEq, Eq, Args)]
pub(crate) struct DemoResetArgs {
    #[arg(
        long,
        help = "Delete every record, file and user (except administrators) of the installation, including what visitors created, before seeding again"
    )]
    pub(crate) full: bool,
    #[command(flatten)]
    pub(crate) visitor: DemoVisitorArgs,
}

pub(crate) struct DemoCli;

#[async_trait]
impl ModuleCommand for DemoCli {
    fn command(&self) -> Command {
        DemoCommand::augment_subcommands(
            Command::new("demo")
                .about("Create or remove demonstration data")
                .subcommand_required(true)
                .arg_required_else_help(true),
        )
    }

    async fn run(&self, config: AppConfig, matches: &ArgMatches) -> CommandResult {
        run(config, DemoCommand::from_arg_matches(matches)?).await
    }
}

pub(crate) async fn run(config: AppConfig, command: DemoCommand) -> CommandResult {
    let database_url = config
        .database_url
        .as_deref()
        .ok_or_else(|| AppError::Configuration("DATABASE_URL is not configured".to_owned()))?;
    let pool = connect_with_modules(database_url).await?;
    let registry = RecordTypeRegistry::builtin();
    if config.run_migrations {
        info!("running database migrations");
        database::migrations::run(&pool).await?;
        sync_system_fields(&pool, &registry).await?;
        sync_defaults(&pool).await?;
    }
    let secrets = build_secret_vault(&pool, &config.encryption)?;
    let configuration = organization_config_catalog(&pool);
    configuration.refresh().await;
    let storage = DynamicFileStorage::shared(configuration, secrets.clone());
    let scope = AccessScope::system();
    let ctx = Ctx::from_parts(
        &scope,
        pool.clone(),
        Arc::new(registry),
        Arc::new(MySqlEventPublisher::new()),
        Arc::new(MySqlAuditLog::new()),
    )
    .with_storage(storage.clone())
    .with_secrets(secrets);

    let result = match command {
        DemoCommand::Seed(args) => service::seed(
            &ctx,
            DemoSeedCommand {
                force: args.force,
                visitor: visitor(&args.visitor),
            },
        )
        .await
        .map(|report| {
            println!("Demo data created.");
            print_counts(&report.counts);
            if let Some(visitor) = &report.visitor_email {
                println!("Demo visitor: {visitor}");
            }
        }),
        DemoCommand::Remove => service::remove(&ctx, storage.as_ref()).await.map(|report| {
            println!("Demo data removed.");
            print_counts(&report.counts);
            println!("  stored files: {}", report.removed_files);
        }),
        DemoCommand::Reset(args) => {
            service::reset(&ctx, storage.as_ref(), args.full, visitor(&args.visitor))
                .await
                .map(|report| {
                    println!("Removed:");
                    print_counts(&report.removed);
                    println!("  stored files: {}", report.removed_files);
                    println!("Demo data created:");
                    print_counts(&report.seeded);
                    if let Some(visitor) = &report.visitor_email {
                        println!("Demo visitor: {visitor}");
                    }
                })
        }
    };
    pool.close().await;
    result?;
    Ok(())
}

fn visitor(args: &DemoVisitorArgs) -> Option<DemoVisitor> {
    let settings = demo_settings();
    if args.visitor_email.is_none() && !settings.enabled {
        return None;
    }
    Some(DemoVisitor {
        email: args
            .visitor_email
            .clone()
            .unwrap_or_else(|| settings.visitor_email.clone()),
        password: Secret::new(
            args.visitor_password
                .clone()
                .unwrap_or_else(|| settings.visitor_password.clone()),
        ),
    })
}

fn print_counts(counts: &BTreeMap<String, u64>) {
    if counts.is_empty() {
        println!("  nothing to do");
    }
    for (record_type, total) in counts {
        println!("  {record_type}: {total}");
    }
}
