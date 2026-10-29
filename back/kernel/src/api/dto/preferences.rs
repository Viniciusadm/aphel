use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::api::dto::common::double_option;
use crate::application::user_preferences::{
    NavigationPreferences, UpdateUserPreferencesCommand, UserPreferences,
};

#[derive(Debug, Clone, Default, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct NavigationPreferencesDto {
    #[serde(default)]
    #[schema(example = json!(["deals", "contacts", "companies"]))]
    pub order: Vec<String>,
    #[serde(default)]
    #[schema(example = json!(["products"]))]
    pub hidden: Vec<String>,
    #[serde(default)]
    #[schema(example = "/deals")]
    pub home: Option<String>,
}

impl From<NavigationPreferencesDto> for NavigationPreferences {
    fn from(dto: NavigationPreferencesDto) -> Self {
        Self {
            order: dto.order,
            hidden: dto.hidden,
            home: dto.home,
        }
    }
}

impl From<NavigationPreferences> for NavigationPreferencesDto {
    fn from(navigation: NavigationPreferences) -> Self {
        Self {
            order: navigation.order,
            hidden: navigation.hidden,
            home: navigation.home,
        }
    }
}

#[derive(Debug, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct UpdatePreferencesRequest {
    pub keyboard_shortcuts: Option<bool>,
    #[serde(default, deserialize_with = "double_option")]
    #[schema(value_type = Option<String>, example = "en")]
    pub locale: Option<Option<String>>,
    pub navigation: Option<NavigationPreferencesDto>,
}

impl From<UpdatePreferencesRequest> for UpdateUserPreferencesCommand {
    fn from(request: UpdatePreferencesRequest) -> Self {
        Self {
            keyboard_shortcuts: request.keyboard_shortcuts,
            locale: request.locale,
            navigation: request.navigation.map(Into::into),
        }
    }
}

#[derive(Debug, Serialize, ToSchema)]
pub struct PreferencesResponse {
    pub keyboard_shortcuts: bool,
    #[schema(example = "en")]
    pub locale: Option<String>,
    pub navigation: NavigationPreferencesDto,
}

impl From<UserPreferences> for PreferencesResponse {
    fn from(preferences: UserPreferences) -> Self {
        Self {
            keyboard_shortcuts: preferences.keyboard_shortcuts,
            locale: preferences.locale,
            navigation: preferences.navigation.into(),
        }
    }
}
