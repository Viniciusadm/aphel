use crate::modules::plugin_bridge::{
    HookForwarding, PLUGIN_HOOK_PRIORITY, PluginHook, PluginHookArgs, PluginHookFilter,
};
use std::any::{Any, TypeId};
use std::collections::{BTreeSet, HashMap};
use std::sync::{Arc, RwLock};

use utoipa_axum::router::OpenApiRouter;

use crate::api::state::AppState;
use crate::application::fields::{EntityDefinition, RecordTypeRegistry};
use crate::application::jobs::ScheduledJobDefinition;
use crate::application::module_admin::Compatibility;
use crate::application::public_events::PublicEventDefinition;
use crate::application::shared::RecordType;
use crate::application::shared::{EventConsumer, JobHandler};
use crate::domain::access::{Permission, PermissionDefinition};
use crate::errors::AppError;
use crate::modules::catalog::{Catalog, OpenApiTag, PermissionGrant};
use crate::modules::enabled::EnabledModules;
use crate::modules::hooks::{
    ActionFn, BoxFuture, DEFAULT_PRIORITY, FilterFn, HookBus, HookCtx, HookRef, HookUsage,
};
use crate::modules::module::{Manifest, Module};
use crate::modules::services::Services;
use crate::modules::worker::{WorkerEnv, WorkerFactory, WorkerParts};
use crate::records::types::{assemble_registry, describe_records};
use crate::records::{
    RecordContributionInfo, RecordHandlerFactory, RecordTypeExtension, RecordTypeHandler,
};

pub const KERNEL_MODULE: &str = "kernel";
pub const MODULE_DEPENDENCY_CYCLE: &str = "module_dependency_cycle";
pub const APHEL_VERSION: &str = env!("CARGO_PKG_VERSION");

pub struct Contribution<T> {
    pub module: &'static str,
    pub priority: i32,
    pub item: T,
}

pub type RouteFactory = Arc<dyn Fn(&AppState) -> OpenApiRouter<AppState> + Send + Sync>;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RouteMount {
    Api,
    Public,
    Root,
}

impl RouteMount {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Api => "api",
            Self::Public => "public",
            Self::Root => "root",
        }
    }

    pub fn prefix(&self) -> &'static str {
        match self {
            Self::Api | Self::Public => "/api/v1",
            Self::Root => "",
        }
    }
}

#[derive(Clone)]
pub struct RouteContribution {
    pub mount: RouteMount,
    pub factory: RouteFactory,
}

pub struct LoadedModule {
    pub module: Box<dyn Module>,
    pub manifest: Manifest,
    pub compatibility: Compatibility,
}

impl LoadedModule {
    pub fn key(&self) -> &'static str {
        self.manifest.key()
    }

    pub fn is_compatible(&self) -> bool {
        self.compatibility.is_compatible()
    }
}

pub struct AppBuilder {
    pending: Vec<Box<dyn Module>>,
    current: &'static str,
    loaded: Vec<&'static str>,
    pub routes: Vec<Contribution<RouteContribution>>,
    pub hooks: HookBus,
    pub services: Services,
    pub catalog: Catalog,
    pub workers: Vec<Contribution<WorkerFactory>>,

    record_types: Vec<Contribution<EntityDefinition>>,
    record_extensions: Vec<Contribution<RecordTypeExtension>>,
    entity_transforms: Vec<crate::application::fields::RegistryTransform>,
    custom_record_handler: Option<RecordHandlerFactory>,
    contributions: Contributions,
    enabled: Arc<EnabledModules>,
}

type Contributions = HashMap<TypeId, Vec<Contribution<Box<dyn Any + Send + Sync>>>>;

pub struct PermissionEntry<'a> {
    builder: &'a mut AppBuilder,
    permission: Permission,
}

impl PermissionEntry<'_> {
    pub fn grant_to(self, roles: impl IntoIterator<Item = &'static str>) {
        self.grant(roles, false);
    }

    pub fn grant_organization_wide_to(self, roles: impl IntoIterator<Item = &'static str>) {
        self.grant(roles, true);
    }

    fn grant(self, roles: impl IntoIterator<Item = &'static str>, organization_wide: bool) {
        let grant = PermissionGrant {
            permission: self.permission,
            roles: roles.into_iter().collect(),
            organization_wide,
        };
        let contribution = self.builder.contribution(DEFAULT_PRIORITY, grant);
        self.builder.catalog.grants.push(contribution);
    }
}

impl Default for AppBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl AppBuilder {
    pub fn new() -> Self {
        let enabled = Arc::new(EnabledModules::new([], []));
        Self {
            pending: Vec::new(),
            current: KERNEL_MODULE,
            loaded: Vec::new(),
            routes: Vec::new(),
            hooks: HookBus::new(enabled.clone()),
            services: Services::new(enabled.clone()),
            catalog: Catalog::default(),
            workers: Vec::new(),

            record_types: Vec::new(),
            record_extensions: Vec::new(),
            entity_transforms: Vec::new(),
            custom_record_handler: None,
            contributions: HashMap::new(),
            enabled,
        }
    }

    pub fn module(mut self, module: Box<dyn Module>) -> Self {
        self.pending.push(module);
        self
    }

    pub fn modules(mut self, modules: impl IntoIterator<Item = Box<dyn Module>>) -> Self {
        self.pending.extend(modules);
        self
    }

    fn contribution<T>(&self, priority: i32, item: T) -> Contribution<T> {
        Contribution {
            module: self.current,
            priority,
            item,
        }
    }

    pub fn module_key(&self) -> &'static str {
        self.current
    }

    fn route(&mut self, mount: RouteMount, factory: RouteFactory) -> &mut Self {
        let contribution =
            self.contribution(DEFAULT_PRIORITY, RouteContribution { mount, factory });
        self.routes.push(contribution);
        self
    }

    pub fn routes<F>(&mut self, factory: F) -> &mut Self
    where
        F: Fn(&AppState) -> OpenApiRouter<AppState> + Send + Sync + 'static,
    {
        self.route(RouteMount::Api, Arc::new(factory))
    }

    pub fn public_routes<F>(&mut self, factory: F) -> &mut Self
    where
        F: Fn(&AppState) -> OpenApiRouter<AppState> + Send + Sync + 'static,
    {
        self.route(RouteMount::Public, Arc::new(factory))
    }

    pub fn root_routes<F>(&mut self, factory: F) -> &mut Self
    where
        F: Fn(&AppState) -> OpenApiRouter<AppState> + Send + Sync + 'static,
    {
        self.route(RouteMount::Root, Arc::new(factory))
    }

    pub fn on<K, F>(&mut self, hook: K, priority: i32, handler: F) -> &mut Self
    where
        K: HookRef,
        F: for<'r, 'c> Fn(&'r mut HookCtx<'c>, &'r K::Args) -> BoxFuture<'r, Result<(), AppError>>
            + Send
            + Sync
            + 'static,
    {
        let handler: ActionFn<K::Args> = Arc::new(handler);
        let contribution = self.contribution(priority, handler);
        self.hooks.on(hook, contribution);
        self
    }

    pub fn filter<K, F>(&mut self, hook: K, priority: i32, handler: F) -> &mut Self
    where
        K: HookRef,
        F: for<'r, 'c> Fn(&'r mut HookCtx<'c>, K::Args) -> BoxFuture<'r, Result<K::Args, AppError>>
            + Send
            + Sync
            + 'static,
    {
        let handler: FilterFn<K::Args> = Arc::new(handler);
        let contribution = self.contribution(priority, handler);
        self.hooks.filter(hook, contribution);
        self
    }

    pub fn forward_hook<K>(&mut self, hook: K, plugin_hook: PluginHook) -> &mut Self
    where
        K: HookRef,
        K::Args: PluginHookArgs,
    {
        self.forward_hook_with(hook, plugin_hook, HookForwarding::of())
    }

    pub fn forward_hook_with<K>(
        &mut self,
        hook: K,
        plugin_hook: PluginHook,
        forwarding: HookForwarding<K::Args>,
    ) -> &mut Self
    where
        K: HookRef,
    {
        self.contribute::<PluginHook>(Arc::new(plugin_hook));
        self.on(hook, PLUGIN_HOOK_PRIORITY, move |ctx, args| {
            Box::pin(async move {
                crate::modules::plugin_bridge::forward_args(ctx, plugin_hook, forwarding, args)
                    .await
                    .map(drop)
            })
        })
    }

    pub fn forward_filter_hook<K>(&mut self, hook: K, plugin_hook: PluginHook) -> &mut Self
    where
        K: HookRef,
        K::Args: PluginHookFilter,
    {
        self.forward_filter_hook_with(hook, plugin_hook, HookForwarding::filtering())
    }

    pub fn forward_filter_hook_with<K>(
        &mut self,
        hook: K,
        plugin_hook: PluginHook,
        forwarding: HookForwarding<K::Args>,
    ) -> &mut Self
    where
        K: HookRef,
    {
        self.contribute::<PluginHook>(Arc::new(plugin_hook));
        self.filter(hook, PLUGIN_HOOK_PRIORITY, move |ctx, args| {
            Box::pin(crate::modules::plugin_bridge::forward_filtered(
                ctx,
                plugin_hook,
                forwarding,
                args,
            ))
        })
    }

    pub fn declare_hook<K: HookRef>(
        &mut self,
        hook: K,
        usages: &'static [HookUsage],
        description: &'static str,
    ) -> &mut Self {
        self.hooks.declare(hook, self.current, usages, description);
        self
    }

    pub fn provide<T>(&mut self, service: Arc<T>) -> &mut Self
    where
        T: ?Sized + Send + Sync + 'static,
    {
        self.services.provide(self.current, service);
        self
    }

    pub fn decorate<T, F>(&mut self, wrap: F) -> Result<&mut Self, AppError>
    where
        T: ?Sized + Send + Sync + 'static,
        F: FnOnce(Arc<T>) -> Arc<T>,
    {
        self.services.decorate(self.current, wrap)?;
        Ok(self)
    }

    pub fn permission(&mut self, definition: PermissionDefinition) -> PermissionEntry<'_> {
        let permission = definition.permission.clone();
        let contribution = self.contribution(DEFAULT_PRIORITY, definition);
        self.catalog.permissions.push(contribution);
        PermissionEntry {
            builder: self,
            permission,
        }
    }

    pub fn public_event(&mut self, definition: PublicEventDefinition) -> &mut Self {
        let contribution = self.contribution(DEFAULT_PRIORITY, definition);
        self.catalog.public_events.push(contribution);
        self
    }

    pub fn openapi_tag(&mut self, name: &'static str, description: &'static str) -> &mut Self {
        let contribution = self.contribution(DEFAULT_PRIORITY, OpenApiTag { name, description });
        self.catalog.tags.push(contribution);
        self
    }

    pub fn worker<F>(&mut self, factory: F) -> &mut Self
    where
        F: Fn(&WorkerEnv) -> WorkerParts + Send + Sync + 'static,
    {
        let contribution = self.contribution(DEFAULT_PRIORITY, Arc::new(factory) as WorkerFactory);
        self.workers.push(contribution);
        self
    }

    pub fn job<F>(&mut self, factory: F) -> &mut Self
    where
        F: Fn(&WorkerEnv) -> Arc<dyn JobHandler> + Send + Sync + 'static,
    {
        self.worker(move |env| WorkerParts::default().handler(factory(env)))
    }

    pub fn consumer<F>(&mut self, factory: F) -> &mut Self
    where
        F: Fn(&WorkerEnv) -> Arc<dyn EventConsumer> + Send + Sync + 'static,
    {
        self.worker(move |env| WorkerParts::default().consumer(factory(env)))
    }

    pub fn entity(&mut self, mut definition: EntityDefinition) -> &mut Self {
        if !definition.key.contains('.') {
            definition.key = format!("{}.{}", self.current, definition.key);
        }
        let contribution = self.contribution(DEFAULT_PRIORITY, definition);
        self.record_types.push(contribution);
        self
    }

    pub fn transform_entities(
        &mut self,
        key: &'static str,
        apply: fn(&mut EntityDefinition) -> Result<(), AppError>,
    ) -> &mut Self {
        self.entity_transforms
            .push(crate::application::fields::RegistryTransform {
                module: self.current,
                key,
                apply,
            });
        self
    }

    pub fn extend<F>(&mut self, record_type: &str, extend: F) -> &mut Self
    where
        F: FnOnce(&mut RecordTypeExtension),
    {
        let mut extension = RecordTypeExtension::new(self.current, record_type);
        extend(&mut extension);
        let contribution = self.contribution(DEFAULT_PRIORITY, extension);
        self.record_extensions.push(contribution);
        self
    }

    pub fn custom_record_handler<F>(&mut self, factory: F) -> &mut Self
    where
        F: Fn(&RecordType) -> Arc<dyn RecordTypeHandler> + Send + Sync + 'static,
    {
        if self.custom_record_handler.is_some() {
            tracing::info!(replacement = self.current, "custom record handler replaced");
        }
        self.custom_record_handler = Some(RecordHandlerFactory::new(Arc::new(factory)));
        self
    }

    pub fn contribute<T>(&mut self, item: Arc<T>) -> &mut Self
    where
        T: ?Sized + Send + Sync + 'static,
    {
        let contribution = self.contribution(
            DEFAULT_PRIORITY,
            Box::new(item) as Box<dyn Any + Send + Sync>,
        );
        self.contributions
            .entry(TypeId::of::<Arc<T>>())
            .or_default()
            .push(contribution);
        self
    }

    pub fn contract(&mut self, descriptor: aphel_contract::ContractDescriptor) -> &mut Self {
        self.contribute::<aphel_contract::ContractDescriptor>(Arc::new(descriptor))
    }

    pub fn operation<I, O, F>(
        &mut self,
        key: aphel_contract::OperationKey<I, O>,
        spec: crate::modules::operations::OperationSpec,
        handler: F,
    ) -> &mut Self
    where
        I: serde::de::DeserializeOwned + 'static,
        O: serde::Serialize + 'static,
        F: for<'a, 'b> Fn(&'a crate::Ctx<'b>, I) -> BoxFuture<'a, Result<O, AppError>>
            + Send
            + Sync
            + 'static,
    {
        match crate::modules::operations::entry(key, spec, handler) {
            Ok(entry) => {
                let projection = entry.plugin_operation();
                let entry = Arc::new(entry);
                self.contribute::<crate::modules::operations::OperationEntry>(entry);
                self.contribute::<crate::modules::plugin_bridge::PluginOperation>(Arc::new(
                    projection,
                ));
            }
            Err(error) => tracing::error!("operation {key} not registered: {error}"),
        }
        self
    }

    pub fn schedule(&mut self, schedule: ScheduledJobDefinition) -> &mut Self {
        self.worker(move |_| WorkerParts::default().schedule(schedule.clone()))
    }

    pub fn build(mut self) -> Result<App, AppError> {
        let mut entries: Vec<(Box<dyn Module>, Manifest)> = Vec::new();
        for module in std::mem::take(&mut self.pending) {
            let manifest = module.manifest();
            if entries
                .iter()
                .any(|(_, existing)| existing.key() == manifest.key())
            {
                tracing::error!("module {} is compiled twice; skipped", manifest.key());
                continue;
            }
            entries.push((module, manifest));
        }
        let compatibility = compatibilities(&entries);
        let order = load_order(&entries)?;
        let mut slots: Vec<Option<(Box<dyn Module>, Manifest)>> =
            entries.into_iter().map(Some).collect();
        let mut modules: Vec<LoadedModule> = Vec::new();
        for index in order {
            let (module, manifest) = slots[index].take().expect("each module is loaded once");
            modules.push(LoadedModule {
                module,
                manifest,
                compatibility: compatibility[index].clone(),
            });
        }

        let enabled = Arc::new(EnabledModules::new(
            modules
                .iter()
                .filter(|entry| entry.manifest.required)
                .map(LoadedModule::key),
            modules.iter().map(LoadedModule::key),
        ));
        self.enabled = enabled.clone();
        self.hooks = HookBus::new(enabled.clone());
        self.services = Services::new(enabled.clone());

        for entry in &modules {
            if !entry.is_compatible() {
                tracing::warn!(
                    module = entry.key(),
                    reason = entry.compatibility.code(),
                    "module not loaded: incompatible"
                );
                continue;
            }
            self.current = entry.key();
            entry.module.register(&mut self).map_err(|error| {
                AppError::Configuration(format!(
                    "module {} failed to register: {error}",
                    entry.key()
                ))
            })?;
            self.loaded.push(entry.key());
        }
        self.catalog.freeze();
        let record_contributions = describe_records(&self.record_types, &self.record_extensions);
        let record_types = Arc::new(assemble_registry(
            std::mem::take(&mut self.record_types),
            std::mem::take(&mut self.record_extensions),
            std::mem::take(&mut self.entity_transforms),
            self.custom_record_handler.take(),
        )?);

        for definition in record_types.definitions() {
            for field in &definition.system_fields {
                if let (Some(extension), Some(owner)) = (&field.extension, field.contributed_by)
                    && let Some(entry) = modules.iter_mut().find(|entry| entry.key() == owner)
                {
                    let table = crate::modules::intern::intern(&extension.table);
                    if !entry.manifest.owned_tables.contains(&table) {
                        entry.manifest.base.owned_tables.push(table);
                    }
                }
            }
        }
        let app = App {
            record_types,
            record_contributions,
            modules,
            routes: self.routes,
            hooks: self.hooks,
            services: Arc::new(self.services),
            catalog: self.catalog,
            workers: self.workers,

            contributions: self.contributions,
            runtime: RwLock::new(HashMap::new()),
            enabled,
        };
        crate::application::fields::field_types::validate_registry(&app)?;
        Ok(app)
    }
}

fn compatibilities(entries: &[(Box<dyn Module>, Manifest)]) -> Vec<Compatibility> {
    let aphel = semver::Version::parse(APHEL_VERSION).ok();
    let versions: HashMap<&'static str, Option<semver::Version>> = entries
        .iter()
        .map(|(_, manifest)| {
            (
                manifest.key(),
                semver::Version::parse(manifest.version).ok(),
            )
        })
        .collect();
    let mut result: Vec<Compatibility> = entries
        .iter()
        .map(|(_, manifest)| compatibility(manifest, aphel.as_ref(), &versions))
        .collect();
    loop {
        let broken: Vec<&'static str> = entries
            .iter()
            .zip(&result)
            .filter(|(_, compatibility)| !compatibility.is_compatible())
            .map(|((_, manifest), _)| manifest.key())
            .collect();
        let mut changed = false;
        for ((_, manifest), compatibility) in entries.iter().zip(result.iter_mut()) {
            if !compatibility.is_compatible() {
                continue;
            }
            if let Some(dependency) = manifest
                .dependencies
                .iter()
                .find(|dependency| broken.contains(&dependency.key))
            {
                *compatibility = Compatibility::IncompatibleDependency {
                    key: dependency.key.to_owned(),
                    requirement: dependency.version_req.to_owned(),
                };
                changed = true;
            }
        }
        if !changed {
            return result;
        }
    }
}

fn compatibility(
    manifest: &Manifest,
    aphel: Option<&semver::Version>,
    versions: &HashMap<&'static str, Option<semver::Version>>,
) -> Compatibility {
    let required = semver::Version::parse(manifest.min_aphel_version).ok();
    if let (Some(aphel), Some(required)) = (aphel, required.as_ref())
        && aphel < required
    {
        return Compatibility::RequiresAphel(manifest.min_aphel_version.to_owned());
    }
    for dependency in &manifest.dependencies {
        let Some(version) = versions.get(dependency.key) else {
            return Compatibility::MissingDependency(dependency.key.to_owned());
        };
        let matches = semver::VersionReq::parse(dependency.version_req)
            .ok()
            .zip(version.as_ref())
            .is_some_and(|(requirement, version)| requirement.matches(version));
        if !matches {
            return Compatibility::IncompatibleDependency {
                key: dependency.key.to_owned(),
                requirement: dependency.version_req.to_owned(),
            };
        }
    }
    Compatibility::Compatible
}

fn load_order(entries: &[(Box<dyn Module>, Manifest)]) -> Result<Vec<usize>, AppError> {
    let index: HashMap<&'static str, usize> = entries
        .iter()
        .enumerate()
        .map(|(position, (_, manifest))| (manifest.key(), position))
        .collect();
    let mut incoming: Vec<BTreeSet<usize>> = vec![BTreeSet::new(); entries.len()];
    for (position, (_, manifest)) in entries.iter().enumerate() {
        for dependency in manifest.load_after() {
            if let Some(&before) = index.get(dependency)
                && before != position
            {
                incoming[position].insert(before);
            }
        }
        if manifest.key() != KERNEL_MODULE
            && let Some(&kernel) = index.get(KERNEL_MODULE)
        {
            incoming[position].insert(kernel);
        }
    }
    let mut ready: BTreeSet<(&'static str, usize)> = entries
        .iter()
        .enumerate()
        .filter(|(position, _)| incoming[*position].is_empty())
        .map(|(position, (_, manifest))| (manifest.key(), position))
        .collect();
    let mut order: Vec<usize> = Vec::with_capacity(entries.len());
    while let Some(next) = ready.pop_first() {
        let (_, position) = next;
        order.push(position);
        for (candidate, (_, manifest)) in entries.iter().enumerate() {
            if incoming[candidate].remove(&position) && incoming[candidate].is_empty() {
                ready.insert((manifest.key(), candidate));
            }
        }
    }
    if order.len() != entries.len() {
        let cycle: Vec<&'static str> = entries
            .iter()
            .enumerate()
            .filter(|(position, _)| !order.contains(position))
            .map(|(_, (_, manifest))| manifest.key())
            .collect();
        return Err(AppError::conflict_code(
            MODULE_DEPENDENCY_CYCLE,
            format!(
                "modules depend on each other in a cycle: {}",
                cycle.join(", ")
            ),
        ));
    }
    Ok(order)
}

pub struct App {
    record_types: Arc<RecordTypeRegistry>,
    record_contributions: Vec<RecordContributionInfo>,
    modules: Vec<LoadedModule>,
    routes: Vec<Contribution<RouteContribution>>,
    hooks: HookBus,
    services: Arc<Services>,
    catalog: Catalog,
    workers: Vec<Contribution<WorkerFactory>>,

    contributions: Contributions,
    runtime: RwLock<HashMap<&'static str, Contributions>>,
    enabled: Arc<EnabledModules>,
}

#[derive(Default)]
pub struct RuntimeBatch {
    items: HashMap<TypeId, Vec<Box<dyn Any + Send + Sync>>>,
}

impl RuntimeBatch {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_operation(
        &mut self,
        app: &App,
        entry: crate::modules::operations::OperationEntry,
    ) -> Result<(), AppError> {
        if app.has_native_operation(entry.key) {
            return Err(AppError::validation_code(
                "operation_key_taken",
                format!("operation `{}` is already provided natively", entry.key),
            ));
        }
        let projection = entry.plugin_operation();
        self.add(Arc::new(entry));
        self.add(Arc::new(projection));
        Ok(())
    }

    pub fn add<T>(&mut self, item: Arc<T>) -> &mut Self
    where
        T: ?Sized + Send + Sync + 'static,
    {
        self.items
            .entry(TypeId::of::<Arc<T>>())
            .or_default()
            .push(Box::new(item));
        self
    }
}

impl App {
    pub fn replace_runtime(&self, owner: &'static str, batch: RuntimeBatch) {
        let contributions: Contributions = batch
            .items
            .into_iter()
            .map(|(type_id, items)| {
                let items = items
                    .into_iter()
                    .map(|item| Contribution {
                        module: owner,
                        priority: DEFAULT_PRIORITY,
                        item,
                    })
                    .collect();
                (type_id, items)
            })
            .collect();
        let mut runtime = self
            .runtime
            .write()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        if contributions.is_empty() {
            runtime.remove(owner);
        } else {
            runtime.insert(owner, contributions);
        }
    }

    pub fn modules(&self) -> &[LoadedModule] {
        &self.modules
    }

    pub fn module(&self, key: &str) -> Option<&LoadedModule> {
        self.modules.iter().find(|entry| entry.key() == key)
    }

    pub fn has_module(&self, key: &str) -> bool {
        self.module(key).is_some_and(LoadedModule::is_compatible)
    }

    pub fn record_types(&self) -> &Arc<RecordTypeRegistry> {
        &self.record_types
    }

    pub fn record_contributions(&self) -> &[RecordContributionInfo] {
        &self.record_contributions
    }

    pub fn hooks(&self) -> &HookBus {
        &self.hooks
    }

    pub fn shared_services(&self) -> Arc<Services> {
        self.services.clone()
    }

    pub fn services(&self) -> &Services {
        &self.services
    }

    pub fn catalog(&self) -> &Catalog {
        &self.catalog
    }

    pub fn enabled(&self) -> &Arc<EnabledModules> {
        &self.enabled
    }

    pub fn is_enabled(&self, key: &str) -> bool {
        self.enabled.is_enabled(key)
    }

    pub fn route_contributions(&self) -> &[Contribution<RouteContribution>] {
        &self.routes
    }

    pub fn worker_contributions(&self) -> &[Contribution<WorkerFactory>] {
        &self.workers
    }

    pub fn all_contributions<T>(&self) -> Vec<(&'static str, Arc<T>)>
    where
        T: ?Sized + Send + Sync + 'static,
    {
        fn collect<T: ?Sized + Send + Sync + 'static>(
            contributions: &Contributions,
            into: &mut Vec<(&'static str, Arc<T>)>,
        ) {
            into.extend(
                contributions
                    .get(&TypeId::of::<Arc<T>>())
                    .into_iter()
                    .flatten()
                    .filter_map(|contribution| {
                        contribution
                            .item
                            .downcast_ref::<Arc<T>>()
                            .map(|item| (contribution.module, item.clone()))
                    }),
            );
        }
        let mut found = Vec::new();
        collect(&self.contributions, &mut found);
        let runtime = self
            .runtime
            .read()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let mut owners: Vec<_> = runtime.keys().copied().collect();
        owners.sort_unstable();
        for owner in owners {
            if let Some(contributions) = runtime.get(owner) {
                collect(contributions, &mut found);
            }
        }
        found
    }

    pub fn runtime_contributions(
        &self,
        point: &str,
    ) -> Vec<Arc<crate::modules::runtime_contribution::RuntimeContribution>> {
        self.contributions::<crate::modules::runtime_contribution::RuntimeContribution>()
            .into_iter()
            .filter(|contribution| contribution.point == point)
            .collect()
    }

    pub fn has_native_operation(&self, key: &str) -> bool {
        self.contributions
            .get(&TypeId::of::<Arc<crate::modules::operations::OperationEntry>>())
            .into_iter()
            .flatten()
            .filter_map(|contribution| {
                contribution
                    .item
                    .downcast_ref::<Arc<crate::modules::operations::OperationEntry>>()
            })
            .any(|entry| entry.key == key)
    }

    pub fn operation(&self, key: &str) -> Option<Arc<crate::modules::operations::OperationEntry>> {
        self.contributions::<crate::modules::operations::OperationEntry>()
            .into_iter()
            .find(|entry| entry.key == key)
    }

    pub fn operations(
        &self,
    ) -> Vec<(
        &'static str,
        Arc<crate::modules::operations::OperationEntry>,
    )> {
        self.all_contributions::<crate::modules::operations::OperationEntry>()
    }

    pub fn contributions<T>(&self) -> Vec<Arc<T>>
    where
        T: ?Sized + Send + Sync + 'static,
    {
        self.all_contributions::<T>()
            .into_iter()
            .filter(|(module, _)| self.is_enabled(module))
            .map(|(_, item)| item)
            .collect()
    }

    pub fn worker_parts_of(&self, module: &str, env: &WorkerEnv) -> WorkerParts {
        self.workers
            .iter()
            .filter(|contribution| contribution.module == module)
            .fold(WorkerParts::default(), |parts, contribution| {
                parts.merge((contribution.item)(env))
            })
    }
}
