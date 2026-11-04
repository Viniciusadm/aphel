use std::collections::HashMap;

use serde_json::{Value, json};

use aphel_kernel::Ctx;
use aphel_kernel::errors::{AppError, FieldViolation};
use aphel_sdk::configuration::{
    ApplyCommand, ConflictDecision, ExportCommand, INVALID_PACKAGE, PackageSource, PlanAction,
    configuration_packages,
};

pub(crate) async fn export_config_package(ctx: &Ctx<'_>) -> Result<Value, AppError> {
    let package = configuration_packages(ctx)?
        .export(
            ctx,
            ExportCommand {
                name: Some("Configuração da exportação completa".to_owned()),
                ..ExportCommand::default()
            },
            "pt-BR",
        )
        .await?;
    serde_json::to_value(package).map_err(|error| AppError::Internal(error.to_string()))
}

pub(crate) async fn install_config_package(
    ctx: &Ctx<'_>,
    package: Value,
) -> Result<Value, AppError> {
    let packages = configuration_packages(ctx)?;
    let plan = packages
        .plan(ctx, PackageSource::Package(package.clone()))
        .await?;
    if !plan.valid {
        return Err(AppError::invalid_fields_code(
            INVALID_PACKAGE,
            "the configuration package of the export is not valid here",
            plan.issues
                .into_iter()
                .map(|issue| FieldViolation::new(issue.path, issue.message))
                .collect(),
        ));
    }
    let decisions: HashMap<String, ConflictDecision> = plan
        .items
        .iter()
        .filter(|item| {
            item.action == PlanAction::SkipConflict
                && item
                    .conflict
                    .as_ref()
                    .is_some_and(|conflict| conflict.resolvable)
        })
        .map(|item| (item.id.clone(), ConflictDecision::Apply))
        .collect();
    let unresolved: Vec<String> = plan
        .items
        .iter()
        .filter(|item| item.action == PlanAction::SkipConflict && !decisions.contains_key(&item.id))
        .map(|item| item.id.clone())
        .collect();
    let result = packages
        .apply(
            ctx,
            ApplyCommand {
                source: PackageSource::Package(package),
                decisions,
                fingerprint: plan.fingerprint,
                ip: None,
            },
        )
        .await?;
    Ok(json!({
        "install_id": result.install.id.to_string(),
        "create": result.summary.create,
        "update": result.summary.update,
        "skip_conflict": result.summary.skip_conflict,
        "unchanged": result.summary.unchanged,
        "unresolved_conflicts": unresolved,
    }))
}
