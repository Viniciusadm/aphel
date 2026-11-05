use std::sync::Arc;

use serde_json::Value;

use crate::Ctx;
use crate::modules::plugin_bridge::{OperationFuture, OperationHandler};

pub struct RuntimeContribution {
    pub owner: &'static str,
    pub point: &'static str,
    pub name: &'static str,
    pub title: Option<String>,
    pub data: Value,
    handler: Arc<dyn OperationHandler>,
}

impl RuntimeContribution {
    pub fn new(
        owner: &'static str,
        point: &'static str,
        name: &'static str,
        title: Option<String>,
        data: Value,
        handler: Arc<dyn OperationHandler>,
    ) -> Self {
        Self {
            owner,
            point,
            name,
            title,
            data,
            handler,
        }
    }

    pub fn call<'a>(&'a self, ctx: &'a Ctx<'_>, input: Value) -> OperationFuture<'a> {
        self.handler.call(ctx, input)
    }
}
