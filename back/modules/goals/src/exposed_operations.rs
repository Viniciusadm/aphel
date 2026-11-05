use aphel_kernel::modules::OperationSpec;
use aphel_sdk::{OperationKey, operation};

use serde::Deserialize;
use serde_json::{Value, json};

use crate::http::{GoalListParams, GoalResponse};
use crate::service;
use aphel_kernel::Ctx;
use aphel_kernel::modules::{OperationFuture, OperationPage, operation_input, operation_output};

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

const OP_GOALS_LIST: OperationKey<Value, Value> = operation!("goals.list");
const OP_GOALS_GET: OperationKey<Value, Value> = operation!("goals.get");

pub(crate) fn register(app: &mut aphel_kernel::modules::AppBuilder) {
    app.operation(OP_GOALS_LIST, OperationSpec::read("Goals the actor reaches, newest period first, each with its progress: { owner_type?: user|team|organization, user_id?, team_id?, metric?, period?, active_on? (YYYY-MM-DD), current?, progress? (default true), page?, per_page? } -> { data, total }"), list);
    app.operation(
        OP_GOALS_GET,
        OperationSpec::read("One goal with its progress: { id }"),
        get,
    );
}
