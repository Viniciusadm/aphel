use std::sync::Arc;

use clap::{Args, FromArgMatches, Subcommand};
use tracing::info;

use crate::application::{ReindexOutcome, SearchUseCases};
use crate::infrastructure::MySqlSearchPorts;
use aphel_kernel::config::AppConfig;
use aphel_kernel::errors::AppError;
use aphel_kernel::modules::CommandResult as CliResult;
use aphel_sdk::platform::application::custom_objects::RecordTypeCatalog;
use aphel_sdk::platform::application::fields::RecordTypeRegistry;
use aphel_sdk::platform::application::shared::RecordType;
use aphel_sdk::platform::domain::custom_objects::is_custom_record_type;
use aphel_sdk::platform::infrastructure::custom_objects::MySqlCustomSchemaSource;

#[derive(Debug, Clone, PartialEq, Eq, Subcommand)]
pub(crate) enum SearchCommand {
    #[command(about = "Rebuild the global search index (all record types unless filtered)")]
    Reindex(SearchReindexArgs),
}

#[derive(Debug, Clone, PartialEq, Eq, Args)]
pub(crate) struct SearchReindexArgs {
    #[arg(long, help = "Record type to reindex, such as contact or deal")]
    pub(crate) record_type: Option<String>,
}

pub(crate) async fn run(config: AppConfig, command: SearchCommand) -> CliResult {
    match command {
        SearchCommand::Reindex(args) => reindex(config, args).await,
    }
}

async fn reindex(config: AppConfig, args: SearchReindexArgs) -> CliResult {
    let database_url = config
        .database_url
        .as_deref()
        .ok_or_else(|| AppError::Configuration("DATABASE_URL is not configured".to_owned()))?;
    let pool = aphel_sdk::platform::modules::connect_with_modules(database_url).await?;
    let catalog = RecordTypeCatalog::new(
        RecordTypeRegistry::builtin(),
        Arc::new(MySqlCustomSchemaSource::new(pool.clone())),
    );
    let record_type = match args.record_type.as_deref() {
        Some(record_type) if is_custom_record_type(record_type) => {
            Some(RecordType::parse(record_type)?)
        }
        Some(record_type) => Some(
            catalog
                .builtin()
                .require(record_type)
                .map(|definition| definition.record_type.clone())?,
        ),
        None => None,
    };
    let registry = catalog.registry().await?;
    if let Some(record_type) = &record_type
        && registry.get(record_type).is_none()
    {
        pool.close().await;
        return Err(AppError::Configuration(format!(
            "record type {:?} was not found",
            record_type.as_str()
        ))
        .into());
    }
    let use_cases = SearchUseCases::new(MySqlSearchPorts::new(pool.clone(), registry));
    let outcome = use_cases
        .reindex_organization(None, record_type.as_ref(), None)
        .await?;
    let (ReindexOutcome::Finished(summary) | ReindexOutcome::Paused(summary, _)) = outcome;
    info!(
        indexed = summary.indexed,
        removed = summary.removed,
        "search index rebuilt"
    );
    pool.close().await;
    Ok(())
}

pub struct SearchCli;
#[async_trait::async_trait]
impl aphel_kernel::modules::ModuleCommand for SearchCli {
    fn command(&self) -> clap::Command {
        <SearchCommand as clap::Subcommand>::augment_subcommands(
            clap::Command::new("search").subcommand_required(true),
        )
    }
    async fn run(&self, config: AppConfig, matches: &clap::ArgMatches) -> CliResult {
        run(config, SearchCommand::from_arg_matches(matches)?).await
    }
}
