use serde_json::Value;
use uuid::Uuid;

use crate::application::access::Authorizer;
use crate::application::fields::formulas::{
    Dependencies, FIELD_NOT_ACCESSIBLE, FORMULA_CYCLE, FUNCTIONS, FormulaEngine, FormulaError,
    FormulaNode, FormulaPorts, FunctionSpec, INCOMPATIBLE_RESULT, Span, TypeFormulas, ValueKind,
    compile_formula, find_cycle,
};
use crate::application::fields::{FieldSet, ReferenceChecker, value_to_json};
use crate::application::shared::RecordType;
use crate::domain::access::{AccessScope, Permission};
use crate::domain::fields::{
    FieldConfig, FieldDefinition, FieldKey, FormulaResultType, FormulaSettings, MAX_DECIMALS,
};
use crate::errors::{AppError, FieldViolation};

pub const FORMULA_RECORD_NOT_FOUND: &str = "record_not_found";
pub const PREVIEW_KEY: &str = "formula_preview";

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct PreviewFormulaCommand {
    pub record_type: String,
    pub expression: String,
    pub result_type: String,
    pub decimals: Option<u32>,
    pub record_id: Option<String>,
    pub field_key: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct FormulaPreview {
    pub errors: Vec<FormulaError>,
    pub kind: Option<ValueKind>,
    pub dependencies: Dependencies,
    pub value: Option<Value>,
    pub evaluated: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CatalogField {
    pub key: String,
    pub label: String,
    pub kind: ValueKind,
    pub is_formula: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CatalogRelation {
    pub key: String,
    pub label: String,
    pub target: Option<String>,
    pub fields: Vec<(String, String)>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FormulaCatalog {
    pub record_type: String,
    pub fields: Vec<CatalogField>,
    pub relations: Vec<CatalogRelation>,
    pub functions: Vec<FunctionSpec>,
}

pub struct FormulaUseCases<P> {
    ports: P,
}

fn draft_definition(
    record_type: &str,
    key: FieldKey,
    id: Uuid,
    settings: FormulaSettings,
    decimals: Option<u32>,
) -> FieldDefinition {
    FieldDefinition {
        id,
        record_type: record_type.to_owned(),
        key,
        label: PREVIEW_KEY.to_owned(),
        field_type: settings.result_type.field_type(),
        is_system: false,
        column_name: None,
        is_required: false,
        is_unique: false,
        is_searchable: false,
        is_personal: false,
        is_sensitive: false,
        module_key: None,
        is_active: true,
        position: i32::MAX,
        config: FieldConfig {
            decimals: decimals.map(|decimals| decimals.min(MAX_DECIMALS)),
            formula: Some(settings),
            ..FieldConfig::default()
        },
        options: Vec::new(),
    }
}

impl<P> FormulaUseCases<P>
where
    P: FormulaPorts,
{
    pub fn new(ports: P) -> Self {
        Self { ports }
    }

    pub async fn catalog(
        &self,
        scope: &AccessScope,
        record_type: &str,
    ) -> Result<FormulaCatalog, AppError> {
        scope.require(&Permission::SETTINGS_MANAGE)?;
        let definition = self.ports.registry().require(record_type)?;
        let engine = FormulaEngine::new(&self.ports, scope);
        let context = engine.workspace().context(&definition.record_type).await?;
        Ok(FormulaCatalog {
            record_type: definition.record_type.as_str().to_owned(),
            fields: context
                .fields
                .iter()
                .filter_map(|field| {
                    Some(CatalogField {
                        key: field.key().to_owned(),
                        label: field.definition().label.clone(),
                        kind: field.kind?,
                        is_formula: field.is_formula(),
                    })
                })
                .collect(),
            relations: context
                .relations
                .iter()
                .map(|relation| CatalogRelation {
                    key: relation.key.clone(),
                    label: relation.label.clone(),
                    target: relation
                        .source
                        .target_type()
                        .map(|target| target.as_str().to_owned()),
                    fields: relation
                        .fields
                        .iter()
                        .map(|field| (field.key.clone(), field.label.clone()))
                        .collect(),
                })
                .collect(),
            functions: FUNCTIONS.to_vec(),
        })
    }

    pub async fn preview(
        &self,
        scope: &AccessScope,
        command: PreviewFormulaCommand,
    ) -> Result<FormulaPreview, AppError> {
        scope.require(&Permission::SETTINGS_MANAGE)?;
        let definition = self.ports.registry().require(command.record_type.trim())?;
        let record_type = definition.record_type.clone();
        let Some(result_type) = FormulaResultType::parse(command.result_type.trim()) else {
            return Err(AppError::invalid_fields(
                "invalid formula preview",
                vec![FieldViolation::new(
                    "result_type",
                    "must be one of number, currency, date, text, boolean",
                )],
            ));
        };
        let record_id = match command.record_id.as_deref().map(str::trim) {
            None | Some("") => None,
            Some(id) => Some(Uuid::parse_str(id).map_err(|_| {
                AppError::invalid_fields(
                    "invalid formula preview",
                    vec![FieldViolation::new("record_id", "must be a valid id")],
                )
            })?),
        };

        let engine = FormulaEngine::new(&self.ports, scope);
        let mut workspace = engine.workspace();
        let fields = workspace.field_set(&record_type).await?;
        let existing = command
            .field_key
            .as_deref()
            .and_then(|key| fields.find(key.trim()))
            .filter(|field| field.definition.is_formula());
        let settings = FormulaSettings {
            expression: command.expression.clone(),
            result_type,
        };
        let draft = match existing {
            Some(field) => {
                let mut draft = field.definition.clone();
                draft.field_type = result_type.field_type();
                draft.config.formula = Some(settings);
                if command.decimals.is_some() {
                    draft.config.decimals = command.decimals.map(|value| value.min(MAX_DECIMALS));
                }
                draft
            }
            None => {
                let key = command
                    .field_key
                    .as_deref()
                    .and_then(|key| FieldKey::new(key.trim()).ok())
                    .filter(|key| fields.find(key.as_str()).is_none())
                    .map_or_else(|| FieldKey::new(PREVIEW_KEY), Ok)?;
                draft_definition(
                    record_type.as_str(),
                    key,
                    Uuid::nil(),
                    settings,
                    command.decimals,
                )
            }
        };
        let mut workspace = workspace.with_override(draft.clone());
        let context = workspace.context(&record_type).await?;
        let mut preview = FormulaPreview {
            errors: Vec::new(),
            kind: None,
            dependencies: Dependencies::default(),
            value: None,
            evaluated: false,
        };
        let compiled = match compile_formula(&command.expression, &context) {
            Ok(compiled) => compiled,
            Err(errors) => {
                preview.errors = errors;
                return Ok(preview);
            }
        };
        preview.kind = compiled.kind;
        preview.dependencies = compiled.dependencies.clone();
        let whole = Span::new(0, command.expression.chars().count());
        if !ValueKind::fits(compiled.kind, result_type) {
            preview.errors.push(FormulaError::new(
                INCOMPATIBLE_RESULT,
                format!(
                    "the expression results in {} and cannot be stored as {}",
                    compiled.kind.map_or("null", |kind| kind.as_str()),
                    result_type.as_str()
                ),
                whole,
            ));
        }
        let visible = FieldSet::for_scope(self.ports.definitions(), definition, scope).await?;
        for key in &compiled.dependencies.fields {
            if visible.is_hidden(key) {
                preview.errors.push(FormulaError::new(
                    FIELD_NOT_ACCESSIBLE,
                    format!("field {key} is not accessible"),
                    whole,
                ));
            }
        }
        let start = FormulaNode {
            record_type: record_type.clone(),
            key: draft.key.as_str().to_owned(),
        };
        if let Some(path) = find_cycle(&mut workspace, start).await? {
            preview.errors.push(FormulaError::new(
                FORMULA_CYCLE,
                path.iter()
                    .map(FormulaNode::label)
                    .collect::<Vec<_>>()
                    .join(" → "),
                whole,
            ));
        }
        let Some(record_id) = record_id else {
            return Ok(preview);
        };
        if !preview.errors.is_empty() {
            return Ok(preview);
        }
        self.require_visible(scope, &record_type, record_id).await?;
        let formulas = TypeFormulas::new(context);
        let mut evaluated = engine.evaluate(&formulas, &[record_id]).await?;
        let Some(record) = evaluated.remove(&record_id) else {
            return Err(record_not_found());
        };
        preview.evaluated = true;
        preview.value = record
            .results
            .iter()
            .find(|(definition, _)| definition.key == draft.key)
            .and_then(|(definition, value)| {
                value.as_ref().map(|value| value_to_json(definition, value))
            });
        Ok(preview)
    }

    async fn require_visible(
        &self,
        scope: &AccessScope,
        record_type: &RecordType,
        record_id: Uuid,
    ) -> Result<(), AppError> {
        let definition = self.ports.registry().require(record_type.as_str())?;
        let visibility = scope.record_filter(&definition.read_permission)?;
        let existing = self
            .ports
            .references()
            .existing_records(scope, definition, &visibility, &[record_id])
            .await?;
        if existing.contains(&record_id) {
            Ok(())
        } else {
            Err(record_not_found())
        }
    }
}

fn record_not_found() -> AppError {
    AppError::not_found_code(FORMULA_RECORD_NOT_FOUND, "record was not found")
}
