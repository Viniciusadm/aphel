use std::collections::BTreeSet;

use serde::Serialize;
use serde_json::{Map, Value, json};
use uuid::Uuid;

use crate::application::{
    CreateViewCommand, SavedView, SavedViewEntry, SavedViewRepository, UpdateViewCommand,
    ViewPorts, ViewType, ViewVisibility, ViewsTx,
};
use aphel_kernel::application::access::{Authorizer, PERMISSION_DENIED};
use aphel_kernel::application::audit::{DiffOptions, diff_changes};
use aphel_kernel::application::fields::permissions::{NOT_ACCESSIBLE_MESSAGE, query_error};
use aphel_kernel::application::fields::{EntityDefinition, FieldSet};
use aphel_kernel::application::organization_admin::{ListWindow, Listing};
use aphel_kernel::application::query::{
    Filter, RelatedFieldSets, ResolveContext, SortSpec, load_related_sets,
    validate_view_query_related,
};
use aphel_kernel::application::security::Clock;
use aphel_kernel::application::shared::{
    Actor, AuditEntry, AuditLog, DomainEvent, EventPublisher, FieldChange, RecordRef, RecordType,
    TransactionManager, ensure_version,
};
use aphel_kernel::domain::TeamId;
use aphel_kernel::domain::access::{AccessScope, Permission};
use aphel_kernel::errors::{AppError, FieldViolation};

pub const VIEW_NOT_FOUND: &str = "view_not_found";
pub const SAVED_VIEW_CREATED: &str = "saved_view.created";
pub const SAVED_VIEW_UPDATED: &str = "saved_view.updated";
pub const SAVED_VIEW_DELETED: &str = "saved_view.deleted";
pub const SAVED_VIEW_RECORD_TYPE: RecordType = RecordType::from_static("saved_view");

pub const MAX_VIEW_NAME_LENGTH: usize = 140;
pub const MAX_VIEW_COLUMNS: usize = 100;
pub const MAX_VIEW_SETTINGS_BYTES: usize = 16 * 1024;

#[derive(Serialize)]
struct ViewSnapshot<'a> {
    name: &'a str,
    visibility: &'static str,
    team_id: Option<&'a str>,
    view_type: &'static str,
    filter: &'a Option<Filter>,
    sort: &'a [SortSpec],
    columns: &'a [String],
    settings: &'a Map<String, Value>,
    is_default: bool,
}

impl<'a> From<&'a SavedView> for ViewSnapshot<'a> {
    fn from(view: &'a SavedView) -> Self {
        Self {
            name: &view.name,
            visibility: view.visibility.as_str(),
            team_id: view.team_id.as_ref().map(TeamId::as_str),
            view_type: view.view_type.as_str(),
            filter: &view.filter,
            sort: &view.sort,
            columns: &view.columns,
            settings: &view.settings,
            is_default: view.is_default,
        }
    }
}

pub struct ViewUseCases<P> {
    ports: P,
}

impl<P> ViewUseCases<P>
where
    P: ViewPorts,
{
    pub fn new(ports: P) -> Self {
        Self { ports }
    }

    pub async fn list_views(
        &self,
        scope: &AccessScope,
        record_type: &str,
        window: ListWindow,
    ) -> Result<Listing<SavedViewEntry>, AppError> {
        let definition = self.readable(scope, record_type)?;
        let default = self
            .ports
            .views()
            .user_default(scope, &definition.record_type)
            .await?;
        let views: Vec<SavedView> = self
            .ports
            .views()
            .list_visible(scope, &definition.record_type)
            .await?
            .into_iter()
            .filter(|view| is_visible(scope, view))
            .collect();
        let total = views.len() as u64;
        Ok(Listing {
            items: views
                .into_iter()
                .skip(usize::try_from(window.offset).unwrap_or(usize::MAX))
                .take(usize::try_from(window.limit).unwrap_or(usize::MAX))
                .map(|view| self.entry(scope, view, default))
                .collect(),
            total,
        })
    }

    pub async fn create_view(
        &self,
        scope: &AccessScope,
        record_type: &str,
        command: CreateViewCommand,
    ) -> Result<SavedViewEntry, AppError> {
        let definition = self.readable(scope, record_type)?;
        let now = self.ports.clock().now();
        let mut violations = Vec::new();
        let visibility = ViewVisibility::parse(command.visibility.trim());
        if visibility.is_none() {
            violations.push(FieldViolation::new(
                "visibility",
                "must be private, team or organization",
            ));
        }
        let view_type = ViewType::parse(command.view_type.trim());
        if view_type.is_none() {
            violations.push(FieldViolation::new(
                "view_type",
                "must be table, kanban or calendar",
            ));
        }
        let team_id = parse_team(command.team_id.as_deref(), &mut violations);
        let settings = settings_object(&command.settings, &mut violations);
        let (Some(visibility), Some(view_type), Some(settings)) = (visibility, view_type, settings)
        else {
            return Err(query_error("invalid view", violations));
        };
        let view = SavedView {
            id: Uuid::now_v7(),
            record_type: definition.record_type.clone(),
            name: command.name.trim().to_owned(),
            owner_id: Some(scope.user_id().clone()),
            visibility,
            team_id,
            view_type,
            filter: command.filter,
            sort: command.sort,
            columns: command.columns,
            settings,
            is_default: command.is_default,
            position: self
                .ports
                .views()
                .next_position(&definition.record_type)
                .await?,
            created_at: now,
            updated_at: now,
        };
        violations.extend(self.view_violations(scope, definition, &view).await?);
        if !violations.is_empty() {
            return Err(query_error("invalid view", violations));
        }
        self.authorize_audience(scope, &view)?;

        let changes = diff_changes(
            &Value::Null,
            &ViewSnapshot::from(&view),
            &DiffOptions::default(),
        )?;
        let mut tx = self.ports.transactions().begin().await?;
        self.ports.views().insert(&mut tx, &view).await?;
        if view.is_default {
            self.ports
                .views()
                .clear_organization_default(&mut tx, &view.record_type, view.id)
                .await?;
        }
        self.record(
            &mut tx,
            scope,
            SAVED_VIEW_CREATED,
            &view,
            changes,
            command.ip,
        )
        .await?;
        self.ports.transactions().commit(tx).await?;
        self.require_entry(scope, view.id).await
    }

    pub async fn update_view(
        &self,
        scope: &AccessScope,
        record_type: &str,
        view_id: &str,
        command: UpdateViewCommand,
    ) -> Result<SavedViewEntry, AppError> {
        let definition = self.readable(scope, record_type)?;
        let current = self.require_view(scope, definition, view_id).await?;
        self.authorize_edit(scope, &current)?;
        ensure_version(command.expected_updated_at, current.updated_at)?;

        let mut updated = current.clone();
        let mut violations = Vec::new();
        if let Some(name) = &command.name {
            updated.name = name.trim().to_owned();
        }
        if let Some(visibility) = &command.visibility {
            match ViewVisibility::parse(visibility.trim()) {
                Some(visibility) => updated.visibility = visibility,
                None => violations.push(FieldViolation::new(
                    "visibility",
                    "must be private, team or organization",
                )),
            }
        }
        if let Some(team_id) = &command.team_id {
            updated.team_id = parse_team(team_id.as_deref(), &mut violations);
        }
        if let Some(view_type) = &command.view_type {
            match ViewType::parse(view_type.trim()) {
                Some(view_type) => updated.view_type = view_type,
                None => violations.push(FieldViolation::new(
                    "view_type",
                    "must be table, kanban or calendar",
                )),
            }
        }
        if let Some(filter) = command.filter {
            updated.filter = filter;
        }
        if let Some(sort) = command.sort {
            updated.sort = sort;
        }
        if let Some(columns) = command.columns {
            updated.columns = columns;
        }
        if let Some(settings) = &command.settings
            && let Some(settings) = settings_object(settings, &mut violations)
        {
            updated.settings = settings;
        }
        if let Some(is_default) = command.is_default {
            updated.is_default = is_default;
        }
        violations.extend(self.view_violations(scope, definition, &updated).await?);
        if !violations.is_empty() {
            return Err(query_error("invalid view", violations));
        }
        self.authorize_audience(scope, &updated)?;

        let changes = diff_changes(
            &ViewSnapshot::from(&current),
            &ViewSnapshot::from(&updated),
            &DiffOptions::default(),
        )?;
        if changes.is_empty() {
            return self.require_entry(scope, current.id).await;
        }
        updated.updated_at = self.ports.clock().now();
        let mut tx = self.ports.transactions().begin().await?;
        self.ports.views().update(&mut tx, &updated).await?;
        if updated.is_default && !current.is_default {
            self.ports
                .views()
                .clear_organization_default(&mut tx, &updated.record_type, updated.id)
                .await?;
        }
        self.record(
            &mut tx,
            scope,
            SAVED_VIEW_UPDATED,
            &updated,
            changes,
            command.ip,
        )
        .await?;
        self.ports.transactions().commit(tx).await?;
        self.require_entry(scope, updated.id).await
    }

    pub async fn delete_view(
        &self,
        scope: &AccessScope,
        record_type: &str,
        view_id: &str,
        ip: Option<String>,
    ) -> Result<(), AppError> {
        let definition = self.readable(scope, record_type)?;
        let current = self.require_view(scope, definition, view_id).await?;
        self.authorize_edit(scope, &current)?;
        let changes = diff_changes(
            &ViewSnapshot::from(&current),
            &Value::Null,
            &DiffOptions::default(),
        )?;
        let mut tx = self.ports.transactions().begin().await?;
        self.ports.views().delete(&mut tx, current.id).await?;
        self.record(&mut tx, scope, SAVED_VIEW_DELETED, &current, changes, ip)
            .await?;
        self.ports.transactions().commit(tx).await
    }

    pub async fn set_user_default(
        &self,
        scope: &AccessScope,
        record_type: &str,
        view_id: &str,
    ) -> Result<SavedViewEntry, AppError> {
        let definition = self.readable(scope, record_type)?;
        let view = self.require_view(scope, definition, view_id).await?;
        self.ports
            .views()
            .set_user_default(scope, &definition.record_type, Some(view.id))
            .await?;
        self.require_entry(scope, view.id).await
    }

    pub async fn clear_user_default(
        &self,
        scope: &AccessScope,
        record_type: &str,
        view_id: &str,
    ) -> Result<(), AppError> {
        let definition = self.readable(scope, record_type)?;
        let view = self.require_view(scope, definition, view_id).await?;
        let current = self
            .ports
            .views()
            .user_default(scope, &definition.record_type)
            .await?;
        if current == Some(view.id) {
            self.ports
                .views()
                .set_user_default(scope, &definition.record_type, None)
                .await?;
        }
        Ok(())
    }

    fn readable(
        &self,
        scope: &AccessScope,
        record_type: &str,
    ) -> Result<&EntityDefinition, AppError> {
        let definition = self.ports.registry().require(record_type)?;
        scope.require(&definition.read_permission)?;
        Ok(definition)
    }

    async fn require_view(
        &self,
        scope: &AccessScope,
        definition: &EntityDefinition,
        view_id: &str,
    ) -> Result<SavedView, AppError> {
        Uuid::parse_str(view_id)
            .ok()
            .map(|id| self.ports.views().find(id))
            .ok_or_else(view_not_found)?
            .await?
            .filter(|view| view.record_type == definition.record_type && is_visible(scope, view))
            .ok_or_else(view_not_found)
    }

    async fn require_entry(
        &self,
        scope: &AccessScope,
        view_id: Uuid,
    ) -> Result<SavedViewEntry, AppError> {
        let view = self
            .ports
            .views()
            .find(view_id)
            .await?
            .ok_or_else(view_not_found)?;
        let default = self
            .ports
            .views()
            .user_default(scope, &view.record_type)
            .await?;
        Ok(self.entry(scope, view, default))
    }

    fn entry(&self, scope: &AccessScope, view: SavedView, default: Option<Uuid>) -> SavedViewEntry {
        SavedViewEntry {
            is_user_default: default == Some(view.id),
            can_edit: self.authorize_edit(scope, &view).is_ok(),
            view,
        }
    }

    fn authorize_edit(&self, scope: &AccessScope, view: &SavedView) -> Result<(), AppError> {
        let is_owner = view.owner_id.as_ref() == Some(scope.user_id());
        let manages = scope.require(&Permission::SETTINGS_MANAGE).is_ok();
        if manages || (is_owner && view.visibility != ViewVisibility::Organization) {
            return Ok(());
        }
        Err(AppError::forbidden_code(
            PERMISSION_DENIED,
            "only the owner or someone with settings.manage can change this view",
        ))
    }

    fn authorize_audience(&self, scope: &AccessScope, view: &SavedView) -> Result<(), AppError> {
        let manages = scope.require(&Permission::SETTINGS_MANAGE).is_ok();
        let allowed = match view.visibility {
            ViewVisibility::Private => !view.is_default,
            ViewVisibility::Team => {
                !view.is_default
                    && view
                        .team_id
                        .as_ref()
                        .is_some_and(|team| manages || scope.team_ids().contains(team))
            }
            ViewVisibility::Organization => manages,
        };
        if allowed {
            return Ok(());
        }
        Err(AppError::forbidden_code(
            PERMISSION_DENIED,
            "missing permission to share this view with this audience",
        ))
    }

    async fn view_violations(
        &self,
        scope: &AccessScope,
        definition: &EntityDefinition,
        view: &SavedView,
    ) -> Result<Vec<FieldViolation>, AppError> {
        let mut violations = Vec::new();
        if view.name.is_empty() || view.name.chars().count() > MAX_VIEW_NAME_LENGTH {
            violations.push(FieldViolation::new(
                "name",
                format!("must have between 1 and {MAX_VIEW_NAME_LENGTH} characters"),
            ));
        }
        match (view.visibility, &view.team_id) {
            (ViewVisibility::Team, None) => {
                violations.push(FieldViolation::new("team_id", "is required for team views"))
            }
            (ViewVisibility::Team, Some(team)) => {
                let exists = self.ports.views().team_exists(team).await?;
                if !exists {
                    violations.push(FieldViolation::new("team_id", "team was not found"));
                }
            }
            (_, Some(_)) => violations.push(FieldViolation::new(
                "team_id",
                "is only accepted by team views",
            )),
            _ => {}
        }
        if view.is_default && view.visibility != ViewVisibility::Organization {
            violations.push(FieldViolation::new(
                "is_default",
                "only organization views can be the default of the organization",
            ));
        }

        let fields = FieldSet::for_scope(self.ports.definitions(), definition, scope).await?;
        let context = ResolveContext {
            now: self.ports.clock().now(),
            timezone: chrono_tz::UTC,
            user_id: scope.user_id().clone(),
        };
        let relations = match &view.filter {
            Some(filter) => {
                load_related_sets(
                    self.ports.registry(),
                    self.ports.definitions(),
                    scope,
                    definition,
                    &filter.relation_keys(),
                )
                .await?
            }
            None => RelatedFieldSets::default(),
        };
        violations.extend(validate_view_query_related(
            &fields,
            &context,
            view.filter.as_ref(),
            &view.sort,
            &relations,
        ));
        if view.columns.len() > MAX_VIEW_COLUMNS {
            violations.push(FieldViolation::new(
                "columns",
                format!("at most {MAX_VIEW_COLUMNS} columns are allowed"),
            ));
        }
        let mut seen = BTreeSet::new();
        for (index, column) in view.columns.iter().enumerate() {
            if fields.is_hidden(column) {
                violations.push(FieldViolation::new(
                    format!("columns.{index}"),
                    NOT_ACCESSIBLE_MESSAGE,
                ));
            } else if fields.find_active(column).is_none() {
                violations.push(FieldViolation::new(
                    format!("columns.{index}"),
                    "is not a known field",
                ));
            } else if !seen.insert(column) {
                violations.push(FieldViolation::new(
                    format!("columns.{index}"),
                    "is duplicated",
                ));
            }
        }
        Ok(violations)
    }

    async fn record(
        &self,
        tx: &mut ViewsTx<P>,
        scope: &AccessScope,
        action: &str,
        view: &SavedView,
        changes: Vec<FieldChange>,
        ip: Option<String>,
    ) -> Result<(), AppError> {
        let actor = Actor::from_scope(scope);
        let now = self.ports.clock().now();
        let record = RecordRef::new(SAVED_VIEW_RECORD_TYPE, view.id);
        let event = DomainEvent::new(actor.clone(), action, now)
            .with_record(record.clone())
            .with_payload(json!({
                "record_type": view.record_type.as_str(),
                "name": view.name,
                "visibility": view.visibility.as_str(),
            }));
        let entry = AuditEntry::new(actor, action, now)
            .with_record(record)
            .with_changes(changes)
            .with_ip(ip);
        self.ports.events().publish(tx, event).await?;
        self.ports.audit().record(tx, entry).await
    }
}

pub fn is_visible(scope: &AccessScope, view: &SavedView) -> bool {
    match view.visibility {
        ViewVisibility::Organization => true,
        ViewVisibility::Private => view.owner_id.as_ref() == Some(scope.user_id()),
        ViewVisibility::Team => {
            view.owner_id.as_ref() == Some(scope.user_id())
                || view
                    .team_id
                    .as_ref()
                    .is_some_and(|team| scope.team_ids().contains(team))
        }
    }
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
        Value::Object(object) if value.to_string().len() <= MAX_VIEW_SETTINGS_BYTES => {
            Some(object.clone())
        }
        Value::Object(_) => {
            violations.push(FieldViolation::new(
                "settings",
                format!("must have at most {MAX_VIEW_SETTINGS_BYTES} bytes"),
            ));
            None
        }
        _ => {
            violations.push(FieldViolation::new("settings", "must be an object"));
            None
        }
    }
}

fn view_not_found() -> AppError {
    AppError::not_found_code(VIEW_NOT_FOUND, "view was not found")
}
