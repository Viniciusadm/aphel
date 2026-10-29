use std::sync::Arc;

use serde::Deserialize;
use serde_json::{Value, json};

use crate::http::{CreateFavoriteRequest, FavoriteResponse};
use crate::service;
use aphel_kernel::Ctx;
use aphel_kernel::errors::AppError;
use aphel_kernel::modules::{OperationFuture, PluginOperation, operation_input, operation_output};

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

pub(crate) fn operations() -> Vec<Arc<PluginOperation>> {
    vec![
        Arc::new(PluginOperation::read(
            "favorites.list",
            "Favorites of the acting user in display order (needs a user actor): {} -> { data }",
            list,
        )),
        Arc::new(PluginOperation::write(
            "favorites.add",
            "Add a favorite for the acting user (needs a user actor): { favorite_type: record|view, record_type?, record_id?, view_id? }",
            add,
        )),
        Arc::new(PluginOperation::write(
            "favorites.remove",
            "Remove a favorite of the acting user (needs a user actor): { id }",
            remove,
        )),
    ]
}
