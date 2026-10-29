use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecordSnapshot {
    pub values: Map<String, Value>,
    pub custom_fields: Map<String, Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HookState {
    pub operation_id: String,
    pub record_type: String,
    pub record_id: String,
    pub phase: String,
    pub before: Option<RecordSnapshot>,
    pub before_available: bool,
    pub proposed: Option<RecordSnapshot>,
    pub persisted: Option<RecordSnapshot>,
    pub effects_allowed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionActor {
    pub kind: String,
    pub id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionContext {
    pub id: String,
    pub correlation_id: String,
    pub parent_id: Option<String>,
    pub event_id: Option<String>,
    pub executor: String,
    pub executor_kind: String,
    pub automation: Option<AutomationExecution>,
    pub initiator: Option<ExecutionActor>,
    pub trigger: String,
    pub chain: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum EffectCommand {
    Create {
        record_type: String,
        values: Value,
    },
    Update {
        record_type: String,
        id: String,
        values: Value,
    },
    Delete {
        record_type: String,
        id: String,
    },
    Archive {
        record_type: String,
        id: String,
    },
    Restore {
        record_type: String,
        id: String,
    },
    KvSet {
        collection: String,
        id: String,
        value: Value,
    },
    KvDelete {
        collection: String,
        id: String,
    },
    Emit {
        name: String,
        record_type: Option<String>,
        record_id: Option<String>,
        payload: Value,
    },
    Notify {
        name: String,
        recipient_user_id: String,
        title: String,
        body: Option<String>,
        record_type: Option<String>,
        record_id: Option<String>,
        link: Option<String>,
        data: Value,
    },
    Invoke {
        operation: String,
        input: Value,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutomationExecution {
    pub automation_id: String,
    pub run_id: String,
    pub step_id: String,
}
