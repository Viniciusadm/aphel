use chrono::{DateTime, SecondsFormat, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::domain::{
    BusinessType, CompleteOnboardingCommand, Offering, OnboardingAnswers, OnboardingStatus,
    Recommendation, SaveAnswersCommand, TeamSize, TemplateSuggestion,
};
use crate::service::OnboardingView;

#[derive(Debug, Clone, Copy, Serialize, ToSchema)]
#[serde(rename_all = "snake_case")]
#[schema(as = OnboardingStatus)]
pub(crate) enum OnboardingStatusSchema {
    Pending,
    Completed,
    Skipped,
    Dismissed,
}

impl From<OnboardingStatus> for OnboardingStatusSchema {
    fn from(value: OnboardingStatus) -> Self {
        match value {
            OnboardingStatus::Pending => Self::Pending,
            OnboardingStatus::Completed => Self::Completed,
            OnboardingStatus::Skipped => Self::Skipped,
            OnboardingStatus::Dismissed => Self::Dismissed,
        }
    }
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, ToSchema)]
#[serde(rename_all = "snake_case")]
#[schema(as = OnboardingBusinessType)]
pub(crate) enum BusinessTypeSchema {
    RealEstate,
    Agency,
    Consulting,
    Software,
    B2bSales,
    Other,
}

impl From<BusinessType> for BusinessTypeSchema {
    fn from(value: BusinessType) -> Self {
        match value {
            BusinessType::RealEstate => Self::RealEstate,
            BusinessType::Agency => Self::Agency,
            BusinessType::Consulting => Self::Consulting,
            BusinessType::Software => Self::Software,
            BusinessType::B2bSales => Self::B2bSales,
            BusinessType::Other => Self::Other,
        }
    }
}

impl From<BusinessTypeSchema> for BusinessType {
    fn from(value: BusinessTypeSchema) -> Self {
        match value {
            BusinessTypeSchema::RealEstate => Self::RealEstate,
            BusinessTypeSchema::Agency => Self::Agency,
            BusinessTypeSchema::Consulting => Self::Consulting,
            BusinessTypeSchema::Software => Self::Software,
            BusinessTypeSchema::B2bSales => Self::B2bSales,
            BusinessTypeSchema::Other => Self::Other,
        }
    }
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, ToSchema)]
#[serde(rename_all = "snake_case")]
#[schema(as = OnboardingOffering)]
pub(crate) enum OfferingSchema {
    Product,
    Service,
    Recurring,
}

impl From<Offering> for OfferingSchema {
    fn from(value: Offering) -> Self {
        match value {
            Offering::Product => Self::Product,
            Offering::Service => Self::Service,
            Offering::Recurring => Self::Recurring,
        }
    }
}

impl From<OfferingSchema> for Offering {
    fn from(value: OfferingSchema) -> Self {
        match value {
            OfferingSchema::Product => Self::Product,
            OfferingSchema::Service => Self::Service,
            OfferingSchema::Recurring => Self::Recurring,
        }
    }
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, ToSchema)]
#[serde(rename_all = "snake_case")]
#[schema(as = OnboardingTeamSize)]
pub(crate) enum TeamSizeSchema {
    Solo,
    Small,
    Medium,
    Large,
}

impl From<TeamSize> for TeamSizeSchema {
    fn from(value: TeamSize) -> Self {
        match value {
            TeamSize::Solo => Self::Solo,
            TeamSize::Small => Self::Small,
            TeamSize::Medium => Self::Medium,
            TeamSize::Large => Self::Large,
        }
    }
}

impl From<TeamSizeSchema> for TeamSize {
    fn from(value: TeamSizeSchema) -> Self {
        match value {
            TeamSizeSchema::Solo => Self::Solo,
            TeamSizeSchema::Small => Self::Small,
            TeamSizeSchema::Medium => Self::Medium,
            TeamSizeSchema::Large => Self::Large,
        }
    }
}

#[derive(Debug, Serialize, ToSchema)]
pub(crate) struct OnboardingAnswersResponse {
    pub(crate) business_type: Option<BusinessTypeSchema>,
    pub(crate) offering: Vec<OfferingSchema>,
    pub(crate) team_size: Option<TeamSizeSchema>,
}

impl From<&OnboardingAnswers> for OnboardingAnswersResponse {
    fn from(answers: &OnboardingAnswers) -> Self {
        Self {
            business_type: answers.business_type.map(Into::into),
            offering: answers.offering.iter().copied().map(Into::into).collect(),
            team_size: answers.team_size.map(Into::into),
        }
    }
}

#[derive(Debug, Serialize, ToSchema)]
pub(crate) struct TemplateSuggestionResponse {
    #[schema(example = "agency")]
    pub(crate) key: String,
    #[schema(example = json!(["business_agency", "offering_service"]))]
    pub(crate) reasons: Vec<String>,
}

impl From<TemplateSuggestion> for TemplateSuggestionResponse {
    fn from(suggestion: TemplateSuggestion) -> Self {
        Self {
            key: suggestion.key.to_owned(),
            reasons: suggestion.reasons.into_iter().map(str::to_owned).collect(),
        }
    }
}

#[derive(Debug, Serialize, ToSchema)]
pub(crate) struct OnboardingRecommendationResponse {
    pub(crate) recommended: TemplateSuggestionResponse,
    pub(crate) alternatives: Vec<TemplateSuggestionResponse>,
}

impl From<Recommendation> for OnboardingRecommendationResponse {
    fn from(recommendation: Recommendation) -> Self {
        Self {
            recommended: recommendation.recommended.into(),
            alternatives: recommendation
                .alternatives
                .into_iter()
                .map(Into::into)
                .collect(),
        }
    }
}

#[derive(Debug, Serialize, ToSchema)]
pub(crate) struct OnboardingResponse {
    pub(crate) status: OnboardingStatusSchema,
    pub(crate) answers: OnboardingAnswersResponse,
    pub(crate) recommendation: Option<OnboardingRecommendationResponse>,
    pub(crate) template_key: Option<String>,
    pub(crate) install_id: Option<String>,
    pub(crate) completed_by: Option<String>,
    pub(crate) completed_at: Option<String>,
    pub(crate) skipped_by: Option<String>,
    pub(crate) skipped_at: Option<String>,
    pub(crate) updated_at: Option<String>,
}

fn timestamp(value: Option<DateTime<Utc>>) -> Option<String> {
    value.map(|instant| instant.to_rfc3339_opts(SecondsFormat::Secs, true))
}

impl From<OnboardingView> for OnboardingResponse {
    fn from(view: OnboardingView) -> Self {
        let state = view.state;
        Self {
            status: state.status.into(),
            answers: (&state.answers).into(),
            recommendation: view.recommendation.map(Into::into),
            template_key: state.template_key,
            install_id: state.install_id.map(|id| id.to_string()),
            completed_by: state.completed_by.map(|id| id.into_inner()),
            completed_at: timestamp(state.completed_at),
            skipped_by: state.skipped_by.map(|id| id.into_inner()),
            skipped_at: timestamp(state.skipped_at),
            updated_at: timestamp(state.updated_at),
        }
    }
}

#[derive(Debug, Default, Deserialize, ToSchema)]
pub(crate) struct SaveOnboardingAnswersRequest {
    pub(crate) business_type: Option<BusinessTypeSchema>,
    #[serde(default)]
    pub(crate) offering: Vec<OfferingSchema>,
    pub(crate) team_size: Option<TeamSizeSchema>,
}

impl From<SaveOnboardingAnswersRequest> for SaveAnswersCommand {
    fn from(request: SaveOnboardingAnswersRequest) -> Self {
        Self {
            business_type: request.business_type.map(Into::into),
            offering: request.offering.into_iter().map(Into::into).collect(),
            team_size: request.team_size.map(Into::into),
        }
    }
}

#[derive(Debug, Default, Deserialize, ToSchema)]
pub(crate) struct CompleteOnboardingRequest {
    #[schema(example = "agency")]
    pub(crate) template_key: Option<String>,
    pub(crate) install_id: Option<String>,
}

impl CompleteOnboardingRequest {
    pub(crate) fn into_command(self, ip: Option<String>) -> CompleteOnboardingCommand {
        CompleteOnboardingCommand {
            template_key: self.template_key,
            install_id: self.install_id,
            ip,
        }
    }
}
