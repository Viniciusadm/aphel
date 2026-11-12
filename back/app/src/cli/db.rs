use clap::{Args, Subcommand};
use serde_json::Map;
use sqlx::MySql;
use sqlx::migrate::MigrateDatabase;
use sqlx::mysql::MySqlConnectOptions;
use tracing::warn;

use crate::cli::{CliResult, migrate};
use aphel_kernel::api::routes::v1::installer::install_organization;
use aphel_kernel::api::state::AppState;
use aphel_kernel::application::installer::InstallCommand;
use aphel_kernel::application::module_admin::missing_required;
use aphel_kernel::config::AppConfig;
use aphel_kernel::database;
use aphel_kernel::domain::Secret;
use aphel_kernel::errors::AppError;
use aphel_kernel::infrastructure::organization_profile::organization_config_catalog;
use aphel_kernel::infrastructure::secrets::{EncryptionKeyring, build_secret_vault};

#[derive(Debug, Clone, PartialEq, Eq, Subcommand)]
pub(crate) enum DbCommand {
    #[command(
        about = "Delete all database data, recreate the database and apply migrations",
        long_about = "Delete the database selected by DATABASE_URL, recreate it and apply all core and compiled module migrations. This permanently deletes the company, users, records and database secrets, making the installer available again. Stop all Aphel servers and workers first. Stored files are not deleted. Requires DROP and CREATE privileges."
    )]
    Reset(ResetArgs),
    #[command(
        about = "Reset the database, then install a default company with a default administrator",
        long_about = "Do everything `db reset` does, then install a company and its first administrator without the web installer. Defaults: company \"Aphel\", administrator admin@aphel.local with password aphel1234. Pass --all-modules to enable every compatible optional module, or --no-modules (the default) to enable only the required ones. Meant for development: stop all Aphel servers and workers first."
    )]
    Fresh(FreshArgs),
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

#[derive(Debug, Clone, PartialEq, Eq, Args)]
pub(crate) struct FreshArgs {
    #[arg(
        long,
        value_name = "DATABASE",
        help = "Confirm permanent deletion by supplying the exact database name from DATABASE_URL"
    )]
    pub(crate) confirm: String,
    #[arg(
        long,
        conflicts_with = "no_modules",
        help = "Enable every compatible optional module"
    )]
    pub(crate) all_modules: bool,
    #[arg(long, help = "Enable only the required modules (default)")]
    pub(crate) no_modules: bool,
    #[arg(long, default_value = "Aphel", help = "Company name")]
    pub(crate) company: String,
    #[arg(long, default_value = "Administrador", help = "Administrator name")]
    pub(crate) admin_name: String,
    #[arg(
        long,
        default_value = "admin@aphel.local",
        help = "Administrator email"
    )]
    pub(crate) email: String,
    #[arg(long, default_value = "aphel1234", help = "Administrator password")]
    pub(crate) password: String,
    #[arg(long, default_value = "America/Sao_Paulo", help = "Time zone")]
    pub(crate) timezone: String,
    #[arg(long, default_value = "BRL", help = "ISO currency code")]
    pub(crate) currency: String,
    #[arg(long, default_value = "pt-BR", help = "Locale (pt-BR or en)")]
    pub(crate) locale: String,
}

pub(crate) async fn run(config: AppConfig, command: DbCommand) -> CliResult {
    match command {
        DbCommand::Reset(args) => reset(config, args).await,
        DbCommand::Fresh(args) => fresh(config, args).await,
    }
}

async fn fresh(config: AppConfig, args: FreshArgs) -> CliResult {
    recreate(&config, &args.confirm).await?;
    let database_url = config
        .database_url
        .as_deref()
        .ok_or_else(|| AppError::Configuration("DATABASE_URL is not configured".to_owned()))?;
    let pool = database::pool::connect(database_url).await?;
    let secrets = build_secret_vault(&pool, &config.encryption)?;
    organization_config_catalog(&pool).refresh().await;
    let state = AppState::new(Some(pool.clone()))
        .with_secrets(secrets)
        .with_encryption(EncryptionKeyring::from_config(&config.encryption)?)
        .with_outbound_http(config.outbound_http.clone())
        .with_mysql_platform()
        .with_organization_storage();
    aphel_kernel::state::install_runtime_state(state.clone());
    let (modules, skipped) = if args.all_modules {
        let (installable, needs_settings): (Vec<_>, Vec<_>) = aphel_kernel::modules::app()
            .modules()
            .iter()
            .filter(|entry| !entry.manifest.required && entry.is_compatible())
            .partition(|entry| {
                missing_required(&entry.manifest.settings, &Map::new(), &[]).is_empty()
            });
        let keys = |entries: Vec<&aphel_kernel::modules::LoadedModule>| -> Vec<String> {
            entries
                .into_iter()
                .map(|entry| entry.key().to_owned())
                .collect()
        };
        (keys(installable), keys(needs_settings))
    } else {
        (Vec::new(), Vec::new())
    };
    let enabled = modules.len();
    let installation = install_organization(
        &state,
        InstallCommand {
            organization_name: args.company.clone(),
            timezone: args.timezone,
            currency: args.currency,
            locale: args.locale,
            admin_name: args.admin_name,
            admin_email: args.email.clone(),
            admin_password: Secret::new(args.password.clone()),
            public_base_url: None,
            business: None,
            ip: None,
            modules,
            module_settings: Default::default(),
        },
    )
    .await?;
    pool.close().await;
    println!(
        "Company \"{}\" installed with {} optional modules enabled.\nLogin: {} / {}",
        args.company, enabled, installation.admin_email, args.password
    );
    if !skipped.is_empty() {
        println!(
            "Skipped (required settings with no default): {}",
            skipped.join(", ")
        );
    }
    Ok(())
}

async fn reset(config: AppConfig, args: ResetArgs) -> CliResult {
    recreate(&config, &args.confirm).await?;
    println!("Database reset complete. Start Aphel and open /install to set up the company again.");
    Ok(())
}

async fn recreate(config: &AppConfig, confirm: &str) -> CliResult {
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
    if confirm != database {
        return Err(AppError::Validation(format!(
            "reset refused: --confirm must match the configured database name ({database})"
        ))
        .into());
    }

    warn!(database, "permanently deleting and recreating the database");
    MySql::drop_database(database_url).await?;
    MySql::create_database(database_url).await?;
    migrate::run(config.clone(), None).await?;
    Ok(())
}
