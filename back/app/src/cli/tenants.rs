use std::path::PathBuf;

use clap::{Args, Subcommand};

use crate::cli::CliResult;
use aphel_kernel::application::installer::InstallCommand;
use aphel_kernel::config::{AppConfig, MultiTenancyConfig, Tenancy};
use aphel_kernel::domain::Secret;
use aphel_kernel::errors::AppError;
use aphel_kernel::infrastructure::system::read_password;
use aphel_kernel::tenancy::{self, ControlPlane, OperatorActor, TenantRegistration};

#[derive(Debug, Clone, PartialEq, Eq, Subcommand)]
pub(crate) enum TenantsCommand {
    #[command(about = "List the tenants of the control database with their hosts and state")]
    List,
    #[command(
        about = "Create a tenant: register it, create and migrate its database, install the organization and activate it",
        long_about = "Create a tenant: register it as provisioning, create its database on the control server, apply the migrations, install the organization with its first administrator and mark it active. The administrator password is read from the terminal without echo, or from standard input when it is not a terminal. Requires CREATE privileges on the server."
    )]
    Create(Box<CreateArgs>),
    #[command(about = "Stop serving a tenant and its jobs; its data is kept")]
    Suspend(SlugArgs),
    #[command(about = "Serve a suspended tenant again")]
    Resume(SlugArgs),
    #[command(
        about = "Export a tenant, then delete its files, its database and its registration",
        long_about = "Mark the tenant as deleting, write a full export with `aphel org export` (unless --no-export), delete every stored file under the tenant prefix, drop its database and remove it from the control database. This cannot be undone. Requires DROP privileges on the server."
    )]
    Delete(DeleteArgs),
    #[command(
        about = "Apply pending migrations to the control database and to one or every tenant"
    )]
    Migrate(MigrateArgs),
}

#[derive(Debug, Clone, PartialEq, Eq, Args)]
pub(crate) struct SlugArgs {
    #[arg(help = "Tenant slug")]
    pub(crate) slug: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Args)]
pub(crate) struct MigrateArgs {
    #[arg(help = "Tenant slug (default: every active or suspended tenant)")]
    pub(crate) slug: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Args)]
pub(crate) struct CreateArgs {
    #[arg(
        long,
        help = "Short identifier: 3 to 40 lowercase letters, digits or hyphens"
    )]
    pub(crate) slug: String,
    #[arg(long, help = "Organization name")]
    pub(crate) name: String,
    #[arg(
        long = "host",
        required = true,
        help = "Host served for this tenant, without port; repeat for more hosts, the first is the primary"
    )]
    pub(crate) hosts: Vec<String>,
    #[arg(long, help = "Email of the first administrator")]
    pub(crate) admin_email: String,
    #[arg(long, help = "Name of the first administrator")]
    pub(crate) admin_name: String,
    #[arg(long, help = "Blueprint key, as offered by the installer")]
    pub(crate) blueprint: Option<String>,
    #[arg(
        long = "module",
        help = "Optional module to enable; repeat for more modules"
    )]
    pub(crate) modules: Vec<String>,
    #[arg(long, default_value = "America/Sao_Paulo", help = "Time zone")]
    pub(crate) timezone: String,
    #[arg(long, default_value = "BRL", help = "ISO currency code")]
    pub(crate) currency: String,
    #[arg(long, default_value = "pt-BR", help = "Locale (pt-BR or en)")]
    pub(crate) locale: String,
    #[arg(
        long,
        help = "Public address used in email links (default: https:// plus the primary host)"
    )]
    pub(crate) public_url: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Args)]
pub(crate) struct DeleteArgs {
    #[arg(help = "Tenant slug")]
    pub(crate) slug: String,
    #[arg(long, help = "Confirm the permanent deletion by repeating the slug")]
    pub(crate) confirm: String,
    #[arg(
        long,
        conflicts_with = "no_export",
        help = "Path of the export zip to write first"
    )]
    pub(crate) export: Option<PathBuf>,
    #[arg(long, help = "Delete without writing an export")]
    pub(crate) no_export: bool,
}

pub(crate) async fn run(config: AppConfig, command: TenantsCommand) -> CliResult {
    let Tenancy::Multi(multi) = config.tenancy()? else {
        return Err(AppError::Configuration(
            "tenant commands need CONTROL_DATABASE_URL".to_owned(),
        )
        .into());
    };
    let control = ControlPlane::connect(&multi).await?;
    control.migrate().await?;
    let result = match command {
        TenantsCommand::List => list(&control).await,
        TenantsCommand::Create(args) => create(&control, &config, &multi, *args).await,
        TenantsCommand::Suspend(args) => {
            let tenant =
                tenancy::suspend_tenant(&control, &OperatorActor::command_line(), &args.slug)
                    .await?;
            println!("Tenant {} suspended.", tenant.slug);
            Ok(())
        }
        TenantsCommand::Resume(args) => {
            let tenant =
                tenancy::resume_tenant(&control, &OperatorActor::command_line(), &args.slug)
                    .await?;
            println!("Tenant {} resumed.", tenant.slug);
            Ok(())
        }
        TenantsCommand::Delete(args) => delete(&control, &config, args).await,
        TenantsCommand::Migrate(args) => migrate(&control, args.slug.as_deref()).await,
    };
    control.close().await;
    result
}

pub(crate) async fn migrate(control: &ControlPlane, slug: Option<&str>) -> CliResult {
    match slug {
        Some(slug) => {
            control.migrate().await?;
            let tenant = control.require_tenant(slug).await?;
            tenancy::migrate_tenant(control, &tenant).await?;
            println!("Tenant {} migrated.", tenant.slug);
            Ok(())
        }
        None => {
            let summary = tenancy::migrate_all(control).await?;
            println!(
                "Control database migrated; {} tenants migrated, {} failed.",
                summary.migrated, summary.failed
            );
            if summary.failed > 0 {
                return Err(AppError::Persistence(format!(
                    "{} tenants failed to migrate (see `aphel tenants list`)",
                    summary.failed
                ))
                .into());
            }
            Ok(())
        }
    }
}

async fn list(control: &ControlPlane) -> CliResult {
    let tenants = tenancy::list_tenants(control).await?;
    if tenants.is_empty() {
        println!("No tenants.");
        return Ok(());
    }
    for summary in tenants {
        let tenant = &summary.tenant;
        println!(
            "{}  [{}]  {}",
            tenant.slug,
            tenant.status.as_str(),
            tenant.name
        );
        println!(
            "  database: {} ({})",
            tenant.database_name,
            summary.database_bytes.map_or_else(
                || "missing".to_owned(),
                |bytes| format!("{} MB", bytes / 1_048_576)
            )
        );
        let hosts: Vec<String> = summary
            .hosts
            .iter()
            .map(|host| {
                if host.is_primary {
                    format!("{} (primary)", host.host)
                } else {
                    host.host.clone()
                }
            })
            .collect();
        println!(
            "  hosts: {}",
            if hosts.is_empty() {
                "none".to_owned()
            } else {
                hosts.join(", ")
            }
        );
        println!(
            "  migrated: {}",
            summary
                .migrated_at
                .map_or_else(|| "never".to_owned(), |at| at.to_rfc3339())
        );
        if let Some(error) = &summary.last_error {
            println!("  last error: {error}");
        }
    }
    Ok(())
}

async fn create(
    control: &ControlPlane,
    config: &AppConfig,
    multi: &MultiTenancyConfig,
    args: CreateArgs,
) -> CliResult {
    tenancy::validate_slug(&args.slug)?;
    if control.find_tenant(&args.slug).await?.is_some() {
        return Err(AppError::conflict_code(
            "tenant_slug_taken",
            "another tenant already uses this slug",
        )
        .into());
    }
    let password = read_password("Administrator password: ")?;
    if password.chars().count() < 8 {
        return Err(AppError::Validation(
            "the administrator password needs at least 8 characters".to_owned(),
        )
        .into());
    }
    let public_url = args.public_url.clone().or_else(|| {
        args.hosts
            .first()
            .and_then(|host| tenancy::normalize_host(host))
            .map(|host| format!("https://{host}"))
    });
    let tenant = tenancy::create_tenant(
        control,
        config,
        &OperatorActor::command_line(),
        TenantRegistration {
            slug: args.slug,
            name: args.name.clone(),
            hosts: args.hosts,
        },
        InstallCommand {
            organization_name: args.name,
            timezone: args.timezone,
            currency: args.currency,
            locale: args.locale,
            admin_name: args.admin_name,
            admin_email: args.admin_email,
            admin_password: Secret::new(password),
            public_base_url: public_url,
            business: args
                .blueprint
                .map(|blueprint| serde_json::json!({ "business_type": blueprint })),
            ip: None,
            modules: args.modules,
            module_settings: Default::default(),
        },
    )
    .await?;
    println!(
        "Tenant {} created on database {}{}.",
        tenant.slug,
        tenant.database_name,
        if multi.console_host.is_some() {
            "; it is listed in the console"
        } else {
            ""
        }
    );
    Ok(())
}

async fn delete(control: &ControlPlane, config: &AppConfig, args: DeleteArgs) -> CliResult {
    if args.confirm != args.slug {
        return Err(AppError::Validation(
            "deletion refused: --confirm must repeat the tenant slug".to_owned(),
        )
        .into());
    }
    if args.export.is_none() && !args.no_export {
        return Err(AppError::Validation(
            "pass --export <zip> to keep a copy of the tenant, or --no-export to delete without one"
                .to_owned(),
        )
        .into());
    }
    let actor = OperatorActor::command_line();
    let tenant =
        tenancy::begin_deletion(control, &actor, &args.slug, args.export.as_deref()).await?;
    let removed =
        tenancy::delete_tenant(control, config, &actor, &tenant, args.export.as_deref()).await?;
    if let Some(path) = &args.export {
        println!("Tenant {} exported to {}.", tenant.slug, path.display());
    }
    println!(
        "Tenant {} deleted ({removed} stored files removed, database {} dropped).",
        tenant.slug, tenant.database_name
    );
    Ok(())
}
