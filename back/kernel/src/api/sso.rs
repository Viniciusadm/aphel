use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::errors::AppError;
use crate::state::AppState;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LoginOption {
    pub provider: String,
    pub label: String,
    pub url: String,
    pub required: bool,
}

#[async_trait]
pub trait LoginProvider: Send + Sync {
    fn key(&self) -> &'static str;

    async fn discover(&self, state: &AppState, email: &str) -> Result<Vec<LoginOption>, AppError>;
}

pub async fn login_options(state: &AppState, email: &str) -> Vec<LoginOption> {
    let email = email.trim().to_lowercase();
    if email.is_empty() {
        return Vec::new();
    }
    let mut options = Vec::new();
    for provider in state.app.contributions::<dyn LoginProvider>() {
        match provider.discover(state, &email).await {
            Ok(found) => options.extend(found),
            Err(error) => tracing::warn!("login provider {} failed: {error}", provider.key()),
        }
    }
    options
}
