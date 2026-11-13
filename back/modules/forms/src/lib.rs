use aphel_contract_automation::AutomationBuilder;
pub(crate) mod appearance;
pub(crate) mod http;
pub(crate) mod model;
pub(crate) mod public_http;
pub(crate) mod render;
mod site_embed;
pub(crate) mod store;
pub(crate) mod submit;
pub(crate) mod triggers;
pub(crate) mod validate;

use axum::http::StatusCode;
use uuid::Uuid;

use aphel_kernel::Ctx;
use aphel_kernel::domain::UserId;
use aphel_kernel::errors::{AppError, CodedKind, FieldViolation};
use aphel_kernel::modules::{AppBuilder, Manifest, Migration, Module, SettingField, SettingKind};
use aphel_sdk::platform::application::access::{AccessRepository, build_access_scope};
use aphel_sdk::platform::application::public_events::{PublicEventDefinition, event};
use aphel_sdk::platform::domain::access::{AccessScope, Permission, PermissionDefinition};
use aphel_sdk::platform::infrastructure::repositories::mysql_access::MySqlAccessRepository;
use aphel_sdk::platform::modules::builder::APHEL_VERSION;

pub(crate) const KEY: &str = "forms";
pub(crate) const TAG: &str = "forms";
pub(crate) const MANAGE: Permission = Permission::from_static("forms.manage");
pub(crate) const CREATED: &str = "lead_form.created";
pub(crate) const UPDATED: &str = "lead_form.updated";
pub(crate) const DELETED: &str = "lead_form.deleted";
pub(crate) const SUBMITTED: &str = "lead_form.submitted";

const PUBLIC_EVENTS: &[PublicEventDefinition] = &[event(
    SUBMITTED,
    aphel_sdk::people::RESOURCE,
    "A lead form was submitted, creating or updating the contact",
)];
pub(crate) const FORM_NOT_FOUND: &str = "lead_form_not_found";
pub(crate) const PUBLIC_FORMS_PATH: &str = "/api/v1/public/forms/forms";

pub(crate) fn form_not_found() -> AppError {
    AppError::not_found_code(FORM_NOT_FOUND, "lead form not found")
}

pub(crate) fn user_uuid(scope: &AccessScope) -> Result<Uuid, AppError> {
    Uuid::parse_str(scope.user_id().as_str()).map_err(|error| AppError::Internal(error.to_string()))
}

pub(crate) fn error_code(error: &AppError) -> &'static str {
    match error {
        AppError::Coded { code, .. }
        | AppError::CodedFields { code, .. }
        | AppError::CodedDetails { code, .. } => code,
        AppError::Configuration(_) => "configuration_error",
        AppError::Conflict(_) => "conflict",
        AppError::Forbidden(_) => "forbidden",
        AppError::NotFound(_) => "not_found",
        AppError::RateLimited(_) => "rate_limited",
        AppError::Unauthorized(_) => "unauthorized",
        AppError::Validation(_) | AppError::InvalidFields { .. } => "validation_error",
        AppError::Persistence(_) | AppError::Internal(_) => "internal_error",
    }
}

pub(crate) fn error_fields(error: &AppError) -> &[FieldViolation] {
    match error {
        AppError::InvalidFields { fields, .. } | AppError::CodedFields { fields, .. } => fields,
        _ => &[],
    }
}

pub(crate) fn error_status(error: &AppError) -> StatusCode {
    match error {
        AppError::Coded { kind, .. } | AppError::CodedDetails { kind, .. } => match kind {
            CodedKind::Conflict => StatusCode::CONFLICT,
            CodedKind::Forbidden => StatusCode::FORBIDDEN,
            CodedKind::NotFound => StatusCode::NOT_FOUND,
            CodedKind::Unauthorized => StatusCode::UNAUTHORIZED,
            CodedKind::Validation => StatusCode::UNPROCESSABLE_ENTITY,
            CodedKind::TooManyRequests => StatusCode::TOO_MANY_REQUESTS,
            CodedKind::Unavailable => StatusCode::SERVICE_UNAVAILABLE,
            CodedKind::PayloadTooLarge => StatusCode::PAYLOAD_TOO_LARGE,
        },
        AppError::Configuration(_) => StatusCode::SERVICE_UNAVAILABLE,
        AppError::Conflict(_) => StatusCode::CONFLICT,
        AppError::Forbidden(_) => StatusCode::FORBIDDEN,
        AppError::NotFound(_) => StatusCode::NOT_FOUND,
        AppError::RateLimited(_) => StatusCode::TOO_MANY_REQUESTS,
        AppError::Unauthorized(_) => StatusCode::UNAUTHORIZED,
        AppError::Validation(_) | AppError::InvalidFields { .. } | AppError::CodedFields { .. } => {
            StatusCode::UNPROCESSABLE_ENTITY
        }
        AppError::Persistence(_) | AppError::Internal(_) => StatusCode::INTERNAL_SERVER_ERROR,
    }
}

pub(crate) async fn scope_for_user(
    ctx: &Ctx<'_>,
    user_id: Uuid,
) -> Result<Option<AccessScope>, AppError> {
    let user = UserId::new(user_id.to_string())?;
    let Some(access) = MySqlAccessRepository::new(ctx.pool().clone())
        .load_access(&user)
        .await?
    else {
        return Ok(None);
    };
    let scope = build_access_scope(user, access);
    Ok(scope.module_enabled(KEY).then_some(scope))
}

pub struct FormsModule;

impl Module for FormsModule {
    fn manifest(&self) -> Manifest {
        Manifest::new(KEY, "Lead forms", APHEL_VERSION)
            .description("Public forms that create people and deals")
            .depends_on("people", "*")
            .optional("leads")
            .optional("deals")
            .optional("tasks")
            .optional("assignment")
            .setting(appearance::choice_setting(
                appearance::SETTING_THEME,
                "Tema padrão",
                "Visual dos formulários que não escolhem um tema próprio",
                &[
                    ("clean", "Limpo"),
                    ("soft", "Suave"),
                    ("dark", "Escuro"),
                    ("outline", "Contorno"),
                ],
                "clean",
            ))
            .setting(appearance::choice_setting(
                appearance::SETTING_RADIUS,
                "Arredondamento padrão",
                "Cantos dos campos e do botão",
                &[
                    ("sharp", "Reto"),
                    ("rounded", "Arredondado"),
                    ("pill", "Pílula"),
                ],
                "rounded",
            ))
            .setting(appearance::choice_setting(
                appearance::SETTING_FONT,
                "Fonte padrão",
                "Família de letra usada nos formulários",
                &[
                    ("system", "Do sistema"),
                    ("serif", "Serifada"),
                    ("rounded", "Arredondada"),
                ],
                "system",
            ))
            .setting(appearance::choice_setting(
                appearance::SETTING_WIDTH,
                "Largura padrão",
                "Largura máxima do formulário",
                &[
                    ("narrow", "Estreita"),
                    ("medium", "Média"),
                    ("wide", "Larga"),
                ],
                "medium",
            ))
            .setting(
                SettingField::new(
                    appearance::SETTING_LOGO,
                    "Mostrar o logo da organização",
                    SettingKind::Boolean,
                )
                .default_value(serde_json::json!(true))
                .help("O logo aparece no topo da página do formulário quando a organização tem um"),
            )
            .migration(Migration::new(
                1,
                "create lead forms and submissions",
                include_str!("../migrations/0001_forms.sql"),
            ))
    }

    fn register(&self, app: &mut AppBuilder) -> Result<(), AppError> {
        app.openapi_tag(TAG, "Public forms that create people and deals")
            .routes(|_| http::router())
            .public_routes(|_| public_http::router())
            .trigger(triggers::submitted_trigger());
        app.permission(PermissionDefinition::administrative(MANAGE));
        app.contribute::<dyn aphel_contract_site::SiteEmbedSource>(std::sync::Arc::new(
            site_embed::FormEmbeds,
        ));
        for definition in PUBLIC_EVENTS {
            app.public_event(*definition);
        }
        Ok(())
    }
}
