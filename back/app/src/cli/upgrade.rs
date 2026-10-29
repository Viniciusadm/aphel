use std::io::{BufRead, IsTerminal, Write};

use chrono::Utc;
use clap::Args;
use uuid::Uuid;

use crate::cli::doctor::{
    connect_quietly, disk_space_checks, migrations_check, mysql_check, mysql_version, storage_check,
};
use crate::cli::{AlreadyReported, CliResult};
use aphel_kernel::application::fields::RecordTypeRegistry;
use aphel_kernel::application::system::doctor::{DoctorCheck, DoctorStatus};
use aphel_kernel::application::system::upgrade::{
    UpgradeOutcome, UpgradeRecord, recovery_instructions,
};
use aphel_kernel::application::system::{APHEL_VERSION, MigrationStatus};
use aphel_kernel::config::AppConfig;
use aphel_kernel::database;
use aphel_kernel::errors::AppError;
use aphel_kernel::infrastructure::crm::sync_crm_defaults;
use aphel_kernel::infrastructure::custom_objects::sync_custom_object_fields;
use aphel_kernel::infrastructure::fields::sync_system_fields;
use aphel_kernel::infrastructure::system::migrations::migration_status;
use aphel_kernel::infrastructure::system::{MySqlUpgradeLog, SystemSettings, hostname};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Args)]
pub(crate) struct UpgradeArgs {
    #[arg(
        long,
        help = "Confirm that a backup of the database and of the files was taken right before this upgrade"
    )]
    pub(crate) backup_confirmed: bool,
    #[arg(long, help = "Only check the requirements and list pending migrations")]
    pub(crate) check: bool,
}

fn print_check(check: &DoctorCheck) {
    println!(
        "  [{}] {}: {}",
        check.status.label(),
        check.name,
        check.detail
    );
}

fn print_plan(status: &MigrationStatus) {
    println!("\nPending migrations:");
    if status.pending_count() == 0 {
        println!("  none");
    }
    for entry in &status.core.pending {
        println!("  core {} {}", entry.version, entry.description);
    }
    for module in status
        .modules
        .iter()
        .filter(|module| module.is_compatible())
    {
        for entry in &module.pending {
            println!(
                "  module {} {} {}",
                module.key, entry.version, entry.description
            );
        }
    }
    println!("\nModules:");
    if status.modules.is_empty() {
        println!("  none compiled in");
    }
    for module in &status.modules {
        let compatibility = match &module.compatibility_detail {
            Some(detail) => format!("{} ({detail})", module.compatibility),
            None => module.compatibility.clone(),
        };
        println!(
            "  {} {} (requires Aphel >= {}): {}{}",
            module.key,
            module.version,
            module.min_aphel_version,
            compatibility,
            if module.is_compatible() {
                String::new()
            } else {
                format!(
                    "; its {} pending migration(s) will be skipped and it stays unavailable",
                    module.pending.len()
                )
            }
        );
    }
}

fn confirm_backup(args: &UpgradeArgs) -> bool {
    if args.backup_confirmed {
        return true;
    }
    if !std::io::stdin().is_terminal() {
        return false;
    }
    print!(
        "\nHave you taken a backup of the database and of the files right before this upgrade (docs/scripts/backup.sh)? Type \"yes\" to continue: "
    );
    let _ = std::io::stdout().flush();
    let mut answer = String::new();
    if std::io::stdin().lock().read_line(&mut answer).is_err() {
        return false;
    }
    matches!(
        answer.trim().to_ascii_lowercase().as_str(),
        "yes" | "y" | "sim" | "s"
    )
}

fn applied_between(before: &MigrationStatus, after: &MigrationStatus) -> Vec<String> {
    let mut applied = Vec::new();
    for entry in &before.core.pending {
        let not_applied = after
            .core
            .pending
            .iter()
            .chain(after.core.failed.iter())
            .any(|still| still.version == entry.version);
        if !not_applied {
            applied.push(format!("core:{}", entry.version));
        }
    }
    for module in &before.modules {
        let remaining = after
            .modules
            .iter()
            .find(|candidate| candidate.key == module.key);
        for entry in &module.pending {
            let still_pending = remaining.is_some_and(|candidate| {
                candidate
                    .pending
                    .iter()
                    .any(|still| still.version == entry.version)
            });
            if !still_pending {
                applied.push(format!("{}:{}", module.key, entry.version));
            }
        }
    }
    applied
}

async fn apply(pool: &sqlx::MySqlPool) -> Result<(), AppError> {
    database::migrations::run(pool).await?;
    sync_system_fields(pool, &RecordTypeRegistry::builtin()).await?;
    sync_custom_object_fields(pool).await?;
    sync_crm_defaults(pool).await?;
    Ok(())
}

pub(crate) async fn run(config: AppConfig, args: UpgradeArgs) -> CliResult {
    println!("Aphel upgrade to version {APHEL_VERSION}\n");
    let Some(database_url) = config.database_url.as_deref() else {
        return Err(abort("DATABASE_URL is not configured"));
    };
    let settings = SystemSettings::from_env();

    println!("Requirements:");
    let pool = match connect_quietly(database_url).await {
        Ok(pool) => {
            print_check(&DoctorCheck::new("database", DoctorStatus::Ok, "connected"));
            pool
        }
        Err(reason) => {
            print_check(&DoctorCheck::new("database", DoctorStatus::Fail, reason));
            return Err(abort("the database is not reachable"));
        }
    };
    let mysql = mysql_check(mysql_version(&pool).await.as_deref());
    print_check(&mysql);
    let storage = storage_check(&config, &pool).await;
    print_check(&storage);
    let disks = disk_space_checks(&pool, settings.min_free_disk_mb).await;
    for disk in &disks {
        print_check(disk);
    }
    let before = match migration_status(&pool).await {
        Ok(status) => status,
        Err(_) => {
            pool.close().await;
            return Err(abort("could not read the migration history"));
        }
    };
    let migrations = migrations_check(&before);
    print_check(&migrations);
    print_plan(&before);

    if mysql.status != DoctorStatus::Ok {
        return Err(abort("the MySQL server does not meet the requirements"));
    }
    if storage.status == DoctorStatus::Fail {
        return Err(abort("the file storage is not writable"));
    }
    if disks.iter().any(|disk| disk.status == DoctorStatus::Fail) {
        return Err(abort(
            "there is not enough free disk space (free some space or lower UPGRADE_MIN_FREE_DISK_MB)",
        ));
    }
    let problems = before.blocking_problems();
    if !problems.is_empty() {
        return Err(abort(&format!(
            "the migration history is inconsistent: {}",
            problems.join("; ")
        )));
    }
    if before.pending_count() == 0 {
        println!("\nThe database is already up to date. Nothing to do.");
        pool.close().await;
        return Ok(());
    }
    if args.check {
        println!("\nCheck only (--check): nothing was changed.");
        pool.close().await;
        return Ok(());
    }
    if !confirm_backup(&args) {
        pool.close().await;
        return Err(abort(
            "a recent backup was not confirmed. Take one with docs/scripts/backup.sh and run again with --backup-confirmed",
        ));
    }

    let log = MySqlUpgradeLog::new(pool.clone());
    let from_version = log.last_version().await.ok().flatten();
    let started_at = Utc::now();
    println!("\nApplying migrations...");
    let result = match database::pool::connect(database_url).await {
        Ok(run_pool) => {
            let result = apply(&run_pool).await;
            run_pool.close().await;
            result
        }
        Err(error) => Err(AppError::from(error)),
    };
    let after = migration_status(&pool)
        .await
        .unwrap_or_else(|_| before.clone());
    let record = UpgradeRecord {
        id: Uuid::now_v7(),
        from_version: from_version.clone(),
        to_version: APHEL_VERSION.to_owned(),
        from_migration: before.core.latest_applied,
        to_migration: after.core.latest_applied,
        outcome: if result.is_ok() {
            UpgradeOutcome::Succeeded
        } else {
            UpgradeOutcome::Failed
        },
        applied_migrations: applied_between(&before, &after),
        error: result.as_ref().err().map(ToString::to_string),
        hostname: hostname(),
        backup_confirmed: true,
        started_at,
        finished_at: Utc::now(),
    };
    let recorded = log.record(&record).await.unwrap_or(false);
    pool.close().await;

    match result {
        Ok(()) => {
            println!(
                "\nUpgrade finished: {} migration(s) applied ({} -> {}).",
                record.applied_migrations.len(),
                from_version.as_deref().unwrap_or("unknown version"),
                APHEL_VERSION
            );
            if !recorded {
                println!("Warning: the upgrade could not be recorded in system_upgrades.");
            }
            println!(
                "Start (or restart) `aphel serve` and `aphel worker`, then check /health/ready."
            );
            Ok(())
        }
        Err(error) => {
            eprintln!("\nUpgrade FAILED: {error}");
            if !record.applied_migrations.is_empty() {
                eprintln!(
                    "Migrations applied before the failure: {}",
                    record.applied_migrations.join(", ")
                );
            }
            eprintln!("\n{}", recovery_instructions());
            Err(Box::new(AlreadyReported))
        }
    }
}

fn abort(reason: &str) -> Box<dyn std::error::Error + Send + Sync> {
    eprintln!("\nUpgrade aborted: {reason}. Nothing was changed.");
    Box::new(AlreadyReported)
}
