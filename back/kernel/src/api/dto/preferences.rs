use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::api::dto::common::double_option;
use crate::application::user_preferences::{
    ExperiencePreferenceDiagnostic, ExperiencePreferences, NavigationPreferences,
    RecordDetailPreferences, UpdateExperiencePreferences, UpdateUserPreferencesCommand,
    UserPreferencesView,
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
pub struct UpdateExperiencePreferencesDto {
    #[serde(default, deserialize_with = "double_option")]
    #[schema(value_type = Option<String>, nullable = true)]
    pub profile: Option<Option<String>>,
    #[serde(default, deserialize_with = "double_option")]
    #[schema(value_type = Option<RecordDetailPreferences>, nullable = true)]
    pub record_detail: Option<Option<RecordDetailPreferences>>,
}

impl From<UpdateExperiencePreferencesDto> for UpdateExperiencePreferences {
    fn from(value: UpdateExperiencePreferencesDto) -> Self {
        Self {
            profile: value.profile,
            record_detail: value.record_detail,
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
    #[serde(default, deserialize_with = "double_option")]
    #[schema(value_type = Option<UpdateExperiencePreferencesDto>, nullable = true)]
    pub experience: Option<Option<UpdateExperiencePreferencesDto>>,
}

impl From<UpdatePreferencesRequest> for UpdateUserPreferencesCommand {
    fn from(request: UpdatePreferencesRequest) -> Self {
        Self {
            keyboard_shortcuts: request.keyboard_shortcuts,
            locale: request.locale,
            navigation: request.navigation.map(Into::into),
            experience: request.experience.map(|value| value.map(Into::into)),
        }
    }
}

#[derive(Debug, Serialize, ToSchema)]
pub struct PreferencesResponse {
    pub keyboard_shortcuts: bool,
    #[schema(example = "en")]
    pub locale: Option<String>,
    pub navigation: NavigationPreferencesDto,
    pub experience: ExperiencePreferences,
    pub stored_experience: ExperiencePreferences,
    pub stored_navigation: NavigationPreferencesDto,
    pub experience_diagnostics: Vec<ExperiencePreferenceDiagnostic>,
}

impl From<UserPreferencesView> for PreferencesResponse {
    fn from(view: UserPreferencesView) -> Self {
        let preferences = view.effective;
        Self {
            keyboard_shortcuts: preferences.keyboard_shortcuts,
            locale: preferences.locale,
            navigation: preferences.navigation.into(),
            experience: preferences.experience,
            stored_experience: view.stored_experience,
            stored_navigation: view.stored_navigation.into(),
            experience_diagnostics: view.experience_diagnostics,
        }
    }
}
