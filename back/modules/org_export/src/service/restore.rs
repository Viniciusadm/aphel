use aphel_contract_crm::CrmPermissions;
use std::collections::{BTreeMap, HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use chrono::{DateTime, SecondsFormat, Utc};
use serde::Serialize;
use serde_json::{Map, Value, json};
use uuid::Uuid;

use sqlx::MySqlPool;

use crate::service::install_config_package;
use crate::store::{InstallationSnapshot, OrgDataStore};
use crate::{
    ArchiveReader, CONFIG_KEYS_FILE, CONFIG_PACKAGE_FILE, ColumnInfo, ColumnRole, ConfigKind,
    DATASETS, DOMAIN_EVENTS_DATASET, DataRow, Dataset, FILES_FILE, MANIFEST_FILE, MEMBERS_FILE,
    Manifest, MemberRow, ORG_EXPORT_FORMAT, ORG_EXPORT_FORMAT_VERSION, ORGANIZATION_NOT_EMPTY,
    ORGANIZATION_RESTORED, RECORD_SHARES_DATASET, TempFile, WEBHOOK_DISABLED_REASON,
    WEBHOOK_ENDPOINTS_DATASET, column_role, dataset, file_stream, invalid_archive, record_priority,
};
use aphel_kernel::Ctx;
use aphel_kernel::application::access::Authorizer;
use aphel_kernel::application::custom_objects::RecordTypeCatalog;
use aphel_kernel::application::fields::{
    CustomFieldValueRepository, EntityDefinition, FieldDefinitionRepository, RecordTypeRegistry,
    raw_from_json,
};
use aphel_kernel::application::files::storage_key;
use aphel_kernel::application::organizations::{
    NewOrganization, NewOwner, organization_registration,
};
use aphel_kernel::application::shared::{
    Actor, AuditEntry, AuditLog, DomainEvent, EventPublisher, FieldChange, RecordRef, RecordType,
    SharedFileStorage,
};
use aphel_kernel::database::transaction::MySqlTx;
use aphel_kernel::domain::access::{AccessScope, Permission, PermissionSet, RoleKey};
use aphel_kernel::domain::fields::{FieldDefinition, FieldType, validate_value};
use aphel_kernel::domain::{Email, NonEmptyString, UserId};
use aphel_kernel::errors::AppError;
use aphel_kernel::infrastructure::fields::{
    MySqlCustomFieldValueRepository, MySqlFieldDefinitionRepository,
};
use aphel_kernel::infrastructure::platform::{SharedAuditLog, SharedEventPublisher};

const ID_CHECK_CHUNK: usize = 500;
const FILES_TABLE: &str = "files";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) enum IdStrategy {
    #[default]
    Auto,
    Preserve,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RestoreCommand {
    pub(crate) archive: PathBuf,
    pub(crate) ids: IdStrategy,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize)]
pub(crate) struct RestoreReport {
    pub(crate) source_organization: String,
    pub(crate) source_aphel_version: String,
    pub(crate) export_id: String,
    pub(crate) ids_preserved: bool,
    pub(crate) config: Value,
    pub(crate) records: BTreeMap<String, u64>,
    pub(crate) datasets: BTreeMap<String, u64>,
    pub(crate) files: u64,
    pub(crate) file_bytes: u64,
    pub(crate) custom_values: u64,
    pub(crate) fallback_config_items: u64,
    pub(crate) mapped_members: Vec<String>,
    pub(crate) unmapped_members: Vec<String>,
    pub(crate) skipped: BTreeMap<String, u64>,
    pub(crate) warnings: BTreeMap<String, u64>,
}

impl RestoreReport {
    fn warn(&mut self, message: impl Into<String>) {
        *self.warnings.entry(message.into()).or_default() += 1;
    }

    fn skip(&mut self, table: &str) {
        *self.skipped.entry(table.to_owned()).or_default() += 1;
    }
}

type DatasetEntry = (&'static Dataset, String);

struct RestoreData<'a> {
    records: &'a [(&'a EntityDefinition, String)],
    datasets: &'a [DatasetEntry],
}

struct RestoreState {
    mapper: IdMapper,
    columns: HashMap<String, Vec<ColumnInfo>>,
    definitions: HashMap<String, Vec<FieldDefinition>>,
    report: RestoreReport,
    stored_keys: Vec<String>,
    self_references: HashMap<String, Vec<String>>,
    deferred: Vec<DeferredReference>,
}

struct DeferredReference {
    table: String,
    column: String,
    id: String,
    value: String,
}

struct IdMapper {
    remap: bool,
    ids: HashMap<Uuid, Uuid>,
    config: HashMap<ConfigKind, HashMap<Uuid, Uuid>>,
    users: HashMap<String, UserId>,
    operator: UserId,
    now: String,
}

fn parse_uuid(value: &Value) -> Option<Uuid> {
    value.as_str().and_then(|text| Uuid::parse_str(text).ok())
}

fn format_instant(at: DateTime<Utc>) -> String {
    at.to_rfc3339_opts(SecondsFormat::Micros, true)
}

impl IdMapper {
    fn data_id(&self, value: &Value) -> Value {
        match parse_uuid(value).and_then(|id| self.ids.get(&id)) {
            Some(new) => Value::String(new.to_string()),
            None => value.clone(),
        }
    }

    fn deep(&self, value: &Value) -> Value {
        if !self.remap {
            return value.clone();
        }
        match value {
            Value::String(_) => self.data_id(value),
            Value::Array(items) => Value::Array(items.iter().map(|item| self.deep(item)).collect()),
            Value::Object(map) => Value::Object(
                map.iter()
                    .map(|(key, item)| (key.clone(), self.deep(item)))
                    .collect(),
            ),
            other => other.clone(),
        }
    }

    fn user(&self, value: &Value) -> Option<Value> {
        value
            .as_str()
            .and_then(|old| self.users.get(old))
            .map(|user| Value::String(user.as_str().to_owned()))
    }

    fn config(&self, kind: ConfigKind, value: &Value) -> Option<Value> {
        parse_uuid(value)
            .and_then(|old| self.config.get(&kind).and_then(|items| items.get(&old)))
            .map(|new| Value::String(new.to_string()))
    }

    fn map_row(
        &self,
        table: &str,
        row: &DataRow,
        columns: &[ColumnInfo],
        report: &mut RestoreReport,
    ) -> Option<DataRow> {
        let mut mapped = Map::new();
        for column in columns.iter().filter(|column| !column.generated) {
            let Some(value) = row.get(&column.name) else {
                continue;
            };
            let role = column_role(table, &column.name);
            let value = if value.is_null() {
                Value::Null
            } else {
                match role {
                    ColumnRole::OwnId => self.data_id(value),
                    ColumnRole::User => match self.user(value) {
                        Some(user) => user,
                        None if column.nullable => Value::Null,
                        None => Value::String(self.operator.as_str().to_owned()),
                    },
                    ColumnRole::Config(kind) => match self.config(kind, value) {
                        Some(id) => id,
                        None if column.nullable => {
                            report.warn(format!(
                                "{table}.{}: {} not found in the destination, left empty",
                                column.name,
                                kind.as_str()
                            ));
                            Value::Null
                        }
                        None => {
                            report.warn(format!(
                                "{table}: rows skipped because the {} does not exist in the destination",
                                kind.as_str()
                            ));
                            report.skip(table);
                            return None;
                        }
                    },
                    ColumnRole::Reference => self.data_id(value),
                    ColumnRole::Clear if column.nullable => Value::Null,
                    ColumnRole::Clear => value.clone(),
                    ColumnRole::Value => self.deep(value),
                }
            };
            mapped.insert(column.name.clone(), value);
        }
        Some(mapped)
    }
}

pub(crate) struct RestoreEnv {
    pool: MySqlPool,
    catalog: Arc<RecordTypeCatalog>,
    storage: SharedFileStorage,
    events: SharedEventPublisher,
    audit: SharedAuditLog,
    data: OrgDataStore,
}

impl RestoreEnv {
    pub(crate) fn new(
        pool: MySqlPool,
        catalog: Arc<RecordTypeCatalog>,
        storage: SharedFileStorage,
        events: SharedEventPublisher,
        audit: SharedAuditLog,
    ) -> Self {
        Self {
            data: OrgDataStore::new(pool.clone()),
            pool,
            catalog,
            storage,
            events,
            audit,
        }
    }

    fn invalidate_caches(&self) {
        self.catalog.invalidate();
    }

    async fn registry(&self) -> Result<Arc<RecordTypeRegistry>, AppError> {
        self.catalog.invalidate();
        self.catalog.registry().await
    }

    fn ctx<'s>(&self, scope: &'s AccessScope) -> Ctx<'s> {
        Ctx::from_parts(
            scope,
            self.pool.clone(),
            self.catalog.builtin(),
            self.events.clone(),
            self.audit.clone(),
        )
        .with_storage(self.storage.clone())
    }
}

struct Restore<'a> {
    env: &'a RestoreEnv,
    ctx: Ctx<'a>,
    definitions: MySqlFieldDefinitionRepository,
    values: MySqlCustomFieldValueRepository,
}

fn read_manifest(reader: &ArchiveReader) -> Result<Manifest, AppError> {
    let manifest: Manifest = serde_json::from_value(reader.json(MANIFEST_FILE)?)
        .map_err(|error| invalid_archive(format!("invalid manifest: {error}")))?;
    if manifest.format != ORG_EXPORT_FORMAT {
        return Err(invalid_archive(format!(
            "{} is not an Aphel organization export",
            manifest.format
        )));
    }
    if manifest.format_version > ORG_EXPORT_FORMAT_VERSION {
        return Err(invalid_archive(format!(
            "format version {} is newer than this Aphel supports ({ORG_EXPORT_FORMAT_VERSION}); upgrade Aphel first",
            manifest.format_version
        )));
    }
    reader.verify(&manifest.entries)?;
    Ok(manifest)
}

fn archive_members(reader: &ArchiveReader) -> Result<Vec<MemberRow>, AppError> {
    reader
        .lines(MEMBERS_FILE)?
        .into_iter()
        .map(|line| {
            serde_json::from_value(Value::Object(line))
                .map_err(|error| invalid_archive(format!("invalid member: {error}")))
        })
        .collect()
}

fn source_keys(
    reader: &ArchiveReader,
) -> Result<HashMap<ConfigKind, HashMap<Uuid, String>>, AppError> {
    let Value::Object(kinds) = reader.json(CONFIG_KEYS_FILE)? else {
        return Err(invalid_archive("config/keys.json must be an object"));
    };
    let mut result = HashMap::new();
    for (kind, items) in kinds {
        let Some(kind) = ConfigKind::parse(&kind) else {
            continue;
        };
        let Value::Object(items) = items else {
            continue;
        };
        let entry: &mut HashMap<Uuid, String> = result.entry(kind).or_default();
        for (id, key) in items {
            if let (Ok(id), Some(key)) = (Uuid::parse_str(&id), key.as_str()) {
                entry.insert(id, key.to_owned());
            }
        }
    }
    Ok(result)
}

pub(crate) fn inspect(archive: &Path) -> Result<Manifest, AppError> {
    read_manifest(&ArchiveReader::open(archive)?)
}

pub(crate) async fn restore_installation(
    env: &RestoreEnv,
    command: RestoreCommand,
) -> Result<RestoreReport, AppError> {
    let reader = ArchiveReader::open(&command.archive)?;
    let manifest = read_manifest(&reader)?;
    let members = archive_members(&reader)?;
    let owner = members
        .iter()
        .find(|member| {
            member.status == "active"
                && member.role == RoleKey::ADMIN
                && member.password_hash.is_some()
        })
        .ok_or_else(|| invalid_archive("the backup has no active administrator who can sign in"))?;
    let now = chrono::Utc::now();
    let owner_id = UserId::new(owner.id.clone())?;
    let registration = organization_registration(
        NewOrganization {
            name: NonEmptyString::new(manifest.organization.name.clone(), "name")?,
            locale: manifest.organization.locale.clone(),
            timezone: manifest.organization.timezone.clone(),
            currency: manifest.organization.currency.clone(),
        },
        NewOwner {
            id: owner_id.clone(),
            email: Email::new(owner.email.clone())?,
            display_name: NonEmptyString::new(owner.display_name.clone(), "display_name")?,
            password_hash: owner.password_hash.clone().unwrap_or_default(),
        },
        now,
    )?;
    let registry = env.registry().await?;
    let snapshot = InstallationSnapshot::capture(&env.pool).await?;
    let mut tx = aphel_kernel::database::transaction::MySqlTx::from(env.pool.begin().await?);
    if !env
        .data
        .create_installation(&mut tx, &registration, &registry)
        .await?
    {
        return Err(AppError::conflict_code(
            ORGANIZATION_NOT_EMPTY,
            "this installation is already set up; a backup is only restored into an empty installation",
        ));
    }
    tx.commit().await?;
    let result = restore_into_installation(env, command, owner_id).await;
    if let Err(error) = &result {
        tracing::error!(error = %error, "the backup restore failed; undoing the partial installation");
        if let Err(rollback) = snapshot.rollback(&env.pool).await {
            tracing::error!(error = %rollback, "failed to undo a partial backup restore");
        }
        env.invalidate_caches();
    }
    result
}

async fn restore_into_installation(
    env: &RestoreEnv,
    command: RestoreCommand,
    owner_id: UserId,
) -> Result<RestoreReport, AppError> {
    let scope = AccessScope::new(
        owner_id,
        RoleKey::admin(),
        Vec::new(),
        PermissionSet::everything(),
    );
    Restore {
        env,
        ctx: env.ctx(&scope),
        definitions: MySqlFieldDefinitionRepository::new(env.pool.clone()),
        values: MySqlCustomFieldValueRepository::new(env.pool.clone()),
    }
    .restore(command)
    .await
}

impl Restore<'_> {
    async fn restore(&self, command: RestoreCommand) -> Result<RestoreReport, AppError> {
        let scope = self.ctx.scope;
        scope.require(&Permission::ORGANIZATION_EXPORT)?;
        scope.require(&Permission::SETTINGS_MANAGE)?;
        let reader = ArchiveReader::open(&command.archive)?;
        let manifest = read_manifest(&reader)?;
        let mut report = RestoreReport {
            source_organization: manifest.organization.name.clone(),
            source_aphel_version: manifest.aphel_version.clone(),
            export_id: manifest.export_id.clone(),
            ..RestoreReport::default()
        };

        let registry = self.env.registry().await?;
        let mut business_tables: Vec<String> = registry
            .definitions()
            .map(|definition| definition.table.clone())
            .collect();
        business_tables.extend(
            ["notes", "attachments", "deal_items"]
                .into_iter()
                .map(str::to_owned),
        );
        business_tables.sort();
        business_tables.dedup();
        if self.env.data.has_business_data(&business_tables).await? {
            return Err(AppError::conflict_code(
                ORGANIZATION_NOT_EMPTY,
                "the installation already has records; a backup is only restored into an empty installation",
            ));
        }

        let package = reader.json(CONFIG_PACKAGE_FILE)?;
        report.config = install_config_package(&self.ctx, package).await?;
        let registry = self.env.registry().await?;

        let restore = &self.env.data;
        let now = self.ctx.now();
        let operator = scope.user_id().clone();

        let mut members_by_email = restore.members_by_email().await?;
        let mut users = HashMap::new();
        let mut created = Vec::new();
        for member in archive_members(&reader)? {
            match members_by_email.remove(&member.email.to_lowercase()) {
                Some(user) => {
                    users.insert(member.id.clone(), user);
                    report.mapped_members.push(member.email.clone());
                }
                None => match UserId::new(member.id.clone()) {
                    Ok(user) if Email::new(member.email.clone()).is_ok() => {
                        users.insert(member.id.clone(), user);
                        created.push(member);
                    }
                    _ => report.unmapped_members.push(member.email.clone()),
                },
            }
        }
        if !created.is_empty() {
            let mut tx = self.ctx.begin().await?;
            for member in &created {
                restore.insert_member(&mut tx, member, now).await?;
                report.mapped_members.push(member.email.clone());
            }
            tx.commit().await?;
        }

        let mut record_files: Vec<(&EntityDefinition, String)> = Vec::new();
        for entry in manifest
            .entries
            .iter()
            .filter(|entry| entry.kind == "records")
        {
            let Some(record_type) = entry.name.as_deref() else {
                continue;
            };
            let Some(definition) = RecordType::parse(record_type)
                .ok()
                .and_then(|record_type| registry.get(&record_type))
            else {
                if entry.count > 0 {
                    report.warn(format!(
                        "{record_type}: record type does not exist in the destination, {} records skipped",
                        entry.count
                    ));
                }
                continue;
            };
            record_files.push((definition, entry.path.clone()));
        }
        record_files.sort_by_key(|(definition, _)| {
            (
                record_priority(definition),
                definition.record_type.as_str().to_owned(),
            )
        });
        let mut datasets: Vec<DatasetEntry> = Vec::new();
        for entry in manifest
            .entries
            .iter()
            .filter(|entry| entry.kind == "dataset")
        {
            let Some(dataset) = entry.name.as_deref().and_then(dataset) else {
                continue;
            };
            if !restore.table_exists(dataset.table).await? {
                if entry.count > 0 {
                    report.warn(format!(
                        "{}: the module that owns this data is not installed in the destination, {} rows skipped",
                        dataset.key, entry.count
                    ));
                }
                continue;
            }
            if entry.count > 0
                && let Some(warning) = dataset.warning
            {
                report.warn(warning);
            }
            datasets.push((dataset, entry.path.clone()));
        }
        datasets.sort_by_key(|(dataset, _)| {
            DATASETS
                .iter()
                .position(|candidate| candidate.key == dataset.key)
                .unwrap_or(usize::MAX)
        });

        let mut owned: BTreeMap<&str, Vec<Uuid>> = BTreeMap::new();
        for (definition, path) in &record_files {
            let ids = owned.entry(definition.table.as_str()).or_default();
            reader.each_row(path, |line| {
                ids.extend(line.get("id").and_then(parse_uuid));
                Ok(())
            })?;
        }
        for (dataset, path) in &datasets {
            let ids = owned.entry(dataset.table).or_default();
            reader.each_row(path, |row| {
                ids.extend(row.get("id").and_then(parse_uuid));
                Ok(())
            })?;
        }
        let file_ids = owned.entry(FILES_TABLE).or_default();
        reader.each_row(FILES_FILE, |row| {
            file_ids.extend(row.get("id").and_then(parse_uuid));
            Ok(())
        })?;
        let mut taken = 0;
        for (table, ids) in &owned {
            for chunk in ids.chunks(ID_CHECK_CHUNK) {
                taken += restore.taken_ids(table, chunk).await?;
            }
        }
        let remap = match command.ids {
            IdStrategy::Auto => taken > 0,
            IdStrategy::Preserve if taken > 0 => {
                return Err(AppError::conflict_code(
                    ORGANIZATION_NOT_EMPTY,
                    format!(
                        "{taken} ids of the archive already exist in this installation; restore with new ids"
                    ),
                ));
            }
            IdStrategy::Preserve => false,
        };
        report.ids_preserved = !remap;
        let mut ids = HashMap::new();
        for id in owned.values().flatten() {
            ids.insert(*id, if remap { Uuid::now_v7() } else { *id });
        }

        let old_keys = source_keys(&reader)?;
        let new_keys = restore.config_keys().await?;
        let mut config: HashMap<ConfigKind, HashMap<Uuid, Uuid>> = HashMap::new();
        for (kind, items) in &old_keys {
            let Some(targets) = new_keys.get(kind) else {
                continue;
            };
            let by_key: HashMap<&str, Uuid> = targets
                .iter()
                .map(|(id, key)| (key.as_str(), *id))
                .collect();
            let entry = config.entry(*kind).or_default();
            for (old, key) in items {
                if let Some(new) = by_key.get(key.as_str()) {
                    entry.insert(*old, *new);
                }
            }
        }

        let mut state = RestoreState {
            mapper: IdMapper {
                remap,
                ids,
                config,
                users,
                operator,
                now: format_instant(now),
            },
            columns: HashMap::new(),
            definitions: HashMap::new(),
            report,
            stored_keys: Vec::new(),
            self_references: HashMap::new(),
            deferred: Vec::new(),
        };
        let result = self
            .write_all(
                &reader,
                &mut state,
                RestoreData {
                    records: &record_files,
                    datasets: &datasets,
                },
            )
            .await;
        if let Err(error) = result {
            for key in &state.stored_keys {
                if let Err(cleanup) = self.env.storage.delete(key).await {
                    tracing::warn!(error = %cleanup, "failed to remove a restored file after an aborted restore");
                }
            }
            return Err(error);
        }
        Ok(state.report)
    }

    async fn write_all(
        &self,
        reader: &ArchiveReader,
        state: &mut RestoreState,
        data: RestoreData<'_>,
    ) -> Result<(), AppError> {
        let scope = self.ctx.scope;
        let restore = &self.env.data;
        let now = self.ctx.now();
        let mut tx = self.ctx.begin().await?;

        for kind in ConfigKind::FALLBACK {
            let table = kind.table();
            let table_columns = self.columns(state, table).await?;
            let used: HashSet<Uuid> = state
                .mapper
                .config
                .get(&kind)
                .map(|items| items.values().copied().collect())
                .unwrap_or_default();
            let Some(mut rows) = reader.rows(&kind.items_path())? else {
                continue;
            };
            while let Some(row) = rows.next_row()? {
                let Some(old) = row.get("id").and_then(parse_uuid) else {
                    continue;
                };
                if state
                    .mapper
                    .config
                    .get(&kind)
                    .is_some_and(|items| items.contains_key(&old))
                {
                    continue;
                }
                let new = if state.mapper.remap
                    || used.contains(&old)
                    || restore.taken_ids(table, &[old]).await? > 0
                {
                    Uuid::now_v7()
                } else {
                    old
                };
                let Some(mut mapped) =
                    state
                        .mapper
                        .map_row(table, &row, &table_columns, &mut state.report)
                else {
                    continue;
                };
                mapped.insert("id".to_owned(), Value::String(new.to_string()));
                if kind == ConfigKind::Pipeline {
                    mapped.insert("is_default".to_owned(), json!(0));
                }
                restore
                    .insert(&mut tx, table, &table_columns, &mapped)
                    .await?;
                state
                    .mapper
                    .config
                    .entry(kind)
                    .or_default()
                    .insert(old, new);
                state.report.fallback_config_items += 1;
            }
        }

        let file_columns = self.columns(state, FILES_TABLE).await?;
        let mut files = reader.rows(FILES_FILE)?;
        while let Some(row) = match files.as_mut() {
            Some(files) => files.next_row()?,
            None => None,
        } {
            let row = &row;
            let Some(old) = row.get("id").and_then(parse_uuid) else {
                continue;
            };
            let new = state.mapper.ids.get(&old).copied().unwrap_or(old);
            let Some(path) = row
                .get("path")
                .and_then(Value::as_str)
                .filter(|path| reader.contains(path))
            else {
                state.report.warn(
                    "files: binary content missing from the archive, attachments to it were not restored",
                );
                state.report.skip(FILES_TABLE);
                continue;
            };
            let temp = TempFile::new("aphel-org-restore", "blob");
            let Some((size, sha256)) = reader.extract(path, temp.path())? else {
                return Err(invalid_archive(format!(
                    "{path} is missing from the archive"
                )));
            };
            if row
                .get("checksum")
                .and_then(Value::as_str)
                .is_some_and(|checksum| checksum != sha256)
            {
                return Err(invalid_archive(format!(
                    "file {old} does not match its checksum"
                )));
            }
            let Some(mut mapped) =
                state
                    .mapper
                    .map_row(FILES_TABLE, row, &file_columns, &mut state.report)
            else {
                continue;
            };
            let key = storage_key(new, now);
            state.stored_keys.push(key.clone());
            self.env
                .storage
                .put_stream(&key, file_stream(temp.path())?)
                .await?;
            drop(temp);
            mapped.insert("storage_key".to_owned(), Value::String(key));
            restore
                .insert(&mut tx, FILES_TABLE, &file_columns, &mapped)
                .await?;
            state.report.files += 1;
            state.report.file_bytes += size;
        }

        for dataset in data
            .datasets
            .iter()
            .filter(|(dataset, _)| dataset.before_records)
        {
            self.restore_dataset(reader, &mut tx, state, dataset)
                .await?;
        }

        for (definition, path) in data.records {
            let table = definition.table.as_str();
            let table_columns = self.columns(state, table).await?;
            let mut count = 0;
            let Some(mut lines) = reader.rows(path)? else {
                continue;
            };
            while let Some(line) = lines.next_row()? {
                let Some(Value::Object(row)) = line.get("row") else {
                    continue;
                };
                let Some(mut mapped) =
                    state
                        .mapper
                        .map_row(table, row, &table_columns, &mut state.report)
                else {
                    continue;
                };
                if let Some(partition) = &definition.partition {
                    mapped.insert(
                        partition.column.clone(),
                        Value::String(partition.value.to_string()),
                    );
                }
                let Some(record_id) = mapped.get("id").and_then(parse_uuid) else {
                    state.report.skip(table);
                    continue;
                };
                self.defer_self_references(state, table, &table_columns, &mut mapped)
                    .await?;
                restore
                    .insert(&mut tx, table, &table_columns, &mapped)
                    .await?;
                count += 1;
                if let Some(Value::Object(custom)) = line.get("custom_fields")
                    && !custom.is_empty()
                {
                    let record = RecordRef::new(definition.record_type.clone(), record_id);
                    let written = self
                        .write_custom_fields(&mut tx, state, &record, custom)
                        .await?;
                    state.report.custom_values += written;
                }
            }
            state
                .report
                .records
                .insert(definition.record_type.as_str().to_owned(), count);
        }

        for dataset in data
            .datasets
            .iter()
            .filter(|(dataset, _)| !dataset.before_records)
        {
            self.restore_dataset(reader, &mut tx, state, dataset)
                .await?;
        }

        for reference in std::mem::take(&mut state.deferred) {
            restore
                .update_reference(
                    &mut tx,
                    &reference.table,
                    &reference.column,
                    &reference.id,
                    &reference.value,
                )
                .await?;
        }

        let record = RecordRef::organization();
        let report = &state.report;
        let summary = json!({
            "source_organization": report.source_organization,
            "export_id": report.export_id,
            "ids_preserved": report.ids_preserved,
            "records": report.records,
            "datasets": report.datasets,
            "files": report.files,
            "unmapped_members": report.unmapped_members.len(),
        });
        self.ctx
            .events()
            .publish(
                &mut tx,
                DomainEvent::new(Actor::from_scope(scope), ORGANIZATION_RESTORED, now)
                    .with_record(record.clone())
                    .with_payload(summary.clone()),
            )
            .await?;
        self.ctx
            .audit()
            .record(
                &mut tx,
                AuditEntry::new(Actor::from_scope(scope), ORGANIZATION_RESTORED, now)
                    .with_record(record)
                    .with_changes(vec![FieldChange::new("restore", Value::Null, summary)]),
            )
            .await?;
        tx.commit().await?;
        Ok(())
    }

    async fn columns(
        &self,
        state: &mut RestoreState,
        table: &str,
    ) -> Result<Vec<ColumnInfo>, AppError> {
        if let Some(columns) = state.columns.get(table) {
            return Ok(columns.clone());
        }
        let columns = self.env.data.columns(table).await?;
        if columns.is_empty() {
            return Err(AppError::Internal(format!("table {table} does not exist")));
        }
        state.columns.insert(table.to_owned(), columns.clone());
        Ok(columns)
    }

    async fn defer_self_references(
        &self,
        state: &mut RestoreState,
        table: &str,
        columns: &[ColumnInfo],
        mapped: &mut DataRow,
    ) -> Result<(), AppError> {
        if !state.self_references.contains_key(table) {
            let references = self.env.data.self_references(table).await?;
            state.self_references.insert(table.to_owned(), references);
        }
        let Some(id) = mapped.get("id").and_then(Value::as_str).map(str::to_owned) else {
            return Ok(());
        };
        for column in state.self_references.get(table).into_iter().flatten() {
            let nullable = columns
                .iter()
                .any(|info| info.name == *column && info.nullable);
            if !nullable {
                continue;
            }
            if let Some(Value::String(value)) = mapped.get(column).cloned() {
                mapped.insert(column.clone(), Value::Null);
                state.deferred.push(DeferredReference {
                    table: table.to_owned(),
                    column: column.clone(),
                    id: id.clone(),
                    value,
                });
            }
        }
        Ok(())
    }

    async fn restore_dataset(
        &self,
        reader: &ArchiveReader,
        tx: &mut MySqlTx,
        state: &mut RestoreState,
        dataset: &DatasetEntry,
    ) -> Result<(), AppError> {
        let (dataset, path) = dataset;
        let key = &dataset.key;
        let table = dataset.table;
        let table_columns = self.columns(state, table).await?;
        let mut count = 0;
        let Some(mut rows) = reader.rows(path)? else {
            return Ok(());
        };
        while let Some(row) = rows.next_row()? {
            let row = &row;
            if dataset.excludes(row) {
                state.report.skip(table);
                continue;
            }
            let Some(mut mapped) =
                state
                    .mapper
                    .map_row(table, row, &table_columns, &mut state.report)
            else {
                continue;
            };
            if *key == RECORD_SHARES_DATASET {
                let grantee = row.get("grantee_id");
                let mapped_grantee = match row.get("grantee_type").and_then(Value::as_str) {
                    Some("user") => grantee.and_then(|grantee| state.mapper.user(grantee)),
                    _ => grantee.map(|grantee| state.mapper.data_id(grantee)),
                };
                let Some(mapped_grantee) = mapped_grantee else {
                    state.report.skip(table);
                    continue;
                };
                mapped.insert("grantee_id".to_owned(), mapped_grantee);
            }
            self.defer_self_references(state, table, &table_columns, &mut mapped)
                .await?;
            if *key == WEBHOOK_ENDPOINTS_DATASET {
                mapped.insert("is_active".to_owned(), json!(0));
                mapped.insert(
                    "disabled_reason".to_owned(),
                    Value::String(WEBHOOK_DISABLED_REASON.to_owned()),
                );
                mapped.insert(
                    "disabled_at".to_owned(),
                    Value::String(state.mapper.now.clone()),
                );
            }
            if *key == DOMAIN_EVENTS_DATASET {
                let actor_type = row.get("actor_type").and_then(Value::as_str);
                let actor = match (actor_type, row.get("actor_id")) {
                    (Some(Actor::USER), Some(actor_id)) => state.mapper.user(actor_id),
                    _ => None,
                };
                match actor {
                    Some(actor) => {
                        mapped.insert("actor_id".to_owned(), actor);
                    }
                    None => {
                        mapped.insert(
                            "actor_type".to_owned(),
                            Value::String(Actor::SYSTEM.to_owned()),
                        );
                        mapped.insert("actor_id".to_owned(), Value::Null);
                    }
                }
                if mapped.get("dispatched_at").is_none_or(Value::is_null) {
                    mapped.insert(
                        "dispatched_at".to_owned(),
                        Value::String(state.mapper.now.clone()),
                    );
                }
            }
            if dataset.upsert {
                self.env
                    .data
                    .upsert(tx, table, &table_columns, &mapped)
                    .await?;
            } else {
                self.env
                    .data
                    .insert(tx, table, &table_columns, &mapped)
                    .await?;
            }
            count += 1;
        }
        state.report.datasets.insert((*key).to_owned(), count);
        Ok(())
    }

    async fn write_custom_fields(
        &self,
        tx: &mut MySqlTx,
        state: &mut RestoreState,
        record: &RecordRef,
        values: &Map<String, Value>,
    ) -> Result<u64, AppError> {
        let record_type = record.record_type.as_str();
        if !state.definitions.contains_key(record_type) {
            let definitions = self.definitions.list(&record.record_type).await?;
            state
                .definitions
                .insert(record_type.to_owned(), definitions);
        }
        let RestoreState {
            mapper,
            definitions,
            report,
            ..
        } = state;
        let definitions = &definitions[record_type];
        let mut written = 0;
        for (key, value) in values {
            if value.is_null() {
                continue;
            }
            let Some(definition) = definitions
                .iter()
                .find(|definition| definition.key.as_str() == key && !definition.is_system)
            else {
                report.warn(format!(
                    "{record_type}.custom_fields.{key}: field does not exist in the destination"
                ));
                continue;
            };
            let value = match definition.field_type.base_type() {
                FieldType::User => match mapper.user(value) {
                    Some(user) => user,
                    None => continue,
                },
                FieldType::Record => mapper.data_id(value),
                _ => value.clone(),
            };
            let Some(raw) = raw_from_json(&value) else {
                report.warn(format!(
                    "{record_type}.custom_fields.{key}: unsupported value"
                ));
                continue;
            };
            match validate_value(definition, &raw) {
                Ok(Some(field_value)) => {
                    self.values
                        .upsert(tx, record, definition, &field_value)
                        .await?;
                    written += 1;
                }
                Ok(None) => {}
                Err(error) => report.warn(format!(
                    "{record_type}.custom_fields.{key}: {}",
                    error.message()
                )),
            }
        }
        Ok(written)
    }
}
