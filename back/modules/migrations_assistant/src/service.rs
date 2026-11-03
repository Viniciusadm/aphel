use aphel_contract_crm::CrmPermissions;
use std::collections::BTreeSet;

use bytes::Bytes;

use crate::history::{self, HistoryReport};
use crate::store::MemberStore;
use crate::{
    NormalizedFile, SourceStep, SourceTool, StepKind, ValueMaps, encode, normalize, source_tool,
};
use aphel_kernel::application::access::Authorizer;
use aphel_kernel::context::Ctx;
use aphel_kernel::domain::access::Permission;
use aphel_kernel::errors::AppError;
use imports::{
    ConfigureImportCommand, CreateImportCommand, DuplicateStrategy, ImportDetails, ImportOptions,
    create_permission, parse_source,
};

pub(crate) const MIGRATION_SOURCE_NOT_FOUND: &str = "migration_source_not_found";
pub(crate) const MIGRATION_HISTORY_STEP: &str = "migration_history_step";

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct MigrationHistory {
    pub(crate) preview: MigrationPreview,
    pub(crate) report: HistoryReport,
}

fn step_permission(step: &SourceStep) -> Option<Permission> {
    match step.kind {
        StepKind::Import => create_permission(step.record_type),
        StepKind::Activities => Some(Permission::ACTIVITIES_CREATE),
        StepKind::Notes => Some(Permission::NOTES_CREATE),
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct MigrationFileCommand {
    pub(crate) source: String,
    pub(crate) step: String,
    pub(crate) file_name: String,
    pub(crate) content: Bytes,
    pub(crate) maps: ValueMaps,
    pub(crate) ip: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct MigrationPreview {
    pub(crate) tool: &'static SourceTool,
    pub(crate) step: &'static SourceStep,
    pub(crate) row_count: u64,
    pub(crate) file: NormalizedFile,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct MigrationImport {
    pub(crate) preview: MigrationPreview,
    pub(crate) import: ImportDetails,
}

fn source_not_found() -> AppError {
    AppError::not_found_code(
        MIGRATION_SOURCE_NOT_FOUND,
        "the source tool or step is not supported",
    )
}

pub(crate) fn resolve_step(
    source: &str,
    step: &str,
) -> Result<(&'static SourceTool, &'static SourceStep), AppError> {
    let tool = source_tool(source.trim()).ok_or_else(source_not_found)?;
    let step = tool.step(step.trim()).ok_or_else(source_not_found)?;
    Ok((tool, step))
}

pub(crate) async fn preview(
    ctx: &Ctx<'_>,
    command: &MigrationFileCommand,
) -> Result<MigrationPreview, AppError> {
    let (tool, step) = resolve_step(&command.source, &command.step)?;
    let permission = step_permission(step).ok_or_else(source_not_found)?;
    ctx.scope.require(&permission)?;
    let parsed = parse_source(&command.content)?;
    let members = MemberStore::of(ctx).active_members().await?;
    let file = normalize(
        tool,
        step,
        &parsed,
        &members,
        &command.maps.clone().normalized(),
    );
    Ok(MigrationPreview {
        tool,
        step,
        row_count: parsed.rows.len() as u64,
        file,
    })
}

pub(crate) async fn create_import(
    ctx: &Ctx<'_>,
    command: MigrationFileCommand,
) -> Result<MigrationImport, AppError> {
    let preview = preview(ctx, &command).await?;
    if preview.step.kind != StepKind::Import {
        return Err(AppError::validation_code(
            MIGRATION_HISTORY_STEP,
            "activities and notes are migrated with POST /migration-assistant/history",
        ));
    }
    let content = encode(&preview.file)?;
    let stem = command
        .file_name
        .rsplit_once('.')
        .map_or(command.file_name.as_str(), |(stem, _)| stem);
    let created = imports::service::create(
        ctx,
        CreateImportCommand {
            record_type: preview.step.record_type.to_owned(),
            file_name: format!("{}-{stem}.csv", preview.tool.key),
            content,
            retry_of: None,
            ip: command.ip.clone(),
        },
    )
    .await?;
    let targets: BTreeSet<&str> = created
        .targets
        .iter()
        .map(|target| target.key.as_str())
        .collect();
    let mut used = BTreeSet::new();
    let mut mapping: Vec<Option<String>> = vec![None; created.job.headers.len()];
    for column in &preview.file.recognized {
        if column.index < mapping.len()
            && targets.contains(column.target.as_str())
            && used.insert(column.target.clone())
        {
            mapping[column.index] = Some(column.target.clone());
        }
    }
    for (index, suggestion) in created.job.mapping.iter().enumerate() {
        if mapping[index].is_some()
            || preview
                .file
                .recognized
                .iter()
                .any(|column| column.index == index)
        {
            continue;
        }
        if let Some(key) = suggestion
            && used.insert(key.clone())
        {
            mapping[index] = Some(key.clone());
        }
    }
    let record_type = preview.step.record_type;
    let duplicate_key = if ctx
        .registry()
        .require(record_type)
        .is_ok_and(|definition| preview.step.duplicate_key.supported_by(definition))
    {
        preview.step.duplicate_key
    } else {
        ImportOptions::default_for(record_type).duplicate_key
    };
    let import = imports::service::configure(
        ctx,
        &created.job.id.to_string(),
        ConfigureImportCommand {
            mapping,
            options: ImportOptions {
                duplicate_key,
                duplicate_strategy: DuplicateStrategy::Skip,
                create_missing_companies: true,
                distribution_rule_id: None,
            },
        },
    )
    .await?;
    Ok(MigrationImport { preview, import })
}

pub(crate) async fn migrate_history(
    ctx: &Ctx<'_>,
    command: MigrationFileCommand,
) -> Result<MigrationHistory, AppError> {
    let preview = preview(ctx, &command).await?;
    if preview.step.kind == StepKind::Import {
        return Err(AppError::validation_code(
            MIGRATION_HISTORY_STEP,
            "organizations, people and deals are migrated with POST /migration-assistant/imports",
        ));
    }
    let module = match preview.step.kind {
        StepKind::Notes => notes::KEY,
        _ => activities::KEY,
    };
    if !ctx.app().is_enabled(module) {
        return Err(source_not_found());
    }
    let report = history::run(ctx, preview.step.kind, &preview.file, command.ip).await?;
    Ok(MigrationHistory { preview, report })
}
