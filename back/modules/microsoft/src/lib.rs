pub(crate) mod http;
pub(crate) mod mail;
pub(crate) mod provider;

use std::sync::Arc;

use async_trait::async_trait;
use serde_json::{Value, json};
use utoipa_axum::router::OpenApiRouter;

use aphel_kernel::api::sso::LoginProvider;
use aphel_kernel::domain::access::{Permission, PermissionDefinition};
use aphel_kernel::errors::{AppError, FieldViolation};
use aphel_kernel::modules::{AppBuilder, Manifest, Module, SettingField, SettingKind};
use aphel_sdk::calendar::CalendarAvailability;
use aphel_sdk::email::mailbox::MailboxProvider;
use aphel_sdk::sso::IdentityLogin;
use aphel_sdk::sso::settings::{self, login_setting_fields, validate_login_settings};
use aphel_sdk::tasks::remote_calendar::CalendarBusy;
use aphel_sdk::tasks::remote_calendar::calendar_setting_fields;

pub(crate) use mail::OutlookProvider;
pub(crate) use provider::MicrosoftProvider;

pub(crate) const KEY: &str = "microsoft";
pub(crate) const TENANT: &str = "tenant";
pub(crate) const DEFAULT_TENANT: &str = "organizations";
pub(crate) const CALENDAR_SYNC: &str = "microsoft.calendar_sync";
pub(crate) const CALENDAR_CONNECTED: &str = "microsoft.calendar_connected";
pub(crate) const CALENDAR_DISCONNECTED: &str = "microsoft.calendar_disconnected";

pub struct MicrosoftModule;

#[async_trait]
impl Module for MicrosoftModule {
    fn manifest(&self) -> Manifest {
        let mut manifest = Manifest::new(KEY, "Microsoft", env!("CARGO_PKG_VERSION"))
            .description("Sign-in with a Microsoft account (Entra ID), meeting sync with Outlook and Outlook mailboxes")
            .requires_aphel("0.1.0")
            .depends_on(aphel_sdk::sso::KEY, "*")
            .setting(
                SettingField::new(TENANT, "Tenant (locatário)", SettingKind::Text)
                    .default_value(json!(DEFAULT_TENANT))
                    .help("ID do diretório (GUID) para aceitar só a sua empresa, ou organizations para qualquer conta corporativa"),
            );
        for field in login_setting_fields()
            .into_iter()
            .chain(calendar_setting_fields())
        {
            manifest = manifest.setting(field);
        }
        manifest
    }

    fn register(&self, app: &mut AppBuilder) -> Result<(), AppError> {
        app.contribute::<dyn aphel_sdk::tasks::remote_calendar::CalendarApi>(Arc::new(
            MicrosoftProvider,
        ));

        app.permission(PermissionDefinition::record(Permission::from_static(
            CALENDAR_SYNC,
        )));
        app.openapi_tag(
            http::TAG,
            "Login with a Microsoft account (Entra ID), Outlook calendar meeting sync and Outlook mailboxes",
        )
        .routes(|_| http::router())
        .public_routes(|_| OpenApiRouter::new().merge(http::public_router()))
        .contribute::<dyn LoginProvider>(Arc::new(IdentityLogin(MicrosoftProvider)))
        .contribute::<dyn MailboxProvider>(Arc::new(OutlookProvider))
        .contribute::<dyn CalendarAvailability>(Arc::new(CalendarBusy::new(Arc::new(
            MicrosoftProvider,
        ))));
        Ok(())
    }

    fn validate_settings(&self, settings: &Value) -> Vec<FieldViolation> {
        let mut violations = validate_login_settings(settings);
        if let Some(tenant) = settings::text(settings, TENANT)
            && !provider::is_valid_tenant(&tenant)
        {
            violations.push(FieldViolation::new(
                TENANT,
                "use o ID do diretório, um domínio verificado, organizations ou common",
            ));
        }
        violations
    }
}
