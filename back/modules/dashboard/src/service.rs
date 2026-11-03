use std::collections::HashSet;

use chrono::{DateTime, Utc};
use serde_json::Value;

use crate::engine::{
    DashboardPorts, DashboardRepository, LayoutSubject, PeriodQuery, SubjectKind, SubjectOptions,
    WidgetContext, WidgetDescriptor, WidgetRegistry, WidgetSize, WidgetSubject, parse_period,
    period_violation, visible_subjects,
};
use crate::engine_domain::{ResolvedPeriod, previous_period, resolve_period};
use crate::layout::{
    DashboardLayout, LAYOUT_VERSION, LayoutPeriod, LayoutView, LayoutWidget, MAX_LAYOUT_WIDGETS,
    MAX_WIDGET_ID_LENGTH, MAX_WIDGET_SETTINGS_BYTES, SubjectQuery, WidgetDataQuery,
};
use crate::repository::{MySqlDashboardPorts, MySqlDashboardRepository};
use crate::store::LayoutStore;

use aphel_kernel::application::access::PERMISSION_DENIED;
use aphel_kernel::application::query::{RecordQueryExecutor, RecordQueryPorts};
use aphel_kernel::application::security::Clock;
use aphel_kernel::context::Ctx;
use aphel_kernel::domain::access::AccessScope;
use aphel_kernel::errors::{AppError, FieldViolation};

pub(crate) const WIDGET_NOT_FOUND: &str = "widget_not_found";

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct WidgetData {
    pub(crate) key: String,
    pub(crate) period: Option<ResolvedPeriod>,
    pub(crate) previous_period: Option<ResolvedPeriod>,
    pub(crate) subject: Option<WidgetSubject>,
    pub(crate) settings: Value,
    pub(crate) generated_at: DateTime<Utc>,
    pub(crate) data: Value,
}

fn widget_not_found() -> AppError {
    AppError::not_found_code(WIDGET_NOT_FOUND, "widget was not found")
}

fn subject_denied() -> AppError {
    AppError::forbidden_code(
        PERMISSION_DENIED,
        "the user or team is not visible to the current user",
    )
}

fn subject_request(query: &SubjectQuery) -> Result<Option<(SubjectKind, String)>, AppError> {
    let present = |value: &Option<String>| {
        value
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_owned)
    };
    match (present(&query.user_id), present(&query.team_id)) {
        (None, None) => Ok(None),
        (Some(user_id), None) => Ok(Some((SubjectKind::User, user_id))),
        (None, Some(team_id)) => Ok(Some((SubjectKind::Team, team_id))),
        (Some(_), Some(_)) => Err(AppError::invalid_fields(
            "invalid subject",
            vec![FieldViolation::new(
                "team_id",
                "use either user_id or team_id, not both",
            )],
        )),
    }
}

fn valid_widget_id(id: &str) -> bool {
    !id.is_empty()
        && id.len() <= MAX_WIDGET_ID_LENGTH
        && id
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || matches!(character, '-' | '_'))
}

type Widgets = WidgetRegistry<MySqlDashboardPorts>;

fn widget_ports(ctx: &Ctx<'_>) -> MySqlDashboardPorts {
    MySqlDashboardPorts::new(
        ctx.pool().clone(),
        ctx.query(),
        std::sync::Arc::new(MySqlDashboardRepository::new(ctx.pool().clone())),
    )
}

pub(crate) fn catalog(ctx: &Ctx<'_>) -> Vec<WidgetDescriptor> {
    Widgets::builtin()
        .available(ctx.scope)
        .map(|provider| provider.descriptor())
        .collect()
}

pub(crate) async fn widget_data(
    ctx: &Ctx<'_>,
    key: &str,
    query: WidgetDataQuery,
) -> Result<WidgetData, AppError> {
    let scope = ctx.scope;
    let registry = Widgets::builtin();
    let provider = registry.get(key).ok_or_else(widget_not_found)?;
    if !provider.is_available(scope) {
        return Err(AppError::forbidden_code(
            PERMISSION_DENIED,
            format!("missing permission for widget {key}"),
        ));
    }
    let settings = provider
        .normalize_settings(query.settings.as_ref().unwrap_or(&Value::Null))
        .map_err(|violations| AppError::invalid_fields("invalid widget settings", violations))?;
    let selection = parse_period(&query.period, "period", "from", "to")
        .map_err(|violations| AppError::invalid_fields("invalid period", violations))?;
    let subject = match subject_request(&query.subject)? {
        None => None,
        Some((kind, id)) => Some(
            subjects(ctx)
                .await?
                .resolve(kind, &id)
                .ok_or_else(subject_denied)?,
        ),
    };
    let ports = widget_ports(ctx);
    let now = ports.query().clock().now();
    let timezone = ports.query().executor().timezone().await?;
    let period = resolve_period(&selection, now, timezone).map_err(|error| {
        AppError::invalid_fields("invalid period", vec![period_violation("to", error)])
    })?;
    let compare_to_previous = query.compare_to_previous && provider.uses_period();
    let previous = compare_to_previous.then(|| previous_period(&period, timezone));
    let context = WidgetContext {
        scope,
        ports: &ports,
        now,
        timezone,
        period,
        subject: subject.clone(),
        compare_to_previous,
        previous_period: previous,
    };
    let data = provider.data(&context, &settings).await?;
    Ok(WidgetData {
        key: provider.key().to_owned(),
        period: provider.uses_period().then_some(period),
        previous_period: previous,
        subject,
        settings,
        generated_at: now,
        data,
    })
}

pub(crate) async fn subjects(ctx: &Ctx<'_>) -> Result<SubjectOptions, AppError> {
    let directory = MySqlDashboardRepository::new(ctx.pool().clone())
        .subject_directory()
        .await?;
    let permissions: Vec<_> = ctx
        .registry()
        .definitions()
        .map(|definition| definition.read_permission.clone())
        .collect();
    Ok(visible_subjects(ctx.scope, &permissions, directory))
}

fn default_layout_with(registry: &Widgets, scope: &AccessScope) -> DashboardLayout {
    let mut defaults: Vec<_> = registry
        .available(scope)
        .filter_map(|provider| provider.default_order().map(|order| (order, provider)))
        .collect();
    defaults.sort_by(|(left_order, left), (right_order, right)| {
        left_order
            .cmp(right_order)
            .then_with(|| left.key().cmp(right.key()))
    });
    let widgets = defaults
        .into_iter()
        .take(MAX_LAYOUT_WIDGETS)
        .map(|(_, provider)| LayoutWidget {
            id: format!("default-{}", provider.key().replace(['_', '.'], "-")),
            key: provider.key().to_owned(),
            size: provider.default_size().as_str().to_owned(),
            settings: provider.default_settings(),
        })
        .collect();
    DashboardLayout {
        widgets,
        period: LayoutPeriod::default(),
        subject: None,
        compare_to_previous: false,
    }
}

pub(crate) async fn layout(ctx: &Ctx<'_>) -> Result<LayoutView, AppError> {
    let registry = Widgets::builtin();
    let stored = LayoutStore::of(ctx).find(ctx.scope).await?;
    let parsed = stored.and_then(|value| serde_json::from_value::<DashboardLayout>(value).ok());
    Ok(match parsed {
        Some(layout) => {
            let subjects = subjects(ctx).await?;
            LayoutView {
                layout: visible_layout(&registry, ctx.scope, &subjects, layout),
                is_default: false,
            }
        }
        None => LayoutView {
            layout: fallback_layout(ctx, &registry).await?,
            is_default: true,
        },
    })
}

async fn fallback_layout(ctx: &Ctx<'_>, registry: &Widgets) -> Result<DashboardLayout, AppError> {
    let organization_default = LayoutStore::of(ctx)
        .find_default()
        .await?
        .and_then(|value| serde_json::from_value::<DashboardLayout>(value).ok());
    Ok(match organization_default {
        Some(layout) => {
            let subjects = subjects(ctx).await?;
            visible_layout(registry, ctx.scope, &subjects, layout)
        }
        None => default_layout_with(registry, ctx.scope),
    })
}

pub(crate) async fn save_layout(
    ctx: &Ctx<'_>,
    layout: DashboardLayout,
) -> Result<LayoutView, AppError> {
    let registry = Widgets::builtin();
    let subjects = subjects(ctx).await?;
    let layout = validate_layout(ctx, &registry, &subjects, layout)?;
    let mut stored =
        serde_json::to_value(&layout).map_err(|error| AppError::Internal(error.to_string()))?;
    if let Value::Object(map) = &mut stored {
        map.insert("version".to_owned(), Value::from(LAYOUT_VERSION));
    }
    LayoutStore::of(ctx)
        .save(ctx.scope, &stored, ctx.query().clock().now())
        .await?;
    Ok(LayoutView {
        layout,
        is_default: false,
    })
}

pub(crate) async fn reset_layout(ctx: &Ctx<'_>) -> Result<LayoutView, AppError> {
    LayoutStore::of(ctx).delete(ctx.scope).await?;
    Ok(LayoutView {
        layout: fallback_layout(ctx, &Widgets::builtin()).await?,
        is_default: true,
    })
}

fn visible_layout(
    registry: &Widgets,
    scope: &AccessScope,
    subjects: &SubjectOptions,
    layout: DashboardLayout,
) -> DashboardLayout {
    let widgets = layout
        .widgets
        .into_iter()
        .filter_map(|widget| {
            let provider = registry.get(&widget.key)?;
            if !provider.is_available(scope) {
                return None;
            }
            let size = WidgetSize::parse(&widget.size).unwrap_or(provider.default_size());
            let settings = provider
                .normalize_settings(&widget.settings)
                .unwrap_or_else(|_| provider.default_settings());
            Some(LayoutWidget {
                size: size.as_str().to_owned(),
                settings,
                ..widget
            })
        })
        .collect();
    let period = parse_period(
        &PeriodQuery {
            period: Some(layout.period.preset.clone()),
            from: layout.period.from.clone(),
            to: layout.period.to.clone(),
        },
        "period.preset",
        "period.from",
        "period.to",
    )
    .map(|selection| LayoutPeriod::from_selection(&selection))
    .unwrap_or_default();
    let subject = layout
        .subject
        .and_then(|subject| subjects.resolve(subject.kind, &subject.id))
        .map(|subject| subject.layout());
    DashboardLayout {
        widgets,
        period,
        subject,
        compare_to_previous: layout.compare_to_previous,
    }
}

fn validate_layout(
    ctx: &Ctx<'_>,
    registry: &Widgets,
    subjects: &SubjectOptions,
    layout: DashboardLayout,
) -> Result<DashboardLayout, AppError> {
    let mut violations = Vec::new();
    if layout.widgets.len() > MAX_LAYOUT_WIDGETS {
        violations.push(FieldViolation::new(
            "widgets",
            format!("must have at most {MAX_LAYOUT_WIDGETS} widgets"),
        ));
    }
    let mut ids = HashSet::new();
    let mut widgets = Vec::with_capacity(layout.widgets.len());
    for (index, widget) in layout.widgets.into_iter().enumerate() {
        let path = format!("widgets.{index}");
        if !valid_widget_id(&widget.id) {
            violations.push(FieldViolation::new(
                format!("{path}.id"),
                "must have 1 to 64 letters, digits, hyphens or underscores",
            ));
        } else if !ids.insert(widget.id.clone()) {
            violations.push(FieldViolation::new(format!("{path}.id"), "is repeated"));
        }
        let size = WidgetSize::parse(&widget.size);
        if size.is_none() {
            violations.push(FieldViolation::new(
                format!("{path}.size"),
                "must be small, medium or large",
            ));
        }
        let Some(provider) = registry
            .get(&widget.key)
            .filter(|provider| provider.is_available(ctx.scope))
        else {
            violations.push(FieldViolation::new(
                format!("{path}.key"),
                "is not an available widget",
            ));
            continue;
        };
        if widget.settings.to_string().len() > MAX_WIDGET_SETTINGS_BYTES {
            violations.push(FieldViolation::new(
                format!("{path}.settings"),
                format!("must have at most {MAX_WIDGET_SETTINGS_BYTES} bytes"),
            ));
            continue;
        }
        match provider.normalize_settings(&widget.settings) {
            Ok(settings) => widgets.push(LayoutWidget {
                id: widget.id,
                key: widget.key,
                size: size.unwrap_or(WidgetSize::Medium).as_str().to_owned(),
                settings,
            }),
            Err(errors) => violations.extend(errors.into_iter().map(|violation| {
                FieldViolation::new(format!("{path}.{}", violation.field), violation.message)
            })),
        }
    }
    let period = match parse_period(
        &PeriodQuery {
            period: Some(layout.period.preset.clone()),
            from: layout.period.from.clone(),
            to: layout.period.to.clone(),
        },
        "period.preset",
        "period.from",
        "period.to",
    ) {
        Ok(selection) => {
            if let Err(error) =
                resolve_period(&selection, ctx.query().clock().now(), chrono_tz::UTC)
            {
                violations.push(period_violation("period.to", error));
            }
            LayoutPeriod::from_selection(&selection)
        }
        Err(errors) => {
            violations.extend(errors);
            LayoutPeriod::default()
        }
    };
    let subject: Option<LayoutSubject> = match layout.subject {
        None => None,
        Some(subject) => match subjects.resolve(subject.kind, &subject.id) {
            Some(resolved) => Some(resolved.layout()),
            None => {
                violations.push(FieldViolation::new(
                    "subject.id",
                    "is not a user or team visible to you",
                ));
                None
            }
        },
    };
    if violations.is_empty() {
        Ok(DashboardLayout {
            widgets,
            period,
            subject,
            compare_to_previous: layout.compare_to_previous,
        })
    } else {
        Err(AppError::invalid_fields(
            "invalid dashboard layout",
            violations,
        ))
    }
}
