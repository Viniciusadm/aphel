use aphel_kernel::modules::OperationSpec;
use aphel_sdk::{OperationKey, operation};

use serde::Deserialize;
use serde_json::{Value, json};

use crate::http::{CreateFavoriteRequest, FavoriteResponse};
use crate::service;
use aphel_kernel::Ctx;
use aphel_kernel::errors::AppError;
use aphel_kernel::modules::{OperationFuture, operation_input, operation_output};

#[derive(Deserialize)]
struct FavoriteId {
    id: String,
}

fn require_user_actor(ctx: &Ctx<'_>) -> Result<(), AppError> {
    if ctx.scope.plugin_key().is_some() {
        return Err(AppError::Validation(
            "this operation needs a user actor: call it from a hook or an authenticated route"
                .into(),
        ));
    }
    Ok(())
}

fn list<'a>(ctx: &'a Ctx<'_>, _input: Value) -> OperationFuture<'a> {
    Box::pin(async move {
        require_user_actor(ctx)?;
        let favorites: Vec<FavoriteResponse> = service::list(ctx)
            .await?
            .into_iter()
            .map(Into::into)
            .collect();
        Ok(json!({ "data": operation_output(favorites)? }))
    })
}

fn add<'a>(ctx: &'a Ctx<'_>, input: Value) -> OperationFuture<'a> {
    Box::pin(async move {
        require_user_actor(ctx)?;
        let request: CreateFavoriteRequest = operation_input(input)?;
        operation_output(FavoriteResponse::from(
            service::create(ctx, request.into()).await?,
        ))
    })
}

fn remove<'a>(ctx: &'a Ctx<'_>, input: Value) -> OperationFuture<'a> {
    Box::pin(async move {
        require_user_actor(ctx)?;
        let input: FavoriteId = operation_input(input)?;
        service::delete(ctx, &input.id).await?;
        Ok(json!({ "deleted": true }))
    })
}

const OP_FAVORITES_LIST: OperationKey<Value, Value> = operation!("favorites.list");
const OP_FAVORITES_ADD: OperationKey<Value, Value> = operation!("favorites.add");
const OP_FAVORITES_REMOVE: OperationKey<Value, Value> = operation!("favorites.remove");

pub(crate) fn register(app: &mut aphel_kernel::modules::AppBuilder) {
    app.operation(
        OP_FAVORITES_LIST,
        OperationSpec::read(
            "Favorites of the acting user in display order (needs a user actor): {} -> { data }",
        ),
        list,
    );
    app.operation(OP_FAVORITES_ADD, OperationSpec::write("Add a favorite for the acting user (needs a user actor): { favorite_type: record|view, record_type?, record_id?, view_id? }").durable(), add);
    app.operation(
        OP_FAVORITES_REMOVE,
        OperationSpec::write("Remove a favorite of the acting user (needs a user actor): { id }")
            .durable(),
        remove,
    );
}
