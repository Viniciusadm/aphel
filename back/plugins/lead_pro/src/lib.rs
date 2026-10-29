pub(crate) mod converter;
pub(crate) mod domain;
pub(crate) mod hooks;
pub(crate) mod http;
pub(crate) mod store;
pub(crate) mod touches;

use std::sync::Arc;

use serde_json::json;

use leads::{LeadConverter, LeadConverting, LeadQualifying};
use aphel_kernel::application::fields::SystemField;
use aphel_kernel::application::public_events::{PublicEventDefinition, event};
use aphel_kernel::domain::access::PermissionDefinition;
use aphel_kernel::domain::fields::FieldType;
use aphel_kernel::domain::notifications::NotificationKindDefinition;
use aphel_kernel::errors::AppError;
use aphel_kernel::modules::{
    AppBuilder, Manifest, Migration, Module, SettingField, SettingKind, WorkerParts,
};
use aphel_kernel::records::{RecordSaving, RecordValidating};

use crate::domain::{
    COOLDOWN_DAYS_SETTING, COOLDOWN_POINTS_SETTING, DEFAULT_COOLDOWN_DAYS, DEFAULT_COOLDOWN_POINTS,
    DEFAULT_HOT_THRESHOLD, DEFAULT_INITIAL_TEMPERATURE, DEFAULT_TOUCH_POINTS,
    HOT_THRESHOLD_SETTING, INITIAL_TEMPERATURE_SETTING, KEY, LEAD_HOT, MAX_TEMPERATURE, READ,
    TEMPERATURE_CHANGED, TEMPERATURE_FIELD, TOUCH_POINTS_SETTING,
};
use crate::touches::{Cooldown, TouchConsumer, cooldown_schedule};

pub(crate) const VERSION: &str = env!("CARGO_PKG_VERSION");

const SAVING_PRIORITY: i32 = 20;
const VALIDATING_PRIORITY: i32 = 15;
const QUALIFYING_PRIORITY: i32 = 10;
const CONVERTING_PRIORITY: i32 = 10;

const PUBLIC_EVENTS: &[PublicEventDefinition] = &[event(
    TEMPERATURE_CHANGED,
    contacts::CONTACT_RESOURCE,
    "The lead temperature of the contact changed after a touch (activity, note, e-mail, meeting or manual)",
)];

fn temperature_setting(key: &'static str, label: &'static str, default: i64) -> SettingField {
    SettingField::new(
        key,
        label,
        SettingKind::Number {
            min: Some(0),
            max: Some(MAX_TEMPERATURE),
        },
    )
    .default_value(json!(default))
}

pub struct LeadPro;

impl Module for LeadPro {
    fn manifest(&self) -> Manifest {
        Manifest::new(KEY, "Lead Pro", VERSION)
            .description(
                "Lead temperature: interactions warm up the contact, qualification requires a hot lead and conversion carries the temperature over to the deal",
            )
            .requires_aphel("0.1.0")
            .depends_on(contacts::KEY, "*")
            .depends_on(leads::KEY, "*")
            .depends_on(deals::KEY, "*")
            .enabled_by_default()
            .migration(Migration::new(
                1,
                "lead temperature and touches",
                include_str!("../migrations/0001_lead_pro.sql"),
            ))
            .uninstall_sql(include_str!("../migrations/uninstall.sql"))
            .setting(
                temperature_setting(
                    HOT_THRESHOLD_SETTING,
                    "Temperatura de lead quente",
                    DEFAULT_HOT_THRESHOLD,
                )
                .help("A qualificação de um lead exige pelo menos esta temperatura"),
            )
            .setting(temperature_setting(
                INITIAL_TEMPERATURE_SETTING,
                "Temperatura inicial",
                DEFAULT_INITIAL_TEMPERATURE,
            ))
            .setting(temperature_setting(
                TOUCH_POINTS_SETTING,
                "Pontos por interação",
                DEFAULT_TOUCH_POINTS,
            ))
            .setting(temperature_setting(
                COOLDOWN_POINTS_SETTING,
                "Pontos perdidos por dia sem interação",
                DEFAULT_COOLDOWN_POINTS,
            ))
            .setting(
                temperature_setting(
                    COOLDOWN_DAYS_SETTING,
                    "Dias sem interação até esfriar",
                    DEFAULT_COOLDOWN_DAYS,
                )
                .help("O resfriamento roda todo dia às 3h para os leads sem interação neste período"),
            )
    }

    fn register(&self, app: &mut AppBuilder) -> Result<(), AppError> {
        app.permission(PermissionDefinition::administrative(READ))
            .grant_to(["admin", "manager", "member"]);
        let field = SystemField::new(
            TEMPERATURE_FIELD,
            "Temperatura do lead",
            FieldType::Number,
            TEMPERATURE_FIELD,
        )?;
        app.extend_record_type("contact", |contact| {
            contact.system_field(field);
        });
        app.filter::<RecordSaving, _>(SAVING_PRIORITY, |ctx, saving| {
            Box::pin(hooks::default_temperature(ctx, saving))
        })
        .on::<RecordValidating, _>(VALIDATING_PRIORITY, |_, changed| {
            Box::pin(hooks::validate_temperature(changed))
        })
        .on::<LeadQualifying, _>(QUALIFYING_PRIORITY, |ctx, lead| {
            Box::pin(hooks::require_temperature(ctx, lead))
        })
        .filter::<LeadConverting, _>(CONVERTING_PRIORITY, |ctx, conversion| {
            Box::pin(hooks::enrich_deal_values(ctx, conversion))
        });
        app.decorate::<dyn LeadConverter, _>(|inner| Arc::new(converter::WithTouches(inner)))?;
        app.worker(|env| {
            WorkerParts::default()
                .consumer(Arc::new(TouchConsumer::new(
                    env.pool.clone(),
                    env.record_types.clone(),
                )))
                .handler(Arc::new(Cooldown::new(env.pool.clone())))
                .schedule(cooldown_schedule())
        });
        for definition in PUBLIC_EVENTS {
            app.public_event(*definition);
        }
        app.notification_kind(NotificationKindDefinition::new(LEAD_HOT))
            .openapi_tag(
                http::TAG,
                "Lead temperature (Lead Pro plugin): hot leads board, temperature and touches of a contact",
            )
            .routes(|_| http::router());
        Ok(())
    }
}
