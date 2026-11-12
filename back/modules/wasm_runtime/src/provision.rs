use std::collections::{BTreeMap, BTreeSet};

use async_trait::async_trait;
use serde::Serialize;
use serde_json::{Map, Value, json};
use sha2::{Digest, Sha256};
use utoipa::ToSchema;

use crate::pack::{PackManifest, PackPluginError, ProvisionSpec, provision_order};
use crate::store::{self, SavedResource};
use aphel_kernel::Ctx;
use aphel_kernel::errors::{AppError, CodedKind};
use aphel_sdk::platform::api::state::AppState;
use aphel_sdk::platform::domain::access::AccessScope;
use aphel_sdk::platform::modules::builder::APHEL_VERSION;
use aphel_sdk::provisioning::{
    ExistingResource, MAX_SUFFIX_ATTEMPTS, PathConflict, ProvisionAction, ProvisionMode,
    ProvisionOutcome, ProvisionRequest, Provisioner, provisioner, suffixed,
};
use aphel_sdk::records::RecordCtx;

pub const RECORD_KIND: &str = "record";
pub const PACK_APHEL_INCOMPATIBLE: &str = "plugin_pack_aphel_incompatible";
pub const PACK_MODULE_UNAVAILABLE: &str = "plugin_pack_module_unavailable";
pub const PACK_PROVISIONER_UNKNOWN: &str = "plugin_pack_provisioner_unknown";
pub const PACK_PROVISION_REFERENCE: &str = "plugin_pack_provision_reference";
const FAILED: &str = "failed";
const SKIPPED: &str = "skipped";
const CONFLICT_CODES: &[&str] = &[
    "site_path_taken",
    "entity_unique_conflict",
    "plugin_field_unique_conflict",
];

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct ProvisionResultResponse {
    pub key: String,
    pub kind: String,
    #[schema(example = "created")]
    pub status: String,
    pub resource_id: Option<String>,
    pub error: Option<PackPluginError>,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct ProvisionPlanResponse {
    pub key: String,
    pub kind: String,
    #[schema(example = "create")]
    pub action: String,
    pub error: Option<PackPluginError>,
}

pub fn data_hash(spec: &ProvisionSpec) -> String {
    let source = json!({
        "kind": spec.kind,
        "target": spec.target,
        "mode": spec.mode.as_str(),
        "data": spec.data,
        "unique": spec.unique,
    })
    .to_string();
    Sha256::digest(source.as_bytes())
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

fn reference_error(key: &str, message: String) -> AppError {
    AppError::Coded {
        kind: CodedKind::Validation,
        code: PACK_PROVISION_REFERENCE,
        message: format!("{key}: {message}"),
    }
}

fn resolve(key: &str, value: &Value, applied: &BTreeMap<String, Value>) -> Result<Value, AppError> {
    match value {
        Value::Object(map)
            if map.contains_key("provision")
                && map.contains_key("field")
                && map
                    .keys()
                    .all(|name| ["provision", "field", "prefix"].contains(&name.as_str())) =>
        {
            let source = map["provision"].as_str().unwrap_or_default();
            let field = map["field"].as_str().unwrap_or_default();
            let found = applied
                .get(source)
                .and_then(|values| values.get(field))
                .cloned()
                .ok_or_else(|| {
                    reference_error(key, format!("{source}.{field} is not available"))
                })?;
            match (map.get("prefix").and_then(Value::as_str), found) {
                (Some(prefix), Value::String(text)) => Ok(Value::String(format!("{prefix}{text}"))),
                (_, value) => Ok(value),
            }
        }
        Value::Object(map) => Ok(Value::Object(
            map.iter()
                .map(|(name, item)| Ok((name.clone(), resolve(key, item, applied)?)))
                .collect::<Result<Map<String, Value>, AppError>>()?,
        )),
        Value::Array(items) => Ok(Value::Array(
            items
                .iter()
                .map(|item| resolve(key, item, applied))
                .collect::<Result<Vec<_>, _>>()?,
        )),
        other => Ok(other.clone()),
    }
}

fn applied_status(status: &str) -> bool {
    status != FAILED && status != SKIPPED
}

pub struct Requirements {
    pub modules_to_enable: Vec<String>,
}

pub async fn requirements(
    state: &AppState,
    scope: &AccessScope,
    manifest: &PackManifest,
    declares_entities: bool,
) -> Result<Requirements, AppError> {
    if let Some(requirement) = &manifest.requires_aphel {
        let matches = semver::VersionReq::parse(requirement)
            .ok()
            .zip(semver::Version::parse(APHEL_VERSION).ok())
            .is_some_and(|(requirement, version)| requirement.matches(&version));
        if !matches {
            return Err(AppError::conflict_code(
                PACK_APHEL_INCOMPATIBLE,
                format!(
                    "the pack needs Aphel {requirement}; this installation runs {APHEL_VERSION}. Update Aphel before installing it"
                ),
            ));
        }
    }
    let mut required = manifest.requires_modules.clone();
    if declares_entities
        && let Some(module) = crate::schema::object_administration_module(state)
        && !required.iter().any(|key| key == module)
    {
        required.push(module.to_owned());
    }
    if required.is_empty() {
        return Ok(Requirements {
            modules_to_enable: Vec::new(),
        });
    }
    let modules = aphel_sdk::platform::modules::runtime::module_use_cases(state)?
        .list(scope)
        .await?;
    let mut modules_to_enable = Vec::new();
    for key in &required {
        match modules.iter().find(|view| view.descriptor.key() == key) {
            None => {
                return Err(AppError::conflict_code(
                    PACK_MODULE_UNAVAILABLE,
                    format!(
                        "the pack needs the module {key}, which is not part of this Aphel build. Update Aphel before installing it"
                    ),
                ));
            }
            Some(view) if !view.enabled() => modules_to_enable.push(key.clone()),
            Some(_) => {}
        }
    }
    Ok(Requirements { modules_to_enable })
}

pub fn module_contracts(state: &AppState, keys: &[String]) -> Vec<String> {
    let mut contracts: Vec<String> = state
        .app
        .modules()
        .iter()
        .filter(|loaded| keys.iter().any(|key| key == loaded.key()))
        .flat_map(|loaded| loaded.manifest.provides.contracts.clone())
        .collect();
    contracts.extend(
        state
            .app
            .all_contributions::<aphel_sdk::ContractDescriptor>()
            .into_iter()
            .filter(|(provider, _)| keys.iter().any(|key| key == provider))
            .map(|(_, descriptor)| descriptor.contract.requirement()),
    );
    contracts
}

pub async fn enable_modules(
    state: &AppState,
    scope: &AccessScope,
    keys: &[String],
) -> Result<(), AppError> {
    let modules = aphel_sdk::platform::modules::runtime::module_use_cases(state)?;
    for key in keys {
        modules.enable(scope, key).await?;
    }
    Ok(())
}

pub async fn plan(
    state: &AppState,
    scope: &AccessScope,
    manifest: &PackManifest,
    modules_to_enable: &[String],
) -> Result<Vec<ProvisionPlanResponse>, AppError> {
    let ctx = Ctx::from_state(state, scope).await?;
    let existing = store::pack_resources(&state.db()?, &manifest.key).await?;
    let order = provision_order(&manifest.provision).unwrap_or_default();
    Ok(order
        .into_iter()
        .map(|index| {
            let spec = &manifest.provision[index];
            let available = ctx
                .app()
                .all_contributions::<dyn Provisioner>()
                .into_iter()
                .any(|(owner, provider)| {
                    provider.kind() == spec.kind
                        && (ctx.app().is_enabled(owner)
                            || modules_to_enable.iter().any(|module| module == owner))
                });
            let error = (!available).then(|| PackPluginError {
                code: PACK_PROVISIONER_UNKNOWN.to_owned(),
                message: format!("no enabled module provides {}", spec.kind),
                fields: Vec::new(),
            });
            let row = existing
                .get(&spec.key)
                .filter(|row| applied_status(&row.status));
            let action = match (row, spec.mode) {
                (None, _) => "create",
                (Some(_), ProvisionMode::Seed) => "keep",
                (Some(row), ProvisionMode::Managed) if row.data_hash == data_hash(spec) => {
                    "unchanged"
                }
                (Some(_), ProvisionMode::Managed) => "reconcile",
            };
            ProvisionPlanResponse {
                key: spec.key.clone(),
                kind: spec.kind.clone(),
                action: action.to_owned(),
                error,
            }
        })
        .collect())
}

fn saved<'a>(
    manifest: &'a PackManifest,
    spec: &'a ProvisionSpec,
    hash: &'a str,
    status: &'a str,
    resource_id: Option<&'a str>,
    applied: Option<&'a Value>,
    error: Option<Value>,
) -> SavedResource<'a> {
    SavedResource {
        pack: &manifest.key,
        key: &spec.key,
        kind: &spec.kind,
        target: spec.target.as_deref(),
        resource_id,
        mode: spec.mode.as_str(),
        data_hash: hash,
        applied,
        status,
        error,
    }
}

pub async fn run(
    state: &AppState,
    scope: &AccessScope,
    manifest: &PackManifest,
) -> Result<(Vec<ProvisionResultResponse>, bool), AppError> {
    let pool = state.db()?;
    let ctx = Ctx::from_state(state, scope).await?;
    let existing = store::pack_resources(&pool, &manifest.key).await?;
    let order = provision_order(&manifest.provision).map_err(|violations| {
        AppError::invalid_fields_code(
            crate::pack::PACK_INVALID,
            "invalid pack resources",
            violations,
        )
    })?;
    let mut applied: BTreeMap<String, Value> = existing
        .iter()
        .filter(|(_, row)| applied_status(&row.status))
        .map(|(key, row)| (key.clone(), row.applied.clone()))
        .collect();
    let mut unusable: BTreeSet<String> = BTreeSet::new();
    let mut results = Vec::with_capacity(order.len());
    let mut failed = false;
    for index in order {
        let spec = &manifest.provision[index];
        let hash = data_hash(spec);
        if spec
            .depends_on
            .iter()
            .any(|dependency| unusable.contains(dependency))
        {
            unusable.insert(spec.key.clone());
            results.push(ProvisionResultResponse {
                key: spec.key.clone(),
                kind: spec.kind.clone(),
                status: SKIPPED.to_owned(),
                resource_id: None,
                error: None,
            });
            continue;
        }
        let row = existing
            .get(&spec.key)
            .filter(|row| applied_status(&row.status));
        if let Some(row) = row
            && (spec.mode == ProvisionMode::Seed || row.data_hash == hash)
        {
            results.push(ProvisionResultResponse {
                key: spec.key.clone(),
                kind: spec.kind.clone(),
                status: ProvisionAction::Kept.as_str().to_owned(),
                resource_id: row.resource_id.clone(),
                error: None,
            });
            continue;
        }
        let outcome = async {
            let found = provisioner(&ctx, &spec.kind).ok_or_else(|| AppError::Coded {
                kind: CodedKind::Validation,
                code: PACK_PROVISIONER_UNKNOWN,
                message: format!("no enabled module provides {}", spec.kind),
            })?;
            let request = ProvisionRequest {
                pack: manifest.key.clone(),
                key: spec.key.clone(),
                target: spec.target.clone(),
                mode: spec.mode,
                data: resolve(&spec.key, &spec.data, &applied)?,
                unique: spec.unique.clone(),
                conflict: spec.path_conflict,
                existing: row.and_then(|row| {
                    row.resource_id.clone().map(|id| ExistingResource {
                        id,
                        applied: row.applied.clone(),
                    })
                }),
            };
            found.apply(&ctx, &request).await
        }
        .await;
        match outcome {
            Ok(outcome) => {
                store::save_pack_resource(
                    &pool,
                    saved(
                        manifest,
                        spec,
                        &hash,
                        outcome.action.as_str(),
                        Some(&outcome.id),
                        Some(&outcome.applied),
                        None,
                    ),
                )
                .await?;
                applied.insert(spec.key.clone(), outcome.applied.clone());
                results.push(ProvisionResultResponse {
                    key: spec.key.clone(),
                    kind: spec.kind.clone(),
                    status: outcome.action.as_str().to_owned(),
                    resource_id: Some(outcome.id),
                    error: None,
                });
            }
            Err(error) => {
                failed = true;
                unusable.insert(spec.key.clone());
                let problem = PackPluginError::from_error(&error);
                store::save_pack_resource(
                    &pool,
                    saved(
                        manifest,
                        spec,
                        &hash,
                        FAILED,
                        None,
                        None,
                        serde_json::to_value(&problem).ok(),
                    ),
                )
                .await?;
                results.push(ProvisionResultResponse {
                    key: spec.key.clone(),
                    kind: spec.kind.clone(),
                    status: FAILED.to_owned(),
                    resource_id: None,
                    error: Some(problem),
                });
            }
        }
    }
    Ok((results, failed))
}

fn error_code(error: &AppError) -> Option<&'static str> {
    match error {
        AppError::Coded { code, .. }
        | AppError::CodedDetails { code, .. }
        | AppError::CodedFields { code, .. } => Some(code),
        _ => None,
    }
}

pub struct RecordProvisioner;

impl RecordProvisioner {
    fn target(request: &ProvisionRequest) -> Result<&str, AppError> {
        request.target.as_deref().ok_or_else(|| {
            AppError::validation_code(
                crate::pack::PACK_INVALID,
                format!("{}: a record resource needs target", request.key),
            )
        })
    }

    fn with_id(values: &Value, id: &str) -> Value {
        let mut values = values.clone();
        if let Some(map) = values.as_object_mut() {
            map.insert("id".to_owned(), json!(id));
        }
        values
    }
}

#[async_trait]
impl Provisioner for RecordProvisioner {
    fn kind(&self) -> &'static str {
        RECORD_KIND
    }

    async fn apply(
        &self,
        ctx: &Ctx<'_>,
        request: &ProvisionRequest,
    ) -> Result<ProvisionOutcome, AppError> {
        let target = Self::target(request)?;
        let state = aphel_sdk::runtime()?;
        let record_ctx = RecordCtx::from_runtime(ctx.scope, &state);
        let handler = ctx.registry().handler(target)?;
        if let Some(existing) = &request.existing {
            let id = uuid::Uuid::parse_str(&existing.id)
                .map_err(|_| AppError::Internal("invalid provisioned record id".to_owned()))?;
            let Some(current) = handler.load(&record_ctx, id).await? else {
                return Ok(ProvisionOutcome {
                    action: ProvisionAction::Removed,
                    id: existing.id.clone(),
                    applied: existing.applied.clone(),
                });
            };
            let untouched = existing.applied.as_object().is_some_and(|applied| {
                applied
                    .iter()
                    .filter(|(field, _)| field.as_str() != "id")
                    .all(|(field, value)| current.get(field) == Some(value))
            });
            if !untouched {
                return Ok(ProvisionOutcome {
                    action: ProvisionAction::UserChanged,
                    id: existing.id.clone(),
                    applied: existing.applied.clone(),
                });
            }
            handler
                .update(&record_ctx, id, request.data.clone())
                .await?;
            return Ok(ProvisionOutcome {
                action: ProvisionAction::Updated,
                id: existing.id.clone(),
                applied: Self::with_id(&request.data, &existing.id),
            });
        }
        let mut data = request.data.clone();
        let base = request
            .unique
            .as_deref()
            .and_then(|field| data.get(field))
            .and_then(Value::as_str)
            .map(str::to_owned);
        let mut attempt = 1;
        loop {
            match handler.create(&record_ctx, data.clone()).await {
                Ok(created) => {
                    let id = created
                        .get("id")
                        .and_then(Value::as_str)
                        .unwrap_or_default()
                        .to_owned();
                    return Ok(ProvisionOutcome {
                        action: ProvisionAction::Created,
                        applied: Self::with_id(&data, &id),
                        id,
                    });
                }
                Err(error)
                    if request.conflict == PathConflict::Suffix
                        && attempt < MAX_SUFFIX_ATTEMPTS
                        && error_code(&error)
                            .is_some_and(|code| CONFLICT_CODES.contains(&code)) =>
                {
                    let (Some(field), Some(base)) = (request.unique.as_deref(), base.as_deref())
                    else {
                        return Err(error);
                    };
                    attempt += 1;
                    if let Some(map) = data.as_object_mut() {
                        map.insert(field.to_owned(), json!(suffixed(base, attempt)));
                    }
                }
                Err(error) => return Err(error),
            }
        }
    }
}
