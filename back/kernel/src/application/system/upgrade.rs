use chrono::{DateTime, Utc};
use uuid::Uuid;

pub const MIN_MYSQL_MAJOR: u64 = 8;

pub fn mysql_version_supported(version: &str) -> bool {
    if version.to_ascii_lowercase().contains("mariadb") {
        return false;
    }
    version
        .split(['.', '-'])
        .next()
        .and_then(|major| major.trim().parse::<u64>().ok())
        .is_some_and(|major| major >= MIN_MYSQL_MAJOR)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UpgradeOutcome {
    Succeeded,
    Failed,
}

impl UpgradeOutcome {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Succeeded => "succeeded",
            Self::Failed => "failed",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UpgradeRecord {
    pub id: Uuid,
    pub from_version: Option<String>,
    pub to_version: String,
    pub from_migration: Option<i64>,
    pub to_migration: Option<i64>,
    pub outcome: UpgradeOutcome,
    pub applied_migrations: Vec<String>,
    pub error: Option<String>,
    pub hostname: Option<String>,
    pub backup_confirmed: bool,
    pub started_at: DateTime<Utc>,
    pub finished_at: DateTime<Utc>,
}

pub fn recovery_instructions() -> &'static str {
    concat!(
        "The upgrade did not finish. MySQL does not roll back schema changes (DDL), ",
        "so the database may be partially migrated.\n",
        "To recover:\n",
        "  1. Stop every Aphel process (serve and worker).\n",
        "  2. Restore the backup taken before the upgrade, for example:\n",
        "       docs/scripts/restore.sh /srv/backups/aphel/aphel-backup-<timestamp>\n",
        "     (Docker: add --compose docker-compose.prod.yml; see docs/backup.md).\n",
        "  3. Put the previous Aphel binary or image back in place and start it.\n",
        "  4. Run `aphel doctor` and attach its report when opening an issue with the error above.",
    )
}
