use std::path::PathBuf;
use std::sync::Arc;

use async_trait::async_trait;
use clap::{ArgMatches, Args, Command, FromArgMatches, Subcommand};
use tracing::info;
use uuid::Uuid;

use crate::service::{inspect, restore_installation};
use crate::store::OrgDataStore;
use crate::{IdStrategy, RestoreCommand, RestoreEnv, build_archive};
use aphel_kernel::Ctx;
use aphel_kernel::application::access::{AccessRepository, build_access_scope};
use aphel_kernel::application::custom_objects::RecordTypeCatalog;
use aphel_kernel::application::fields::RecordTypeRegistry;
use aphel_kernel::application::shared::SharedFileStorage;
use aphel_kernel::config::AppConfig;
use aphel_kernel::domain::access::AccessScope;
use aphel_kernel::errors::AppError;
use aphel_kernel::infrastructure::audit::MySqlAuditLog;
use aphel_kernel::infrastructure::custom_objects::MySqlCustomSchemaSource;
use aphel_kernel::infrastructure::events::MySqlEventPublisher;
use aphel_kernel::infrastructure::organization_profile::organization_config_catalog;
use aphel_kernel::infrastructure::repositories::mysql_access::MySqlAccessRepository;
use aphel_kernel::infrastructure::secrets::build_secret_vault;
use aphel_kernel::infrastructure::storage::DynamicFileStorage;
use aphel_kernel::modules::{CommandResult, ModuleCommand, connect_with_modules};
use aphel_sdk::search::SearchMaintenance;

#[derive(Debug, Clone, PartialEq, Eq, Subcommand)]
pub(crate) enum OrgCommand {
    #[command(
        about = "Write a full backup of the installation (records, relationships, files, configuration, members and manifest) to a zip file"
    )]
    Export(OrgExportArgs),
    #[command(about = "Restore a full backup into an empty installation (not installed yet)")]
    Import(OrgImportArgs),
}

#[derive(Debug, Clone, PartialEq, Eq, Args)]
pub(crate) struct OrgExportArgs {
    #[arg(long, help = "Path of the zip file to write")]
    pub(crate) output: PathBuf,
    #[arg(long, help = "Leave the attached files out of the archive")]
    pub(crate) without_files: bool,
    #[arg(
        long = "as",
        help = "Email of the active user whose access is used (default: the first administrator)"
    )]
    pub(crate) actor: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Args)]
pub(crate) struct OrgImportArgs {
    #[arg(help = "Path of the zip produced by a backup (organization export)")]
    pub(crate) archive: PathBuf,
}

pub(crate) struct OrgCli;

#[async_trait]
impl ModuleCommand for OrgCli {
    fn command(&self) -> Command {
        OrgCommand::augment_subcommands(
            Command::new("org")
                .about("Export an organization or restore an export into an empty organization")
                .subcommand_required(true)
                .arg_required_else_help(true),
        )
    }

    async fn run(&self, config: AppConfig, matches: &ArgMatches) -> CommandResult {
        run(config, OrgCommand::from_arg_matches(matches)?).await
    }
}

pub(crate) async fn run(config: AppConfig, command: OrgCommand) -> CommandResult {
    let database_url = config
        .database_url
        .as_deref()
        .ok_or_else(|| AppError::Configuration("DATABASE_URL is not configured".to_owned()))?;
    let pool = connect_with_modules(database_url).await?;
    let result = match command {
        OrgCommand::Export(args) => export(&config, pool.clone(), args).await,
        OrgCommand::Import(args) => import(&config, pool.clone(), args).await,
    };
    pool.close().await;
    result
}

async fn operator_scope(
    pool: &sqlx::MySqlPool,
    actor: Option<&str>,
) -> Result<AccessScope, AppError> {
    let user_id = OrgDataStore::new(pool.clone())
        .operator(actor)
        .await?
        .ok_or_else(|| {
            AppError::Configuration(match actor {
                Some(email) => format!("{email} is not an active user"),
                None => "the installation has no active administrator".to_owned(),
            })
        })?;
    let access = MySqlAccessRepository::new(pool.clone())
        .load_access(&user_id)
        .await?
        .ok_or_else(|| AppError::Configuration("the operator has no access".to_owned()))?;
    Ok(build_access_scope(user_id, access))
}

fn catalog(pool: &sqlx::MySqlPool) -> Arc<RecordTypeCatalog> {
    Arc::new(RecordTypeCatalog::new(
        RecordTypeRegistry::builtin(),
        Arc::new(MySqlCustomSchemaSource::new(pool.clone())),
    ))
}

async fn file_storage(
    config: &AppConfig,
    pool: &sqlx::MySqlPool,
) -> Result<SharedFileStorage, AppError> {
    let secrets = build_secret_vault(pool, &config.encryption)?;
    let configuration = organization_config_catalog(pool);
    configuration.refresh().await;
    Ok(DynamicFileStorage::shared(configuration, secrets))
}

async fn export(config: &AppConfig, pool: sqlx::MySqlPool, args: OrgExportArgs) -> CommandResult {
    let scope = operator_scope(&pool, args.actor.as_deref()).await?;
    let registry = catalog(&pool).registry().await?;
    let storage = file_storage(config, &pool).await?;
    let ctx = Ctx::from_parts(
        &scope,
        pool.clone(),
        registry,
        Arc::new(MySqlEventPublisher::new()),
        Arc::new(MySqlAuditLog::new()),
    )
    .with_storage(storage.clone());
    let summary = match build_archive(
        &ctx,
        storage.as_ref(),
        Uuid::now_v7(),
        !args.without_files,
        &args.output,
    )
    .await
    {
        Ok(summary) => summary,
        Err(error) => {
            let _ = std::fs::remove_file(&args.output);
            return Err(error.into());
        }
    };
    println!(
        "{}",
        serde_json::to_string_pretty(&summary).unwrap_or_default()
    );
    info!(output = %args.output.display(), bytes = summary.archive_bytes, "installation exported");
    Ok(())
}

async fn import(config: &AppConfig, pool: sqlx::MySqlPool, args: OrgImportArgs) -> CommandResult {
    let archive = args.archive;
    let catalog = catalog(&pool);
    let env = RestoreEnv::new(
        pool.clone(),
        catalog.clone(),
        file_storage(config, &pool).await?,
        Arc::new(MySqlEventPublisher::new()),
        Arc::new(MySqlAuditLog::new()),
    );
    let manifest = inspect(&archive)?;
    info!(
        source = %manifest.organization.name,
        aphel_version = %manifest.aphel_version,
        exported_at = %manifest.exported_at,
        "restoring backup"
    );
    let report = restore_installation(
        &env,
        RestoreCommand {
            archive,
            ids: IdStrategy::Preserve,
        },
    )
    .await?;
    catalog.invalidate();
    let registry = catalog.registry().await?;
    if let Some(search) = aphel_kernel::modules::app()
        .services()
        .get::<dyn SearchMaintenance>()
    {
        let summary = search.rebuild(&pool, &registry).await?;
        info!(indexed = summary.indexed, "search index rebuilt");
    }
    println!(
        "{}",
        serde_json::to_string_pretty(&report).unwrap_or_default()
    );
    Ok(())
}
