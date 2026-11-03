use std::collections::HashMap;
use std::str::FromStr;
use std::sync::{Arc, Mutex, OnceLock};
use std::time::{Duration, Instant};

use chrono::{NaiveDate, Utc};
use chrono_tz::Tz;
use serde_json::Value;
use uuid::Uuid;

use crate::config::{self, AiSettings, Feature};
use crate::provider::{AiProvider, AiRequest, AiResponse};
use crate::store::{self, NewCall};
use aphel_kernel::api::rate_limit::{SlidingWindowLimiter, shared_hit};
use aphel_kernel::context::Ctx;
use aphel_kernel::domain::access::AccessScope;
use aphel_kernel::errors::AppError;
use aphel_kernel::infrastructure::repositories::mysql_organization_profile::find_organization;

pub(crate) const LIMIT_REACHED: &str = "ai_limit_reached";
pub(crate) const USER_LIMIT_REACHED: &str = "ai_user_limit_reached";

pub(crate) struct OrganizationInfo {
    pub(crate) timezone: Tz,
    pub(crate) locale: String,
}

pub(crate) async fn organization(ctx: &Ctx<'_>) -> Result<OrganizationInfo, AppError> {
    let organization = find_organization(ctx.pool()).await?.ok_or_else(|| {
        AppError::not_found_code("organization_not_found", "organization not found")
    })?;
    Ok(OrganizationInfo {
        timezone: Tz::from_str(&organization.timezone).unwrap_or(chrono_tz::America::Sao_Paulo),
        locale: organization.locale,
    })
}

pub(crate) fn actor_id(scope: &AccessScope) -> Uuid {
    if scope.is_system() {
        return Uuid::nil();
    }
    Uuid::parse_str(scope.user_id().as_str()).unwrap_or_else(|_| Uuid::nil())
}

async fn allow(key: &str, limit: u32, window: Duration) -> Result<bool, AppError> {
    static LIMITERS: OnceLock<Mutex<HashMap<String, Arc<SlidingWindowLimiter>>>> = OnceLock::new();
    let limiter = {
        let mut limiters = LIMITERS
            .get_or_init(|| Mutex::new(HashMap::new()))
            .lock()
            .map_err(|_| AppError::Internal("rate limiter lock poisoned".to_owned()))?;
        limiters
            .entry(format!("{limit}:{}", window.as_secs()))
            .or_insert_with(|| Arc::new(SlidingWindowLimiter::new(limit, window)))
            .clone()
    };
    Ok(shared_hit("ai", key, limit, window, &limiter)
        .await?
        .allowed)
}

pub(crate) fn error_code(error: &AppError) -> &'static str {
    match error {
        AppError::Coded { code, .. }
        | AppError::CodedDetails { code, .. }
        | AppError::CodedFields { code, .. } => code,
        AppError::Configuration(_) => "configuration_error",
        AppError::Validation(_) | AppError::InvalidFields { .. } => "validation_error",
        AppError::RateLimited(_) => "rate_limited",
        _ => "internal_error",
    }
}

pub(crate) struct AiSession {
    pub(crate) settings: AiSettings,
    pub(crate) provider: Arc<dyn AiProvider>,
    pub(crate) organization: OrganizationInfo,
}

impl AiSession {
    pub(crate) async fn open(ctx: &Ctx<'_>, feature: Feature) -> Result<Self, AppError> {
        let settings = AiSettings::load(ctx).await?;
        settings.require_feature(feature)?;
        let provider = config::provider(ctx, &settings).await?;
        let organization = organization(ctx).await?;
        Ok(Self {
            settings,
            provider,
            organization,
        })
    }

    pub(crate) fn today(&self) -> NaiveDate {
        Utc::now()
            .with_timezone(&self.organization.timezone)
            .date_naive()
    }

    pub(crate) fn language(&self) -> &'static str {
        language_name(&self.organization.locale)
    }

    pub(crate) fn provider_name(&self) -> String {
        self.provider.name().to_owned()
    }

    pub(crate) fn model(&self) -> String {
        self.provider.model().to_owned()
    }

    pub(crate) async fn complete(
        &self,
        ctx: &Ctx<'_>,
        feature: Feature,
        request: AiRequest,
    ) -> Result<AiResponse, AppError> {
        let user_id = actor_id(ctx.scope);
        if !allow(
            &format!("user:{user_id}"),
            self.settings.user_hourly_limit,
            Duration::from_secs(3600),
        )
        .await?
        {
            return Err(AppError::too_many_requests_code(
                USER_LIMIT_REACHED,
                "you reached your hourly limit of AI requests, try again later",
            ));
        }
        if !store::reserve_call(ctx.pool(), self.today(), self.settings.daily_limit).await? {
            return Err(AppError::too_many_requests_code(
                LIMIT_REACHED,
                "the organization reached its daily limit of AI requests",
            ));
        }
        let started = Instant::now();
        let result = self.provider.complete(request).await;
        let duration_ms = u32::try_from(started.elapsed().as_millis()).unwrap_or(u32::MAX);
        let (status, input_tokens, output_tokens, error_code) = match &result {
            Ok(response) => (
                "success",
                response.input_tokens,
                response.output_tokens,
                None,
            ),
            Err(error) => ("error", None, None, Some(error_code(error))),
        };
        store::record_call(
            ctx.pool(),
            NewCall {
                user_id: Some(user_id),
                feature: feature.as_str(),
                provider: self.provider.name(),
                model: self.provider.model(),
                status,
                input_tokens,
                output_tokens,
                duration_ms,
                error_code,
            },
        )
        .await?;
        result
    }
}

pub(crate) fn language_name(locale: &str) -> &'static str {
    let locale = locale.to_ascii_lowercase();
    if locale.starts_with("pt") {
        "Brazilian Portuguese (pt-BR)"
    } else if locale.starts_with("es") {
        "Spanish"
    } else {
        "English"
    }
}

pub(crate) fn invalid_response(message: impl Into<String>) -> AppError {
    AppError::unavailable_code("ai_invalid_response", message)
}

pub(crate) fn json_object(text: &str) -> Option<Value> {
    let start = text.find('{')?;
    let end = text.rfind('}')?;
    if end < start {
        return None;
    }
    serde_json::from_str(&text[start..=end]).ok()
}
