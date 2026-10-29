use std::sync::Arc;

use serde::Deserialize;
use serde_json::{Value, json};

use crate::http::{GoalListParams, GoalResponse};
use crate::service;
use aphel_kernel::Ctx;
use aphel_kernel::modules::{
    OperationFuture, OperationPage, PluginOperation, operation_input, operation_output,
};

#[derive(Deserialize)]
struct ListInput {
    #[serde(flatten)]
    filter: GoalListParams,
    #[serde(flatten)]
    page: OperationPage,
}

#[derive(Deserialize)]
struct GoalId {
    id: String,
}

fn list<'a>(ctx: &'a Ctx<'_>, input: Value) -> OperationFuture<'a> {
    Box::pin(async move {
        let input: ListInput = operation_input(input)?;
        let listing = service::list(ctx, input.filter.into_query(), input.page.window()).await?;
        let goals: Vec<GoalResponse> = listing.items.into_iter().map(Into::into).collect();
        Ok(json!({ "data": operation_output(goals)?, "total": listing.total }))
    })
}

fn get<'a>(ctx: &'a Ctx<'_>, input: Value) -> OperationFuture<'a> {
    Box::pin(async move {
        let input: GoalId = operation_input(input)?;
        operation_output(GoalResponse::from(service::get(ctx, &input.id).await?))
    })
}

pub(crate) fn operations() -> Vec<Arc<PluginOperation>> {
    vec![
        Arc::new(PluginOperation::read(
            "goals.list",
            "Goals the actor reaches, newest period first, each with its progress: { owner_type?: user|team|organization, user_id?, team_id?, metric?, period?, active_on? (YYYY-MM-DD), current?, progress? (default true), page?, per_page? } -> { data, total }",
            list,
        )),
        Arc::new(PluginOperation::read(
            "goals.get",
            "One goal with its progress: { id }",
            get,
        )),
    ]
}
