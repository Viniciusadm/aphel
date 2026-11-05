use aphel_sdk::maintenance::TemplateOnboarding;
use aphel_sdk::{AppError, Ctx};
use async_trait::async_trait;

use crate::domain::CompleteOnboardingCommand;
use crate::service::{ONBOARDING_NOT_PENDING, complete};

pub(crate) struct TemplateCompletion;

#[async_trait]
impl TemplateOnboarding for TemplateCompletion {
    async fn complete_with_template(
        &self,
        ctx: &Ctx<'_>,
        template_key: &str,
        install_id: &str,
    ) -> Result<(), AppError> {
        match complete(
            ctx,
            CompleteOnboardingCommand {
                template_key: Some(template_key.to_owned()),
                install_id: Some(install_id.to_owned()),
                ip: None,
            },
        )
        .await
        {
            Ok(_)
            | Err(AppError::Coded {
                code: ONBOARDING_NOT_PENDING,
                ..
            }) => Ok(()),
            Err(error) => Err(error),
        }
    }
}
