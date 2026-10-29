use clap::{Args, Subcommand};
use sqlx::MySql;
use sqlx::migrate::MigrateDatabase;
use sqlx::mysql::MySqlConnectOptions;
use tracing::warn;

use crate::cli::{CliResult, migrate};
use aphel_kernel::config::AppConfig;
use aphel_kernel::errors::AppError;

#[derive(Debug, Clone, PartialEq, Eq, Subcommand)]
pub(crate) enum DbCommand {
    #[command(
        about = "Delete all database data, recreate the database and apply migrations",
        long_about = "Delete the database selected by DATABASE_URL, recreate it and apply all core and compiled module migrations. This permanently deletes the company, users, records and database secrets, making the installer available again. Stop all Aphel servers and workers first. Stored files are not deleted. Requires DROP and CREATE privileges."
    )]
    Reset(ResetArgs),
}

#[derive(Debug, Clone, PartialEq, Eq, Args)]
pub(crate) struct ResetArgs {
    #[arg(
        long,
        value_name = "DATABASE",
        help = "Confirm permanent deletion by supplying the exact database name from DATABASE_URL"
    )]
    pub(crate) confirm: String,
}

pub(crate) async fn run(config: AppConfig, command: DbCommand) -> CliResult {
    match command {
        DbCommand::Reset(args) => reset(config, args).await,
    }
}

async fn reset(config: AppConfig, args: ResetArgs) -> CliResult {
    let database_url = config
        .database_url
        .as_deref()
        .ok_or_else(|| AppError::Configuration("DATABASE_URL is not configured".to_owned()))?;
    let options: MySqlConnectOptions = database_url.parse()?;
    let database = options
        .get_database()
        .filter(|name| !name.is_empty())
        .ok_or_else(|| {
            AppError::Configuration("DATABASE_URL must specify a database name".to_owned())
        })?;
    if database.len() > 64
        || !database.chars().all(|character| {
            character.is_ascii_alphanumeric() || matches!(character, '_' | '-' | '$')
        })
    {
        return Err(AppError::Configuration(
            "database reset requires a name of at most 64 ASCII letters, digits, underscores, hyphens or dollar signs".to_owned(),
        )
        .into());
    }
    if matches!(
        database.to_ascii_lowercase().as_str(),
        "mysql" | "information_schema" | "performance_schema" | "sys"
    ) {
        return Err(AppError::Configuration("system databases cannot be reset".to_owned()).into());
    }
    if args.confirm != database {
        return Err(AppError::Validation(format!(
            "reset refused: --confirm must match the configured database name ({database})"
        ))
        .into());
    }

    warn!(database, "permanently deleting and recreating the database");
    MySql::drop_database(database_url).await?;
    MySql::create_database(database_url).await?;
    migrate::run(config).await?;
    println!("Database reset complete. Start Aphel and open /install to set up the company again.");
    Ok(())
}
