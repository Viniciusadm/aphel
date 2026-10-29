use std::any::{Any, TypeId, type_name};
use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use sqlx::MySqlPool;

use crate::database::transaction::MySqlTx;
use crate::domain::access::AccessScope;
use crate::errors::AppError;
use crate::modules::builder::Contribution;
use crate::modules::enabled::EnabledModules;
use crate::modules::services::Services;

pub const DEFAULT_PRIORITY: i32 = 10;

pub type BoxFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

pub trait Hook: 'static {
    type Args: Send + Sync + 'static;
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

pub type ActionHandler<H> = Arc<
    dyn for<'r, 'c> Fn(
            &'r mut HookCtx<'c>,
            &'r <H as Hook>::Args,
        ) -> BoxFuture<'r, Result<(), AppError>>
        + Send
        + Sync,
>;

pub type FilterHandler<H> = Arc<
    dyn for<'r, 'c> Fn(
            &'r mut HookCtx<'c>,
            <H as Hook>::Args,
        ) -> BoxFuture<'r, Result<<H as Hook>::Args, AppError>>
        + Send
        + Sync,
>;

struct Handlers<H: Hook> {
    actions: Vec<Contribution<ActionHandler<H>>>,
    filters: Vec<Contribution<FilterHandler<H>>>,
}

impl<H: Hook> Default for Handlers<H> {
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
    handlers: HashMap<TypeId, Box<dyn Any + Send + Sync>>,
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

    fn entry<H: Hook>(&mut self) -> &mut Handlers<H> {
        self.handlers
            .entry(TypeId::of::<H>())
            .or_insert_with(|| Box::new(Handlers::<H>::default()))
            .downcast_mut::<Handlers<H>>()
            .expect("hook handlers keyed by their own type")
    }

    fn handlers<H: Hook>(&self) -> Option<&Handlers<H>> {
        self.handlers
            .get(&TypeId::of::<H>())
            .and_then(|handlers| handlers.downcast_ref::<Handlers<H>>())
    }

    pub fn declare<H: Hook>(
        &mut self,
        module: &'static str,
        usages: &'static [HookUsage],
        description: &'static str,
    ) {
        if self
            .declared
            .iter()
            .any(|declaration| declaration.hook == type_name::<H>())
        {
            return;
        }
        self.declared.push(HookDeclaration {
            hook: type_name::<H>(),
            args: type_name::<H::Args>(),
            module,
            usages,
            description,
        });
    }

    pub fn on<H: Hook>(&mut self, contribution: Contribution<ActionHandler<H>>) {
        self.registered.push(HookInfo {
            hook: type_name::<H>(),
            kind: HookKind::Action,
            module: contribution.module,
            priority: contribution.priority,
        });
        let actions = &mut self.entry::<H>().actions;
        actions.push(contribution);
        actions.sort_by_key(|contribution| contribution.priority);
    }

    pub fn filter<H: Hook>(&mut self, contribution: Contribution<FilterHandler<H>>) {
        self.registered.push(HookInfo {
            hook: type_name::<H>(),
            kind: HookKind::Filter,
            module: contribution.module,
            priority: contribution.priority,
        });
        let filters = &mut self.entry::<H>().filters;
        filters.push(contribution);
        filters.sort_by_key(|contribution| contribution.priority);
    }

    pub fn has<H: Hook>(&self) -> bool {
        self.handlers::<H>()
            .is_some_and(|handlers| !handlers.actions.is_empty() || !handlers.filters.is_empty())
    }

    pub async fn run<H: Hook>(
        &self,
        ctx: &mut HookCtx<'_>,
        args: &H::Args,
    ) -> Result<(), AppError> {
        let Some(handlers) = self.handlers::<H>() else {
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

    pub async fn apply<H: Hook>(
        &self,
        ctx: &mut HookCtx<'_>,
        value: H::Args,
    ) -> Result<H::Args, AppError> {
        let Some(handlers) = self.handlers::<H>() else {
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

    pub fn describe(&self) -> &[HookInfo] {
        &self.registered
    }

    pub fn declarations(&self) -> &[HookDeclaration] {
        &self.declared
    }
}
