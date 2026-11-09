use std::future::Future;

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::application::shared::Actor;
use crate::errors::AppError;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
pub struct ExecutionActor {
    pub kind: String,
    pub id: Option<String>,
}

impl From<&Actor> for ExecutionActor {
    fn from(actor: &Actor) -> Self {
        Self {
            kind: actor.kind().to_owned(),
            id: actor.id().map(str::to_owned),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
pub struct AutomationExecution {
    pub automation_id: String,
    pub run_id: String,
    pub step_id: String,
}

fn plugin_kind() -> String {
    "plugin".to_owned()
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
pub struct ExecutionContext {
    #[schema(value_type = String)]
    pub id: Uuid,
    #[schema(value_type = String)]
    pub correlation_id: Uuid,
    #[schema(value_type = Option<String>)]
    pub parent_id: Option<Uuid>,
    #[schema(value_type = Option<String>)]
    pub event_id: Option<Uuid>,
    pub executor: String,
    #[serde(default = "plugin_kind")]
    pub executor_kind: String,
    #[serde(default)]
    pub automation: Option<AutomationExecution>,
    pub initiator: Option<ExecutionActor>,
    pub trigger: String,
    pub chain: Vec<String>,
    #[serde(default)]
    pub depth: u32,
}

pub const MAX_EXECUTION_DEPTH: u32 = 64;

fn too_deep() -> AppError {
    AppError::conflict_code(
        "plugin_execution_cycle",
        "the plugin execution chain is cyclic or exceeds its depth",
    )
}

tokio::task_local! {
    static CURRENT: ExecutionContext;
}

impl ExecutionContext {
    pub fn event_origin(id: Uuid, actor: &Actor) -> Self {
        Self {
            id,
            correlation_id: id,
            parent_id: None,
            event_id: Some(id),
            executor: actor.id().unwrap_or("system").to_owned(),
            executor_kind: "event".to_owned(),
            automation: None,
            initiator: (!(matches!(actor, Actor::System) || actor.is_service(Actor::PLUGIN)))
                .then(|| actor.into()),
            trigger: "event".to_owned(),
            chain: match actor {
                Actor::Service { kind, id } if kind == Actor::PLUGIN => vec![id.clone()],
                _ => vec![],
            },
            depth: 0,
        }
    }

    pub async fn origin_scope<F: Future>(&self, future: F) -> F::Output {
        let mut origin = self.clone();
        origin.executor_kind = "event".to_owned();
        origin.scope(future).await
    }

    pub fn current() -> Option<Self> {
        CURRENT.try_with(Clone::clone).ok()
    }

    pub fn plugin(
        executor: &str,
        trigger: &str,
        initiator: Option<&Actor>,
        parent: Option<&Self>,
        event_id: Option<Uuid>,
        execution_id: Uuid,
    ) -> Result<Self, AppError> {
        let mut chain = parent
            .map(|parent| parent.chain.clone())
            .unwrap_or_default();
        let depth = parent.map(|parent| parent.depth + 1).unwrap_or_default();
        if chain.iter().any(|key| key == executor)
            || chain.len() >= 16
            || depth >= MAX_EXECUTION_DEPTH
        {
            return Err(too_deep());
        }
        chain.push(executor.to_owned());
        Ok(Self {
            id: execution_id,
            correlation_id: parent
                .map(|parent| parent.correlation_id)
                .unwrap_or(execution_id),
            parent_id: parent.map(|parent| parent.id),
            event_id: event_id.or_else(|| parent.and_then(|parent| parent.event_id)),
            executor: executor.to_owned(),
            executor_kind: "plugin".to_owned(),
            automation: parent.and_then(|parent| parent.automation.clone()),
            initiator: parent
                .and_then(|parent| parent.initiator.clone())
                .or_else(|| initiator.map(ExecutionActor::from)),
            trigger: trigger.to_owned(),
            chain,
            depth,
        })
    }

    pub fn external_response(&self, executor: &str, execution_id: Uuid) -> Result<Self, AppError> {
        let depth = self.depth + 1;
        if depth >= MAX_EXECUTION_DEPTH {
            return Err(too_deep());
        }
        Ok(Self {
            id: execution_id,
            parent_id: Some(self.id),
            executor: executor.to_owned(),
            executor_kind: "plugin".to_owned(),
            trigger: "http_response".to_owned(),
            chain: vec![executor.to_owned()],
            depth,
            ..self.clone()
        })
    }

    pub async fn scope<F: Future>(&self, future: F) -> F::Output {
        CURRENT.scope(self.clone(), future).await
    }
}
