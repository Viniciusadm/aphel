use std::any::{Any, type_name};
use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use aphel_contract::HookKey;
use sqlx::MySqlPool;

use crate::database::transaction::MySqlTx;
use crate::domain::access::AccessScope;
use crate::errors::AppError;
use crate::modules::builder::Contribution;
use crate::modules::enabled::EnabledModules;
use crate::modules::services::Services;

pub const DEFAULT_PRIORITY: i32 = 10;

pub trait HookRef: Copy {
    type Args: Send + Sync + 'static;

    fn key(&self) -> &'static str;

    fn kind(&self) -> Option<aphel_contract::HookKind> {
        None
    }
}

impl<H: Hook + Copy> HookRef for H {
    type Args = H::Args;

    fn key(&self) -> &'static str {
        H::key()
    }
}

impl<A: Send + Sync + 'static> HookRef for HookKey<A> {
    type Args = A;

    fn key(&self) -> &'static str {
        HookKey::key(self)
    }

    fn kind(&self) -> Option<aphel_contract::HookKind> {
        Some(HookKey::kind(self))
    }
}

pub type BoxFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

pub trait Hook: 'static {
    type Args: Send + Sync + 'static;

    fn key() -> &'static str {
        type_name::<Self>()
    }
}

pub struct HookCtx<'a> {
    pub tx: &'a mut MySqlTx,
    pub pool: &'a MySqlPool,
    pub scope: &'a AccessScope,
    pub services: &'a Services,
    pub module: &'a str,
}

impl<'a> HookCtx<'a> {
    pub fn new(
        tx: &'a mut MySqlTx,
        pool: &'a MySqlPool,
        scope: &'a AccessScope,
        services: &'a Services,
        module: &'a str,
    ) -> Self {
        Self {
            tx,
            pool,
            scope,
            services,
            module,
        }
    }

    pub fn tx(&mut self) -> &mut MySqlTx {
        self.tx
    }
}

pub type ActionFn<A> = Arc<
    dyn for<'r, 'c> Fn(&'r mut HookCtx<'c>, &'r A) -> BoxFuture<'r, Result<(), AppError>>
        + Send
        + Sync,
>;

pub type FilterFn<A> = Arc<
    dyn for<'r, 'c> Fn(&'r mut HookCtx<'c>, A) -> BoxFuture<'r, Result<A, AppError>> + Send + Sync,
>;

pub type ActionHandler<H> = ActionFn<<H as Hook>::Args>;

pub type FilterHandler<H> = FilterFn<<H as Hook>::Args>;

struct Handlers<A> {
    actions: Vec<Contribution<ActionFn<A>>>,
    filters: Vec<Contribution<FilterFn<A>>>,
}

impl<A> Default for Handlers<A> {
    fn default() -> Self {
        Self {
            actions: Vec::new(),
            filters: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HookKind {
    Action,
    Filter,
}

impl HookKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Action => "action",
            Self::Filter => "filter",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HookUsage {
    Action,
    Veto,
    Filter,
}

impl HookUsage {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Action => "action",
            Self::Veto => "veto",
            Self::Filter => "filter",
        }
    }

    pub fn registration(&self) -> &'static str {
        match self {
            Self::Action | Self::Veto => "app.on",
            Self::Filter => "app.filter",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HookDeclaration {
    pub hook: &'static str,
    pub args: &'static str,
    pub module: &'static str,
    pub usages: &'static [HookUsage],
    pub description: &'static str,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HookInfo {
    pub hook: &'static str,
    pub kind: HookKind,
    pub module: &'static str,
    pub priority: i32,
}

pub struct HookBus {
    handlers: HashMap<&'static str, Box<dyn Any + Send + Sync>>,
    registered: Vec<HookInfo>,
    declared: Vec<HookDeclaration>,
    enabled: Arc<EnabledModules>,
}

impl HookBus {
    pub fn new(enabled: Arc<EnabledModules>) -> Self {
        Self {
            handlers: HashMap::new(),
            registered: Vec::new(),
            declared: Vec::new(),
            enabled,
        }
    }

    fn entry<A: Send + Sync + 'static>(&mut self, key: &'static str) -> &mut Handlers<A> {
        self.handlers
            .entry(key)
            .or_insert_with(|| Box::new(Handlers::<A>::default()))
            .downcast_mut::<Handlers<A>>()
            .unwrap_or_else(|| panic!("hook `{key}` is used with two different argument types"))
    }

    fn handlers<A: Send + Sync + 'static>(&self, key: &'static str) -> Option<&Handlers<A>> {
        self.handlers
            .get(key)
            .and_then(|handlers| handlers.downcast_ref::<Handlers<A>>())
    }

    fn declare_entry(
        &mut self,
        key: &'static str,
        args: &'static str,
        module: &'static str,
        usages: &'static [HookUsage],
        description: &'static str,
    ) {
        if self
            .declared
            .iter()
            .any(|declaration| declaration.hook == key)
        {
            return;
        }
        self.declared.push(HookDeclaration {
            hook: key,
            args,
            module,
            usages,
            description,
        });
    }

    pub fn declare<K: HookRef>(
        &mut self,
        hook: K,
        module: &'static str,
        usages: &'static [HookUsage],
        description: &'static str,
    ) {
        self.declare_entry(
            hook.key(),
            type_name::<K::Args>(),
            module,
            usages,
            description,
        );
    }

    fn register_action<A: Send + Sync + 'static>(
        &mut self,
        key: &'static str,
        contribution: Contribution<ActionFn<A>>,
    ) {
        self.registered.push(HookInfo {
            hook: key,
            kind: HookKind::Action,
            module: contribution.module,
            priority: contribution.priority,
        });
        let actions = &mut self.entry::<A>(key).actions;
        actions.push(contribution);
        actions.sort_by_key(|contribution| contribution.priority);
    }

    fn register_filter<A: Send + Sync + 'static>(
        &mut self,
        key: &'static str,
        contribution: Contribution<FilterFn<A>>,
    ) {
        self.registered.push(HookInfo {
            hook: key,
            kind: HookKind::Filter,
            module: contribution.module,
            priority: contribution.priority,
        });
        let filters = &mut self.entry::<A>(key).filters;
        filters.push(contribution);
        filters.sort_by_key(|contribution| contribution.priority);
    }

    pub fn on<K: HookRef>(&mut self, hook: K, contribution: Contribution<ActionFn<K::Args>>) {
        if hook.kind() == Some(aphel_contract::HookKind::Filter) {
            tracing::error!(
                "hook {} is a filter and cannot take an action handler",
                hook.key()
            );
            return;
        }
        self.register_action(hook.key(), contribution);
    }

    pub fn filter<K: HookRef>(&mut self, hook: K, contribution: Contribution<FilterFn<K::Args>>) {
        if hook
            .kind()
            .is_some_and(|kind| kind != aphel_contract::HookKind::Filter)
        {
            tracing::error!(
                "hook {} is not a filter and cannot take a filter handler",
                hook.key()
            );
            return;
        }
        self.register_filter(hook.key(), contribution);
    }

    pub fn has<K: HookRef>(&self, hook: K) -> bool {
        self.has_except(hook, None)
    }

    pub fn has_except<K: HookRef>(&self, hook: K, excluded_module: Option<&str>) -> bool {
        self.has_kind(hook.key(), excluded_module, HookKind::Action)
            || self.has_kind(hook.key(), excluded_module, HookKind::Filter)
    }

    pub fn has_actions_except<K: HookRef>(&self, hook: K, excluded_module: Option<&str>) -> bool {
        self.has_kind(hook.key(), excluded_module, HookKind::Action)
    }

    pub fn has_filters_except<K: HookRef>(&self, hook: K, excluded_module: Option<&str>) -> bool {
        self.has_kind(hook.key(), excluded_module, HookKind::Filter)
    }

    fn has_kind(&self, key: &str, excluded_module: Option<&str>, kind: HookKind) -> bool {
        self.registered.iter().any(|entry| {
            entry.hook == key
                && entry.kind == kind
                && excluded_module != Some(entry.module)
                && self.enabled.is_enabled(entry.module)
        })
    }

    async fn run_entry<A: Send + Sync + 'static>(
        &self,
        key: &'static str,
        ctx: &mut HookCtx<'_>,
        args: &A,
    ) -> Result<(), AppError> {
        let Some(handlers) = self.handlers::<A>(key) else {
            return Ok(());
        };
        for contribution in &handlers.actions {
            if !self.enabled.is_enabled(contribution.module) {
                continue;
            }
            (contribution.item)(ctx, args).await?;
        }
        Ok(())
    }

    async fn apply_entry<A: Send + Sync + 'static>(
        &self,
        key: &'static str,
        ctx: &mut HookCtx<'_>,
        value: A,
    ) -> Result<A, AppError> {
        let Some(handlers) = self.handlers::<A>(key) else {
            return Ok(value);
        };
        let mut value = value;
        for contribution in &handlers.filters {
            if !self.enabled.is_enabled(contribution.module) {
                continue;
            }
            value = (contribution.item)(ctx, value).await?;
        }
        Ok(value)
    }

    pub async fn run<K: HookRef>(
        &self,
        hook: K,
        ctx: &mut HookCtx<'_>,
        args: &K::Args,
    ) -> Result<(), AppError> {
        self.run_entry(hook.key(), ctx, args).await
    }

    pub async fn apply<K: HookRef>(
        &self,
        hook: K,
        ctx: &mut HookCtx<'_>,
        value: K::Args,
    ) -> Result<K::Args, AppError> {
        self.apply_entry(hook.key(), ctx, value).await
    }

    pub fn run_order<K: HookRef>(&self, hook: K) -> Vec<HookInfo> {
        let Some(handlers) = self.handlers::<K::Args>(hook.key()) else {
            return Vec::new();
        };
        let info = |kind: HookKind, module: &'static str, priority: i32| HookInfo {
            hook: hook.key(),
            kind,
            module,
            priority,
        };
        handlers
            .actions
            .iter()
            .map(|entry| info(HookKind::Action, entry.module, entry.priority))
            .chain(
                handlers
                    .filters
                    .iter()
                    .map(|entry| info(HookKind::Filter, entry.module, entry.priority)),
            )
            .filter(|entry| self.enabled.is_enabled(entry.module))
            .collect()
    }

    pub fn describe(&self) -> &[HookInfo] {
        &self.registered
    }

    pub fn declarations(&self) -> &[HookDeclaration] {
        &self.declared
    }
}
