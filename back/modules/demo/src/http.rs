use axum::Json;
use serde::Serialize;
use utoipa::ToSchema;
use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;

use aphel_kernel::api::dto::common::Data;
use aphel_kernel::api::state::AppState;
use aphel_kernel::infrastructure::demo::demo_settings;

pub(crate) const TAG: &str = "demo";

pub(crate) fn router() -> OpenApiRouter<AppState> {
    OpenApiRouter::new().routes(routes!(demo_status))
}

#[derive(Debug, Serialize, ToSchema)]
pub(crate) struct DemoVisitorResponse {
    #[schema(example = "demo@aphel.demo")]
    pub(crate) email: String,
    #[schema(example = "aphel-demo")]
    pub(crate) password: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub(crate) struct DemoStatusResponse {
    pub(crate) enabled: bool,
    pub(crate) visitor: Option<DemoVisitorResponse>,
    #[schema(example = "03:00")]
    pub(crate) reset_time_utc: Option<String>,
}

#[utoipa::path(
    get,
    path = "/demo",
    tag = TAG,
    operation_id = "demo_status",
    description = "Public. Tells whether this installation runs in demonstration mode (`DEMO_MODE=true`) and, when it does, the public credentials of the limited demo visitor and the daily reset time. Outside demo mode only `enabled: false` is returned.",
    responses(
        (status = 200, description = "Demonstration mode status", body = Data<DemoStatusResponse>),
    )
)]
async fn demo_status() -> Json<Data<DemoStatusResponse>> {
    let settings = demo_settings();
    Json(Data::new(if settings.enabled {
        DemoStatusResponse {
            enabled: true,
            visitor: Some(DemoVisitorResponse {
                email: settings.visitor_email.clone(),
                password: settings.visitor_password.clone(),
            }),
            reset_time_utc: Some(settings.reset_time_utc.clone()),
        }
    } else {
        DemoStatusResponse {
            enabled: false,
            visitor: None,
            reset_time_utc: None,
        }
    }))
}
