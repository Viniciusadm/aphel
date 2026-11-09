use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use std::collections::BTreeMap;

use crate::api::dto::common::double_option;
use crate::application::user_preferences::{
    NavigationPreferences, UpdateUserPreferencesCommand, UserPreferencesView,
};

#[derive(Debug, Clone, Default, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct NavigationPreferencesDto {
    #[serde(default)]
    #[schema(example = json!(["deals", "people", "organization_profile"]))]
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
    #[schema(example = json!({ "pos.pay": ["f10"], "core.create": [] }))]
    pub keybindings: Option<BTreeMap<String, Vec<String>>>,
    #[serde(default, deserialize_with = "double_option")]
    #[schema(value_type = Option<String>, example = "en")]
    pub locale: Option<Option<String>>,
    pub navigation: Option<NavigationPreferencesDto>,
    #[serde(default, deserialize_with = "double_option")]
    #[schema(value_type = Option<String>, nullable = true, example = "midnight")]
    pub ui_plugin: Option<Option<String>>,
}

impl From<UpdatePreferencesRequest> for UpdateUserPreferencesCommand {
    fn from(request: UpdatePreferencesRequest) -> Self {
        Self {
            keyboard_shortcuts: request.keyboard_shortcuts,
            keybindings: request.keybindings,
            locale: request.locale,
            navigation: request.navigation.map(Into::into),
            ui_plugin: request.ui_plugin,
        }
    }
}

#[derive(Debug, Serialize, ToSchema)]
pub struct PreferencesResponse {
    pub keyboard_shortcuts: bool,
    #[schema(example = json!({ "pos.pay": ["f10"] }))]
    pub keybindings: BTreeMap<String, Vec<String>>,
    #[schema(example = json!({ "pos.pay": ["f9"] }))]
    pub keybindings_default: BTreeMap<String, Vec<String>>,
    #[schema(example = "en")]
    pub locale: Option<String>,
    pub navigation: NavigationPreferencesDto,
    pub ui_plugin: Option<String>,
    pub ui_plugin_default: Option<String>,
    pub effective_ui_plugin: Option<String>,
}

impl From<UserPreferencesView> for PreferencesResponse {
    fn from(view: UserPreferencesView) -> Self {
        let preferences = view.preferences;
        Self {
            keyboard_shortcuts: preferences.keyboard_shortcuts,
            keybindings: preferences.keybindings,
            keybindings_default: view.keybindings_default,
            locale: preferences.locale,
            navigation: preferences.navigation.into(),
            ui_plugin: preferences.ui_plugin,
            ui_plugin_default: view.ui_plugin_default,
            effective_ui_plugin: view.effective_ui_plugin,
        }
    }
}
