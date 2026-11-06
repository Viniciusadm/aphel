use std::sync::Arc;

use aphel_kernel::application::access::Authorizer;
use aphel_kernel::application::shared::{Actor, EventCausation};
use aphel_kernel::domain::access::{
    AccessScope, Permission, PermissionGrant, PermissionSet, RecordScope, RecordScopeFilter,
    RoleKey, permission_catalog,
};
use aphel_kernel::domain::{TeamId, UserId};
use aphel_kernel::modules::builder::Contribution;
use aphel_kernel::modules::hooks::ActionFn;
use aphel_kernel::modules::{EnabledModules, Hook, HookBus, HookCtx};

use crate::cli::CliResult;

#[derive(Clone, Copy)]
struct OrderProbe;

impl Hook for OrderProbe {
    type Args = ();
}

fn handler() -> ActionFn<()> {
    Arc::new(|_: &mut HookCtx<'_>, _: &()| Box::pin(async { Ok(()) }))
}

fn user(id: &str) -> UserId {
    UserId::new(id.to_owned()).expect("probe user id")
}

fn scope_with(grants: &[(&'static str, RecordScope)]) -> AccessScope {
    AccessScope::new(
        user("probe-user"),
        RoleKey::new("probe_role").expect("probe role"),
        vec![TeamId::new("probe-team".to_owned()).expect("probe team")],
        PermissionSet::from_grants(grants.iter().map(|(key, scope)| PermissionGrant {
            permission: Permission::from_static(key),
            scope: *scope,
        })),
    )
}

fn access_scope_filters(failures: &mut Vec<String>) {
    let scope = scope_with(&[
        ("probe.own", RecordScope::Own),
        ("probe.team", RecordScope::Team),
        ("probe.all", RecordScope::Organization),
    ]);
    let own = scope.filter_for(&Permission::from_static("probe.own"));
    if !matches!(own, Some(RecordScopeFilter::Owner { ref user_id, .. }) if user_id.as_str() == "probe-user")
    {
        failures.push("access: an own-scope grant must filter by owner".into());
    }
    let team = scope.filter_for(&Permission::from_static("probe.team"));
    if !matches!(team, Some(RecordScopeFilter::Teams { ref team_ids, .. }) if team_ids.len() == 1) {
        failures.push("access: a team-scope grant must filter by the user's teams".into());
    }
    if !matches!(
        scope.filter_for(&Permission::from_static("probe.all")),
        Some(RecordScopeFilter::Organization)
    ) {
        failures.push("access: an organization grant must not filter".into());
    }
    if scope
        .record_filter(&Permission::from_static("probe.missing"))
        .is_ok()
        || scope
            .require(&Permission::from_static("probe.missing"))
            .is_ok()
    {
        failures.push("access: a permission without a grant must be denied".into());
    }
    let restricted = scope_with(&[("probe.all", RecordScope::Organization)])
        .restricted(|permission| permission.as_str() != "probe.all");
    if restricted
        .require(&Permission::from_static("probe.all"))
        .is_ok()
    {
        failures.push("access: restricting a scope must remove the blocked permission".into());
    }
    let system = AccessScope::system();
    if !system.is_system()
        || !matches!(
            system.filter_for(&Permission::from_static("probe.any")),
            Some(RecordScopeFilter::Organization)
        )
    {
        failures.push("access: the system scope must see the whole organization".into());
    }
}

fn hook_order(failures: &mut Vec<String>) {
    let enabled = Arc::new(EnabledModules::new(["a"], ["a", "b", "c"]));
    enabled.replace(["b".to_owned()]);
    let mut bus = HookBus::new(enabled);
    for (module, priority) in [("a", 30), ("b", 10), ("c", 20), ("a", 10)] {
        bus.on(
            OrderProbe,
            Contribution {
                module,
                priority,
                item: handler(),
            },
        );
    }
    let order: Vec<(&str, i32)> = bus
        .run_order(OrderProbe)
        .into_iter()
        .map(|entry| (entry.module, entry.priority))
        .collect();
    if order != [("b", 10), ("a", 10), ("a", 30)] {
        failures.push(format!(
            "hooks: handlers must run by ascending priority, in registration order on ties, skipping disabled modules; got {order:?}"
        ));
    }
}

fn record_validation_order(failures: &mut Vec<String>) {
    let app = aphel_kernel::modules::app();
    let key = <aphel_kernel::records::RecordValidating as Hook>::key();
    let expected = [
        ("approvals", 10),
        ("workflow", 20),
        ("validation_rules", 30),
        ("leads", 40),
    ];
    let registered: Vec<(&str, i32)> = app
        .hooks()
        .describe()
        .iter()
        .filter(|entry| entry.hook == key)
        .map(|entry| (entry.module, entry.priority))
        .collect();
    for (module, priority) in expected {
        let found: Vec<i32> = registered
            .iter()
            .filter(|(owner, _)| *owner == module)
            .map(|(_, value)| *value)
            .collect();
        if !found.is_empty() && !found.contains(&priority) {
            failures.push(format!(
                "pipeline: `{module}` must validate records at priority {priority}, found {found:?}"
            ));
        }
    }
}

fn registry_integrity(failures: &mut Vec<String>) {
    let app = aphel_kernel::modules::app();
    let permissions: Vec<Permission> = permission_catalog()
        .into_iter()
        .map(|definition| definition.permission)
        .collect();
    let registry = app.record_types();
    for definition in registry.definitions() {
        if !permissions.contains(&definition.read_permission) {
            failures.push(format!(
                "registry: `{}` reads with an unregistered permission {}",
                definition.key, definition.read_permission
            ));
        }
        if let Some(title) = &definition.title_field
            && definition.system_field_by_key(title).is_none()
        {
            failures.push(format!(
                "query: `{}` has a title field `{title}` that is not a system field",
                definition.key
            ));
        }
        for reference in &definition.references {
            if definition.system_field_by_key(&reference.field).is_none() {
                failures.push(format!(
                    "references: `{}.{}` is not a system field",
                    definition.key, reference.field
                ));
            }
            if let aphel_kernel::application::fields::ReferenceTarget::Entity { entity } =
                &reference.target
                && registry.require(entity).is_err()
            {
                failures.push(format!(
                    "references: `{}.{}` points at the unregistered entity `{entity}`",
                    definition.key, reference.field
                ));
            }
        }
    }
}

fn operations(failures: &mut Vec<String>) {
    let app = aphel_kernel::modules::app();
    let mut seen = std::collections::BTreeSet::new();
    for (owner, entry) in app.operations() {
        if !seen.insert(entry.key) {
            failures.push(format!(
                "operations: `{}` is registered twice (second owner `{owner}`)",
                entry.key
            ));
        }
        if let Some(permission) = entry.spec.permission
            && Permission::parse(permission).is_err()
        {
            failures.push(format!(
                "operations: `{}` requires the invalid permission `{permission}`",
                entry.key
            ));
        }
    }
}

fn probe_operation<'a>(
    _ctx: &'a aphel_kernel::Ctx<'_>,
    input: serde_json::Value,
) -> aphel_kernel::modules::OperationFuture<'a> {
    Box::pin(async move { Ok(input) })
}

fn runtime_operation_entry(key: &'static str) -> aphel_kernel::modules::OperationEntry {
    let run: aphel_kernel::modules::plugin_bridge::OperationFn = probe_operation;
    aphel_kernel::modules::OperationEntry::runtime(
        key,
        aphel_kernel::modules::OperationSpec::read("probe"),
        Arc::new(run),
    )
}

fn provided_operations(failures: &mut Vec<String>) {
    const OWNER: &str = "probe_runtime";
    const KEY: &str = "probe_runtime.echo";
    let app = aphel_kernel::modules::app();
    let visible = |expected: bool, step: &str, failures: &mut Vec<String>| {
        let found = app.operations().iter().any(|(_, entry)| entry.key == KEY)
            && app
                .all_contributions::<aphel_kernel::modules::PluginOperation>()
                .iter()
                .any(|(_, operation)| operation.key == KEY);
        if found != expected {
            failures.push(format!(
                "provided operations: `{KEY}` visibility is {found} {step}, expected {expected}"
            ));
        }
    };
    let mut batch = aphel_kernel::modules::RuntimeBatch::new();
    if let Err(error) = batch.add_operation(app, runtime_operation_entry(KEY), &[]) {
        failures.push(format!(
            "provided operations: a new key must be accepted: {error}"
        ));
    }
    app.replace_runtime(OWNER, batch);
    visible(true, "after replace_runtime", failures);
    app.replace_runtime(OWNER, aphel_kernel::modules::RuntimeBatch::new());
    visible(false, "after replacing with an empty batch", failures);

    let Some((_, native)) = app
        .operations()
        .into_iter()
        .find(|(owner, _)| *owner != OWNER)
    else {
        return;
    };
    let mut shadow = aphel_kernel::modules::RuntimeBatch::new();
    if shadow
        .add_operation(app, runtime_operation_entry(native.key), &[])
        .is_ok()
    {
        failures.push(format!(
            "provided operations: a WebAssembly operation must not shadow native `{}`",
            native.key
        ));
    }
}

fn provided_contributions(failures: &mut Vec<String>) {
    const OWNER: &str = "probe_contribution";
    const POINT: &str = "probe.point";
    let app = aphel_kernel::modules::app();
    let present = || {
        app.all_contributions::<aphel_kernel::modules::RuntimeContribution>()
            .iter()
            .any(|(owner, contribution)| *owner == OWNER && contribution.point == POINT)
    };
    let run: aphel_kernel::modules::plugin_bridge::OperationFn = probe_operation;
    let mut batch = aphel_kernel::modules::RuntimeBatch::new();
    batch.add(Arc::new(aphel_kernel::modules::RuntimeContribution::new(
        OWNER,
        POINT,
        "item",
        None,
        serde_json::Value::Null,
        Arc::new(run),
    )));
    app.replace_runtime(OWNER, batch);
    if !present() {
        failures.push(
            "contributions: a runtime contribution must be visible after replace_runtime".into(),
        );
    }
    app.replace_runtime(OWNER, aphel_kernel::modules::RuntimeBatch::new());
    if present() {
        failures.push(
            "contributions: a runtime contribution must disappear after an empty batch".into(),
        );
    }
}

fn contract_resolution(failures: &mut Vec<String>) {
    let uses = aphel_kernel::modules::Uses {
        required: vec!["party@1".into(), "tasks@1".into()],
        optional: vec!["assignment@1".into()],
    };
    if uses.unmet(&["party@1".into()]) != vec!["tasks@1".to_owned()] {
        failures.push("contracts: only missing required contracts must be reported".into());
    }
    if !uses.unmet(&["party@1".into(), "tasks@1".into()]).is_empty() {
        failures.push("contracts: provided requirements must resolve".into());
    }
    if uses.unmet(&["party@2".into(), "tasks@1".into()]) != vec!["party@1".to_owned()] {
        failures.push("contracts: a different major must not satisfy a requirement".into());
    }
}

fn event_envelope(failures: &mut Vec<String>) {
    let service = scope_with(&[]).acting_as("plugin", "probe");
    match Actor::from_scope(&service) {
        Actor::Service { kind, id } if kind == "plugin" && id == "probe" => {}
        other => failures.push(format!(
            "events: a service scope must publish as a service actor, got {other:?}"
        )),
    }
    let parsed = Actor::from_parts("plugin", Some("probe".into()));
    if !matches!(parsed, Ok(Actor::Service { ref kind, .. }) if kind == "plugin") {
        failures.push("events: an unknown actor kind must round-trip as a service".into());
    }
    if Actor::from_parts("plugin", None).is_ok() {
        failures.push("events: a service actor without an id must be rejected".into());
    }
    let legacy = serde_json::from_value::<EventCausation>(serde_json::json!({
        "automation_id": "legacy",
        "run_id": "00000000-0000-7000-8000-000000000000",
        "depth": 1
    }));
    match legacy {
        Ok(causation)
            if causation.executor_kind == "automation" && causation.executor_id == "legacy" => {}
        _ => failures.push(
            "events: legacy `automation_id` causation must read as an automation executor".into(),
        ),
    }
}

fn ui_plugin_selection(failures: &mut Vec<String>) {
    use aphel_kernel::application::organization_profile::is_ui_plugin_key;

    for key in ["midnight", "dock_frame", "a1", "ui"] {
        if !is_ui_plugin_key(key) {
            failures.push(format!("ui plugin: `{key}` must be an accepted key"));
        }
    }
    for key in [
        "",
        "Midnight",
        "1plugin",
        "has-dash",
        "a.b",
        "../x",
        &"a".repeat(33),
    ] {
        if is_ui_plugin_key(key) {
            failures.push(format!("ui plugin: `{key}` must be rejected as a key"));
        }
    }
    #[cfg(feature = "plugins")]
    module_wasm_runtime::ui::probe(failures);
}

pub(crate) fn run() -> CliResult {
    let mut failures = Vec::new();
    access_scope_filters(&mut failures);
    hook_order(&mut failures);
    record_validation_order(&mut failures);
    registry_integrity(&mut failures);
    operations(&mut failures);
    provided_operations(&mut failures);
    provided_contributions(&mut failures);
    contract_resolution(&mut failures);
    event_envelope(&mut failures);
    ui_plugin_selection(&mut failures);
    if failures.is_empty() {
        return Ok(());
    }
    for failure in &failures {
        eprintln!("probe: {failure}");
    }
    Err(format!("{} probe check(s) failed", failures.len()).into())
}
