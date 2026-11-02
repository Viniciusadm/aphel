use serde::Serialize;
use serde_json::{Map, Value, json};
use uuid::Uuid;

use crate::domain::custom::{
    CSV_DELIMITER, CreateReportCommand, MAX_REPORT_DESCRIPTION_LENGTH, MAX_REPORT_NAME_LENGTH,
    MAX_REPORT_SETTINGS_BYTES, REPORT_CREATED, REPORT_DELETED, REPORT_RECORD_TYPE, REPORT_UPDATED,
    ReportEntry, ReportExport, RunOverrides, SavedReport, UpdateReportCommand, authorize_edit,
    is_report_visible, prefixed, report_not_found, with_overrides,
};
use crate::store::ReportStore;
use crate::table::report_table;
use aphel_kernel::Ctx;
use aphel_kernel::application::access::{Authorizer, PERMISSION_DENIED};
use aphel_kernel::application::audit::{DiffOptions, diff_changes};
use aphel_kernel::application::fields::EntityDefinition;
use aphel_kernel::application::fields::permissions::query_error;
use aphel_kernel::application::query::{AggregateEngine, AggregateQuery, AggregateResult};
use aphel_kernel::application::shared::{FieldChange, RecordRef, RecordType, ensure_version};
use aphel_kernel::application::views::ViewVisibility;
use aphel_kernel::database::transaction::MySqlTx;
use aphel_kernel::domain::TeamId;
use aphel_kernel::domain::access::{AccessScope, Permission};
use aphel_kernel::errors::{AppError, FieldViolation};
use aphel_kernel::records::{RecordMutation, lock_version, publish_mutation};
use exports::{CSV_MIME_TYPE, ExportFormat, XLSX_MIME_TYPE, encode_csv, encode_xlsx};

#[derive(Serialize)]
struct ReportSnapshot<'a> {
    name: &'a str,
    description: Option<&'a str>,
    visibility: &'static str,
    team_id: Option<&'a str>,
    visualization: &'static str,
    definition: &'a AggregateQuery,
    settings: &'a Map<String, Value>,
}

impl<'a> From<&'a SavedReport> for ReportSnapshot<'a> {
    fn from(report: &'a SavedReport) -> Self {
        Self {
            name: &report.name,
            description: report.description.as_deref(),
            visibility: report.visibility.as_str(),
            team_id: report.team_id.as_ref().map(TeamId::as_str),
            visualization: report.visualization.as_str(),
            definition: &report.definition,
            settings: &report.settings,
        }
    }
}

pub(crate) async fn list_reports(
    ctx: &Ctx<'_>,
    record_type: Option<&str>,
) -> Result<Vec<ReportEntry>, AppError> {
    let wanted = match record_type.map(str::trim).filter(|value| !value.is_empty()) {
        Some(record_type) => Some(readable(ctx, record_type)?.record_type.clone()),
        None => None,
    };
    Ok(ReportStore::of(ctx)
        .list_visible(ctx.scope)
        .await?
        .into_iter()
        .filter(|report| is_report_visible(ctx.scope, report))
        .filter(|report| can_read_type(ctx, ctx.scope, &report.record_type))
        .filter(|report| {
            wanted
                .as_ref()
                .is_none_or(|wanted| &report.record_type == wanted)
        })
        .map(|report| entry(ctx.scope, report))
        .collect())
}

pub(crate) async fn get_report(ctx: &Ctx<'_>, report_id: &str) -> Result<ReportEntry, AppError> {
    let report = require_report(ctx, report_id).await?;
    Ok(entry(ctx.scope, report))
}

pub(crate) async fn create_report(
    ctx: &Ctx<'_>,
    command: CreateReportCommand,
) -> Result<ReportEntry, AppError> {
    let definition = readable(ctx, &command.definition.record_type)?;
    let record_type = definition.record_type.clone();
    let now = ctx.now();
    let mut violations = Vec::new();
    let visibility = parse_visibility(&command.visibility, &mut violations);
    let team_id = parse_team(command.team_id.as_deref(), &mut violations);
    let settings = settings_object(&command.settings, &mut violations);
    let visualization = command.visualization;
    let (Some(visibility), Some(settings)) = (visibility, settings) else {
        return Err(query_error("invalid report", violations));
    };
    let mut query = command.definition;
    query.record_type = record_type.as_str().to_owned();
    let report = SavedReport {
        id: Uuid::now_v7(),
        record_type,
        name: command.name.trim().to_owned(),
        description: normalize_description(command.description),
        owner_id: Some(ctx.scope.user_id().clone()),
        visibility,
        team_id,
        visualization,
        definition: query,
        settings,
        created_at: now,
        updated_at: now,
    };
    violations.extend(report_violations(ctx, &report).await?);
    if !violations.is_empty() {
        return Err(query_error("invalid report", violations));
    }
    authorize_audience(ctx, &report)?;
    validate_definition(ctx, &report.definition).await?;

    let changes = diff_changes(
        &Value::Null,
        &ReportSnapshot::from(&report),
        &DiffOptions::default(),
    )?;
    let mut tx = ctx.begin().await?;
    ReportStore::of(ctx).insert(&mut tx, &report).await?;
    record(ctx, &mut tx, REPORT_CREATED, &report, changes, command.ip).await?;
    tx.commit().await?;
    Ok(entry(ctx.scope, report))
}

pub(crate) async fn update_report(
    ctx: &Ctx<'_>,
    report_id: &str,
    command: UpdateReportCommand,
) -> Result<ReportEntry, AppError> {
    let current = require_report(ctx, report_id).await?;
    authorize_edit(ctx.scope, &current)?;
    ensure_version(command.expected_updated_at, current.updated_at)?;

    let mut updated = current.clone();
    let mut violations = Vec::new();
    if let Some(name) = &command.name {
        updated.name = name.trim().to_owned();
    }
    if let Some(description) = command.description {
        updated.description = normalize_description(description);
    }
    if let Some(visibility) = &command.visibility
        && let Some(visibility) = parse_visibility(visibility, &mut violations)
    {
        updated.visibility = visibility;
    }
    if let Some(team_id) = &command.team_id {
        updated.team_id = parse_team(team_id.as_deref(), &mut violations);
    }
    if let Some(visualization) = command.visualization {
        updated.visualization = visualization;
    }
    if let Some(settings) = &command.settings
        && let Some(settings) = settings_object(settings, &mut violations)
    {
        updated.settings = settings;
    }
    let definition_changed = command.definition.is_some();
    if let Some(mut definition) = command.definition {
        let record_type = readable(ctx, &definition.record_type)?.record_type.clone();
        definition.record_type = record_type.as_str().to_owned();
        updated.record_type = record_type;
        updated.definition = definition;
    }
    violations.extend(report_violations(ctx, &updated).await?);
    if !violations.is_empty() {
        return Err(query_error("invalid report", violations));
    }
    authorize_audience(ctx, &updated)?;
    if definition_changed {
        validate_definition(ctx, &updated.definition).await?;
    }

    let changes = diff_changes(
        &ReportSnapshot::from(&current),
        &ReportSnapshot::from(&updated),
        &DiffOptions::default(),
    )?;
    if changes.is_empty() {
        return Ok(entry(ctx.scope, current));
    }
    updated.updated_at = ctx.now();
    let mut tx = ctx.begin().await?;
    lock_version(&mut tx, "reports", current.id, command.expected_updated_at).await?;
    ReportStore::of(ctx).update(&mut tx, &updated).await?;
    record(ctx, &mut tx, REPORT_UPDATED, &updated, changes, command.ip).await?;
    tx.commit().await?;
    Ok(entry(ctx.scope, updated))
}

pub(crate) async fn delete_report(
    ctx: &Ctx<'_>,
    report_id: &str,
    ip: Option<String>,
) -> Result<(), AppError> {
    let current = require_report(ctx, report_id).await?;
    authorize_edit(ctx.scope, &current)?;
    let changes = diff_changes(
        &ReportSnapshot::from(&current),
        &Value::Null,
        &DiffOptions::default(),
    )?;
    let mut tx = ctx.begin().await?;
    ReportStore::of(ctx).delete(&mut tx, current.id).await?;
    record(ctx, &mut tx, REPORT_DELETED, &current, changes, ip).await?;
    tx.commit().await?;
    Ok(())
}

pub(crate) async fn run_report(
    ctx: &Ctx<'_>,
    report_id: &str,
    overrides: RunOverrides,
) -> Result<(SavedReport, AggregateResult), AppError> {
    let report = require_report(ctx, report_id).await?;
    let query = with_overrides(&report.definition, &overrides)?;
    let result = AggregateEngine::new(&ctx.query())
        .run(ctx.scope, query)
        .await?;
    Ok((report, result))
}

pub(crate) async fn export_report(
    ctx: &Ctx<'_>,
    report_id: &str,
    format: &str,
    overrides: RunOverrides,
) -> Result<ReportExport, AppError> {
    let format = ExportFormat::parse(format.trim()).ok_or_else(|| {
        AppError::invalid_fields(
            "invalid export".to_owned(),
            vec![FieldViolation::new("format", "must be csv or xlsx")],
        )
    })?;
    let (report, result) = run_report(ctx, report_id, overrides).await?;
    encode_report(&report, &result, format)
}

pub(crate) fn encode_report(
    report: &SavedReport,
    result: &AggregateResult,
    format: ExportFormat,
) -> Result<ReportExport, AppError> {
    let table = report_table(result);
    let mut rows: Vec<Vec<_>> = table
        .rows
        .iter()
        .map(|row| row.iter().map(|cell| cell.to_export()).collect())
        .collect();
    if !result.groups.is_empty() {
        rows.push(table.totals.iter().map(|cell| cell.to_export()).collect());
    }
    let (content, mime_type) = match format {
        ExportFormat::Csv => (
            encode_csv(&table.headers, &rows, CSV_DELIMITER)?,
            CSV_MIME_TYPE,
        ),
        ExportFormat::Xlsx => (encode_xlsx(&table.headers, &rows)?, XLSX_MIME_TYPE),
    };
    Ok(ReportExport {
        file_name: format!("{}.{}", file_stem(&report.name), format.as_str()),
        mime_type,
        content,
    })
}

pub(crate) fn readable<'c>(
    ctx: &'c Ctx<'_>,
    record_type: &str,
) -> Result<&'c EntityDefinition, AppError> {
    let definition = ctx.registry().require(record_type)?;
    ctx.scope.require(&definition.read_permission)?;
    Ok(definition)
}

pub(crate) fn can_read_type(ctx: &Ctx<'_>, scope: &AccessScope, record_type: &RecordType) -> bool {
    ctx.registry()
        .get(record_type)
        .is_some_and(|definition| scope.filter_for(&definition.read_permission).is_some())
}

pub(crate) async fn require_report(
    ctx: &Ctx<'_>,
    report_id: &str,
) -> Result<SavedReport, AppError> {
    let store = ReportStore::of(ctx);
    let report = Uuid::parse_str(report_id.trim())
        .ok()
        .map(|id| store.find(id))
        .ok_or_else(report_not_found)?
        .await?
        .filter(|report| is_report_visible(ctx.scope, report))
        .ok_or_else(report_not_found)?;
    readable(ctx, report.record_type.as_str())?;
    Ok(report)
}

fn entry(scope: &AccessScope, report: SavedReport) -> ReportEntry {
    ReportEntry {
        can_edit: authorize_edit(scope, &report).is_ok(),
        report,
    }
}

fn authorize_audience(ctx: &Ctx<'_>, report: &SavedReport) -> Result<(), AppError> {
    let scope = ctx.scope;
    let manages = scope.require(&Permission::SETTINGS_MANAGE).is_ok();
    let allowed = match report.visibility {
        ViewVisibility::Private => true,
        ViewVisibility::Team => report
            .team_id
            .as_ref()
            .is_some_and(|team| manages || scope.team_ids().contains(team)),
        ViewVisibility::Organization => manages,
    };
    if allowed {
        return Ok(());
    }
    Err(AppError::forbidden_code(
        PERMISSION_DENIED,
        "missing permission to share this report with this audience",
    ))
}

async fn report_violations(
    ctx: &Ctx<'_>,
    report: &SavedReport,
) -> Result<Vec<FieldViolation>, AppError> {
    let mut violations = Vec::new();
    if report.name.is_empty() || report.name.chars().count() > MAX_REPORT_NAME_LENGTH {
        violations.push(FieldViolation::new(
            "name",
            format!("must have between 1 and {MAX_REPORT_NAME_LENGTH} characters"),
        ));
    }
    if report
        .description
        .as_ref()
        .is_some_and(|description| description.chars().count() > MAX_REPORT_DESCRIPTION_LENGTH)
    {
        violations.push(FieldViolation::new(
            "description",
            format!("must have at most {MAX_REPORT_DESCRIPTION_LENGTH} characters"),
        ));
    }
    match (report.visibility, &report.team_id) {
        (ViewVisibility::Team, None) => violations.push(FieldViolation::new(
            "team_id",
            "is required for team reports",
        )),
        (ViewVisibility::Team, Some(team)) => {
            let exists = ReportStore::of(ctx).team_exists(team).await?;
            if !exists {
                violations.push(FieldViolation::new("team_id", "team was not found"));
            }
        }
        (_, Some(_)) => violations.push(FieldViolation::new(
            "team_id",
            "is only accepted by team reports",
        )),
        _ => {}
    }
    Ok(violations)
}

async fn validate_definition(ctx: &Ctx<'_>, definition: &AggregateQuery) -> Result<(), AppError> {
    let mut probe = definition.clone();
    probe.limit = Some(1);
    probe.compare_to_previous = false;
    AggregateEngine::new(&ctx.query())
        .run(ctx.scope, probe)
        .await
        .map(|_| ())
        .map_err(|error| prefixed(error, "definition"))
}

async fn record(
    ctx: &Ctx<'_>,
    tx: &mut MySqlTx,
    action: &'static str,
    report: &SavedReport,
    changes: Vec<FieldChange>,
    ip: Option<String>,
) -> Result<(), AppError> {
    publish_mutation(
        ctx.events().as_ref(),
        ctx.audit().as_ref(),
        tx,
        ctx.scope,
        ctx.now(),
        RecordMutation {
            record: RecordRef::new(REPORT_RECORD_TYPE, report.id),
            event_type: action,
            payload: json!({
                "record_type": report.record_type.as_str(),
                "name": report.name,
                "visibility": report.visibility.as_str(),
                "visualization": report.visualization.as_str(),
            }),
            changes,
            ip,
        },
    )
    .await
}

fn file_stem(name: &str) -> String {
    let stem: String = name
        .chars()
        .map(|character| {
            if character.is_alphanumeric() || matches!(character, '-' | '_' | ' ') {
                character
            } else {
                '_'
            }
        })
        .collect();
    let stem = stem.trim();
    if stem.is_empty() {
        "relatorio".to_owned()
    } else {
        stem.chars().take(100).collect()
    }
}

fn normalize_description(description: Option<String>) -> Option<String> {
    description
        .map(|description| description.trim().to_owned())
        .filter(|description| !description.is_empty())
}

fn parse_visibility(value: &str, violations: &mut Vec<FieldViolation>) -> Option<ViewVisibility> {
    let visibility = ViewVisibility::parse(value.trim());
    if visibility.is_none() {
        violations.push(FieldViolation::new(
            "visibility",
            "must be private, team or organization",
        ));
    }
    visibility
}

fn parse_team(value: Option<&str>, violations: &mut Vec<FieldViolation>) -> Option<TeamId> {
    let value = value.map(str::trim).filter(|value| !value.is_empty())?;
    match Uuid::parse_str(value) {
        Ok(id) => TeamId::new(id.to_string()).ok(),
        Err(_) => {
            violations.push(FieldViolation::new("team_id", "must be a valid id"));
            None
        }
    }
}

fn settings_object(
    value: &Value,
    violations: &mut Vec<FieldViolation>,
) -> Option<Map<String, Value>> {
    match value {
        Value::Null => Some(Map::new()),
        Value::Object(object) if value.to_string().len() <= MAX_REPORT_SETTINGS_BYTES => {
            Some(object.clone())
        }
        Value::Object(_) => {
            violations.push(FieldViolation::new(
                "settings",
                format!("must have at most {MAX_REPORT_SETTINGS_BYTES} bytes"),
            ));
            None
        }
        _ => {
            violations.push(FieldViolation::new("settings", "must be an object"));
            None
        }
    }
}
