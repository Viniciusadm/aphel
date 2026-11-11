mod catalog;
mod console;
mod db;
mod doctor;
mod entity_schema;
mod healthcheck;
mod migrate;
mod modules;
mod mounts;
mod openapi;
mod probe;
mod secrets;
mod serve;
mod tenancy_check;
mod tenants;
mod upgrade;
mod worker;

use std::error::Error;

use clap::{Args, CommandFactory, FromArgMatches, Parser, Subcommand};
use tracing::{info, warn};
use tracing_subscriber::EnvFilter;

use std::sync::Arc;

use aphel_kernel::config::{AppConfig, MultiTenancyConfig, Tenancy};
use aphel_kernel::errors::AppError;
use aphel_kernel::modules::ModuleCommand;
use aphel_kernel::tenancy::{ControlPlane, TenantRuntime};
pub(crate) use catalog::CatalogArgs;
pub(crate) use console::ConsoleCommand;
pub(crate) use db::DbCommand;
pub(crate) use healthcheck::HealthcheckArgs;
pub(crate) use modules::ModulesCommand;
pub(crate) use mounts::MountsCommand;
pub(crate) use secrets::SecretsCommand;
pub(crate) use tenants::TenantsCommand;
pub(crate) use upgrade::UpgradeArgs;

pub(crate) type CliResult = Result<(), Box<dyn Error + Send + Sync>>;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct AlreadyReported;

impl std::fmt::Display for AlreadyReported {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("the command failed (details above)")
    }
}

impl Error for AlreadyReported {}

#[derive(Debug, Parser)]
#[command(name = "aphel", version, about = "Aphel backend")]
pub(crate) struct Cli {
    #[command(subcommand)]
    pub(crate) command: Option<Command>,
    #[arg(
        long,
        global = true,
        value_name = "SLUG",
        help = "Tenant to act on when CONTROL_DATABASE_URL is set (migrate, doctor, modules, mounts, secrets rotate, search, org, demo)"
    )]
    pub(crate) tenant: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Subcommand)]
pub(crate) enum Command {
    #[command(about = "Start the HTTP server (default when no subcommand is given)")]
    Serve(ServeArgs),
    #[command(about = "Run the background job worker and the job scheduler")]
    Worker,
    #[command(
        about = "Apply pending database migrations and exit (with CONTROL_DATABASE_URL: the control database and every tenant, or only --tenant)"
    )]
    Migrate,
    #[command(subcommand, about = "Maintain the configured database")]
    Db(DbCommand),
    #[command(about = "Print the OpenAPI document as JSON to stdout")]
    Openapi(openapi::OpenapiArgs),
    #[command(
        about = "Generate managed entity DDL from a JSON declaration without connecting to a database"
    )]
    EntitySchema(entity_schema::EntitySchemaArgs),
    #[command(subcommand, about = "Manage the encryption of stored secrets")]
    Secrets(SecretsCommand),
    #[command(about = "Print a sanitized diagnostic report (no secrets) to attach to issues")]
    Doctor,
    #[command(about = "Check requirements, confirm a backup and apply pending migrations")]
    Upgrade(UpgradeArgs),
    #[command(
        about = "Exit 0 when the local HTTP server (or, with --worker, this host's worker) is healthy"
    )]
    Healthcheck(HealthcheckArgs),
    #[command(
        subcommand,
        about = "Inspect and uninstall the modules compiled into this build"
    )]
    Modules(ModulesCommand),
    #[command(subcommand, about = "Manage where the internal app is served")]
    Mounts(MountsCommand),
    #[command(
        subcommand,
        about = "Create, list, suspend, resume, delete and migrate the tenants of a multi-tenant installation"
    )]
    Tenants(TenantsCommand),
    #[command(subcommand, about = "Manage the operators of the multi-tenant console")]
    Console(ConsoleCommand),
    #[command(
        about = "Create two throwaway tenants, check that neither reaches the other's data, then delete them",
        long_about = "Create two throwaway tenants on the control server, sign in to each through the real host routing, and fail when one of them reaches the other's database, stored files, tenant-local state, enabled modules, sessions, records or events, or when state answers outside a tenant scope. Both tenants are deleted at the end, also on failure. Needs CONTROL_DATABASE_URL with CREATE and DROP privileges."
    )]
    TenancyCheck,
    #[command(
        about = "List everything this build offers to plugins: plugins, operations, hooks, events, permissions"
    )]
    Catalog(CatalogArgs),
    #[command(
        about = "Create, build and pack runtime plugins: new, build and pack",
        disable_help_flag = true
    )]
    Plugin(PluginArgs),
    #[command(
        about = "Check the platform invariants of this build: access filters, hook order, registry, operations, event actors (no database)"
    )]
    Probe,
}

#[derive(Debug, Clone, PartialEq, Eq, Args)]
pub(crate) struct PluginArgs {
    #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
    pub(crate) args: Vec<String>,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Args)]
pub(crate) struct ServeArgs {
    #[arg(
        long,
        help = "Also run the job worker and scheduler in the same process"
    )]
    pub(crate) with_worker: bool,
}

impl Cli {
    pub(crate) fn command(&self) -> Command {
        self.command
            .clone()
            .unwrap_or(Command::Serve(ServeArgs::default()))
    }
}

pub(crate) async fn run() -> CliResult {
    let commands = aphel_kernel::modules::app().all_contributions::<dyn ModuleCommand>();
    let mut command = <Cli as CommandFactory>::command();
    for (_, module_command) in &commands {
        command = command.subcommand(module_command.command());
    }
    let matches = command.get_matches();
    if matches.subcommand_name() == Some("probe") {
        return probe::run().await;
    }
    if let Some(("plugin", arguments)) = matches.subcommand() {
        let args = arguments
            .get_many::<String>("args")
            .map(|values| values.cloned().collect::<Vec<_>>())
            .unwrap_or_default();
        return match aphel_plugin::run("aphel plugin", &args) {
            code if code == std::process::ExitCode::SUCCESS => Ok(()),
            _ => Err(Box::new(AlreadyReported)),
        };
    }
    let dotenv_result = dotenvy::dotenv();
    init_tracing();

    match dotenv_result {
        Ok(path) => info!("loaded environment from {}", path.display()),
        Err(err) if err.not_found() => {}
        Err(err) => warn!("failed to load .env file: {err}"),
    }

    let mut config = AppConfig::from_env();
    let tenancy = config.tenancy()?;
    aphel_kernel::tenancy::install_mode((&tenancy).into())?;
    let tenant = matches.get_one::<String>("tenant").cloned();
    let name = matches.subcommand_name().unwrap_or("serve").to_owned();
    let scoped = match (&tenancy, tenant.as_deref()) {
        (Tenancy::Single, Some(_)) => {
            return Err(
                AppError::Configuration("--tenant needs CONTROL_DATABASE_URL".to_owned()).into(),
            );
        }
        (Tenancy::Multi(_), _) if name == "migrate" => None,
        (Tenancy::Multi(multi), Some(slug)) if acts_on_one_tenant(&matches) => {
            Some(open_tenant(&mut config, multi, slug).await?)
        }
        (Tenancy::Multi(_), Some(_)) => {
            return Err(
                AppError::Configuration(format!("--tenant does not apply to `{name}`")).into(),
            );
        }
        (Tenancy::Multi(_), None) if acts_on_one_tenant(&matches) => {
            return Err(AppError::Configuration(format!(
                "`{name}` acts on one tenant: pass --tenant <slug>"
            ))
            .into());
        }
        (Tenancy::Multi(_), None) if matches!(name.as_str(), "db" | "upgrade") => {
            return Err(AppError::Configuration(format!(
                "`{name}` works on a single-company installation; use `aphel tenants` with CONTROL_DATABASE_URL"
            ))
            .into());
        }
        _ => None,
    };
    let dispatched = dispatch(config, &matches, &commands, tenant);
    match scoped {
        Some(runtime) => {
            let result = aphel_kernel::tenancy::scope(runtime.clone(), dispatched).await;
            runtime.stop();
            result
        }
        None => dispatched.await,
    }
}

fn acts_on_one_tenant(matches: &clap::ArgMatches) -> bool {
    match matches.subcommand() {
        Some(("doctor" | "modules" | "mounts" | "search" | "org" | "demo", _)) => true,
        Some(("secrets", arguments)) => arguments.subcommand_name() == Some("rotate"),
        _ => false,
    }
}

async fn open_tenant(
    config: &mut AppConfig,
    multi: &MultiTenancyConfig,
    slug: &str,
) -> Result<Arc<TenantRuntime>, Box<dyn Error + Send + Sync>> {
    let control = ControlPlane::connect(multi).await?;
    let tenant = control.require_tenant(slug).await?;
    let runtime = TenantRuntime::open(&control, config, tenant.clone()).await?;
    config.database_url = Some(control.tenant_database_url(&tenant));
    config.control_database_url = None;
    config.console_host = None;
    control.close().await;
    Ok(runtime)
}

async fn dispatch(
    config: AppConfig,
    matches: &clap::ArgMatches,
    commands: &[(&'static str, Arc<dyn ModuleCommand>)],
    tenant: Option<String>,
) -> CliResult {
    if let Some((name, arguments)) = matches.subcommand()
        && let Some((_, module_command)) = commands
            .iter()
            .find(|(_, module_command)| module_command.command().get_name() == name)
    {
        return module_command.run(config, arguments).await;
    }
    let cli = Cli::from_arg_matches(matches)?;
    match cli.command() {
        Command::Serve(args) => serve::run(config, args).await,
        Command::Worker => worker::run(config).await,
        Command::Migrate => migrate::run(config, tenant.as_deref()).await,
        Command::Db(command) => db::run(config, command).await,
        Command::Openapi(args) => openapi::run(args),
        Command::EntitySchema(args) => entity_schema::run(args),
        Command::Secrets(command) => secrets::run(config, command).await,
        Command::Doctor => doctor::run(config).await,
        Command::Upgrade(args) => upgrade::run(config, args).await,
        Command::Healthcheck(args) => healthcheck::run(config, args).await,
        Command::Modules(command) => modules::run(config, command).await,
        Command::Mounts(command) => mounts::run(config, command).await,
        Command::Tenants(command) => tenants::run(config, command).await,
        Command::Console(command) => console::run(config, command).await,
        Command::TenancyCheck => tenancy_check::run(config).await,
        Command::Catalog(args) => catalog::run(args),
        Command::Probe => unreachable!("the probe runs before configuration is loaded"),
        Command::Plugin(_) => unreachable!("plugin commands run before configuration is loaded"),
    }
}

fn init_tracing() {
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_writer(std::io::stderr)
        .init();
}
