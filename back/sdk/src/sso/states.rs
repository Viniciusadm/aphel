use crate::{AppError, Ctx};
use uuid::Uuid;
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewState {
    pub state: String,
    pub nonce: String,
    pub code_verifier: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoredState {
    pub purpose: String,
    pub nonce: String,
    pub code_verifier: String,
    pub user_id: Option<Uuid>,
    pub redirect_path: Option<String>,
}

#[async_trait::async_trait]
pub trait OAuthStates: Send + Sync {
    async fn create(
        &self,
        ctx: &Ctx<'_>,
        key: &str,
        purpose: &str,
        user_id: Option<Uuid>,
        redirect_path: Option<&str>,
    ) -> Result<NewState, AppError>;
    async fn consume(
        &self,
        ctx: &Ctx<'_>,
        key: &str,
        state: &str,
    ) -> Result<Option<StoredState>, AppError>;
}
fn store(ctx: &Ctx<'_>) -> Result<std::sync::Arc<dyn OAuthStates>, AppError> {
    ctx.services().get::<dyn OAuthStates>().ok_or_else(|| {
        AppError::unavailable_code(
            "oauth_states_unavailable",
            "OAuth state support is not enabled",
        )
    })
}
pub async fn create(
    ctx: &Ctx<'_>,
    key: &str,
    purpose: &str,
    user_id: Option<Uuid>,
    redirect_path: Option<&str>,
) -> Result<NewState, AppError> {
    store(ctx)?
        .create(ctx, key, purpose, user_id, redirect_path)
        .await
}
pub async fn consume(
    ctx: &Ctx<'_>,
    key: &str,
    state: &str,
) -> Result<Option<StoredState>, AppError> {
    store(ctx)?.consume(ctx, key, state).await
}
