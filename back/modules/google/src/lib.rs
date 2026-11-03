pub(crate) mod http;
pub(crate) mod mail;
pub(crate) mod provider;

use std::sync::Arc;

use async_trait::async_trait;
use serde_json::Value;
use utoipa_axum::router::OpenApiRouter;

use aphel_kernel::api::sso::LoginProvider;
use aphel_kernel::domain::access::{Permission, PermissionDefinition};
use aphel_kernel::errors::{AppError, FieldViolation};
use aphel_kernel::modules::{AppBuilder, Manifest, Migration, Module};
use aphel_sdk::calendar::CalendarAvailability;
use email::MailboxProvider;
use sso::IdentityLogin;
use sso::sso::settings::{login_setting_fields, validate_login_settings};
use tasks::remote_calendar::availability::CalendarBusy;
use tasks::remote_calendar::jobs;
use tasks::remote_calendar::model::calendar_setting_fields;

pub(crate) use mail::GmailProvider;
pub(crate) use provider::GoogleProvider;

pub(crate) const KEY: &str = "google";
pub(crate) const CALENDAR_SYNC: &str = "google.calendar_sync";
pub(crate) const CALENDAR_CONNECTED: &str = "google.calendar_connected";
pub(crate) const CALENDAR_DISCONNECTED: &str = "google.calendar_disconnected";

pub struct GoogleModule;

#[async_trait]
impl Module for GoogleModule {
    fn manifest(&self) -> Manifest {
        let mut manifest = Manifest::new(KEY, "Google", env!("CARGO_PKG_VERSION"))
            .description(
                "Sign-in with Google, meeting sync with Google Calendar and Gmail mailboxes",
            )
            .requires_aphel("0.1.0")
            .depends_on(sso::KEY, "*");
        for field in login_setting_fields()
            .into_iter()
            .chain(calendar_setting_fields())
        {
            manifest = manifest.setting(field);
        }
        manifest.migration(Migration::new(
            1,
            "create oauth states and calendar sync tables",
            include_str!("../migrations/0001_sso_calendar.sql"),
        ))
    }

    fn register(&self, app: &mut AppBuilder) -> Result<(), AppError> {
        app.permission(PermissionDefinition::record(Permission::from_static(
            CALENDAR_SYNC,
        )));
        app.openapi_tag(
            http::TAG,
            "Login with Google, Google Calendar meeting sync and Gmail mailboxes",
        )
        .routes(|_| http::router())
        .public_routes(|_| OpenApiRouter::new().merge(http::public_router()))
        .contribute::<dyn LoginProvider>(Arc::new(IdentityLogin(GoogleProvider)))
        .contribute::<dyn MailboxProvider>(Arc::new(GmailProvider))
        .contribute::<dyn CalendarAvailability>(Arc::new(CalendarBusy::new(Arc::new(
            GoogleProvider,
        ))))
        .schedule(jobs::schedule(KEY)?)
        .worker(|env| jobs::worker(Arc::new(GoogleProvider), env));
        Ok(())
    }

    fn validate_settings(&self, settings: &Value) -> Vec<FieldViolation> {
        validate_login_settings(settings)
    }
}
