use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};

use crate::application::directory::DirectoryUser;

#[derive(Debug, Deserialize, IntoParams)]
#[into_params(parameter_in = Query)]
pub struct DirectoryParams {
    pub search: Option<String>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct DirectoryUserResponse {
    pub id: String,
    pub display_name: String,
    pub email: String,
}

impl From<DirectoryUser> for DirectoryUserResponse {
    fn from(user: DirectoryUser) -> Self {
        Self {
            id: user.id.into_inner(),
            display_name: user.display_name,
            email: user.email,
        }
    }
}
